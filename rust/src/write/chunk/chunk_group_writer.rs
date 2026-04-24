// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Chunk group writer for one device.

use std::collections::HashMap;

use crate::error::{TsFileError, TsFileResult};
use crate::write::chunk::ChunkWriter;
use crate::write::record::{DataPointValue, TSRecord};
use crate::write::schema::MeasurementSchema;

pub struct ChunkGroupWriter {
    device_id: String,
    chunk_writers: HashMap<String, ChunkWriter>,
}

impl ChunkGroupWriter {
    pub fn new(device_id: String) -> Self {
        ChunkGroupWriter {
            device_id,
            chunk_writers: HashMap::new(),
        }
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub fn register_schema(&mut self, schema: MeasurementSchema) {
        self.chunk_writers.entry(schema.measurement_id.clone()).or_insert_with(|| {
            ChunkWriter::new(
                schema.measurement_id,
                schema.data_type,
                schema.encoding,
                schema.compression,
            )
        });
    }

    pub fn write(&mut self, record: &TSRecord, schemas: &[MeasurementSchema]) -> TsFileResult<()> {
        for data_point in &record.data_points {
            let schema = schemas
                .iter()
                .find(|schema| schema.measurement_id == data_point.measurement_id)
                .ok_or_else(|| {
                    TsFileError::MeasurementNotFound(format!(
                        "{}.{}",
                        self.device_id, data_point.measurement_id
                    ))
                })?
                .clone();
            self.register_schema(schema);
            let writer = self.chunk_writers.get_mut(&data_point.measurement_id).unwrap();
            match &data_point.value {
                DataPointValue::Boolean(value) => writer.write_bool(record.timestamp, *value)?,
                DataPointValue::Int32(value) => writer.write_i32(record.timestamp, *value)?,
                DataPointValue::Int64(value) => writer.write_i64(record.timestamp, *value)?,
                DataPointValue::Float(value) => writer.write_f32(record.timestamp, *value)?,
                DataPointValue::Double(value) => writer.write_f64(record.timestamp, *value)?,
                DataPointValue::Text(value) => writer.write_binary(record.timestamp, value.clone())?,
                DataPointValue::Null => {}
            }
        }
        Ok(())
    }

    pub fn into_chunk_writers(self) -> HashMap<String, ChunkWriter> {
        self.chunk_writers
    }
}
