// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Chunk metadata and chunk group metadata.

use std::io::{Read, Write};

use crate::common::enums::TSDataType;
use crate::error::TsFileResult;
use crate::file::metadata::device_id::DeviceId;
use crate::file::metadata::statistics::Statistics;
use crate::file::metadata::traits::{ChunkMetadataView, Metadata};
use crate::utils::ReadWriteIOUtils;

/// Metadata about a single chunk.
///
/// Mirrors Java's ChunkMetadata.
#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    /// The measurement ID (sensor name).
    pub measurement_uid: String,
    /// Offset of the chunk header in the TsFile.
    pub offset_of_chunk_header: i64,
    /// The data type of this chunk.
    pub data_type: TSDataType,
    /// Statistics for this chunk.
    pub statistics: Statistics,
    /// Whether this chunk is from the deletion period (legacy unseq file).
    pub deleted: bool,
    /// Mask byte indicating chunk type (multi-page vs. single-page, aligned, etc.).
    pub mask: u8,
    /// Whether this metadata has deletion/modification information attached.
    pub modified: bool,
    /// Whether this chunk belongs to a sequence file.
    pub seq: bool,
    /// Version propagated from modification/deletion metadata.
    pub version: i64,
    /// Whether the chunk metadata is closed and immutable.
    pub closed: bool,
    /// Data type after schema evolution; None means unchanged.
    pub new_type: Option<TSDataType>,
    /// Whether statistics cannot be trusted after type modification.
    pub data_type_modified_and_cannot_use_statistics: bool,
}

impl ChunkMetadata {
    /// Create a new ChunkMetadata.
    pub fn new(
        measurement_uid: String,
        data_type: TSDataType,
        offset_of_chunk_header: i64,
        statistics: Statistics,
    ) -> Self {
        ChunkMetadata {
            measurement_uid,
            offset_of_chunk_header,
            data_type,
            statistics,
            deleted: false,
            mask: 0,
            modified: false,
            seq: true,
            version: 0,
            closed: false,
            new_type: None,
            data_type_modified_and_cannot_use_statistics: false,
        }
    }

    /// Serialize to writer.
    /// If include_statistics is false, only serialize offsetOfChunkHeader (for timeSeriesMetadataType == 0).
    pub fn serialize<W: Write>(&self, writer: &mut W, include_statistics: bool) -> TsFileResult<usize> {
        let mut written = 0;
        written += ReadWriteIOUtils::write_i64(self.offset_of_chunk_header, writer)?;
        if include_statistics {
            written += self.statistics.serialize(writer)?;
        }
        Ok(written)
    }

    /// Deserialize (measurement_uid and data_type are passed in from TimeseriesMetadata).
    pub fn deserialize<R: Read>(
        reader: &mut R,
        measurement_uid: String,
        data_type: TSDataType,
    ) -> TsFileResult<Self> {
        Self::deserialize_with_statistics(reader, measurement_uid, data_type, true)
    }

    pub fn deserialize_with_statistics<R: Read>(
        reader: &mut R,
        measurement_uid: String,
        data_type: TSDataType,
        include_statistics: bool,
    ) -> TsFileResult<Self> {
        let offset = ReadWriteIOUtils::read_i64(reader)?;
        let statistics = if include_statistics {
            Statistics::deserialize(reader, data_type)?
        } else {
            Statistics::new(data_type)
        };
        Ok(ChunkMetadata {
            measurement_uid,
            offset_of_chunk_header: offset,
            data_type,
            statistics,
            deleted: false,
            mask: 0,
            modified: false,
            seq: true,
            version: 0,
            closed: false,
            new_type: None,
            data_type_modified_and_cannot_use_statistics: false,
        })
    }

    pub fn statistics(&self) -> &Statistics {
        &self.statistics
    }

    pub fn start_time(&self) -> i64 {
        self.statistics.start_time
    }

