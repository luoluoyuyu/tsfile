// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TsFileWriter: high-level API for writing TsFiles.
//!
//! Mirrors Java's TsFileWriter.

use std::collections::HashMap;
use std::path::Path;

use crate::error::{TsFileError, TsFileResult};
use crate::file::metadata::chunk_metadata::ChunkMetadata;
use crate::file::metadata::{ColumnCategory, ColumnSchema, TableSchema};
use crate::write::chunk::ChunkWriter;
use crate::write::api::{DataWriter, TsFileWriteApi};
use crate::write::record::{DataPointValue, TSRecord};
use crate::write::schema::{MeasurementSchema, Schema};
use crate::write::tablet::Tablet;
use crate::write::writer::TsFileIOWriter;

/// High-level TsFile writer.
///
/// Usage:
/// 1. Create a new TsFileWriter with a file path.
/// 2. Register measurement schemas.
/// 3. Write TSRecords.
/// 4. Close the writer.
pub struct TsFileWriter {
    /// Low-level IO writer.
    io_writer: TsFileIOWriter,
    /// Schema for all devices/measurements.
    schema: Schema,
    /// Chunk writers for each device+measurement pair.
    chunk_writers: HashMap<String, HashMap<String, ChunkWriter>>,
    /// Record count.
    record_count: u64,
    /// Record count for next memory check.
    record_count_for_next_check: u64,
    /// Chunk group size threshold.
    chunk_group_size_threshold: usize,
    /// Whether writing is sequential (ordered timestamps).
    is_unseq: bool,
    /// Last timestamps per device for sequential check.
    last_timestamps: HashMap<String, i64>,
    /// Named schema templates for bulk device registration.
    schema_templates: HashMap<String, Vec<MeasurementSchema>>,
    /// Registered table schemas tracked at the high-level writer.
    registered_table_schemas: HashMap<String, TableSchema>,
    /// Whether table writes should be treated as aligned writes.
    table_write_aligned: bool,
    /// Whether table writes should auto-generate table schema from tablets.
    generate_table_schema: bool,
}

