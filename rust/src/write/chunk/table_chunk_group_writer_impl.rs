// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use crate::file::metadata::table_schema::TableSchema;
use crate::write::chunk::NonAlignedChunkGroupWriterImpl;

pub struct TableChunkGroupWriterImpl {
    table_schema: TableSchema,
    inner: NonAlignedChunkGroupWriterImpl,
}

impl TableChunkGroupWriterImpl {
    pub fn new(device_id: String, table_schema: TableSchema) -> Self {
        TableChunkGroupWriterImpl { table_schema, inner: NonAlignedChunkGroupWriterImpl::new(device_id) }
    }
    pub fn table_schema(&self) -> &TableSchema { &self.table_schema }
    pub fn inner(&self) -> &NonAlignedChunkGroupWriterImpl { &self.inner }
    pub fn inner_mut(&mut self) -> &mut NonAlignedChunkGroupWriterImpl { &mut self.inner }
}
