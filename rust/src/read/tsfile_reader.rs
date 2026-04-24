// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TsFileReader: high-level API for reading TsFiles.
//!
//! Mirrors Java's TsFileReader.

use std::collections::HashMap;
use std::path::Path;

use crate::error::TsFileResult;
use crate::read::api::TsFileReadApi;
use crate::read::common::{Field, RowRecord, TimeRange};
use crate::read::reader::VecPointReader;
use crate::read::result_set::{QueryExpression, ResultSet};
use crate::read::time_value_pair::TimeValue;
use crate::read::time_value_pair::TimeValuePair;
use crate::read::tsfile_sequence_reader::TsFileSequenceReader;

/// High-level TsFile reader.
///
/// Usage:
/// ```no_run
/// use tsfile::read::TsFileReader;
///
/// let mut reader = TsFileReader::new("data.tsfile").unwrap();
/// let data = reader.read_timeseries("device1", "temperature").unwrap();
/// for pair in data {
///     println!("ts={}, val={:?}", pair.timestamp, pair.value);
/// }
/// ```
pub struct TsFileReader {
    /// Sequence reader for low-level access.
    sequence_reader: TsFileSequenceReader,
    /// Cached data: device -> measurement -> time-value pairs.
    cache: Option<HashMap<String, HashMap<String, Vec<TimeValuePair>>>>,
}

impl TsFileReader {
    /// Open a TsFile for reading.
    pub fn new<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        let sequence_reader = TsFileSequenceReader::new(path)?;
        Ok(TsFileReader {
            sequence_reader,
            cache: None,
        })
    }

    /// Load all data into memory cache.
    fn load_cache(&mut self) -> TsFileResult<()> {
        if self.cache.is_some() {
            return Ok(());
        }

        self.cache = Some(self.sequence_reader.read_all_data()?);
        Ok(())
    }

    /// Read all time-value pairs for a specific device and measurement.
    pub fn read_timeseries(
        &mut self,
        device_id: &str,
        measurement_id: &str,
    ) -> TsFileResult<Vec<TimeValuePair>> {
        self.load_cache()?;
        let cache = self.cache.as_ref().unwrap();
        Ok(cache
            .get(device_id)
            .and_then(|d| d.get(measurement_id))
            .cloned()
            .unwrap_or_default())
    }

    /// Read a timeseries restricted by a time range.
    pub fn read_timeseries_by_time_range(
        &mut self,
        device_id: &str,
        measurement_id: &str,
        time_range: TimeRange,
    ) -> TsFileResult<Vec<TimeValuePair>> {
        Ok(self
            .read_timeseries(device_id, measurement_id)?
            .into_iter()
            .filter(|pair| time_range.contains_time(pair.timestamp))
            .collect())
    }

    pub fn get_point_reader(
        &mut self,
        device_id: &str,
        measurement_id: &str,
    ) -> TsFileResult<VecPointReader> {
        Ok(VecPointReader::new(self.read_timeseries(device_id, measurement_id)?))
    }

    pub fn query(&mut self, expression: QueryExpression) -> TsFileResult<ResultSet> {
        let mut rows = self.read_rows(&expression.device_id, &expression.measurements)?;
        if let Some(time_range) = expression.time_range {
            rows.retain(|row| time_range.contains_time(row.timestamp));
        }
        Ok(ResultSet::new(expression.measurements, rows))
    }

    /// Read rows for a device. Each returned row contains fields in the requested measurement order.
    pub fn read_rows(
        &mut self,
        device_id: &str,
        measurements: &[String],
    ) -> TsFileResult<Vec<RowRecord>> {
        self.load_cache()?;
        let Some(device_map) = self.cache.as_ref().unwrap().get(device_id) else {
            return Ok(Vec::new());
        };

        let mut timestamps: Vec<i64> = device_map
            .values()
            .flat_map(|pairs| pairs.iter().map(|pair| pair.timestamp))
            .collect();
        timestamps.sort_unstable();
        timestamps.dedup();

        let mut rows = Vec::with_capacity(timestamps.len());
        for timestamp in timestamps {
            let mut row = RowRecord::new(timestamp);
            for measurement in measurements {
                let value = device_map
                    .get(measurement)
                    .and_then(|pairs| pairs.iter().find(|pair| pair.timestamp == timestamp))
                    .map(|pair| pair.value.clone())
                    .unwrap_or(TimeValue::Null);
                row.add_field(field_from_time_value(value));
            }
            rows.push(row);
        }
        Ok(rows)
    }

    /// Get all device IDs in the file.
    pub fn get_all_devices(&mut self) -> TsFileResult<Vec<String>> {
        self.load_cache()?;
        let cache = self.cache.as_ref().unwrap();
        let mut devices: Vec<String> = cache.keys().cloned().collect();
        devices.sort();
        Ok(devices)
    }

    /// Get all measurement IDs for a device.
    pub fn get_measurements_for_device(
        &mut self,
        device_id: &str,
    ) -> TsFileResult<Vec<String>> {
        self.load_cache()?;
        let cache = self.cache.as_ref().unwrap();
        let mut measurements: Vec<String> = cache
            .get(device_id)
            .map(|d| d.keys().cloned().collect())
            .unwrap_or_default();
        measurements.sort();
        Ok(measurements)
    }

    /// Check whether a device exists in the file.
    pub fn contains_device(&mut self, device_id: &str) -> TsFileResult<bool> {
        self.load_cache()?;
        Ok(self.cache.as_ref().unwrap().contains_key(device_id))
    }

    /// Check whether a specific timeseries exists in the file.
    pub fn contains_timeseries(
        &mut self,
        device_id: &str,
        measurement_id: &str,
    ) -> TsFileResult<bool> {
        self.load_cache()?;
        Ok(self
            .cache
            .as_ref()
            .unwrap()
            .get(device_id)
            .is_some_and(|device| device.contains_key(measurement_id)))
    }

    /// Read all data in the file.
    pub fn read_all(
        &mut self,
    ) -> TsFileResult<&HashMap<String, HashMap<String, Vec<TimeValuePair>>>> {
        self.load_cache()?;
        Ok(self.cache.as_ref().unwrap())
    }
}

impl TsFileReadApi for TsFileReader {
    fn read_timeseries(
        &mut self,
        device_id: &str,
        measurement_id: &str,
    ) -> TsFileResult<Vec<TimeValuePair>> {
        TsFileReader::read_timeseries(self, device_id, measurement_id)
    }

    fn query(&mut self, expression: QueryExpression) -> TsFileResult<ResultSet> {
        TsFileReader::query(self, expression)
    }
}

fn field_from_time_value(value: TimeValue) -> Field {
    match value {
        TimeValue::Boolean(value) => Field::boolean(value),
        TimeValue::Int32(value) => Field::int32(value),
        TimeValue::Int64(value) => Field::int64(value),
        TimeValue::Float(value) => Field::float(value),
        TimeValue::Double(value) => Field::double(value),
        TimeValue::Text(value) => Field::text(value),
        TimeValue::Null => Field::null(),
    }
}
