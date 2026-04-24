// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use crate::common::enums::{TSDataType, TSEncoding};
use crate::write::chunk::page_writer::PageWriter as InnerPageWriter;

pub use crate::write::chunk::page_writer::EncodedPage;

pub struct PageWriter {
    inner: InnerPageWriter,
}

impl PageWriter {
    pub fn new(data_type: TSDataType, encoding: TSEncoding, max_points: usize) -> Self {
        PageWriter { inner: InnerPageWriter::new(data_type, encoding, max_points) }
    }
    pub fn inner(&self) -> &InnerPageWriter { &self.inner }
    pub fn inner_mut(&mut self) -> &mut InnerPageWriter { &mut self.inner }
    pub fn into_inner(self) -> InnerPageWriter { self.inner }
}
