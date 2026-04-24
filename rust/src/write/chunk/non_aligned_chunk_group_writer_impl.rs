// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use std::collections::HashMap;

use crate::error::TsFileResult;
use crate::write::chunk::{ChunkGroupWriter, ChunkWriter};
use crate::write::record::TSRecord;
use crate::write::schema::MeasurementSchema;

pub struct NonAlignedChunkGroupWriterImpl {
    inner: ChunkGroupWriter,
}

impl NonAlignedChunkGroupWriterImpl {
    pub fn new(device_id: String) -> Self { Self { inner: ChunkGroupWriter::new(device_id) } }
    pub fn device_id(&self) -> &str { self.inner.device_id() }
    pub fn register_schema(&mut self, schema: MeasurementSchema) { self.inner.register_schema(schema); }
    pub fn write(&mut self, record: &TSRecord, schemas: &[MeasurementSchema]) -> TsFileResult<()> { self.inner.write(record, schemas) }
    pub fn into_chunk_writers(self) -> HashMap<String, ChunkWriter> { self.inner.into_chunk_writers() }
}
