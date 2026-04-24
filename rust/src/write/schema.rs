// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Write module: schema definitions.

use std::collections::HashMap;
use std::io::{Read, Write};

use crate::common::enums::{CompressionType, TSDataType, TSEncoding};
use crate::error::{TsFileError, TsFileResult};
use crate::utils::ReadWriteIOUtils;

/// Schema for a single measurement (timeseries).
///
/// Mirrors Java's MeasurementSchema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementSchema {
    /// Measurement name (sensor ID).
    pub measurement_id: String,
    /// Data type.
    pub data_type: TSDataType,
    /// Encoding type.
    pub encoding: TSEncoding,
    /// Compression type.
    pub compression: CompressionType,
    /// Optional encoder properties.
    pub props: HashMap<String, String>,
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
            props: HashMap::new(),
        }
    }

    pub fn with_props(
        measurement_id: String,
        data_type: TSDataType,
        encoding: TSEncoding,
        compression: CompressionType,
        props: HashMap<String, String>,
    ) -> Self {
        MeasurementSchema {
            measurement_id,
            data_type,
            encoding,
            compression,
            props,
        }
    }

    pub fn validate(&self) -> TsFileResult<()> {
        if self.measurement_id.is_empty() {
            return Err(TsFileError::SchemaError(
                "Measurement name must not be empty".to_string(),
            ));
        }
        if !self.encoding.is_supported_for(&self.data_type) {
            return Err(TsFileError::SchemaError(format!(
                "Encoding {} is not supported for data type {}",
                self.encoding, self.data_type
            )));
        }
        Ok(())
    }

    /// Serialize exactly like Java's `MeasurementSchema.serializeTo(OutputStream)`.
    pub fn serialize<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        let mut written = ReadWriteIOUtils::write_string(&self.measurement_id, writer)?;
        written += ReadWriteIOUtils::write_byte(self.data_type.serialize(), writer)?;
        written += ReadWriteIOUtils::write_byte(self.encoding.serialize(), writer)?;
        written += ReadWriteIOUtils::write_byte(self.compression.serialize(), writer)?;

        let mut props: Vec<_> = self.props.iter().collect();
        props.sort_by(|a, b| a.0.cmp(b.0));
        written += ReadWriteIOUtils::write_i32(props.len() as i32, writer)?;
        for (key, value) in props {
            written += ReadWriteIOUtils::write_string(key, writer)?;
            written += ReadWriteIOUtils::write_string(value, writer)?;
        }
        Ok(written)
    }

    pub fn deserialize<R: Read>(reader: &mut R) -> TsFileResult<Self> {
        let measurement_id = ReadWriteIOUtils::read_string(reader)?;
        let data_type = TSDataType::deserialize(ReadWriteIOUtils::read_byte(reader)?)?;
        let encoding = TSEncoding::deserialize(ReadWriteIOUtils::read_byte(reader)?)?;
        let compression = CompressionType::deserialize(ReadWriteIOUtils::read_byte(reader)?)?;
        let props_count = ReadWriteIOUtils::read_i32(reader)?;
        if props_count < 0 {
            return Err(TsFileError::InvalidFileFormat(format!(
                "Negative MeasurementSchema props count: {}",
                props_count
            )));
        }
        let mut props = HashMap::with_capacity(props_count as usize);
        for _ in 0..props_count {
            let key = ReadWriteIOUtils::read_string(reader)?;
            let value = ReadWriteIOUtils::read_string(reader)?;
            props.insert(key, value);
        }
        Ok(MeasurementSchema::with_props(
            measurement_id,
            data_type,
            encoding,
            compression,
            props,
        ))
    }
}

/// Schema for a device (collection of measurements).
///
/// Mirrors Java's Schema.
#[derive(Debug, Default)]
pub struct Schema {
    /// Map from device ID to measurement schemas.
    pub measurement_schemas: HashMap<String, Vec<MeasurementSchema>>,
}

impl Schema {
    pub fn new() -> Self {
        Schema::default()
    }

    /// Register a measurement schema for a device.
    pub fn register_timeseries(
        &mut self,
        device_id: String,
        schema: MeasurementSchema,
    ) -> TsFileResult<()> {
        if device_id.is_empty() {
            return Err(TsFileError::SchemaError(
                "Device id must not be empty".to_string(),
            ));
        }
        schema.validate()?;
        let schemas = self.measurement_schemas.entry(device_id.clone()).or_default();
        if let Some(existing) = schemas
            .iter()
            .find(|existing| existing.measurement_id == schema.measurement_id)
        {
            if existing.data_type != schema.data_type
                || existing.encoding != schema.encoding
                || existing.compression != schema.compression
            {
                return Err(TsFileError::SchemaError(format!(
                    "Conflicting schema for {}.{}",
                    device_id, schema.measurement_id
                )));
            }
            return Ok(());
        }
        schemas.push(schema);
        Ok(())
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

    /// Get the measurement schema for a device and measurement.
    pub fn get_schema(
        &self,
        device_id: &str,
        measurement_id: &str,
    ) -> Option<&MeasurementSchema> {
        self.measurement_schemas
            .get(device_id)?
            .iter()
            .find(|schema| schema.measurement_id == measurement_id)
    }

    pub fn contains_device(&self, device_id: &str) -> bool {
        self.measurement_schemas.contains_key(device_id)
    }

    pub fn measurements_for_device(&self, device_id: &str) -> Option<&[MeasurementSchema]> {
        self.measurement_schemas.get(device_id).map(Vec::as_slice)
    }
}
