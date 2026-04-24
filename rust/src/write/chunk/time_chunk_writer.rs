// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

pub struct TimeChunkWriterImpl {
    inner: crate::write::chunk::TimeChunkWriter,
}

impl TimeChunkWriterImpl {
    pub fn new(compression: crate::common::enums::CompressionType) -> Self {
        TimeChunkWriterImpl { inner: crate::write::chunk::TimeChunkWriter::new(compression) }
    }
    pub fn inner(&self) -> &crate::write::chunk::TimeChunkWriter { &self.inner }
    pub fn inner_mut(&mut self) -> &mut crate::write::chunk::TimeChunkWriter { &mut self.inner }
    pub fn into_inner(self) -> crate::write::chunk::TimeChunkWriter { self.inner }
}
