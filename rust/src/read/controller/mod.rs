// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Metadata and chunk loading helpers mirroring Java's `read.controller` package.

use std::collections::{HashMap, VecDeque};
use std::path::{Path as FsPath, PathBuf};

use crate::common::enums::TSDataType;
use crate::error::TsFileResult;
use crate::file::metadata::{
    ChunkMetadata, DeviceId, MetadataIndexNode, TableSchema, TimeSeriesMetadataView,
    TsFileMetadata,
};
use crate::file::metadata::enums::MetadataIndexNodeType;
use crate::read::common::{Chunk, Path as SeriesPath, TimeRange};
use crate::read::filter::Filter;
use crate::read::reader::ChunkReader;
use crate::read::time_value_pair::TimeValuePair;
use crate::read::TsFileSequenceReader;

pub trait IMetadataQuerier {
    fn get_chunk_metadata_list(
        &mut self,
        path: &SeriesPath,
    ) -> TsFileResult<Vec<ChunkMetadata>>;

    fn get_chunk_metadata_map(
        &mut self,
        paths: &[SeriesPath],
    ) -> TsFileResult<HashMap<SeriesPath, Vec<ChunkMetadata>>>;

    fn get_whole_file_metadata(&mut self) -> TsFileResult<&TsFileMetadata>;

    fn get_table_schema_map(&mut self) -> TsFileResult<HashMap<String, TableSchema>>;

    fn load_chunk_metadata(&mut self, paths: &[SeriesPath]) -> TsFileResult<()>;

    fn get_data_type(&mut self, path: &SeriesPath) -> TsFileResult<Option<TSDataType>>;

    fn get_all_devices(&mut self) -> TsFileResult<Vec<String>>;

    fn clear(&mut self);
}

pub struct MetadataQuerierByFileImpl {
    reader: TsFileSequenceReader,
    cache: HashMap<(String, String), Vec<ChunkMetadata>>,
}

impl MetadataQuerierByFileImpl {
    pub fn new<P: AsRef<FsPath>>(path: P) -> TsFileResult<Self> {
        Self::from_reader(TsFileSequenceReader::new(path)?)
    }

    pub fn from_reader(reader: TsFileSequenceReader) -> TsFileResult<Self> {
        Ok(MetadataQuerierByFileImpl {
            reader,
            cache: HashMap::new(),
        })
    }

    pub fn reader(&self) -> &TsFileSequenceReader {
        &self.reader
    }

    pub fn reader_mut(&mut self) -> &mut TsFileSequenceReader {
        &mut self.reader
    }

    pub fn close(self) -> TsFileResult<()> {
        self.reader.close()
    }

    pub fn convert_space_to_time_partition(
        &mut self,
        paths: &[SeriesPath],
        space_partition_start_pos: u64,
        space_partition_end_pos: u64,
    ) -> TsFileResult<Vec<TimeRange>> {
        let metadata_map = self.get_chunk_metadata_map(paths)?;
        let mut ranges = metadata_map
            .values()
            .flat_map(|metadata_list| metadata_list.iter())
            .filter(|chunk_metadata| {
                let offset = chunk_metadata.offset_of_chunk_header as u64;
                offset >= space_partition_start_pos && offset < space_partition_end_pos
            })
            .map(|chunk_metadata| {
                TimeRange::new(chunk_metadata.start_time(), chunk_metadata.end_time())
            })
            .collect::<Vec<_>>();
        ranges.sort_by_key(|range| range.min);
        Ok(merge_time_ranges(ranges))
    }

    pub fn device_iterator(
        &mut self,
        metadata_index_node: MetadataIndexNode,
        tag_filter: Option<&Filter>,
    ) -> TsFileResult<DeviceMetaIterator> {
        DeviceMetaIterator::new(self.reader_mut(), metadata_index_node, tag_filter)
    }
}

