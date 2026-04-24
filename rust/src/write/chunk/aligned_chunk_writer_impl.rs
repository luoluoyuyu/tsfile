// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use crate::common::enums::CompressionType;
use crate::write::chunk::{AlignedChunkWriter, ValueChunkWriter};

pub struct AlignedChunkWriterImpl {
    inner: AlignedChunkWriter,
}

impl AlignedChunkWriterImpl {
    pub fn new(time_compression: CompressionType, value_writers: Vec<ValueChunkWriter>) -> Self {
        AlignedChunkWriterImpl { inner: AlignedChunkWriter::new(time_compression, value_writers) }
    }

    pub fn inner(&self) -> &AlignedChunkWriter { &self.inner }
    pub fn inner_mut(&mut self) -> &mut AlignedChunkWriter { &mut self.inner }
    pub fn into_inner(self) -> AlignedChunkWriter { self.inner }
}
