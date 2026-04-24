// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

pub struct ValueChunkWriterImpl {
    inner: crate::write::chunk::ValueChunkWriter,
}

impl ValueChunkWriterImpl {
    pub fn new(measurement_id: String, data_type: crate::common::enums::TSDataType, encoding: crate::common::enums::TSEncoding, compression: crate::common::enums::CompressionType) -> Self {
        ValueChunkWriterImpl { inner: crate::write::chunk::ValueChunkWriter::new(measurement_id, data_type, encoding, compression) }
    }
    pub fn inner(&self) -> &crate::write::chunk::ValueChunkWriter { &self.inner }
    pub fn inner_mut(&mut self) -> &mut crate::write::chunk::ValueChunkWriter { &mut self.inner }
    pub fn into_inner(self) -> crate::write::chunk::ValueChunkWriter { self.inner }
}