impl IMetadataQuerier for MetadataQuerierByFileImpl {
    fn get_chunk_metadata_list(
        &mut self,
        path: &SeriesPath,
    ) -> TsFileResult<Vec<ChunkMetadata>> {
        let key = (path.device.clone(), path.measurement.clone());
        if let Some(metadata) = self.cache.get(&key) {
            return Ok(metadata.clone());
        }

        let metadata = self
            .reader
            .read_chunk_metadata_list(&path.device, &path.measurement)?;
        self.cache.insert(key, metadata.clone());
        Ok(metadata)
    }

    fn get_chunk_metadata_map(
        &mut self,
        paths: &[SeriesPath],
    ) -> TsFileResult<HashMap<SeriesPath, Vec<ChunkMetadata>>> {
        let mut result = HashMap::with_capacity(paths.len());
        for path in paths {
            result.insert(path.clone(), self.get_chunk_metadata_list(path)?);
        }
        Ok(result)
    }

    fn get_whole_file_metadata(&mut self) -> TsFileResult<&TsFileMetadata> {
        self.reader.read_file_metadata()
    }

    fn get_table_schema_map(&mut self) -> TsFileResult<HashMap<String, TableSchema>> {
        self.reader.get_table_schema_map()
    }

    fn load_chunk_metadata(&mut self, paths: &[SeriesPath]) -> TsFileResult<()> {
        for path in paths {
            let _ = self.get_chunk_metadata_list(path)?;
        }
        Ok(())
    }

    fn get_data_type(&mut self, path: &SeriesPath) -> TsFileResult<Option<TSDataType>> {
        Ok(self
            .reader
            .get_measurement(&path.device)?
            .get(&path.measurement)
            .copied())
    }

    fn get_all_devices(&mut self) -> TsFileResult<Vec<String>> {
        self.reader.get_all_devices()
    }

    fn clear(&mut self) {
        self.cache.clear();
    }
}

pub trait IChunkLoader {
    fn load_chunk(&mut self, chunk_metadata: &ChunkMetadata) -> TsFileResult<Chunk>;

    fn get_chunk_reader(
        &mut self,
        chunk_metadata: &ChunkMetadata,
        _global_time_filter: Option<&Filter>,
    ) -> TsFileResult<ChunkReader>;

    fn load_points(
        &mut self,
        chunk_metadata: &ChunkMetadata,
        global_time_filter: Option<&Filter>,
    ) -> TsFileResult<Vec<TimeValuePair>>;

    fn close(self) -> TsFileResult<()>;
}

pub trait IChunkMetadataLoader {
    fn load_chunk_metadata_list(
        &self,
        time_series_metadata: &dyn TimeSeriesMetadataView,
    ) -> TsFileResult<Vec<ChunkMetadata>>;
}

pub struct CachedChunkLoaderImpl {
    reader: TsFileSequenceReader,
    cache: HashMap<i64, Chunk>,
}

impl CachedChunkLoaderImpl {
    pub fn new<P: AsRef<FsPath>>(path: P) -> TsFileResult<Self> {
        Self::from_reader(TsFileSequenceReader::new(path)?)
    }

    pub fn from_reader(reader: TsFileSequenceReader) -> TsFileResult<Self> {
        Ok(CachedChunkLoaderImpl {
            reader,
            cache: HashMap::new(),
        })
    }

    pub fn reader_mut(&mut self) -> &mut TsFileSequenceReader {
        &mut self.reader
    }

    pub fn close(self) -> TsFileResult<()> {
        self.reader.close()
    }
}

impl IChunkLoader for CachedChunkLoaderImpl {
    fn load_chunk(&mut self, chunk_metadata: &ChunkMetadata) -> TsFileResult<Chunk> {
        let key = chunk_metadata.offset_of_chunk_header;
        if let Some(chunk) = self.cache.get(&key) {
            return Ok(chunk.clone());
        }

        let chunk = self.reader.read_mem_chunk(chunk_metadata)?;
        self.cache.insert(key, chunk.clone());
        Ok(chunk)
    }

