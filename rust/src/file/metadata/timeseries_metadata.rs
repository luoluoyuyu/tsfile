// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TimeseriesMetadata - per-timeseries metadata stored in the file footer.

use std::io::{Read, Write};

use crate::common::enums::TSDataType;
use crate::error::TsFileResult;
use crate::file::metadata::chunk_metadata::ChunkMetadata;
use crate::file::metadata::statistics::Statistics;
use crate::utils::{ReadWriteForEncodingUtils, ReadWriteIOUtils};

/// Metadata for a single timeseries, stored in the TsFile footer.
///
/// Mirrors Java's TimeseriesMetadata.
#[derive(Debug, Clone)]
pub struct TimeseriesMetadata {
    /// Type marker byte (indicates chunk type and page count).
    pub time_series_metadata_type: u8,
    /// Measurement ID.
    pub measurement_id: String,
    /// Data type.
    pub data_type: TSDataType,
    /// Data size (sum of all chunk data sizes).
    pub data_size_of_chunks: u32,
    /// Statistics for the whole timeseries.
    pub statistics: Statistics,
    /// Cached chunk metadata list (may be loaded lazily).
    pub chunk_metadata_list: Vec<ChunkMetadata>,
}

impl TimeseriesMetadata {
    pub fn new(
        time_series_metadata_type: u8,
        measurement_id: String,
        data_type: TSDataType,
        statistics: Statistics,
    ) -> Self {
        TimeseriesMetadata {
            time_series_metadata_type,
            measurement_id,
            data_type,
            data_size_of_chunks: 0,
            statistics,
            chunk_metadata_list: Vec::new(),
        }
    }

    /// Serialize to writer, including chunk metadata list.
    pub fn serialize<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        let mut written = 0;
        written += ReadWriteIOUtils::write_byte(self.time_series_metadata_type, writer)?;
        written += ReadWriteIOUtils::write_var_int_string(&self.measurement_id, writer)?;
        written += ReadWriteIOUtils::write_byte(self.data_type.serialize(), writer)?;
        
        // First serialize all chunk metadata to a buffer to calculate size
        let mut chunk_buf = Vec::new();
        for chunk_meta in &self.chunk_metadata_list {
            // Only serialize statistics in ChunkMetadata if time_series_metadata_type != 0
            // This matches Java's logic: if (timeSeriesMetadataType & 0x3F) != 0
            chunk_meta.serialize(&mut chunk_buf, (self.time_series_metadata_type & 0x3F) != 0)?;
        }
        let chunk_data_size = chunk_buf.len() as u32;
        
        // Write chunk metadata size
        written += ReadWriteForEncodingUtils::write_unsigned_var_int(chunk_data_size, writer)?;
        
        // Write statistics
        written += self.statistics.serialize(writer)?;
        
        // Write chunk metadata bytes
        written += chunk_buf.len();
        writer.write_all(&chunk_buf)?;
        
        Ok(written)
    }

    /// Deserialize from reader.
    pub fn deserialize<R: Read>(reader: &mut R) -> TsFileResult<Self> {
        let type_byte = ReadWriteIOUtils::read_byte(reader)?;
        let measurement_id = ReadWriteIOUtils::read_var_int_string(reader)?;
        let data_type_byte = ReadWriteIOUtils::read_byte(reader)?;
        let data_type = TSDataType::deserialize(data_type_byte)?;
        let data_size = ReadWriteForEncodingUtils::read_unsigned_var_int(reader)?;
        let statistics = Statistics::deserialize(reader, data_type)?;
        Ok(TimeseriesMetadata {
            time_series_metadata_type: type_byte,
            measurement_id,
            data_type,
            data_size_of_chunks: data_size,
            statistics,
            chunk_metadata_list: Vec::new(),
        })
    }
}
