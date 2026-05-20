// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TsFileSequenceReader: sequential scan of a TsFile.
//!
//! Mirrors Java's TsFileSequenceReader.

use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use crate::common::constant::TsFileConstant;
use crate::common::enums::{CompressionType, MetadataIndexNodeType, TSDataType};
use crate::compress::create_decompressor;
use crate::error::{TsFileError, TsFileResult};
use crate::file::header::{ChunkGroupHeader, ChunkHeader, PageHeader};
use crate::file::meta_marker::MetaMarker;
use crate::file::metadata::aligned_metadata::{AlignedChunkMetadata, AlignedTimeSeriesMetadata};
use crate::file::metadata::chunk_metadata::ChunkMetadata;
use crate::file::metadata::metadata_index_node::MetadataIndexNode;
use crate::file::metadata::table_schema::TableSchema;
use crate::file::metadata::timeseries_metadata::TimeseriesMetadata;
use crate::file::metadata::tsfile_metadata::TsFileMetadata;
use crate::read::block::{ColumnValue, TsBlock, TsBlockBuilder};
use crate::read::common::{Chunk, Path as SeriesPath, TimeRange};
use crate::read::filter::Filter;
use crate::read::reader::{ChunkReader, PageReader};
use crate::read::time_value_pair::TimeValuePair;

/// Information about a chunk during sequential reading.
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub device_id: String,
    pub chunk_header: ChunkHeader,
    pub chunk_data: Vec<u8>,
}

/// Sequential reader for TsFile format.
pub struct TsFileSequenceReader {
    reader: BufReader<File>,
    file_size: u64,
    file_version: u8,
    /// Cached file metadata.
    pub file_metadata: Option<TsFileMetadata>,
}

