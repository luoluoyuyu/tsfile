// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Write module: schema definitions.

use crate::common::enums::{CompressionType, TSDataType, TSEncoding};

/// Schema for a single measurement (timeseries).
///
/// Mirrors Java's MeasurementSchema.
#[derive(Debug, Clone)]
pub struct MeasurementSchema {
    /// Measurement name (sensor ID).
    pub measurement_id: String,
    /// Data type.
    pub data_type: TSDataType,
    /// Encoding type.
    pub encoding: TSEncoding,
    /// Compression type.
    pub compression: CompressionType,
}

impl MeasurementSchema {
    pub fn new(
        measurement_id: String,
        data_type: TSDataType,
        encoding: TSEncoding,
        compression: CompressionType,
    ) -> Self {
        MeasurementSchema {
            measurement_id,
            data_type,
            encoding,
            compression,
        }
    }
}

/// Schema for a device (collection of measurements).
///
/// Mirrors Java's Schema.
#[derive(Debug, Default)]
pub struct Schema {
    /// Map from device ID to measurement schemas.
    pub measurement_schemas: std::collections::HashMap<String, Vec<MeasurementSchema>>,
}

impl Schema {
    pub fn new() -> Self {
        Schema::default()
    }

    /// Register a measurement schema for a device.
    pub fn register_timeseries(&mut self, device_id: String, schema: MeasurementSchema) {
        self.measurement_schemas
            .entry(device_id)
            .or_default()
            .push(schema);
    }

    /// Get the measurement schema for a device and measurement.
    pub fn get_schema(
        &self,
        device_id: &str,
        measurement_id: &str,
    ) -> Option<&MeasurementSchema> {
        self.measurement_schemas.get(device_id)?.iter().find(|s| {
            s.measurement_id == measurement_id
        })
    }
}
