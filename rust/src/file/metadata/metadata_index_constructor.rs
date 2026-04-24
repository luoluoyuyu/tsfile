// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Metadata index construction utilities, mirroring Java's MetadataIndexConstructor.

use std::collections::{BTreeMap, VecDeque};
use std::io::Write;
use crate::common::enums::MetadataIndexNodeType;
use crate::error::TsFileResult;
use crate::file::metadata::device_id::DeviceId;
use crate::file::metadata::metadata_index_node::{MetadataIndexEntry, MetadataIndexNode};
use crate::file::metadata::timeseries_metadata::TimeseriesMetadata;

#[derive(Debug, Clone)]
pub struct MetadataIndexConstructor {
    max_degree_of_index_node: usize,
}

#[derive(Debug, Clone)]
pub struct SerializedMetadataIndex {
    pub root: MetadataIndexNode,
    pub payload: Vec<u8>,
}

impl SerializedMetadataIndex {
    pub fn into_absolute_offsets(mut self, base_offset: i64) -> Self {
        add_base_offset_to_node(&mut self.root, base_offset);
        self
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        writer.write_all(&self.payload)?;
        Ok(self.payload.len())
    }
}

impl MetadataIndexConstructor {
    pub fn new(max_degree_of_index_node: usize) -> Self {
        MetadataIndexConstructor {
            max_degree_of_index_node: max_degree_of_index_node.max(1),
        }
    }

    pub fn max_degree_of_index_node(&self) -> usize {
        self.max_degree_of_index_node
    }

    pub fn construct_metadata_index(
        &self,
        timeseries_metadata_map: BTreeMap<DeviceId, Vec<TimeseriesMetadata>>,
    ) -> TsFileResult<SerializedMetadataIndex> {
        let mut payload = Vec::new();
        let mut device_metadata_index_map = BTreeMap::new();

        for (device_id, mut timeseries_metadata_list) in timeseries_metadata_map {
            timeseries_metadata_list.sort_by(|left, right| left.measurement_id.cmp(&right.measurement_id));
            let measurement_root = self.build_measurement_index(&mut payload, &timeseries_metadata_list)?;
            device_metadata_index_map.insert(device_id, measurement_root);
        }

        let root = self.check_and_build_level_index(&mut payload, device_metadata_index_map)?;
        Ok(SerializedMetadataIndex { root, payload })
    }

    pub fn construct_metadata_index_at(
        &self,
        timeseries_metadata_map: BTreeMap<DeviceId, Vec<TimeseriesMetadata>>,
        base_offset: i64,
    ) -> TsFileResult<SerializedMetadataIndex> {
        Ok(self
            .construct_metadata_index(timeseries_metadata_map)?
            .into_absolute_offsets(base_offset))
    }

    pub fn construct_and_write_metadata_index<W: Write>(
        &self,
        timeseries_metadata_map: BTreeMap<DeviceId, Vec<TimeseriesMetadata>>,
        base_offset: i64,
        writer: &mut W,
    ) -> TsFileResult<MetadataIndexNode> {
        let serialized = self.construct_metadata_index_at(timeseries_metadata_map, base_offset)?;
        serialized.write_to(writer)?;
        Ok(serialized.root)
    }

    pub fn split_device_by_table(
        device_metadata_index_map: BTreeMap<DeviceId, MetadataIndexNode>,
    ) -> BTreeMap<String, BTreeMap<DeviceId, MetadataIndexNode>> {
        let mut result: BTreeMap<String, BTreeMap<DeviceId, MetadataIndexNode>> = BTreeMap::new();
        for (device_id, node) in device_metadata_index_map {
            result
                .entry(device_id.table_name())
                .or_default()
                .insert(device_id, node);
        }
        result
    }

