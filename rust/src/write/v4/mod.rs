// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Table-model/tree-model writer facades.

use std::path::Path;

use crate::error::TsFileResult;
use crate::file::metadata::table_schema::TableSchema;
use crate::write::schema::MeasurementSchema;
use crate::write::tablet::Tablet;
use crate::write::tsfile_writer::TsFileWriter;

pub trait ITsFileWriter {
    fn write_tablet(&mut self, tablet: &Tablet) -> TsFileResult<usize>;
}

pub struct TsFileTreeWriter {
    inner: TsFileWriter,
}

impl TsFileTreeWriter {
    pub fn new<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        Ok(TsFileTreeWriter { inner: TsFileWriter::new(path)? })
    }

    pub fn register_timeseries(&mut self, device: String, schema: MeasurementSchema) -> TsFileResult<()> {
        self.inner.register_timeseries(device, schema)
    }

    pub fn close(self) -> TsFileResult<()> {
        self.inner.close()
    }
}

impl ITsFileWriter for TsFileTreeWriter {
    fn write_tablet(&mut self, tablet: &Tablet) -> TsFileResult<usize> {
        self.inner.write_tablet(tablet)
    }
}

pub struct DeviceTableModelWriter {
    inner: TsFileWriter,
    table_schema: TableSchema,
}

impl DeviceTableModelWriter {
    pub fn new<P: AsRef<Path>>(path: P, table_schema: TableSchema) -> TsFileResult<Self> {
        Ok(DeviceTableModelWriter { inner: TsFileWriter::new(path)?, table_schema })
    }

    pub fn table_schema(&self) -> &TableSchema {
        &self.table_schema
    }

    pub fn close(self) -> TsFileResult<()> {
        self.inner.close()
    }
}

impl ITsFileWriter for DeviceTableModelWriter {
    fn write_tablet(&mut self, tablet: &Tablet) -> TsFileResult<usize> {
        self.inner.write_tablet(tablet)
    }
}
pub mod i_tsfile_writer;
pub mod tsfile_tree_writer;
pub mod device_table_model_writer;
pub mod abstract_table_model_tsfile_writer;
pub mod table_tsblock2_tsfile_writer;
pub mod tsfile_tree_writer_builder;
pub mod tsfile_writer_builder;
pub use abstract_table_model_tsfile_writer::AbstractTableModelTsFileWriter;
pub use table_tsblock2_tsfile_writer::TableTsBlock2TsFileWriter;
pub use tsfile_tree_writer_builder::TsFileTreeWriterBuilder;
pub use tsfile_writer_builder::TsFileWriterBuilder;