impl TsFileSequenceReader {
    /// Open a TsFile for reading.
    pub fn new<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        let file = File::open(path.as_ref())?;
        let file_size = file.metadata()?.len();
        let reader = BufReader::new(file);
        let mut seq_reader = TsFileSequenceReader {
            reader,
            file_size,
            file_version: 0,
            file_metadata: None,
        };
        seq_reader.verify_magic()?;
        Ok(seq_reader)
    }

    /// Verify magic string and version.
    fn verify_magic(&mut self) -> TsFileResult<()> {
        let minimum_size = (TsFileConstant::MAGIC_STRING.len() * 2 + 1 + 4) as u64;
        if self.file_size < minimum_size {
            return Err(TsFileError::InvalidFileFormat(format!(
                "TsFile is too small to be valid: {} bytes",
                self.file_size
            )));
        }

        let magic_len = TsFileConstant::MAGIC_STRING.len();
        let mut magic_buf = vec![0u8; magic_len];
        self.reader.read_exact(&mut magic_buf)?;
        if magic_buf != TsFileConstant::MAGIC_STRING.as_bytes() {
            return Err(TsFileError::InvalidMagicString);
        }

        let mut version_buf = [0u8; 1];
        self.reader.read_exact(&mut version_buf)?;
        self.file_version = version_buf[0];
        if self.file_version != TsFileConstant::VERSION_NUMBER {
            return Err(TsFileError::InvalidVersion(self.file_version));
        }
        Ok(())
    }

    pub fn file_version(&self) -> u8 {
        self.file_version
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn seek(&mut self, offset: u64) -> TsFileResult<()> {
        self.reader.seek(SeekFrom::Start(offset))?;
        Ok(())
    }

    pub fn position(&mut self) -> TsFileResult<u64> {
        Ok(self.reader.stream_position()?)
    }

    pub fn read_head_magic(&mut self) -> TsFileResult<String> {
        let current_position = self.position()?;
        self.reader.seek(SeekFrom::Start(0))?;
        let mut buffer = vec![0u8; TsFileConstant::MAGIC_STRING.len()];
        self.reader.read_exact(&mut buffer)?;
        self.reader.seek(SeekFrom::Start(current_position))?;
        String::from_utf8(buffer)
            .map_err(|error| TsFileError::InvalidFileFormat(error.to_string()))
    }

    pub fn read_tail_magic(&mut self) -> TsFileResult<String> {
        let current_position = self.position()?;
        let magic_len = TsFileConstant::MAGIC_STRING.len() as i64;
        self.reader.seek(SeekFrom::End(-magic_len))?;
        let mut buffer = vec![0u8; TsFileConstant::MAGIC_STRING.len()];
        self.reader.read_exact(&mut buffer)?;
        self.reader.seek(SeekFrom::Start(current_position))?;
        String::from_utf8(buffer)
            .map_err(|error| TsFileError::InvalidFileFormat(error.to_string()))
    }

    pub fn read_version_number(&mut self) -> TsFileResult<u8> {
        let current_position = self.position()?;
        self.reader
            .seek(SeekFrom::Start(TsFileConstant::MAGIC_STRING.len() as u64))?;
        let mut version = [0u8; 1];
        self.reader.read_exact(&mut version)?;
        self.reader.seek(SeekFrom::Start(current_position))?;
        Ok(version[0])
    }

    pub fn is_complete(&mut self) -> TsFileResult<bool> {
        let current_position = self.reader.stream_position()?;
        let magic_len = TsFileConstant::MAGIC_STRING.len() as u64;
        let valid = if self.file_size < magic_len {
            false
        } else {
            self.reader.seek(SeekFrom::End(-(magic_len as i64)))?;
            let mut closing_magic = vec![0u8; magic_len as usize];
            self.reader.read_exact(&mut closing_magic)?;
            closing_magic == TsFileConstant::MAGIC_STRING.as_bytes()
        };
        self.reader.seek(SeekFrom::Start(current_position))?;
        Ok(valid)
    }

    pub fn close(self) -> TsFileResult<()> {
        Ok(())
    }

    pub fn read_chunk_header(&mut self, chunk_type: u8) -> TsFileResult<ChunkHeader> {
        ChunkHeader::deserialize(&mut self.reader, chunk_type)
    }

    pub fn read_chunk_header_at(&mut self, offset: u64) -> TsFileResult<ChunkHeader> {
        self.reader.seek(SeekFrom::Start(offset))?;
        let mut chunk_type = [0u8; 1];
        self.reader.read_exact(&mut chunk_type)?;
        self.read_chunk_header(chunk_type[0])
    }

    pub fn read_page_header(
        &mut self,
        data_type: TSDataType,
        has_statistics: bool,
    ) -> TsFileResult<PageHeader> {
        PageHeader::deserialize(&mut self.reader, data_type, has_statistics)
    }

    pub fn read_page_data(&mut self, page_header: &PageHeader) -> TsFileResult<Vec<u8>> {
        let mut page_data = vec![0u8; page_header.compressed_size as usize];
        self.reader.read_exact(&mut page_data)?;
        Ok(page_data)
    }

    pub fn skip_page_data(&mut self, page_header: &PageHeader) -> TsFileResult<()> {
        self.reader
            .seek(SeekFrom::Current(page_header.compressed_size as i64))?;
        Ok(())
    }

    pub fn read_compressed_page(&mut self, page_header: &PageHeader) -> TsFileResult<Vec<u8>> {
        self.read_page_data(page_header)
    }

    pub fn read_page(
        &mut self,
        page_header: &PageHeader,
        compression_type: CompressionType,
    ) -> TsFileResult<Vec<u8>> {
        let compressed = self.read_compressed_page(page_header)?;
        create_decompressor(compression_type).decompress(
            &compressed,
            page_header.uncompressed_size as usize,
        )
    }

    pub fn read_marker(&mut self) -> TsFileResult<u8> {
        let mut marker = [0u8; 1];
        self.reader.read_exact(&mut marker)?;
        Ok(marker[0])
    }

    /// Read the TsFile footer metadata.
    pub fn read_file_metadata(&mut self) -> TsFileResult<&TsFileMetadata> {
        if self.file_metadata.is_some() {
            return Ok(self.file_metadata.as_ref().unwrap());
        }

        let magic_len = TsFileConstant::MAGIC_STRING.len() as i64;

        // Seek to read metadata size (last 4+magic_len bytes)
        self.reader
            .seek(SeekFrom::End(-(magic_len + 4)))?;

        let mut size_buf = [0u8; 4];
        self.reader.read_exact(&mut size_buf)?;
        let meta_size = i32::from_be_bytes(size_buf);
        if meta_size <= 0 {
            return Err(TsFileError::InvalidFileFormat(format!(
                "Invalid metadata size: {}",
                meta_size
            )));
        }
        let meta_size = meta_size as usize;

        // Verify closing magic
        let mut closing_magic = vec![0u8; magic_len as usize];
        self.reader.read_exact(&mut closing_magic)?;
        if closing_magic != TsFileConstant::MAGIC_STRING.as_bytes() {
            return Err(TsFileError::InvalidMagicString);
        }

        // Seek to metadata start
        let meta_offset = self.file_size as i64 - magic_len - 4 - meta_size as i64;
        let header_len = (TsFileConstant::MAGIC_STRING.len() + 1) as i64;
        if meta_offset < header_len {
            return Err(TsFileError::InvalidFileFormat(format!(
                "Metadata offset {} is before file data region",
                meta_offset
            )));
        }
        self.reader.seek(SeekFrom::Start(meta_offset as u64))?;

        let mut meta_buf = vec![0u8; meta_size];
        self.reader.read_exact(&mut meta_buf)?;

        let metadata = TsFileMetadata::deserialize(&mut std::io::Cursor::new(&meta_buf))?;
        self.file_metadata = Some(metadata);
        Ok(self.file_metadata.as_ref().unwrap())
    }

    /// Scan the file sequentially, returning chunks in order.
    /// Reads file metadata first to determine the data region boundary.
    pub fn read_all_chunks(
        &mut self,
    ) -> TsFileResult<Vec<(String, ChunkHeader, Vec<u8>)>> {
        // First, read the file metadata to get the meta_offset boundary.
        // meta_offset is the position right after all chunk data (before TimeseriesMetadata).
        let meta_offset = {
            let metadata = self.read_file_metadata()?;
            metadata.meta_offset as u64
        };

        // Seek past magic + version
        let start = TsFileConstant::MAGIC_STRING.len() + 1;
        self.reader.seek(SeekFrom::Start(start as u64))?;

        let mut results = Vec::new();
        let mut current_device = String::new();

        loop {
            // Stop if we've reached the metadata region
            let current_pos = self.reader.seek(SeekFrom::Current(0))?;
            if current_pos >= meta_offset {
                break;
            }

            let mut marker_buf = [0u8; 1];
            match self.reader.read_exact(&mut marker_buf) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(TsFileError::Io(e)),
            }
            let marker = marker_buf[0];

            match marker {
                MetaMarker::CHUNK_GROUP_HEADER => {
                    let header = ChunkGroupHeader::deserialize(&mut self.reader)?;
                    current_device = header.device_id;
                }
                MetaMarker::CHUNK_HEADER
                | MetaMarker::ONLY_ONE_PAGE_CHUNK_HEADER
                | MetaMarker::TIME_CHUNK_HEADER
                | MetaMarker::VALUE_CHUNK_HEADER
                | MetaMarker::ONLY_ONE_PAGE_TIME_CHUNK_HEADER
                | MetaMarker::ONLY_ONE_PAGE_VALUE_CHUNK_HEADER => {
                    let chunk_header = ChunkHeader::deserialize(&mut self.reader, marker)?;
                    let data_size = chunk_header.data_size as usize;
                    let mut chunk_data = vec![0u8; data_size];
                    self.reader.read_exact(&mut chunk_data)?;
                    results.push((current_device.clone(), chunk_header, chunk_data));
                }
                MetaMarker::SEPARATOR => {
                    // SEPARATOR marks the boundary between chunk data and metadata section.
                    // In Java format, there is only ONE SEPARATOR, written just before the metadata.
                    // Stop scanning here.
                    break;
                }
                MetaMarker::OPERATION_INDEX_RANGE => {
                    // Skip 16 bytes (two longs)
                    let mut skip_buf = [0u8; 16];
                    self.reader.read_exact(&mut skip_buf)?;
                }
                _ => {
                    return Err(TsFileError::InvalidMarker(marker));
                }
            }
        }

        Ok(results)
    }

    /// Read all chunks as structured `ChunkInfo` records.
    pub fn read_chunk_infos(&mut self) -> TsFileResult<Vec<ChunkInfo>> {
        Ok(self
            .read_all_chunks()?
            .into_iter()
            .map(|(device_id, chunk_header, chunk_data)| ChunkInfo {
                device_id,
                chunk_header,
                chunk_data,
            })
            .collect())
    }

    /// Read and decode all points, organized as device -> measurement -> points.
    pub fn read_all_data(&mut self) -> TsFileResult<DeviceChunkMap> {
        let mut data_map: DeviceChunkMap = HashMap::new();
        for chunk_info in self.read_chunk_infos()? {
            let measurement_id = chunk_info.chunk_header.measurement_id.clone();
            let points = Self::read_chunk_data(&chunk_info.chunk_header, &chunk_info.chunk_data)?;
            data_map
                .entry(chunk_info.device_id)
                .or_default()
                .entry(measurement_id)
                .or_default()
                .extend(points);
        }
        for measurement_map in data_map.values_mut() {
            for points in measurement_map.values_mut() {
                points.sort_by_key(|point| point.timestamp);
            }
        }
        Ok(data_map)
    }

    /// Read one chunk by its metadata offset.
    pub fn read_chunk_by_metadata(
        &mut self,
        chunk_metadata: &ChunkMetadata,
    ) -> TsFileResult<(ChunkHeader, Vec<u8>)> {
        self.reader
            .seek(SeekFrom::Start(chunk_metadata.offset_of_chunk_header as u64))?;
        let mut marker = [0u8; 1];
        self.reader.read_exact(&mut marker)?;
        let chunk_header = ChunkHeader::deserialize(&mut self.reader, marker[0])?;
        let mut chunk_data = vec![0u8; chunk_header.data_size as usize];
        self.reader.read_exact(&mut chunk_data)?;
        Ok((chunk_header, chunk_data))
    }

    pub fn read_mem_chunk(&mut self, chunk_metadata: &ChunkMetadata) -> TsFileResult<Chunk> {
        let (header, data) = self.read_chunk_by_metadata(chunk_metadata)?;
        Ok(Chunk {
            header,
            data,
            delete_interval_list: Vec::new(),
            statistics: Some(chunk_metadata.statistics.clone()),
        })
    }

    pub fn read_chunk(&mut self, position: u64, data_size: usize) -> TsFileResult<Vec<u8>> {
        self.reader.seek(SeekFrom::Start(position))?;
        let mut data = vec![0u8; data_size];
        self.reader.read_exact(&mut data)?;
        Ok(data)
    }

    pub fn read_mem_chunk_at(&mut self, offset: u64) -> TsFileResult<Chunk> {
        self.reader.seek(SeekFrom::Start(offset))?;
        let marker = self.read_marker()?;
        let header = self.read_chunk_header(marker)?;
        let data = self.read_chunk(
            offset + 1 + header.serialized_size() as u64 - 1,
            header.data_size as usize,
        )?;
        Ok(Chunk::new(header, data))
    }

    /// Decode chunks referenced by chunk metadata.
    pub fn read_by_chunk_metadata(
        &mut self,
        chunk_metadata_list: &[ChunkMetadata],
    ) -> TsFileResult<Vec<TimeValuePair>> {
        let mut points = Vec::new();
        for chunk_metadata in chunk_metadata_list {
            let (chunk_header, chunk_data) = self.read_chunk_by_metadata(chunk_metadata)?;
            points.extend(ChunkReader::new(chunk_header, chunk_data).read_all()?);
        }
        points.sort_by_key(|point| point.timestamp);
        Ok(points)
    }

    /// Decode chunks referenced by chunk metadata with conservative statistics pushdown.
    pub fn read_by_chunk_metadata_with_filter(
        &mut self,
        chunk_metadata_list: &[ChunkMetadata],
        filter: Option<&Filter>,
    ) -> TsFileResult<Vec<TimeValuePair>> {
        let mut points = Vec::new();
        for chunk_metadata in chunk_metadata_list {
            if let Some(filter) = filter {
                if !filter.satisfy_statistics(&chunk_metadata.statistics) {
                    continue;
                }
            }
            let (chunk_header, chunk_data) = self.read_chunk_by_metadata(chunk_metadata)?;
            points.extend(ChunkReader::new(chunk_header, chunk_data).read_with_filter(filter)?);
        }
        points.sort_by_key(|point| point.timestamp);
        Ok(points)
    }

    /// Read timeseries metadata at a known file offset.
    pub fn read_timeseries_metadata_at(
        &mut self,
        offset: i64,
        need_chunk_metadata: bool,
    ) -> TsFileResult<TimeseriesMetadata> {
        self.reader.seek(SeekFrom::Start(offset as u64))?;
        TimeseriesMetadata::deserialize_with_chunks(&mut self.reader, need_chunk_metadata)
    }

    /// Read a metadata index node from a known file offset.
    pub fn read_metadata_index_node_at(&mut self, offset: i64) -> TsFileResult<MetadataIndexNode> {
        if offset < 0 {
            return Err(TsFileError::InvalidFileFormat(format!(
                "negative metadata index node offset: {}",
                offset
            )));
        }
        self.reader.seek(SeekFrom::Start(offset as u64))?;
        MetadataIndexNode::deserialize(&mut self.reader)
    }

    /// Find the file offset of a timeseries metadata entry through the footer index tree.
    pub fn find_timeseries_metadata_offset(
        &mut self,
        device_id: &str,
        measurement_id: &str,
    ) -> TsFileResult<Option<i64>> {
        let root = {
            let metadata = self.read_file_metadata()?;
            match metadata.metadata_index_node("") {
                Some(node) => node.clone(),
                None => return Ok(None),
            }
        };

        let measurement_node = self.find_measurement_root_node(root, device_id)?;
        let Some(measurement_node) = measurement_node else {
            return Ok(None);
        };
        self.find_measurement_metadata_offset(measurement_node, measurement_id)
    }

    /// Read timeseries metadata through the footer index tree.
    pub fn read_timeseries_metadata(
        &mut self,
        device_id: &str,
        measurement_id: &str,
        need_chunk_metadata: bool,
    ) -> TsFileResult<Option<TimeseriesMetadata>> {
        let Some(offset) = self.find_timeseries_metadata_offset(device_id, measurement_id)? else {
            return Ok(None);
        };
        Ok(Some(self.read_timeseries_metadata_at(offset, need_chunk_metadata)?))
    }

    /// Read one series through metadata index and decode its chunks.
    pub fn read_timeseries_by_index(
        &mut self,
        device_id: &str,
        measurement_id: &str,
    ) -> TsFileResult<Vec<TimeValuePair>> {
        let Some(metadata) = self.read_timeseries_metadata(device_id, measurement_id, true)? else {
            return Ok(Vec::new());
        };
        self.read_by_chunk_metadata_with_filter(&metadata.chunk_metadata_list, None)
    }

    /// Read one series through metadata index with chunk/page filtering.
    pub fn read_timeseries_by_index_with_filter(
        &mut self,
        device_id: &str,
        measurement_id: &str,
        filter: Option<&Filter>,
    ) -> TsFileResult<Vec<TimeValuePair>> {
        let Some(metadata) = self.read_timeseries_metadata(device_id, measurement_id, true)? else {
            return Ok(Vec::new());
        };
        if let Some(filter) = filter {
            if !filter.satisfy_statistics(&metadata.statistics) {
                return Ok(Vec::new());
            }
        }
        self.read_by_chunk_metadata_with_filter(&metadata.chunk_metadata_list, filter)
    }

    fn find_measurement_root_node(
        &mut self,
        mut node: MetadataIndexNode,
        device_id: &str,
    ) -> TsFileResult<Option<MetadataIndexNode>> {
        loop {
            match node.node_type {
                MetadataIndexNodeType::LeafDevice => {
                    let Some((entry, _)) = node.child_index_entry(device_id, true) else {
                        return Ok(None);
                    };
                    return Ok(Some(self.read_metadata_index_node_at(entry.offset)?));
                }
                MetadataIndexNodeType::InternalDevice => {
                    let Some((entry, _)) = node.child_index_entry(device_id, false) else {
                        return Ok(None);
                    };
                    node = self.read_metadata_index_node_at(entry.offset)?;
                }
                MetadataIndexNodeType::InternalMeasurement | MetadataIndexNodeType::LeafMeasurement => {
                    return Ok(Some(node));
                }
            }
        }
    }

    fn find_measurement_metadata_offset(
        &mut self,
        mut node: MetadataIndexNode,
        measurement_id: &str,
    ) -> TsFileResult<Option<i64>> {
        loop {
            match node.node_type {
                MetadataIndexNodeType::LeafMeasurement => {
                    return Ok(node
                        .child_index_entry(measurement_id, true)
                        .map(|(entry, _)| entry.offset));
                }
                MetadataIndexNodeType::InternalMeasurement => {
                    let Some((entry, _)) = node.child_index_entry(measurement_id, false) else {
                        return Ok(None);
                    };
                    node = self.read_metadata_index_node_at(entry.offset)?;
                }
                MetadataIndexNodeType::InternalDevice | MetadataIndexNodeType::LeafDevice => {
                    return Err(TsFileError::InvalidFileFormat(
                        "device metadata index node found while searching measurement".to_string(),
                    ));
                }
            }
        }
    }

    /// Read a timeseries by a timeseries metadata offset.
    pub fn read_timeseries_by_metadata_offset(
        &mut self,
        offset: i64,
    ) -> TsFileResult<Vec<TimeValuePair>> {
        let metadata = self.read_timeseries_metadata_at(offset, true)?;
        self.read_by_chunk_metadata_with_filter(&metadata.chunk_metadata_list, None)
    }

    pub fn read_chunk_metadata_list(
        &mut self,
        device_id: &str,
        measurement_id: &str,
    ) -> TsFileResult<Vec<ChunkMetadata>> {
        let Some(metadata) = self.read_timeseries_metadata(device_id, measurement_id, true)? else {
            return Ok(Vec::new());
        };
        Ok(metadata.chunk_metadata_list)
    }

    pub fn read_chunk_page_headers(
        &mut self,
        chunk_metadata: &ChunkMetadata,
    ) -> TsFileResult<Vec<PageHeader>> {
        let (chunk_header, chunk_data) = self.read_chunk_by_metadata(chunk_metadata)?;
        let mut cursor = Cursor::new(chunk_data.as_slice());
        let has_page_statistics = (chunk_header.chunk_type & 0x3F) == MetaMarker::CHUNK_HEADER;
        let mut page_headers = Vec::new();

        while (cursor.position() as usize) < chunk_data.len() {
            let page_header =
                PageHeader::deserialize(&mut cursor, chunk_header.data_type, has_page_statistics)?;
            if page_header.uncompressed_size == 0 {
                continue;
            }
            cursor.seek(SeekFrom::Current(page_header.compressed_size as i64))?;
            page_headers.push(page_header);
        }

        Ok(page_headers)
    }

    pub fn read_chunk_pages(
        &mut self,
        chunk_metadata: &ChunkMetadata,
        filter: Option<&Filter>,
    ) -> TsFileResult<Vec<Vec<TimeValuePair>>> {
        let (chunk_header, chunk_data) = self.read_chunk_by_metadata(chunk_metadata)?;
        let mut cursor = Cursor::new(chunk_data.as_slice());
        let has_page_statistics = (chunk_header.chunk_type & 0x3F) == MetaMarker::CHUNK_HEADER;
        let decompressor = create_decompressor(chunk_header.compression_type);
        let mut pages = Vec::new();

        while (cursor.position() as usize) < chunk_data.len() {
            let page_header =
                PageHeader::deserialize(&mut cursor, chunk_header.data_type, has_page_statistics)?;
            if page_header.uncompressed_size == 0 {
                continue;
            }
            let mut compressed_page = vec![0u8; page_header.compressed_size as usize];
            cursor.read_exact(&mut compressed_page)?;
            let page_data = decompressor.decompress(
                &compressed_page,
                page_header.uncompressed_size as usize,
            )?;
            let page_reader =
                PageReader::new(chunk_header.data_type, chunk_header.encoding_type, page_data);
            pages.push(page_reader.read_with_filter(filter)?);
        }

        Ok(pages)
    }

    pub fn is_aligned_device(&self, measurement_node: &MetadataIndexNode) -> bool {
        measurement_node
            .children
            .first()
            .is_some_and(|entry| entry.name.is_empty())
    }

    pub fn get_time_column_metadata(
        &mut self,
        root_measurement_node: &MetadataIndexNode,
    ) -> TsFileResult<Option<TimeseriesMetadata>> {
        if !self.is_aligned_device(root_measurement_node) {
            return Ok(None);
        }

        match root_measurement_node.node_type {
            MetadataIndexNodeType::LeafMeasurement => root_measurement_node
                .children
                .first()
                .map(|entry| self.read_timeseries_metadata_at(entry.offset, true))
                .transpose(),
            MetadataIndexNodeType::InternalMeasurement => {
                let Some(first_child) = root_measurement_node.children.first() else {
                    return Ok(None);
                };
                let child_node = self.read_metadata_index_node_at(first_child.offset)?;
                self.get_time_column_metadata(&child_node)
            }
            _ => Ok(None),
        }
    }

    pub fn read_aligned_timeseries_metadata(
        &mut self,
        device_id: &str,
        measurements: &[String],
    ) -> TsFileResult<Option<AlignedTimeSeriesMetadata>> {
        let root = {
            let metadata = self.read_file_metadata()?;
            metadata.metadata_index_node("").cloned()
        };
        let Some(root) = root else {
            return Ok(None);
        };
        let Some(measurement_node) = self.find_measurement_root_node(root, device_id)? else {
            return Ok(None);
        };
        let Some(time_column_metadata) = self.get_time_column_metadata(&measurement_node)? else {
            return Ok(None);
        };

        let selected_measurements = if measurements.is_empty() {
            let mut values = self
                .read_device_metadata(device_id, true)?
                .into_keys()
                .filter(|measurement| !measurement.is_empty())
                .collect::<Vec<_>>();
            values.sort();
            values
        } else {
            measurements.to_vec()
        };

        let mut value_timeseries_metadata_list = Vec::with_capacity(selected_measurements.len());
        for measurement in selected_measurements {
            value_timeseries_metadata_list
                .push(self.read_timeseries_metadata(device_id, &measurement, true)?);
        }

        Ok(Some(AlignedTimeSeriesMetadata::new(
            time_column_metadata,
            value_timeseries_metadata_list,
        )))
    }

    pub fn get_aligned_chunk_metadata(
        &mut self,
        device_id: &str,
        measurements: &[String],
    ) -> TsFileResult<Vec<AlignedChunkMetadata>> {
        let Some(aligned_metadata) =
            self.read_aligned_timeseries_metadata(device_id, measurements)?
        else {
            return Ok(Vec::new());
        };
        aligned_metadata.construct_aligned_chunk_metadata()
    }

    pub fn read_device_data(
        &mut self,
        device_id: &str,
    ) -> TsFileResult<HashMap<String, Vec<TimeValuePair>>> {
        let mut result = HashMap::new();
        let mut measurement_offsets: Vec<(String, i64)> = self
            .read_device_metadata(device_id, false)?
            .into_iter()
            .map(|(measurement_id, metadata)| (measurement_id, metadata.statistics.start_time))
            .collect();
        measurement_offsets.sort_by(|left, right| left.0.cmp(&right.0));

        for (measurement_id, _) in measurement_offsets {
            let points = self.read_timeseries_by_index(device_id, &measurement_id)?;
            result.insert(measurement_id, points);
        }
        Ok(result)
    }

    pub fn read_devices_data(
        &mut self,
        device_ids: &[String],
    ) -> TsFileResult<HashMap<String, HashMap<String, Vec<TimeValuePair>>>> {
        let mut result = HashMap::with_capacity(device_ids.len());
        for device_id in device_ids {
            result.insert(device_id.clone(), self.read_device_data(device_id)?);
        }
        Ok(result)
    }

    pub fn get_all_devices(&mut self) -> TsFileResult<Vec<String>> {
        let roots: Vec<MetadataIndexNode> = {
            let metadata = self.read_file_metadata()?;
            metadata
                .table_metadata_index_node_map
                .values()
                .cloned()
                .collect()
        };

        let mut devices = Vec::new();
        for root in roots {
            self.collect_device_ids_from_node(root, &mut devices)?;
        }
        devices.sort();
        devices.dedup();
        Ok(devices)
    }

    pub fn get_all_paths(&mut self) -> TsFileResult<Vec<SeriesPath>> {
        let mut paths = Vec::new();
        let mut device_ids = self.get_all_devices()?;
        device_ids.sort();
        for device_id in device_ids {
            let mut measurements: Vec<String> =
                self.get_measurement(&device_id)?.into_keys().collect();
            measurements.sort();
            for measurement in measurements {
                paths.push(SeriesPath::new(device_id.clone(), measurement));
            }
        }
        Ok(paths)
    }

    pub fn get_table_devices(&mut self, table_name: &str) -> TsFileResult<Vec<String>> {
        let root = {
            let metadata = self.read_file_metadata()?;
            metadata
                .table_metadata_index_node_map
                .get(&table_name.to_lowercase())
                .or_else(|| metadata.table_metadata_index_node_map.get(table_name))
                .cloned()
        };
        let Some(root) = root else {
            return Ok(Vec::new());
        };

        let mut devices = Vec::new();
        self.collect_device_ids_from_node(root, &mut devices)?;
        devices.sort();
        devices.dedup();
        Ok(devices)
    }

    pub fn read_device_metadata(
        &mut self,
        device_id: &str,
        need_chunk_metadata: bool,
    ) -> TsFileResult<HashMap<String, TimeseriesMetadata>> {
        let root = {
            let metadata = self.read_file_metadata()?;
            metadata.metadata_index_node("").cloned()
        };
        let Some(root) = root else {
            return Ok(HashMap::new());
        };

        let Some(measurement_node) = self.find_measurement_root_node(root, device_id)? else {
            return Ok(HashMap::new());
        };

        let mut measurement_offsets = Vec::new();
        self.collect_measurement_metadata_offsets(measurement_node, &mut measurement_offsets)?;

        let mut result = HashMap::with_capacity(measurement_offsets.len());
        for (measurement_id, offset) in measurement_offsets {
            let metadata = self.read_timeseries_metadata_at(offset, need_chunk_metadata)?;
            result.insert(measurement_id, metadata);
        }
        Ok(result)
    }

    pub fn get_measurement(&mut self, device_id: &str) -> TsFileResult<HashMap<String, TSDataType>> {
        let mut result = HashMap::new();
        for timeseries_metadata in self.read_device_metadata(device_id, false)?.into_values() {
            result.insert(
                timeseries_metadata.measurement_id.clone(),
                timeseries_metadata.effective_data_type(),
            );
        }
        Ok(result)
    }

    pub fn get_all_measurements(&mut self) -> TsFileResult<HashMap<String, TSDataType>> {
        let mut result = HashMap::new();
        for device_id in self.get_all_devices()? {
            for (measurement_id, data_type) in self.get_measurement(&device_id)? {
                result.insert(measurement_id, data_type);
            }
        }
        Ok(result)
    }

    pub fn get_full_path_data_type_map(&mut self) -> TsFileResult<HashMap<String, TSDataType>> {
        let mut result = HashMap::new();
        for device_id in self.get_all_devices()? {
            for (measurement_id, data_type) in self.get_measurement(&device_id)? {
                result.insert(format!("{}.{}", device_id, measurement_id), data_type);
            }
        }
        Ok(result)
    }

    pub fn get_device_measurements_map(&mut self) -> TsFileResult<HashMap<String, Vec<String>>> {
        let mut result = HashMap::new();
        for device_id in self.get_all_devices()? {
            let mut measurements: Vec<String> =
                self.get_measurement(&device_id)?.into_keys().collect();
            measurements.sort();
            result.insert(device_id, measurements);
        }
        Ok(result)
    }

    pub fn get_table_schema(&mut self, table_name: &str) -> TsFileResult<Option<TableSchema>> {
        let metadata = self.read_file_metadata()?;
        Ok(metadata.table_schema_map.get(&table_name.to_lowercase()).cloned())
    }

    pub fn get_all_table_schemas(&mut self) -> TsFileResult<Vec<TableSchema>> {
        let metadata = self.read_file_metadata()?;
        let mut schemas: Vec<TableSchema> = metadata.table_schema_map.values().cloned().collect();
        schemas.sort_by(|left, right| left.table_name.cmp(&right.table_name));
        Ok(schemas)
    }

    pub fn get_table_schema_map(&mut self) -> TsFileResult<HashMap<String, TableSchema>> {
        let metadata = self.read_file_metadata()?;
        Ok(metadata.table_schema_map.clone())
    }

    pub fn read_tsblock(
        &mut self,
        device_id: &str,
        measurements: &[String],
        time_range: Option<TimeRange>,
    ) -> TsFileResult<TsBlock> {
        let measurement_types = self.get_measurement(device_id)?;
        let mut columns = Vec::with_capacity(measurements.len());
        let mut timestamps = BTreeSet::new();

        for measurement in measurements {
            let points = if let Some(time_range) = time_range {
                self.read_timeseries_by_index_with_filter(
                    device_id,
                    measurement,
                    Some(&Filter::TimeBetween {
                        min: time_range.min,
                        max: time_range.max,
                    }),
                )?
            } else {
                self.read_timeseries_by_index(device_id, measurement)?
            };
            for point in &points {
                timestamps.insert(point.timestamp);
            }
            columns.push(points);
        }

        let data_types: Vec<TSDataType> = measurements
            .iter()
            .map(|measurement| {
                measurement_types
                    .get(measurement)
                    .copied()
                    .ok_or_else(|| {
                        TsFileError::MeasurementNotFound(format!("{}.{}", device_id, measurement))
                    })
            })
            .collect::<TsFileResult<_>>()?;

        let mut builder = TsBlockBuilder::new(data_types);
        for timestamp in timestamps {
            let row_values = columns
                .iter()
                .map(|points| {
                    points
                        .iter()
                        .find(|point| point.timestamp == timestamp)
                        .map(|point| ColumnValue::from(point.value.clone()))
                        .unwrap_or(ColumnValue::Null)
                })
                .collect();
            builder.declare_position(timestamp, row_values);
        }
        Ok(builder.build())
    }

    /// Read all time-value pairs from a chunk.
    pub fn read_chunk_data(
        chunk_header: &ChunkHeader,
        chunk_data: &[u8],
    ) -> TsFileResult<Vec<TimeValuePair>> {
        ChunkReader::new(chunk_header.clone(), chunk_data.to_vec()).read_all()
    }

    pub fn self_check(&mut self) -> TsFileResult<u64> {
        let meta_offset = {
            let metadata = self.read_file_metadata()?;
            metadata.meta_offset as u64
        };
        let _ = self.read_all_chunks()?;
        if !self.is_complete()? {
            return Err(TsFileError::InvalidFileFormat(
                "TsFile footer is incomplete".to_string(),
            ));
        }
        Ok(meta_offset)
    }

    fn collect_device_ids_from_node(
        &mut self,
        node: MetadataIndexNode,
        devices: &mut Vec<String>,
    ) -> TsFileResult<()> {
        match node.node_type {
            MetadataIndexNodeType::LeafDevice => {
                devices.extend(node.children.into_iter().map(|entry| entry.name));
            }
            MetadataIndexNodeType::InternalDevice => {
                for child in node.children {
                    let child_node = self.read_metadata_index_node_at(child.offset)?;
                    self.collect_device_ids_from_node(child_node, devices)?;
                }
            }
            MetadataIndexNodeType::InternalMeasurement | MetadataIndexNodeType::LeafMeasurement => {
                return Err(TsFileError::InvalidFileFormat(
                    "measurement index node cannot be used as device root".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn collect_measurement_metadata_offsets(
        &mut self,
        node: MetadataIndexNode,
        measurement_offsets: &mut Vec<(String, i64)>,
    ) -> TsFileResult<()> {
        match node.node_type {
            MetadataIndexNodeType::LeafMeasurement => {
                measurement_offsets.extend(
                    node.children
                        .into_iter()
                        .map(|entry| (entry.name, entry.offset)),
                );
            }
            MetadataIndexNodeType::InternalMeasurement => {
                for child in node.children {
                    let child_node = self.read_metadata_index_node_at(child.offset)?;
                    self.collect_measurement_metadata_offsets(child_node, measurement_offsets)?;
                }
            }
            MetadataIndexNodeType::InternalDevice | MetadataIndexNodeType::LeafDevice => {
                return Err(TsFileError::InvalidFileFormat(
                    "device index node cannot be used as measurement root".to_string(),
                ));
            }
        }
        Ok(())
    }
}

/// Organizes chunk data per device and measurement.
pub type DeviceChunkMap = HashMap<String, HashMap<String, Vec<TimeValuePair>>>;
