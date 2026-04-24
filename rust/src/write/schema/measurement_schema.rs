// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use std::collections::HashMap;

use crate::common::enums::{CompressionType, TSDataType, TSEncoding};
use crate::write::schema::MeasurementSchema as InnerMeasurementSchema;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementSchema {
    inner: InnerMeasurementSchema,
}

impl MeasurementSchema {
    pub fn new(measurement_id: String, data_type: TSDataType, encoding: TSEncoding, compression: CompressionType) -> Self {
        MeasurementSchema { inner: InnerMeasurementSchema::new(measurement_id, data_type, encoding, compression) }
    }
    pub fn with_props(measurement_id: String, data_type: TSDataType, encoding: TSEncoding, compression: CompressionType, props: HashMap<String, String>) -> Self {
        MeasurementSchema { inner: InnerMeasurementSchema::with_props(measurement_id, data_type, encoding, compression, props) }
    }
    pub fn inner(&self) -> &InnerMeasurementSchema { &self.inner }
    pub fn into_inner(self) -> InnerMeasurementSchema { self.inner }
}