impl TsFileWriter {
    /// Create a new TsFileWriter.
    pub fn new<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        let io_writer = TsFileIOWriter::new(path)?;
        Ok(TsFileWriter {
            io_writer,
            schema: Schema::new(),
            chunk_writers: HashMap::new(),
            record_count: 0,
            record_count_for_next_check: 100,
            chunk_group_size_threshold: 128 * 1024 * 1024, // 128MB
            is_unseq: false,
            last_timestamps: HashMap::new(),
            schema_templates: HashMap::new(),
            registered_table_schemas: HashMap::new(),
            table_write_aligned: true,
            generate_table_schema: false,
        })
    }

    /// Create a new TsFileWriter with custom schema.
    pub fn new_with_schema<P: AsRef<Path>>(path: P, schema: Schema) -> TsFileResult<Self> {
        let mut writer = Self::new(path)?;
        writer.schema = schema;
        Ok(writer)
    }

    pub fn set_chunk_group_size_threshold(&mut self, threshold: usize) {
        self.chunk_group_size_threshold = threshold;
    }

    pub fn set_memory_threshold(&mut self, threshold: usize) {
        self.set_chunk_group_size_threshold(threshold);
    }

    pub fn set_unseq(&mut self, is_unseq: bool) {
        self.is_unseq = is_unseq;
    }

    pub fn record_count(&self) -> u64 {
        self.record_count
    }

    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    pub fn get_schema(&self) -> &Schema {
        self.schema()
    }

    pub fn get_io_writer(&mut self) -> &mut TsFileIOWriter {
        &mut self.io_writer
    }

    pub fn register_device(&mut self, device_id: String) {
        self.schema.measurement_schemas.entry(device_id).or_default();
    }

    pub fn register_table_schema(&mut self, table_schema: TableSchema) -> TsFileResult<()> {
        self.registered_table_schemas
            .insert(table_schema.table_name.clone(), table_schema.clone());
        self.io_writer.register_table_schema(table_schema);
        Ok(())
    }

    pub fn register_schema_template<I>(
        &mut self,
        template_name: String,
        schemas: I,
    ) -> TsFileResult<()>
    where
        I: IntoIterator<Item = MeasurementSchema>,
    {
        if template_name.trim().is_empty() {
            return Err(TsFileError::SchemaError(
                "Schema template name must not be empty".to_string(),
            ));
        }

        let mut template_schemas = Vec::new();
        for schema in schemas {
            schema.validate()?;
            template_schemas.push(schema);
        }
        if template_schemas.is_empty() {
            return Err(TsFileError::SchemaError(format!(
                "Schema template {} must contain at least one measurement",
                template_name
            )));
        }

        self.schema_templates.insert(template_name, template_schemas);
        Ok(())
    }

    pub fn register_device_from_template(
        &mut self,
        device_id: String,
        template_name: &str,
    ) -> TsFileResult<()> {
        let template = self
            .schema_templates
            .get(template_name)
            .cloned()
            .ok_or_else(|| {
                TsFileError::SchemaError(format!(
                    "Schema template {} is not registered",
                    template_name
                ))
            })?;
        self.register_timeseries_batch(device_id, template)
    }

    pub fn is_table_write_aligned(&self) -> bool {
        self.table_write_aligned
    }

    pub fn set_table_write_aligned(&mut self, table_write_aligned: bool) {
        self.table_write_aligned = table_write_aligned;
    }

    pub fn is_generate_table_schema(&self) -> bool {
        self.generate_table_schema
    }

    pub fn set_generate_table_schema(&mut self, generate_table_schema: bool) {
        self.generate_table_schema = generate_table_schema;
    }

    pub fn registered_table_schemas(&self) -> &HashMap<String, TableSchema> {
        &self.registered_table_schemas
    }

    /// Register a timeseries measurement schema for a device.
    pub fn register_timeseries(
        &mut self,
        device_id: String,
        schema: MeasurementSchema,
    ) -> TsFileResult<()> {
        // Check for conflicts
        if let Some(existing) =
            self.schema.get_schema(&device_id, &schema.measurement_id)
        {
            if existing.data_type != schema.data_type {
                return Err(TsFileError::SchemaError(format!(
                    "Data type conflict for {}.{}: {:?} vs {:?}",
                    device_id,
                    schema.measurement_id,
                    existing.data_type,
                    schema.data_type
                )));
            }
        }
        self.schema.register_timeseries(device_id, schema)?;
        Ok(())
    }

    pub fn register_aligned_timeseries<I>(
        &mut self,
        device_id: String,
        schemas: I,
    ) -> TsFileResult<()>
    where
        I: IntoIterator<Item = MeasurementSchema>,
    {
        self.register_timeseries_batch(device_id, schemas)
    }

    pub fn register_timeseries_batch<I>(
        &mut self,
        device_id: String,
        schemas: I,
    ) -> TsFileResult<()>
    where
        I: IntoIterator<Item = MeasurementSchema>,
    {
        for schema in schemas {
            self.register_timeseries(device_id.clone(), schema)?;
        }
        Ok(())
    }

    /// Write a TSRecord.
    ///
    /// Returns an error if:
    /// - The device is not registered.
    /// - A measurement is not registered for the device.
    /// - Data is out of order (when `is_unseq` is false).
    pub fn write(&mut self, record: TSRecord) -> TsFileResult<bool> {
        record.validate()?;
        let device_id = record.device_id.clone();
        let timestamp = record.timestamp;

        // Check ordering
        if record.data_points.is_empty() {
            return Ok(false);
        }

        if !self.is_unseq {
            if let Some(&last_ts) = self.last_timestamps.get(&device_id) {
                if timestamp <= last_ts {
                    return Err(TsFileError::OutOfOrderData {
                        timestamp,
                        last_timestamp: last_ts,
                    });
                }
            }
        }
        self.last_timestamps.insert(device_id.clone(), timestamp);

        // Check if device is registered
        if !self.schema.measurement_schemas.contains_key(&device_id) {
            return Err(TsFileError::DeviceNotFound(device_id.clone()));
        }

        // Write each data point
        for data_point in &record.data_points {
            let schema = self
                .schema
                .get_schema(&device_id, &data_point.measurement_id)
                .ok_or_else(|| {
                    TsFileError::MeasurementNotFound(format!(
                        "{}.{}",
                        device_id, data_point.measurement_id
                    ))
                })?
                .clone();

            // Get or create chunk writer
            let chunk_writer = self
                .chunk_writers
                .entry(device_id.clone())
                .or_default()
                .entry(data_point.measurement_id.clone())
                .or_insert_with(|| {
                    ChunkWriter::new(
                        schema.measurement_id.clone(),
                        schema.data_type,
                        schema.encoding,
                        schema.compression,
                    )
                });

            match &data_point.value {
                DataPointValue::Boolean(v) => chunk_writer.write_bool(timestamp, *v)?,
                DataPointValue::Int32(v) => chunk_writer.write_i32(timestamp, *v)?,
                DataPointValue::Int64(v) => chunk_writer.write_i64(timestamp, *v)?,
                DataPointValue::Float(v) => chunk_writer.write_f32(timestamp, *v)?,
                DataPointValue::Double(v) => chunk_writer.write_f64(timestamp, *v)?,
                DataPointValue::Text(v) => chunk_writer.write_binary(timestamp, v.clone())?,
                DataPointValue::Null => {}
            }
        }

        self.record_count += 1;

        // Check if we should flush
        if self.record_count >= self.record_count_for_next_check {
            let total_size: usize = self
                .chunk_writers
                .values()
                .flat_map(|m| m.values())
                .map(|cw| cw.statistics().serialized_size())
                .sum();

            if total_size >= self.chunk_group_size_threshold {
                self.flush_all_chunk_groups()?;
                self.record_count_for_next_check = 100;
            } else {
                self.record_count_for_next_check =
                    (self.record_count_for_next_check * 2).min(10_000);
            }
        }

        Ok(true)
    }

    pub fn write_record(&mut self, record: TSRecord) -> TsFileResult<bool> {
        self.write(record)
    }

    /// Write all rows in a tablet batch.
    pub fn write_tablet(&mut self, tablet: &Tablet) -> TsFileResult<usize> {
        for schema in &tablet.schemas {
            self.register_timeseries(tablet.device_id.clone(), schema.clone())?;
        }
        let records = tablet.to_records();
        let row_count = records.len();
        for record in records {
            self.write(record)?;
        }
        Ok(row_count)
    }

    pub fn write_aligned(&mut self, tablet: &Tablet) -> TsFileResult<usize> {
        self.write_tablet(tablet)
    }

    pub fn write_table(&mut self, tablet: &Tablet) -> TsFileResult<usize> {
        self.ensure_table_schema_for_tablet(tablet)?;
        if self.table_write_aligned {
            self.write_aligned(tablet)
        } else {
            self.write_tablet(tablet)
        }
    }

    pub fn write_tree(&mut self, tablet: &Tablet) -> TsFileResult<usize> {
        self.write_tablet(tablet)
    }

    /// Write a tablet and reset it after successful flush, matching Java's common usage pattern.
    pub fn write_tablet_and_reset(&mut self, tablet: &mut Tablet) -> TsFileResult<usize> {
        let written = self.write_tablet(tablet)?;
        tablet.reset();
        Ok(written)
    }

    /// Flush all chunk groups to disk.
    pub fn flush_all_chunk_groups(&mut self) -> TsFileResult<()> {
        // Sort device IDs to ensure deterministic order (matches Java's TreeMap behavior)
        let mut device_ids: Vec<String> = self.chunk_writers.keys().cloned().collect();
        device_ids.sort();
        
        for device_id in device_ids {
            self.flush_chunk_group(&device_id)?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> TsFileResult<()> {
        self.flush_all_chunk_groups()?;
        self.io_writer.flush()
    }

    fn ensure_table_schema_for_tablet(&mut self, tablet: &Tablet) -> TsFileResult<()> {
        if !self.generate_table_schema {
            return Ok(());
        }

        let table_name = table_name_from_device(&tablet.device_id);
        if self.registered_table_schemas.contains_key(&table_name) {
            return Ok(());
        }

        let columns = tablet
            .schemas
            .iter()
            .map(|schema| {
                ColumnSchema::new(
                    schema.measurement_id.clone(),
                    schema.data_type,
                    ColumnCategory::Field,
                )
            })
            .collect();
        let table_schema = TableSchema::from_columns(table_name, columns)?;
        self.register_table_schema(table_schema)
    }

    fn flush_chunk_group(&mut self, device_id: &str) -> TsFileResult<()> {
        let mut writers = match self.chunk_writers.remove(device_id) {
            Some(w) => w,
            None => return Ok(()),
        };

        if writers.values().all(|cw| !cw.has_data()) {
            return Ok(());
        }

        self.io_writer.start_chunk_group(device_id.to_string())?;

        // Sort measurement IDs to ensure deterministic order
        let mut measurement_ids: Vec<String> = writers.keys().cloned().collect();
        measurement_ids.sort();
        for measurement_id in measurement_ids {
            let mut chunk_writer = writers.remove(&measurement_id).unwrap();
            if !chunk_writer.has_data() {
                continue;
            }

            // Serialize the chunk
            let chunk_offset = self.io_writer.position();
            let mut chunk_buf = Vec::new();
            let (_header_size, _data_size) = chunk_writer.write_to(&mut chunk_buf)?;

            let chunk_stats = chunk_writer.statistics().clone();
            let chunk_metadata = ChunkMetadata::new(
                measurement_id,
                chunk_writer.data_type,
                chunk_offset as i64,
                chunk_stats,
            );

            self.io_writer.write_chunk(&chunk_buf, chunk_metadata)?;
        }

        self.io_writer.end_chunk_group()?;
        Ok(())
    }

    /// Close the writer, flushing remaining data and writing the file footer.
    pub fn close(mut self) -> TsFileResult<()> {
        self.flush_all_chunk_groups()?;
        self.io_writer.end_file()?;
        Ok(())
    }
}

fn table_name_from_device(device_id: &str) -> String {
    device_id
        .split('.')
        .next()
        .unwrap_or(device_id)
        .to_lowercase()
}

impl DataWriter for TsFileWriter {
    fn write_record(&mut self, record: TSRecord) -> TsFileResult<bool> {
        self.write(record)
    }

    fn write_tablet_batch(&mut self, tablet: &Tablet) -> TsFileResult<usize> {
        self.write_tablet(tablet)
    }
}

impl TsFileWriteApi for TsFileWriter {
    fn register_timeseries(
        &mut self,
        device_id: String,
        schema: MeasurementSchema,
    ) -> TsFileResult<()> {
        TsFileWriter::register_timeseries(self, device_id, schema)
    }

    fn close(self) -> TsFileResult<()> {
        TsFileWriter::close(self)
    }
}
