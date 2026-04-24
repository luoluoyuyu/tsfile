// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Tablet batch write structure, corresponding to Java's `Tablet`.

use crate::error::{TsFileError, TsFileResult};
use crate::write::record::{DataPoint, DataPointValue, TSRecord};
use crate::write::schema::MeasurementSchema;

/// In-memory batch of rows for one device.
#[derive(Debug, Clone)]
pub struct Tablet {
    pub device_id: String,
    pub schemas: Vec<MeasurementSchema>,
    pub timestamps: Vec<i64>,
    pub values: Vec<Vec<Option<DataPointValue>>>,
    max_row_number: usize,
}

impl Tablet {
    pub fn new(device_id: String, schemas: Vec<MeasurementSchema>, max_row_number: usize) -> Self {
        Tablet {
            device_id,
            schemas,
            timestamps: Vec::with_capacity(max_row_number),
            values: Vec::with_capacity(max_row_number),
            max_row_number,
        }
    }

    pub fn row_size(&self) -> usize {
        self.timestamps.len()
    }

    pub fn max_row_number(&self) -> usize {
        self.max_row_number
    }

    pub fn is_full(&self) -> bool {
        self.row_size() >= self.max_row_number
    }

    pub fn reset(&mut self) {
        self.timestamps.clear();
        self.values.clear();
    }

    pub fn add_row(
        &mut self,
        timestamp: i64,
        row_values: Vec<Option<DataPointValue>>,
    ) -> TsFileResult<()> {
        if self.is_full() {
            return Err(TsFileError::WriteError(format!(
                "Tablet for {} is full: {} rows",
                self.device_id, self.max_row_number
            )));
        }
        if row_values.len() != self.schemas.len() {
            return Err(TsFileError::WriteError(format!(
                "Tablet row width mismatch: expected {}, got {}",
                self.schemas.len(),
                row_values.len()
            )));
        }
        self.timestamps.push(timestamp);
        self.values.push(row_values);
        Ok(())
    }

    pub fn to_records(&self) -> Vec<TSRecord> {
        self.timestamps
            .iter()
            .zip(self.values.iter())
            .map(|(timestamp, row)| {
                let mut record = TSRecord::new(*timestamp, self.device_id.clone());
                for (schema, value) in self.schemas.iter().zip(row.iter()) {
                    let data_point = match value {
                        Some(DataPointValue::Boolean(value)) => {
                            DataPoint::new_bool(schema.measurement_id.clone(), *value)
                        }
                        Some(DataPointValue::Int32(value)) => {
                            DataPoint::new_i32(schema.measurement_id.clone(), *value)
                        }
                        Some(DataPointValue::Int64(value)) => {
                            DataPoint::new_i64(schema.measurement_id.clone(), *value)
                        }
                        Some(DataPointValue::Float(value)) => {
                            DataPoint::new_f32(schema.measurement_id.clone(), *value)
                        }
                        Some(DataPointValue::Double(value)) => {
                            DataPoint::new_f64(schema.measurement_id.clone(), *value)
                        }
                        Some(DataPointValue::Text(value)) => {
                            DataPoint::new_text(schema.measurement_id.clone(), value.clone())
                        }
                        Some(DataPointValue::Null) | None => continue,
                    };
                    record.add_tuple(data_point);
                }
                record
            })
            .collect()
    }
}
