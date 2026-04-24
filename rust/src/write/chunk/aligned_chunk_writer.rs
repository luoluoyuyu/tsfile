// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Aligned chunk writer primitives: time chunk plus value chunks.

use std::io::Write;

use crate::common::enums::{CompressionType, TSDataType, TSEncoding};
use crate::error::TsFileResult;
use crate::file::header::ChunkHeader;
use crate::write::chunk::ChunkWriter;

pub struct TimeChunkWriter {
    inner: ChunkWriter,
}

impl TimeChunkWriter {
    pub fn new(compression: CompressionType) -> Self {
        TimeChunkWriter {
            inner: ChunkWriter::new(
                "".to_string(),
                TSDataType::Int64,
                TSEncoding::Ts2diff,
                compression,
            ),
        }
    }

    pub fn write_time(&mut self, timestamp: i64) -> TsFileResult<()> {
        self.inner.write_i64(timestamp, timestamp)
    }

    pub fn write_to<W: Write>(&mut self, writer: &mut W) -> TsFileResult<(usize, usize)> {
        self.inner.write_to(writer)
    }
}

pub struct ValueChunkWriter {
    inner: ChunkWriter,
}

impl ValueChunkWriter {
    pub fn new(
        measurement_id: String,
        data_type: TSDataType,
        encoding: TSEncoding,
        compression: CompressionType,
    ) -> Self {
        ValueChunkWriter {
            inner: ChunkWriter::new(measurement_id, data_type, encoding, compression),
        }
    }

    pub fn inner_mut(&mut self) -> &mut ChunkWriter {
        &mut self.inner
    }

    pub fn write_to<W: Write>(&mut self, writer: &mut W) -> TsFileResult<(usize, usize)> {
        self.inner.write_to(writer)
    }
}

pub struct AlignedChunkWriter {
    pub time_writer: TimeChunkWriter,
    pub value_writers: Vec<ValueChunkWriter>,
}

impl AlignedChunkWriter {
    pub fn new(time_compression: CompressionType, value_writers: Vec<ValueChunkWriter>) -> Self {
        AlignedChunkWriter {
            time_writer: TimeChunkWriter::new(time_compression),
            value_writers,
        }
    }

    pub fn chunk_header_for_time(data_size: u32, compression: CompressionType) -> ChunkHeader {
        ChunkHeader::new_with_mask(
            "".to_string(),
            data_size,
            TSDataType::Int64,
            compression,
            TSEncoding::Ts2diff,
            1,
            crate::file::meta_marker::MetaMarker::TIME_CHUNK_HEADER,
        )
    }
}
