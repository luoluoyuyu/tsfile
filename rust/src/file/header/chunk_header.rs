// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! ChunkHeader - metadata header for a data chunk.

use std::io::{Read, Write};

use crate::common::enums::{CompressionType, TSDataType, TSEncoding};
use crate::error::TsFileResult;
use crate::file::meta_marker::MetaMarker;
use crate::utils::{ReadWriteForEncodingUtils, ReadWriteIOUtils};

/// The header for a data chunk.
///
/// Format:
///   chunkType(1) + measurementId(varint-string) + dataSize(uvarint) +
///   dataType(1) + compressionType(1) + encodingType(1)
///
/// Mirrors Java's ChunkHeader.
#[derive(Debug, Clone)]
pub struct ChunkHeader {
    /// Chunk type byte (encodes page count and alignment type).
    pub chunk_type: u8,
    /// Measurement ID (sensor name).
    pub measurement_id: String,
    /// Size of all page data in this chunk (bytes).
    pub data_size: u32,
    /// Data type.
    pub data_type: TSDataType,
    /// Compression type.
    pub compression_type: CompressionType,
    /// Encoding type.
    pub encoding_type: TSEncoding,
    /// Number of pages (not serialized).
    pub num_of_pages: u32,
    /// Cached serialized size.
    serialized_size: usize,
}

impl ChunkHeader {
    /// Create a new ChunkHeader.
    pub fn new(
        measurement_id: String,
        data_size: u32,
        data_type: TSDataType,
        compression_type: CompressionType,
        encoding_type: TSEncoding,
        num_of_pages: u32,
    ) -> Self {
        Self::new_with_mask(
            measurement_id,
            data_size,
            data_type,
            compression_type,
            encoding_type,
            num_of_pages,
            0,
        )
    }

    /// Create with a mask for aligned series (TIME/VALUE chunk).
    pub fn new_with_mask(
        measurement_id: String,
        data_size: u32,
        data_type: TSDataType,
        compression_type: CompressionType,
        encoding_type: TSEncoding,
        num_of_pages: u32,
        mask: u8,
    ) -> Self {
        let chunk_type = if num_of_pages <= 1 {
            MetaMarker::ONLY_ONE_PAGE_CHUNK_HEADER | mask
        } else {
            MetaMarker::CHUNK_HEADER | mask
        };
        let serialized_size =
            Self::calculate_serialized_size(&measurement_id, data_size);
        ChunkHeader {
            chunk_type,
            measurement_id,
            data_size,
            data_type,
            compression_type,
            encoding_type,
            num_of_pages,
            serialized_size,
        }
    }

    /// Calculate the exact serialized size.
    pub fn calculate_serialized_size(measurement_id: &str, data_size: u32) -> usize {
        let id_bytes = measurement_id.as_bytes().len();
        1 // chunkType
        + ReadWriteForEncodingUtils::var_int_size(id_bytes as i32) // measurementId length
        + id_bytes // measurementId
        + ReadWriteForEncodingUtils::u_var_int_size(data_size) // dataSize
        + TSDataType::serialized_size() // dataType
        + CompressionType::serialized_size() // compressionType
        + TSEncoding::serialized_size() // encodingType
    }

    /// Get the cached serialized size.
    pub fn serialized_size(&self) -> usize {
        self.serialized_size
    }

    /// Serialize to writer.
    pub fn serialize<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        let mut written = 0;
        written += ReadWriteIOUtils::write_byte(self.chunk_type, writer)?;
        written += ReadWriteIOUtils::write_var_int_string(&self.measurement_id, writer)?;
        written += ReadWriteForEncodingUtils::write_unsigned_var_int(self.data_size, writer)?;
        written += ReadWriteIOUtils::write_byte(self.data_type.serialize(), writer)?;
        written += ReadWriteIOUtils::write_byte(self.compression_type.serialize(), writer)?;
        written += ReadWriteIOUtils::write_byte(self.encoding_type.serialize(), writer)?;
        Ok(written)
    }

    /// Deserialize from reader (chunk_type marker has already been read).
    pub fn deserialize<R: Read>(reader: &mut R, chunk_type: u8) -> TsFileResult<Self> {
        let measurement_id = ReadWriteIOUtils::read_var_int_string(reader)?;
        let data_size = ReadWriteForEncodingUtils::read_unsigned_var_int(reader)?;
        let data_type_byte = ReadWriteIOUtils::read_byte(reader)?;
        let data_type = TSDataType::deserialize(data_type_byte)?;
        let compression_byte = ReadWriteIOUtils::read_byte(reader)?;
        let compression_type = CompressionType::deserialize(compression_byte)?;
        let encoding_byte = ReadWriteIOUtils::read_byte(reader)?;
        let encoding_type = TSEncoding::deserialize(encoding_byte)?;
        let serialized_size = Self::calculate_serialized_size(&measurement_id, data_size);
        Ok(ChunkHeader {
            chunk_type,
            measurement_id,
            data_size,
            data_type,
            compression_type,
            encoding_type,
            num_of_pages: 0, // not serialized, set from context
            serialized_size,
        })
    }
}

impl std::fmt::Display for ChunkHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CHUNK_HEADER{{measurementID='{}', dataSize={}, dataType={}, compressionType={}, encodingType={}, numOfPages={}, serializedSize={}}}",
            self.measurement_id,
            self.data_size,
            self.data_type,
            self.compression_type,
            self.encoding_type,
            self.num_of_pages,
            self.serialized_size
        )
    }
}
