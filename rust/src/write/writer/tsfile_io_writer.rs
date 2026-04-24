// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TsFileIOWriter: low-level file format writer.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use crate::common::constant::TsFileConstant;
use crate::common::enums::MetadataIndexNodeType;
use crate::error::TsFileResult;
use crate::file::header::ChunkGroupHeader;
use crate::file::meta_marker::MetaMarker;
use crate::file::metadata::chunk_metadata::{ChunkGroupMetadata, ChunkMetadata};
use crate::file::metadata::metadata_index_node::{MetadataIndexEntry, MetadataIndexNode};
use crate::file::metadata::timeseries_metadata::TimeseriesMetadata;
use crate::file::metadata::tsfile_metadata::TsFileMetadata;
use crate::utils::bloom_filter::BloomFilter;

/// Low-level TsFile writer: writes the file format including magic, version, chunks, and footer.
///
/// Mirrors Java's TsFileIOWriter.
pub struct TsFileIOWriter {
    /// Underlying buffered file writer.
    writer: BufWriter<File>,
    /// Current write position.
    position: u64,
    /// All chunk group metadata collected during writing.
    pub chunk_group_metadata_list: Vec<ChunkGroupMetadata>,
    /// Current chunk group being written.
    current_device_id: Option<String>,
    /// Current chunk metadata list within the group.
    current_chunk_metadata_list: Vec<ChunkMetadata>,
    /// Path count for bloom filter.
    path_count: usize,
    /// Position marked for recoverable append/reset operations.
    marked_position: Option<u64>,
    /// Whether this writer still accepts writes.
    can_write: bool,
    /// Whether a complete footer has already been written.
    has_footer: bool,
}