    fn build_measurement_index(
        &self,
        payload: &mut Vec<u8>,
        timeseries_metadata_list: &[TimeseriesMetadata],
    ) -> TsFileResult<MetadataIndexNode> {
        let mut queue = VecDeque::new();
        let mut current = MetadataIndexNode::new(MetadataIndexNodeType::LeafMeasurement, 0);

        for timeseries_metadata in timeseries_metadata_list {
            if current.is_full(self.max_degree_of_index_node) {
                self.add_current_index_node_to_queue(current, &mut queue, payload.len() as i64);
                current = MetadataIndexNode::new(MetadataIndexNodeType::LeafMeasurement, 0);
            }
            current.add_child(MetadataIndexEntry::new(
                timeseries_metadata.measurement_id.clone(),
                payload.len() as i64,
            ));
            timeseries_metadata.serialize(payload)?;
        }
        self.add_current_index_node_to_queue(current, &mut queue, payload.len() as i64);
        self.generate_root_node(&mut queue, payload, MetadataIndexNodeType::InternalMeasurement)
    }

    fn check_and_build_level_index(
        &self,
        payload: &mut Vec<u8>,
        device_metadata_index_map: BTreeMap<DeviceId, MetadataIndexNode>,
    ) -> TsFileResult<MetadataIndexNode> {
        if device_metadata_index_map.len() <= self.max_degree_of_index_node {
            let mut node = MetadataIndexNode::new(MetadataIndexNodeType::LeafDevice, 0);
            for (device_id, measurement_node) in device_metadata_index_map {
                node.add_child(MetadataIndexEntry::new(device_id.to_string(), payload.len() as i64));
                measurement_node.serialize(payload)?;
            }
            node.end_offset = payload.len() as i64;
            return Ok(node);
        }

        let mut queue = VecDeque::new();
        let mut current = MetadataIndexNode::new(MetadataIndexNodeType::LeafDevice, 0);
        for (device_id, measurement_node) in device_metadata_index_map {
            if current.is_full(self.max_degree_of_index_node) {
                self.add_current_index_node_to_queue(current, &mut queue, payload.len() as i64);
                current = MetadataIndexNode::new(MetadataIndexNodeType::LeafDevice, 0);
            }
            current.add_child(MetadataIndexEntry::new(device_id.to_string(), payload.len() as i64));
            measurement_node.serialize(payload)?;
        }
        self.add_current_index_node_to_queue(current, &mut queue, payload.len() as i64);
        let mut root = self.generate_root_node(&mut queue, payload, MetadataIndexNodeType::InternalDevice)?;
        root.end_offset = payload.len() as i64;
        Ok(root)
    }

    fn generate_root_node(
        &self,
        queue: &mut VecDeque<MetadataIndexNode>,
        payload: &mut Vec<u8>,
        node_type: MetadataIndexNodeType,
    ) -> TsFileResult<MetadataIndexNode> {
        while queue.len() > 1 {
            let queue_size = queue.len();
            let mut current = MetadataIndexNode::new(node_type, 0);
            for _ in 0..queue_size {
                let child = queue.pop_front().expect("queue size checked");
                if current.is_full(self.max_degree_of_index_node) {
                    self.add_current_index_node_to_queue(current, queue, payload.len() as i64);
                    current = MetadataIndexNode::new(node_type, 0);
                }
                if let Some(first_entry) = child.children.first() {
                    current.add_child(MetadataIndexEntry::new(
                        first_entry.name.clone(),
                        payload.len() as i64,
                    ));
                }
                child.serialize(payload)?;
            }
            self.add_current_index_node_to_queue(current, queue, payload.len() as i64);
        }
        Ok(queue.pop_front().unwrap_or_else(|| MetadataIndexNode::new(node_type, payload.len() as i64)))
    }

    fn add_current_index_node_to_queue(
        &self,
        mut current: MetadataIndexNode,
        queue: &mut VecDeque<MetadataIndexNode>,
        end_offset: i64,
    ) {
        current.end_offset = end_offset;
        if !current.children.is_empty() {
            queue.push_back(current);
        }
    }
}

impl Default for MetadataIndexConstructor {
    fn default() -> Self {
        MetadataIndexConstructor::new(256)
    }
}

fn add_base_offset_to_node(node: &mut MetadataIndexNode, base_offset: i64) {
    for child in &mut node.children {
        child.offset += base_offset;
    }
    node.end_offset += base_offset;
}
