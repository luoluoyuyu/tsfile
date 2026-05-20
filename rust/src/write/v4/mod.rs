// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Table-model/tree-model writer facades.

use std::path::Path;

use crate::error::{TsFileError, TsFileResult};
use crate::file::metadata::table_schema::TableSchema;
use crate::write::schema::MeasurementSchema;
use crate::write::schema::Schema;
use crate::write::record::TSRecord;
use crate::write::tablet::Tablet;
use crate::write::tsfile_writer::TsFileWriter;

pub trait ITsFileWriter {
    fn write_tablet(&mut self, tablet: &Tablet) -> TsFileResult<usize>;
    fn write_record(&mut self, record: TSRecord) -> TsFileResult<bool>;
    fn flush(&mut self) -> TsFileResult<()>;
}

pub struct TsFileTreeWriter {
    inner: TsFileWriter,
}

impl TsFileTreeWriter {
    pub fn new<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        Ok(TsFileTreeWriter { inner: TsFileWriter::new(path)? })
    }

    pub fn new_with_memory_threshold<P: AsRef<Path>>(
        path: P,
        memory_threshold: usize,
    ) -> TsFileResult<Self> {
        let mut writer = TsFileWriter::new(path)?;
        writer.set_memory_threshold(memory_threshold);
        Ok(TsFileTreeWriter { inner: writer })
    }

    pub fn register_timeseries(&mut self, device: String, schema: MeasurementSchema) -> TsFileResult<()> {
        self.inner.register_timeseries(device, schema)
    }

    pub fn register_device(&mut self, device: String) {
        self.inner.register_device(device);
    }

    pub fn register_timeseries_batch<I>(
        &mut self,
        device: String,
        schemas: I,
    ) -> TsFileResult<()>
    where
        I: IntoIterator<Item = MeasurementSchema>,
    {
        self.inner.register_timeseries_batch(device, schemas)
    }

    pub fn register_aligned_timeseries<I>(
        &mut self,
        device: String,
        schemas: I,
    ) -> TsFileResult<()>
    where
        I: IntoIterator<Item = MeasurementSchema>,
    {
        self.inner.register_aligned_timeseries(device, schemas)
    }

    pub fn write(&mut self, tablet: &Tablet) -> TsFileResult<usize> {
        self.inner.write_tree(tablet)
    }

    pub fn write_tablet_and_reset(&mut self, tablet: &mut Tablet) -> TsFileResult<usize> {
        self.inner.write_tablet_and_reset(tablet)
    }

    pub fn write_record(&mut self, record: TSRecord) -> TsFileResult<bool> {
        self.inner.write_record(record)
    }

    pub fn flush(&mut self) -> TsFileResult<()> {
        self.inner.flush()
    }

    pub fn schema(&self) -> &Schema {
        self.inner.schema()
    }

    pub fn inner_mut(&mut self) -> &mut TsFileWriter {
        &mut self.inner
    }

    pub fn close(self) -> TsFileResult<()> {
        self.inner.close()
    }
}

impl ITsFileWriter for TsFileTreeWriter {
    fn write_tablet(&mut self, tablet: &Tablet) -> TsFileResult<usize> {
        self.inner.write_tablet(tablet)
    }

    fn write_record(&mut self, record: TSRecord) -> TsFileResult<bool> {
        self.inner.write_record(record)
    }

    fn flush(&mut self) -> TsFileResult<()> {
        self.inner.flush()
    }
}

pub struct DeviceTableModelWriter {
    inner: TsFileWriter,
    table_schema: TableSchema,
}

impl DeviceTableModelWriter {
    pub fn new<P: AsRef<Path>>(path: P, table_schema: TableSchema) -> TsFileResult<Self> {
        let mut inner = TsFileWriter::new(path)?;
        inner.register_table_schema(table_schema.clone())?;
        Ok(DeviceTableModelWriter { inner, table_schema })
    }

