// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use std::path::Path;

use crate::error::TsFileResult;
use crate::write::schema::{MeasurementSchema, Schema};
use crate::write::tablet::Tablet;
use crate::write::tsfile_writer::TsFileWriter;

pub struct TsFileWriterImpl {
    inner: TsFileWriter,
}

impl TsFileWriterImpl {
    pub fn new<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        Ok(TsFileWriterImpl { inner: TsFileWriter::new(path)? })
    }

    pub fn new_with_schema<P: AsRef<Path>>(path: P, schema: Schema) -> TsFileResult<Self> {
        Ok(TsFileWriterImpl { inner: TsFileWriter::new_with_schema(path, schema)? })
    }

    pub fn inner(&self) -> &TsFileWriter { &self.inner }
    pub fn inner_mut(&mut self) -> &mut TsFileWriter { &mut self.inner }
    pub fn into_inner(self) -> TsFileWriter { self.inner }
    pub fn register_timeseries(&mut self, device_id: String, schema: MeasurementSchema) -> TsFileResult<()> { self.inner.register_timeseries(device_id, schema) }
    pub fn write_tablet(&mut self, tablet: &Tablet) -> TsFileResult<usize> { self.inner.write_tablet(tablet) }
    pub fn close(self) -> TsFileResult<()> { self.inner.close() }
}
