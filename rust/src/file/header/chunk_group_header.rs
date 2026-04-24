// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! ChunkGroupHeader - marks the start of a chunk group for a device.

use std::io::{Read, Write};

use crate::error::TsFileResult;
use crate::file::meta_marker::MetaMarker;
use crate::utils::ReadWriteIOUtils;

/// The header for a chunk group (one device's data section).
///
/// Format: marker(1) + deviceId(varint-string)
///
/// Mirrors Java's ChunkGroupHeader.
#[derive(Debug, Clone)]
pub struct ChunkGroupHeader {
    /// The device ID string.
    pub device_id: String,
}

impl ChunkGroupHeader {
    pub fn new(device_id: String) -> Self {
        ChunkGroupHeader { device_id }
    }

    /// Calculate the serialized size.
    pub fn serialized_size(device_id: &str) -> usize {
        use crate::utils::ReadWriteForEncodingUtils;
        let bytes = device_id.as_bytes();
        let len = bytes.len() as i32;
        1 // marker byte
        + ReadWriteForEncodingUtils::var_int_size(len) // length varint
        + bytes.len() // device id bytes
    }

    /// Serialize to writer (including the CHUNK_GROUP_HEADER marker).
    pub fn serialize<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        let mut written = 0;
        written += ReadWriteIOUtils::write_byte(MetaMarker::CHUNK_GROUP_HEADER, writer)?;
        written += ReadWriteIOUtils::write_var_int_string(&self.device_id, writer)?;
        Ok(written)
    }

    /// Deserialize from reader (marker has already been read).
    pub fn deserialize<R: Read>(reader: &mut R) -> TsFileResult<Self> {
        let device_id = ReadWriteIOUtils::read_var_int_string(reader)?;
        Ok(ChunkGroupHeader { device_id })
    }
}