    pub fn new_with_memory_threshold<P: AsRef<Path>>(
        path: P,
        table_schema: TableSchema,
        memory_threshold: usize,
    ) -> TsFileResult<Self> {
        let mut writer = TsFileWriter::new(path)?;
        writer.set_memory_threshold(memory_threshold);
        writer.register_table_schema(table_schema.clone())?;
        Ok(DeviceTableModelWriter {
            inner: writer,
            table_schema,
        })
    }

    pub fn table_schema(&self) -> &TableSchema {
        &self.table_schema
    }

    pub fn table_name(&self) -> &str {
        &self.table_schema.table_name
    }

    pub fn write(&mut self, tablet: &Tablet) -> TsFileResult<usize> {
        self.validate_tablet(tablet)?;
        self.ensure_device_schema_registered(&tablet.device_id)?;
        self.inner.write_tablet(tablet)
    }

    pub fn write_record(&mut self, record: TSRecord) -> TsFileResult<bool> {
        self.validate_record(&record)?;
        self.ensure_device_schema_registered(&record.device_id)?;
        self.inner.write_record(record)
    }

    pub fn write_tablet_and_reset(&mut self, tablet: &mut Tablet) -> TsFileResult<usize> {
        self.validate_tablet(tablet)?;
        self.ensure_device_schema_registered(&tablet.device_id)?;
        self.inner.write_tablet_and_reset(tablet)
    }

    pub fn flush(&mut self) -> TsFileResult<()> {
        self.inner.flush()
    }

    pub fn inner_mut(&mut self) -> &mut TsFileWriter {
        &mut self.inner
    }

    pub fn close(self) -> TsFileResult<()> {
        self.inner.close()
    }

    fn validate_tablet(&mut self, tablet: &Tablet) -> TsFileResult<()> {
        for schema in &tablet.schemas {
            let Some(index) = self.table_schema.find_column_index(&schema.measurement_id) else {
                return Err(TsFileError::SchemaError(format!(
                    "Column {} is not registered in table {}",
                    schema.measurement_id, self.table_schema.table_name
                )));
            };
            let registered = &self.table_schema.measurement_schemas[index];
            if registered.data_type != schema.data_type {
                return Err(TsFileError::SchemaError(format!(
                    "Column type mismatch for {}.{}: {:?} vs {:?}",
                    self.table_schema.table_name,
                    schema.measurement_id,
                    registered.data_type,
                    schema.data_type
                )));
            }
        }
        Ok(())
    }

    fn validate_record(&mut self, record: &TSRecord) -> TsFileResult<()> {
        for data_point in &record.data_points {
            let Some(index) = self.table_schema.find_column_index(&data_point.measurement_id) else {
                return Err(TsFileError::SchemaError(format!(
                    "Column {} is not registered in table {}",
                    data_point.measurement_id, self.table_schema.table_name
                )));
            };
            let registered = &self.table_schema.measurement_schemas[index];
            let value_type = data_point.data_type();
            if value_type != registered.data_type && value_type != crate::common::enums::TSDataType::NullType {
                return Err(TsFileError::SchemaError(format!(
                    "Column type mismatch for {}.{}: {:?} vs {:?}",
                    self.table_schema.table_name,
                    data_point.measurement_id,
                    registered.data_type,
                    value_type
                )));
            }
        }
        Ok(())
    }

    fn ensure_device_schema_registered(&mut self, device_id: &str) -> TsFileResult<()> {
        self.inner.register_device(device_id.to_string());
        for schema in &self.table_schema.measurement_schemas {
            self.inner
                .register_timeseries(device_id.to_string(), schema.clone())?;
        }
        Ok(())
    }
}

impl ITsFileWriter for DeviceTableModelWriter {
    fn write_tablet(&mut self, tablet: &Tablet) -> TsFileResult<usize> {
        DeviceTableModelWriter::write(self, tablet)
    }

    fn write_record(&mut self, record: TSRecord) -> TsFileResult<bool> {
        DeviceTableModelWriter::write_record(self, record)
    }

    fn flush(&mut self) -> TsFileResult<()> {
        self.inner.flush()
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
