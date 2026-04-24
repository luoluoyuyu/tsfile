// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TimeseriesMetadata - per-timeseries metadata stored in the file footer.

use std::io::{Read, Write};

use crate::common::enums::TSDataType;
use crate::error::TsFileResult;
use crate::file::metadata::chunk_metadata::ChunkMetadata;
use crate::file::metadata::statistics::Statistics;
use crate::file::metadata::traits::{Metadata, TimeSeriesMetadataView};
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
    /// Data type after schema evolution; None means unchanged.
    pub new_type: Option<TSDataType>,
    /// Whether this metadata has deletion/modification information attached.
    pub modified: bool,
    /// Whether statistics cannot be trusted after type modification.
    pub data_type_modified_and_cannot_use_statistics: bool,
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
            new_type: None,
            modified: false,
            data_type_modified_and_cannot_use_statistics: false,
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
        Self::deserialize_with_chunks(reader, false)
    }

    pub fn deserialize_with_chunks<R: Read>(
        reader: &mut R,
        need_chunk_metadata: bool,
    ) -> TsFileResult<Self> {
        let type_byte = ReadWriteIOUtils::read_byte(reader)?;
        let measurement_id = ReadWriteIOUtils::read_var_int_string(reader)?;
        let data_type_byte = ReadWriteIOUtils::read_byte(reader)?;
        let data_type = TSDataType::deserialize(data_type_byte)?;
        let data_size = ReadWriteForEncodingUtils::read_unsigned_var_int(reader)?;
        let statistics = Statistics::deserialize(reader, data_type)?;
        let mut chunk_bytes = vec![0u8; data_size as usize];
        reader.read_exact(&mut chunk_bytes)?;
        let mut chunk_metadata_list = Vec::new();
        if need_chunk_metadata {
            let include_statistics = (type_byte & 0x3F) != 0;
            let mut cursor = std::io::Cursor::new(chunk_bytes);
            while (cursor.position() as usize) < data_size as usize {
                chunk_metadata_list.push(ChunkMetadata::deserialize_with_statistics(
                    &mut cursor,
                    measurement_id.clone(),
                    data_type,
                    include_statistics,
                )?);
            }
        }
        Ok(TimeseriesMetadata {
            time_series_metadata_type: type_byte,
            measurement_id,
            data_type,
            data_size_of_chunks: data_size,
            statistics,
            chunk_metadata_list,
            new_type: None,
            modified: false,
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

    pub fn set_new_type(&mut self, data_type: TSDataType) {
        self.new_type = Some(data_type);
    }

    pub fn effective_data_type(&self) -> TSDataType {
        self.new_type.unwrap_or(self.data_type)
    }

    pub fn type_match(&self, data_type: TSDataType) -> bool {
        self.effective_data_type().is_compatible(&data_type)
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn set_modified(&mut self, modified: bool) {
        self.modified = modified;
        for chunk_metadata in &mut self.chunk_metadata_list {
            chunk_metadata.set_modified(modified);
        }
    }

    pub fn is_data_type_modified_and_cannot_use_statistics(&self) -> bool {
        self.data_type_modified_and_cannot_use_statistics
    }

    pub fn set_data_type_modified_and_cannot_use_statistics(&mut self, value: bool) {
        self.data_type_modified_and_cannot_use_statistics = value;
        for chunk_metadata in &mut self.chunk_metadata_list {
            chunk_metadata.set_data_type_modified_and_cannot_use_statistics(value);
        }
    }
}

impl Metadata for TimeseriesMetadata {
    fn statistics(&self) -> &Statistics {
        &self.statistics
    }
}

impl TimeSeriesMetadataView for TimeseriesMetadata {
    fn measurement_id(&self) -> &str { &self.measurement_id }
    fn data_type(&self) -> TSDataType { self.data_type }
    fn is_modified(&self) -> bool { self.is_modified() }
    fn set_modified(&mut self, modified: bool) { self.set_modified(modified); }
    fn is_data_type_modified_and_cannot_use_statistics(&self) -> bool { self.is_data_type_modified_and_cannot_use_statistics() }
    fn set_data_type_modified_and_cannot_use_statistics(&mut self, value: bool) { self.set_data_type_modified_and_cannot_use_statistics(value); }
    fn is_seq(&self) -> bool { self.chunk_metadata_list.iter().all(|chunk| chunk.is_seq()) }
    fn set_seq(&mut self, seq: bool) { for chunk in &mut self.chunk_metadata_list { chunk.set_seq(seq); } }
    fn load_chunk_metadata_list(&self) -> Vec<ChunkMetadata> { self.chunk_metadata_list.clone() }
    fn type_match(&mut self, data_types: &[TSDataType]) -> bool { data_types.first().is_none_or(|data_type| TimeseriesMetadata::type_match(self, *data_type)) }
}