    pub fn end_time(&self) -> i64 {
        self.statistics.end_time
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn set_modified(&mut self, modified: bool) {
        self.modified = modified;
    }

    pub fn is_seq(&self) -> bool {
        self.seq
    }

    pub fn set_seq(&mut self, seq: bool) {
        self.seq = seq;
    }

    pub fn version(&self) -> i64 {
        self.version
    }

    pub fn set_version(&mut self, version: i64) {
        self.version = version;
    }

    pub fn set_closed(&mut self, closed: bool) {
        self.closed = closed;
    }

    pub fn set_new_type(&mut self, data_type: TSDataType) {
        self.new_type = Some(data_type);
    }

    pub fn effective_data_type(&self) -> TSDataType {
        self.new_type.unwrap_or(self.data_type)
    }

    pub fn is_data_type_modified_and_cannot_use_statistics(&self) -> bool {
        self.data_type_modified_and_cannot_use_statistics
    }

    pub fn set_data_type_modified_and_cannot_use_statistics(&mut self, value: bool) {
        self.data_type_modified_and_cannot_use_statistics = value;
    }

    pub fn type_match(&self, data_type: TSDataType) -> bool {
        self.effective_data_type().is_compatible(&data_type)
    }
}

impl Metadata for ChunkMetadata {
    fn statistics(&self) -> &Statistics {
        &self.statistics
    }
}

impl ChunkMetadataView for ChunkMetadata {
    fn is_modified(&self) -> bool { self.is_modified() }
    fn set_modified(&mut self, modified: bool) { self.set_modified(modified); }
    fn is_data_type_modified_and_cannot_use_statistics(&self) -> bool { self.is_data_type_modified_and_cannot_use_statistics() }
    fn set_data_type_modified_and_cannot_use_statistics(&mut self, value: bool) { self.set_data_type_modified_and_cannot_use_statistics(value); }
    fn is_seq(&self) -> bool { self.is_seq() }
    fn set_seq(&mut self, seq: bool) { self.set_seq(seq); }
    fn version(&self) -> i64 { self.version() }
    fn set_version(&mut self, version: i64) { self.set_version(version); }
    fn offset_of_chunk_header(&self) -> i64 { self.offset_of_chunk_header }
    fn start_time(&self) -> i64 { self.start_time() }
    fn end_time(&self) -> i64 { self.end_time() }
    fn need_set_chunk_loader(&self) -> bool { true }
    fn set_closed(&mut self, closed: bool) { self.set_closed(closed); }
    fn data_type(&self) -> TSDataType { self.data_type }
    fn new_type(&self) -> Option<TSDataType> { self.new_type }
    fn set_new_type(&mut self, new_type: TSDataType) -> TsFileResult<()> { self.set_new_type(new_type); Ok(()) }
    fn measurement_uid(&self) -> &str { &self.measurement_uid }
    fn set_measurement_uid(&mut self, measurement_uid: String) -> TsFileResult<()> { self.measurement_uid = measurement_uid; Ok(()) }
    fn serialize_to<W: Write>(&self, writer: &mut W, serialize_statistics: bool) -> TsFileResult<usize> { self.serialize(writer, serialize_statistics) }
    fn mask(&self) -> u8 { self.mask }
}

/// Metadata about a chunk group (one device's data in a flush).
///
/// Mirrors Java's ChunkGroupMetadata.
#[derive(Debug, Clone)]
pub struct ChunkGroupMetadata {
    /// The device ID string.
    pub device_id: String,
    /// All chunk metadata in this group.
    pub chunk_metadata_list: Vec<ChunkMetadata>,
}

impl ChunkGroupMetadata {
    pub fn new(device_id: String, chunk_metadata_list: Vec<ChunkMetadata>) -> Self {
        ChunkGroupMetadata {
            device_id,
            chunk_metadata_list,
        }
    }

    pub fn device(&self) -> DeviceId {
        DeviceId::plain(self.device_id.clone())
    }

    pub fn set_device(&mut self, device_id: impl Into<String>) {
        self.device_id = device_id.into();
    }

    pub fn chunk_metadata_list(&self) -> &[ChunkMetadata] {
        &self.chunk_metadata_list
    }

    pub fn chunk_metadata_list_mut(&mut self) -> &mut [ChunkMetadata] {
        &mut self.chunk_metadata_list
    }

    pub fn add_chunk_metadata(&mut self, chunk_metadata: ChunkMetadata) {
        self.chunk_metadata_list.push(chunk_metadata);
    }

    pub fn is_empty(&self) -> bool {
        self.chunk_metadata_list.is_empty()
    }
}
