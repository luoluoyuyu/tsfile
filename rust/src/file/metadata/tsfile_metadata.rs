// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TsFileMetadata - the file-level footer metadata.

use std::collections::HashMap;
use std::io::{Read, Write};

use crate::error::TsFileResult;
use crate::file::metadata::metadata_index_node::MetadataIndexNode;
use crate::utils::bloom_filter::BloomFilter;
use crate::utils::{ReadWriteForEncodingUtils, ReadWriteIOUtils};

/// File-level metadata stored in the TsFile footer.
///
/// Mirrors Java's TsFileMetadata.
#[derive(Debug)]
pub struct TsFileMetadata {
    /// Map from table/device name to MetadataIndexNode root.
    pub table_metadata_index_node_map: HashMap<String, MetadataIndexNode>,
    /// Bloom filter for timeseries paths.
    pub bloom_filter: Option<BloomFilter>,
    /// Offset of the SEPARATOR marker.
    pub meta_offset: i64,
    /// TsFile properties (encryption, etc.).
    pub ts_file_properties: HashMap<String, String>,
}

impl TsFileMetadata {
    pub fn new() -> Self {
        TsFileMetadata {
            table_metadata_index_node_map: HashMap::new(),
            bloom_filter: None,
            meta_offset: 0,
            ts_file_properties: HashMap::new(),
        }
    }

    /// Add a metadata index node for a table/device group.
    pub fn add_table_metadata_index_node(
        &mut self,
        table_name: String,
        node: MetadataIndexNode,
    ) {
        self.table_metadata_index_node_map.insert(table_name, node);
    }

    /// Add a property.
    pub fn add_property(&mut self, key: String, value: String) {
        self.ts_file_properties.insert(key, value);
    }

    /// Serialize to writer using V3 format (compatible with Java's deserializeTsFileMetadataFromV3).
    /// V3 Format:
    /// 1. MetadataIndexNode (directly, no table count/name)
    /// 2. metaOffset (i64)
    /// 3. BloomFilter (optional, with self-description length)
    pub fn serialize<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        let mut written = 0;

        // V3 format: Write the root MetadataIndexNode directly
        // Java expects exactly ONE MetadataIndexNode for V3 files
        if let Some((_, node)) = self.table_metadata_index_node_map.iter().next() {
            written += node.serialize(writer)?;
        } else {
            // Write empty MetadataIndexNode if no table exists
            let empty_node = crate::file::metadata::metadata_index_node::MetadataIndexNode::new(
                crate::common::enums::MetadataIndexNodeType::InternalDevice,
                0,
            );
            written += empty_node.serialize(writer)?;
        }

        // meta_offset
        written += ReadWriteIOUtils::write_i64(self.meta_offset, writer)?;

        // bloom filter (V3 format: byte array with self-description length)
        match &self.bloom_filter {
            Some(bf) => {
                written += bf.serialize_with_self_description_length(writer)?;
            }
            None => {
                // Write nothing if no bloom filter (V3 format makes it optional)
            }
        }

        // V3 format does NOT include properties!
        Ok(written)
    }

    /// Deserialize from reader (V3 format).
    /// V3 format: [MetadataIndexNode] [metaOffset] [bloomFilter]
    pub fn deserialize<R: Read>(reader: &mut R) -> TsFileResult<Self> {
        let mut metadata = TsFileMetadata::new();

        // V3 format: Read the root MetadataIndexNode directly (no table count/name)
        let node = MetadataIndexNode::deserialize(reader)?;
        metadata.table_metadata_index_node_map.insert(String::new(), node);

        // meta_offset
        metadata.meta_offset = ReadWriteIOUtils::read_i64(reader)?;

        // bloom filter (optional)
        let bf_result = BloomFilter::deserialize(reader);
        if let Ok(bf) = bf_result {
            metadata.bloom_filter = Some(bf);
        }

        Ok(metadata)
    }
}

impl Default for TsFileMetadata {
    fn default() -> Self {
        TsFileMetadata::new()
    }
}
