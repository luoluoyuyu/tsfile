// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use crate::common::enums::{TSDataType, TSEncoding};
use crate::error::TsFileResult;
use crate::utils::read_write_io_utils::Binary;
use crate::write::chunk::page_writer::{EncodedPage, PageWriter};

pub struct ValuePageWriter {
    inner: PageWriter,
}

impl ValuePageWriter {
    pub fn new(data_type: TSDataType, encoding: TSEncoding, max_points: usize) -> Self {
        ValuePageWriter { inner: PageWriter::new(data_type, encoding, max_points) }
    }

    pub fn write_bool(&mut self, timestamp: i64, value: bool) -> TsFileResult<()> { self.inner.write_bool(timestamp, value) }
    pub fn write_i32(&mut self, timestamp: i64, value: i32) -> TsFileResult<()> { self.inner.write_i32(timestamp, value) }
    pub fn write_i64(&mut self, timestamp: i64, value: i64) -> TsFileResult<()> { self.inner.write_i64(timestamp, value) }
    pub fn write_f32(&mut self, timestamp: i64, value: f32) -> TsFileResult<()> { self.inner.write_f32(timestamp, value) }
    pub fn write_f64(&mut self, timestamp: i64, value: f64) -> TsFileResult<()> { self.inner.write_f64(timestamp, value) }
    pub fn write_binary(&mut self, timestamp: i64, value: Binary) -> TsFileResult<()> { self.inner.write_binary(timestamp, value) }
    pub fn flush(&mut self) -> TsFileResult<Option<EncodedPage>> { self.inner.flush() }
    pub fn point_count(&self) -> usize { self.inner.point_count() }
    pub fn estimated_size(&self) -> usize { self.inner.estimated_size() }
    pub fn is_full(&self) -> bool { self.inner.is_full() }
}