impl TsFileIOWriter {
    /// Create a new TsFileIOWriter, writing to the given path.
    pub fn new<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        let mut writer = TsFileIOWriter {
            writer: BufWriter::new(file),
            position: 0,
            chunk_group_metadata_list: Vec::new(),
            current_device_id: None,
            current_chunk_metadata_list: Vec::new(),
            path_count: 0,
            marked_position: None,
            can_write: true,
            has_footer: false,
        };
        writer.start_file()?;
        Ok(writer)
    }

    /// Write the TsFile magic string and version number.
    fn start_file(&mut self) -> TsFileResult<()> {
        let magic = TsFileConstant::MAGIC_STRING.as_bytes();
        self.write_bytes(magic)?;
        self.write_bytes(&[TsFileConstant::VERSION_NUMBER])?;
        Ok(())
    }

    fn write_bytes(&mut self, data: &[u8]) -> TsFileResult<()> {
        self.ensure_can_write()?;
        self.writer.write_all(data)?;
        self.position += data.len() as u64;
        Ok(())
    }

    fn write_byte(&mut self, byte: u8) -> TsFileResult<()> {
        self.write_bytes(&[byte])
    }

    /// Get current write position.
    pub fn position(&self) -> u64 {
        self.position
    }

    fn ensure_can_write(&self) -> TsFileResult<()> {
        if self.can_write {
            Ok(())
        } else {
            Err(crate::error::TsFileError::WriteError(
                "TsFileIOWriter is closed for writing".to_string(),
            ))
        }
    }

    /// Flush buffered bytes to the underlying file.
    pub fn flush(&mut self) -> TsFileResult<()> {
        self.writer.flush()?;
        Ok(())
    }

    /// Mark the current logical write position for later reset.
    pub fn mark(&mut self) {
        self.marked_position = Some(self.position);
    }

    /// Return the currently marked position, if any.
    pub fn marked_position(&self) -> Option<u64> {
        self.marked_position
    }

    /// Reset the writer to the last mark and truncate bytes written after it.
    pub fn reset_to_mark(&mut self) -> TsFileResult<()> {
        let mark = self.marked_position.ok_or_else(|| {
            crate::error::TsFileError::WriteError("no marked position to reset".to_string())
        })?;
        self.truncate(mark)
    }

    /// Truncate the underlying file and move the logical write cursor to `offset`.
    pub fn truncate(&mut self, offset: u64) -> TsFileResult<()> {
        self.ensure_can_write()?;
        self.writer.flush()?;
        self.writer.get_mut().set_len(offset)?;
        self.writer.seek(SeekFrom::Start(offset))?;
        self.position = offset;
        self.has_footer = false;
        Ok(())
    }

    /// Close the writer without appending a metadata footer.
    pub fn close_without_footer(&mut self) -> TsFileResult<()> {
        if self.can_write {
            self.writer.flush()?;
            self.can_write = false;
        }
        Ok(())
    }

    /// Whether this writer can still accept bytes.
    pub fn can_write(&self) -> bool {
        self.can_write
    }

    /// Whether a complete metadata/footer section has been written.
    pub fn has_footer(&self) -> bool {
        self.has_footer
    }

    /// Start writing a chunk group for a device.
    pub fn start_chunk_group(&mut self, device_id: String) -> TsFileResult<()> {
        log::debug!("Starting chunk group for device: {}", device_id);
        let header = ChunkGroupHeader::new(device_id.clone());
        let mut buf = Vec::new();
        header.serialize(&mut buf)?;
        self.write_bytes(&buf)?;
        self.current_device_id = Some(device_id);
        self.current_chunk_metadata_list.clear();
        Ok(())
    }

    /// End writing the current chunk group.
    /// Note: Java's TsFileIOWriter does NOT write a SEPARATOR here;
    /// the SEPARATOR is written only once in end_file() before the metadata section.
    pub fn end_chunk_group(&mut self) -> TsFileResult<()> {
        if let Some(device_id) = self.current_device_id.take() {
            // Flush and record metadata
            let chunk_list = std::mem::take(&mut self.current_chunk_metadata_list);
            self.chunk_group_metadata_list
                .push(ChunkGroupMetadata::new(device_id, chunk_list));
            self.writer.flush()?;
        }
        Ok(())
    }

    /// Write a chunk (header + page data) to the file.
    pub fn write_chunk(
        &mut self,
        chunk_data: &[u8],
        chunk_metadata: ChunkMetadata,
    ) -> TsFileResult<()> {
        self.write_bytes(chunk_data)?;
        self.current_chunk_metadata_list.push(chunk_metadata);
        self.path_count += 1;
        Ok(())
    }

    /// Write a separator marker.
    pub fn write_separator(&mut self) -> TsFileResult<u64> {
        self.write_byte(MetaMarker::SEPARATOR)?;
        Ok(self.position)
    }

    /// End the file: write all metadata and the file footer.
    /// Format matches Java exactly: [SEPARATOR] [TimeseriesMetadata...] [MetadataIndexNode...] [TsFileMetadata] [metaSize(i32)] [Magic]
    pub fn end_file(&mut self) -> TsFileResult<()> {
        self.ensure_can_write()?;
        if self.current_device_id.is_some() {
            self.end_chunk_group()?;
        }
        if self.has_footer {
            return Ok(());
        }
        // Record the offset of the metadata section (before SEPARATOR)
        // This matches Java's: long metaOffset = out.getPosition();
        let meta_offset = self.position;

        // Write SEPARATOR marker before the metadata section
        // This matches Java's readChunkMetadataAndConstructIndexTree line 472
        self.write_byte(MetaMarker::SEPARATOR)?;

        // Build metadata index tree from chunk group metadata
        // Java iterates through ALL series (not grouped by device)
        let all_timeseries = self.build_all_timeseries_metadata()?;

        // FIRST: Write all TimeseriesMetadata and their chunk metadata
        // This matches Java's TSMIterator which writes all series metadata before index nodes
        let mut device_measurement_entries: HashMap<String, Vec<(String, i64)>> = HashMap::new();
        
        for (device_id, measurement_id, ts_metadata) in &all_timeseries {
            // Record the offset where this TimeseriesMetadata will be written
            let ts_offset = self.position;
            
            // Serialize TimeseriesMetadata (includes chunk metadata)
            let mut buf = Vec::new();
            ts_metadata.serialize(&mut buf)?;
            
            // Write to file
            self.write_bytes(&buf)?;
            
            // Add to measurement entries for this device's index
            device_measurement_entries
                .entry(device_id.clone())
                .or_default()
                .push((measurement_id.clone(), ts_offset as i64));
        }

        // SECOND: Build measurement-level index tree for each device
        // Java: deviceMetadataIndexMap (Map<IDeviceID, MetadataIndexNode>)
        // MUST always create LEAF_MEASUREMENT nodes, even for single measurement!
        let mut device_to_node_offset: HashMap<String, i64> = HashMap::new();
        
        for (device_id, measurement_entries) in &device_measurement_entries {
            // Record position before writing the node
            let node_position = self.position;
            
            // Always build measurement index node
            let _device_index_node = self.write_measurement_index_tree(
                measurement_entries,
                MetadataIndexNodeType::InternalMeasurement,
            )?;
            
            // Record the offset where this node was written
            device_to_node_offset.insert(device_id.clone(), node_position as i64);
        }
        
        // Build root device index node (LEAF_DEVICE for V3 compatibility)
        // For V3, this will be inlined in TsFileMetadata
        // Using LEAF_DEVICE so Java can extract deviceId from DeviceMetadataIndexEntry
        let mut root_device_node = MetadataIndexNode::new(
            MetadataIndexNodeType::LeafDevice,
            0, // Will be set when serialized inline
        );
        for (device_id, node_offset) in device_to_node_offset {
            root_device_node.add_child(MetadataIndexEntry::new(device_id, node_offset));
        }
        
        // Set end offset for root node
        root_device_node.end_offset = self.position as i64;

        let mut bloom = BloomFilter::new(0.05, self.path_count.max(1));
        for group in &self.chunk_group_metadata_list {
            for chunk in &group.chunk_metadata_list {
                bloom.add(&format!("{}.{}", group.device_id, chunk.measurement_uid));
            }
        }

        // Build TsFileMetadata
        let mut file_metadata = TsFileMetadata::new();
        file_metadata.meta_offset = meta_offset as i64;
        file_metadata.bloom_filter = Some(bloom);
        file_metadata.add_property("encryptLevel".to_string(), "0".to_string());
        file_metadata.add_property(
            "encryptType".to_string(),
            "org.apache.tsfile.encrypt.UNENCRYPTED".to_string(),
        );
        file_metadata.add_property("encryptKey".to_string(), String::new());

        // Empty table name is used by Java for tree-model device metadata.
        file_metadata.add_table_metadata_index_node(String::new(), root_device_node);

        // Serialize the TsFileMetadata
        let mut meta_buf = Vec::new();
        file_metadata.serialize(&mut meta_buf)?;
        self.write_bytes(&meta_buf)?;

        // Write metadata size (i32 big-endian)
        // This matches Java's ReadWriteIOUtils.write(size, out.wrapAsStream())
        let meta_size = meta_buf.len() as i32;
        self.write_bytes(&meta_size.to_be_bytes())?;

        // Write closing magic string
        // This matches Java's: out.write(MAGIC_STRING_BYTES);
        let magic = TsFileConstant::MAGIC_STRING.as_bytes();
        self.write_bytes(magic)?;

        // Flush
        self.writer.flush()?;
        self.has_footer = true;
        self.can_write = false;
        log::debug!("TsFile ended successfully, metadata size: {}", meta_size);
        Ok(())
    }

    /// Build a flat list of all timeseries metadata, sorted by device then measurement.
    /// Returns: (device_id, measurement_id, TimeseriesMetadata)
    /// This matches Java's TSMIterator which iterates through ALL series.
    fn build_all_timeseries_metadata(
        &self,
    ) -> TsFileResult<Vec<(String, String, TimeseriesMetadata)>> {
        // Group chunk metadata by (device, measurement)
        let mut device_measurement_chunks: HashMap<(String, String), Vec<ChunkMetadata>> =
            HashMap::new();

        for group in &self.chunk_group_metadata_list {
            for chunk_meta in &group.chunk_metadata_list {
                device_measurement_chunks
                    .entry((group.device_id.clone(), chunk_meta.measurement_uid.clone()))
                    .or_default()
                    .push(chunk_meta.clone());
            }
        }

        // Convert to timeseries metadata list
        let mut result = Vec::new();
        for ((device_id, measurement_id), chunks) in device_measurement_chunks {
            // Build TimeseriesMetadata for this series
            // time_series_metadata_type: bit 0-5 indicates if chunk has statistics
            // 1 = has statistics, 0 = no statistics (single chunk case)
            let has_statistics = if chunks.len() > 1 { 1 } else { 0 };
            let mut ts_meta = TimeseriesMetadata::new(
                has_statistics,
                measurement_id.clone(),
                chunks[0].data_type,
                chunks[0].statistics.clone(),
            );
            ts_meta.chunk_metadata_list = chunks;
            result.push((device_id, measurement_id, ts_meta));
        }

        // Sort by device then measurement (Java uses TreeMap which sorts naturally)
        result.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

        Ok(result)
    }

    /// Write measurement-level index tree for a device.
    /// Returns the root MetadataIndexNode (which is already written to file).
    /// For simplicity, writes a single LEAF_MEASUREMENT node for all measurements.
    fn write_measurement_index_tree(
        &mut self,
        entries: &[(String, i64)],
        _node_type: MetadataIndexNodeType,
    ) -> TsFileResult<MetadataIndexNode> {
        if entries.is_empty() {
            return Ok(MetadataIndexNode::new(
                MetadataIndexNodeType::LeafMeasurement,
                self.position as i64,
            ));
        }

        // Write a single LEAF_MEASUREMENT node containing all measurements
        let mut leaf_node = MetadataIndexNode::new(
            MetadataIndexNodeType::LeafMeasurement,
            self.position as i64,
        );

        for (measurement_id, offset) in entries {
            leaf_node.add_child(MetadataIndexEntry::new(
                measurement_id.clone(),
                *offset,
            ));
        }

        // Write the leaf node
        let mut node_buf = Vec::new();
        leaf_node.serialize(&mut node_buf)?;
        self.write_bytes(&node_buf)?;
        
        // Update end_offset
        leaf_node.end_offset = self.position as i64;

        Ok(leaf_node)
    }

}