    fn get_chunk_reader(
        &mut self,
        chunk_metadata: &ChunkMetadata,
        _global_time_filter: Option<&Filter>,
    ) -> TsFileResult<ChunkReader> {
        let chunk = self.load_chunk(chunk_metadata)?;
        Ok(ChunkReader::new(chunk.header, chunk.data))
    }

    fn load_points(
        &mut self,
        chunk_metadata: &ChunkMetadata,
        global_time_filter: Option<&Filter>,
    ) -> TsFileResult<Vec<TimeValuePair>> {
        let reader = self.get_chunk_reader(chunk_metadata, global_time_filter)?;
        reader.read_with_filter(global_time_filter)
    }

    fn close(self) -> TsFileResult<()> {
        CachedChunkLoaderImpl::close(self)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SimpleChunkMetadataLoader;

impl IChunkMetadataLoader for SimpleChunkMetadataLoader {
    fn load_chunk_metadata_list(
        &self,
        time_series_metadata: &dyn TimeSeriesMetadataView,
    ) -> TsFileResult<Vec<ChunkMetadata>> {
        Ok(time_series_metadata.load_chunk_metadata_list())
    }
}

#[derive(Debug, Clone)]
pub struct DeviceMetaIterator {
    results: VecDeque<(DeviceId, MetadataIndexNode)>,
}

impl DeviceMetaIterator {
    pub fn new(
        reader: &mut TsFileSequenceReader,
        metadata_index_node: MetadataIndexNode,
        _tag_filter: Option<&Filter>,
    ) -> TsFileResult<Self> {
        let mut pending = VecDeque::from([metadata_index_node]);
        let mut results = VecDeque::new();

        while let Some(node) = pending.pop_front() {
            match node.node_type {
                MetadataIndexNodeType::LeafDevice => {
                    for child in &node.children {
                        let device_id = device_id_from_name(&child.name);
                        let child_node = reader.read_metadata_index_node_at(child.offset)?;
                        results.push_back((device_id, child_node));
                    }
                }
                MetadataIndexNodeType::InternalDevice => {
                    for child in &node.children {
                        pending.push_back(reader.read_metadata_index_node_at(child.offset)?);
                    }
                }
                _ => {}
            }
        }

        Ok(DeviceMetaIterator { results })
    }
}

impl Iterator for DeviceMetaIterator {
    type Item = (DeviceId, MetadataIndexNode);

    fn next(&mut self) -> Option<Self::Item> {
        self.results.pop_front()
    }
}

#[derive(Debug, Clone)]
pub struct ControllerBuilder {
    path: PathBuf,
}

impl ControllerBuilder {
    pub fn new<P: AsRef<FsPath>>(path: P) -> Self {
        ControllerBuilder {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn build_metadata_querier(self) -> TsFileResult<MetadataQuerierByFileImpl> {
        MetadataQuerierByFileImpl::new(self.path)
    }

    pub fn build_chunk_loader(self) -> TsFileResult<CachedChunkLoaderImpl> {
        CachedChunkLoaderImpl::new(self.path)
    }
}

fn merge_time_ranges(mut ranges: Vec<TimeRange>) -> Vec<TimeRange> {
    if ranges.is_empty() {
        return ranges;
    }

    let mut merged = Vec::with_capacity(ranges.len());
    let mut current = ranges.remove(0);
    for next in ranges {
        if current.intersects(next) || current.max + 1 >= next.min {
            current.max = current.max.max(next.max);
            current.left_closed &= next.left_closed;
            current.right_closed &= next.right_closed;
        } else {
            merged.push(current);
            current = next;
        }
    }
    merged.push(current);
    merged
}

fn device_id_from_name(name: &str) -> DeviceId {
    if name.contains('.') {
        DeviceId::string_array(name.split('.').map(ToOwned::to_owned).collect())
            .unwrap_or_else(|_| DeviceId::plain(name.to_string()))
    } else {
        DeviceId::plain(name.to_string())
    }
}
