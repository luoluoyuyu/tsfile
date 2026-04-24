// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Chunk metadata and chunk group metadata.

use std::io::{Read, Write};

use crate::common::enums::{CompressionType, TSDataType, TSEncoding};
use crate::error::TsFileResult;
use crate::file::metadata::statistics::Statistics;
use crate::utils::{ReadWriteForEncodingUtils, ReadWriteIOUtils};

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
        let offset = ReadWriteIOUtils::read_i64(reader)?;
        let statistics = Statistics::deserialize(reader, data_type)?;
        Ok(ChunkMetadata {
            measurement_uid,
            offset_of_chunk_header: offset,
            data_type,
            statistics,
            deleted: false,
            mask: 0,
        })
    }
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
}
