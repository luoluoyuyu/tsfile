// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TSRecord and DataPoint - the write records.

use crate::common::enums::TSDataType;
use crate::utils::read_write_io_utils::Binary;

/// A single data point (measurement + value).
#[derive(Debug, Clone)]
pub struct DataPoint {
    /// Measurement ID.
    pub measurement_id: String,
    /// Value.
    pub value: DataPointValue,
}

/// Value variants for data points.
#[derive(Debug, Clone, PartialEq)]
pub enum DataPointValue {
    Boolean(bool),
    Int32(i32),
    Int64(i64),
    Float(f32),
    Double(f64),
    Text(Binary),
    Null,
}

impl DataPoint {
    pub fn new_bool(measurement_id: String, value: bool) -> Self {
        DataPoint {
            measurement_id,
            value: DataPointValue::Boolean(value),
        }
    }

    pub fn new_i32(measurement_id: String, value: i32) -> Self {
        DataPoint {
            measurement_id,
            value: DataPointValue::Int32(value),
        }
    }

    pub fn new_i64(measurement_id: String, value: i64) -> Self {
        DataPoint {
            measurement_id,
            value: DataPointValue::Int64(value),
        }
    }

    pub fn new_f32(measurement_id: String, value: f32) -> Self {
        DataPoint {
            measurement_id,
            value: DataPointValue::Float(value),
        }
    }

    pub fn new_f64(measurement_id: String, value: f64) -> Self {
        DataPoint {
            measurement_id,
            value: DataPointValue::Double(value),
        }
    }

    pub fn new_text(measurement_id: String, value: Binary) -> Self {
        DataPoint {
            measurement_id,
            value: DataPointValue::Text(value),
        }
    }

    pub fn data_type(&self) -> TSDataType {
        match &self.value {
            DataPointValue::Boolean(_) => TSDataType::Boolean,
            DataPointValue::Int32(_) => TSDataType::Int32,
            DataPointValue::Int64(_) => TSDataType::Int64,
            DataPointValue::Float(_) => TSDataType::Float,
            DataPointValue::Double(_) => TSDataType::Double,
            DataPointValue::Text(_) => TSDataType::Text,
            DataPointValue::Null => TSDataType::NullType,
        }
    }
}

/// A time series record at a single timestamp.
///
/// Mirrors Java's TSRecord.
#[derive(Debug, Clone)]
pub struct TSRecord {
    /// Timestamp of this record (Unix epoch in milliseconds).
    pub timestamp: i64,
    /// Device ID.
    pub device_id: String,
    /// Data points in this record.
    pub data_points: Vec<DataPoint>,
}

impl TSRecord {
    pub fn new(timestamp: i64, device_id: String) -> Self {
        TSRecord {
            timestamp,
            device_id,
            data_points: Vec::new(),
        }
    }

    pub fn add_tuple(&mut self, data_point: DataPoint) {
        self.data_points.push(data_point);
    }
}
