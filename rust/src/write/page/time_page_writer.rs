// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use crate::common::enums::{TSDataType, TSEncoding};
use crate::error::TsFileResult;
use crate::write::chunk::page_writer::{EncodedPage, PageWriter};

pub struct TimePageWriter {
    inner: PageWriter,
}

impl TimePageWriter {
    pub fn new(max_points: usize) -> Self {
        TimePageWriter { inner: PageWriter::new(TSDataType::Int64, TSEncoding::Ts2diff, max_points) }
    }

    pub fn write(&mut self, timestamp: i64) -> TsFileResult<()> {
        self.inner.write_i64(timestamp, timestamp)
    }

    pub fn flush(&mut self) -> TsFileResult<Option<EncodedPage>> { self.inner.flush() }
    pub fn point_count(&self) -> usize { self.inner.point_count() }
    pub fn estimated_size(&self) -> usize { self.inner.estimated_size() }
    pub fn is_full(&self) -> bool { self.inner.is_full() }
}
