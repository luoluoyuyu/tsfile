// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use std::io::Write;

use crate::common::enums::{CompressionType, TSDataType, TSEncoding};
use crate::error::TsFileResult;
use crate::file::metadata::statistics::Statistics;
use crate::utils::read_write_io_utils::Binary;
use crate::write::chunk::ChunkWriter;

pub struct ChunkWriterImpl {
    inner: ChunkWriter,
}

impl ChunkWriterImpl {
    pub fn new(measurement_id: String, data_type: TSDataType, encoding: TSEncoding, compression: CompressionType) -> Self {
        ChunkWriterImpl { inner: ChunkWriter::new(measurement_id, data_type, encoding, compression) }
    }

    pub fn inner(&self) -> &ChunkWriter { &self.inner }
    pub fn inner_mut(&mut self) -> &mut ChunkWriter { &mut self.inner }
    pub fn into_inner(self) -> ChunkWriter { self.inner }
    pub fn write_bool(&mut self, timestamp: i64, value: bool) -> TsFileResult<()> { self.inner.write_bool(timestamp, value) }
    pub fn write_i32(&mut self, timestamp: i64, value: i32) -> TsFileResult<()> { self.inner.write_i32(timestamp, value) }
    pub fn write_i64(&mut self, timestamp: i64, value: i64) -> TsFileResult<()> { self.inner.write_i64(timestamp, value) }
    pub fn write_f32(&mut self, timestamp: i64, value: f32) -> TsFileResult<()> { self.inner.write_f32(timestamp, value) }
    pub fn write_f64(&mut self, timestamp: i64, value: f64) -> TsFileResult<()> { self.inner.write_f64(timestamp, value) }
    pub fn write_binary(&mut self, timestamp: i64, value: Binary) -> TsFileResult<()> { self.inner.write_binary(timestamp, value) }
    pub fn write_to<W: Write>(&mut self, writer: &mut W) -> TsFileResult<(usize, usize)> { self.inner.write_to(writer) }
    pub fn statistics(&self) -> &Statistics { self.inner.statistics() }
    pub fn has_data(&self) -> bool { self.inner.has_data() }
}
