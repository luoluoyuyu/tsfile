// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TsFileReader: high-level API for reading TsFiles.
//!
//! Mirrors Java's TsFileReader.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use crate::common::config::get_config;
use crate::error::TsFileResult;
use crate::file::metadata::table_schema::TableSchema;
use crate::read::api::TsFileReadApi;
use crate::read::common::{Field, RowRecord, TimeRange};
use crate::read::reader::VecPointReader;
use crate::read::result_set::{QueryExpression, ResultSet};
use crate::read::time_value_pair::TimeValue;
use crate::read::time_value_pair::TimeValuePair;
use crate::read::tsfile_sequence_reader::TsFileSequenceReader;
use crate::write::schema::MeasurementSchema;

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

    pub fn file_version(&self) -> u8 {
        self.sequence_reader.file_version()
    }

    pub fn file_size(&self) -> u64 {
        self.sequence_reader.file_size()
    }

    pub fn is_complete(&mut self) -> TsFileResult<bool> {
        self.sequence_reader.is_complete()
    }

    /// Read all time-value pairs for a specific device and measurement.
    pub fn read_timeseries(
        &mut self,
        device_id: &str,
        measurement_id: &str,
    ) -> TsFileResult<Vec<TimeValuePair>> {
        if self.cache.is_none() {
            let points = self
                .sequence_reader
                .read_timeseries_by_index(device_id, measurement_id)?;
            if !points.is_empty() {
                return Ok(points);
            }
        }

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

    pub fn read_device(
        &mut self,
        device_id: &str,
    ) -> TsFileResult<HashMap<String, Vec<TimeValuePair>>> {
        if self.cache.is_none() {
            let device_data = self.sequence_reader.read_device_data(device_id)?;
            if !device_data.is_empty() {
                return Ok(device_data);
            }
        }

        self.load_cache()?;
        Ok(self
            .cache
            .as_ref()
            .unwrap()
            .get(device_id)
            .cloned()
            .unwrap_or_default())
    }

    pub fn read_devices(
        &mut self,
        device_ids: &[String],
    ) -> TsFileResult<HashMap<String, HashMap<String, Vec<TimeValuePair>>>> {
        let mut result = HashMap::with_capacity(device_ids.len());
        for device_id in device_ids {
            result.insert(device_id.clone(), self.read_device(device_id)?);
        }
        Ok(result)
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
        Ok(build_rows_from_measurements(device_map, measurements))
    }

    pub fn read_device_rows(&mut self, device_id: &str) -> TsFileResult<Vec<RowRecord>> {
        let measurements = self.get_measurements_for_device(device_id)?;
        self.read_rows(device_id, &measurements)
    }

    /// Get all device IDs in the file.
    pub fn get_all_device_ids(&mut self) -> TsFileResult<Vec<String>> {
        self.sequence_reader.get_all_devices()
    }

    pub fn get_all_devices(&mut self) -> TsFileResult<Vec<String>> {
        self.get_all_device_ids()
    }

    pub fn get_all_measurements(&mut self) -> TsFileResult<HashMap<String, crate::common::enums::TSDataType>> {
        self.sequence_reader.get_all_measurements()
    }

    pub fn get_measurement(&mut self, device_id: &str) -> TsFileResult<Vec<MeasurementSchema>> {
        let config = get_config();
        let mut schemas: Vec<MeasurementSchema> = self
            .sequence_reader
            .get_measurement(device_id)?
            .into_iter()
            .map(|(measurement_id, data_type)| {
                MeasurementSchema::new(
                    measurement_id,
                    data_type,
                    config.value_encoder(data_type),
                    config.compressor(data_type),
                )
            })
            .collect();
        schemas.sort_by(|left, right| left.measurement_id.cmp(&right.measurement_id));
        Ok(schemas)
    }

    /// Get all measurement IDs for a device.
    pub fn get_measurements_for_device(
        &mut self,
        device_id: &str,
    ) -> TsFileResult<Vec<String>> {
        let mut measurements: Vec<String> = self
            .sequence_reader
            .get_measurement(device_id)?
            .into_keys()
            .collect();
        measurements.sort();
        Ok(measurements)
    }

    pub fn get_device_measurements_map(&mut self) -> TsFileResult<HashMap<String, Vec<String>>> {
        self.sequence_reader.get_device_measurements_map()
    }

    pub fn get_full_path_data_type_map(
        &mut self,
    ) -> TsFileResult<HashMap<String, crate::common::enums::TSDataType>> {
        self.sequence_reader.get_full_path_data_type_map()
    }

    /// Check whether a device exists in the file.
    pub fn contains_device(&mut self, device_id: &str) -> TsFileResult<bool> {
        Ok(self
            .sequence_reader
            .get_all_devices()?
            .iter()
            .any(|current| current == device_id))
    }

    /// Check whether a specific timeseries exists in the file.
    pub fn contains_timeseries(
        &mut self,
        device_id: &str,
        measurement_id: &str,
    ) -> TsFileResult<bool> {
        Ok(self
            .sequence_reader
            .get_measurement(device_id)?
            .contains_key(measurement_id))
    }

    /// Read all data in the file.
    pub fn read_all(
        &mut self,
    ) -> TsFileResult<&HashMap<String, HashMap<String, Vec<TimeValuePair>>>> {
        self.load_cache()?;
        Ok(self.cache.as_ref().unwrap())
    }

    pub fn read_all_rows(&mut self) -> TsFileResult<HashMap<String, Vec<RowRecord>>> {
        let device_ids = self.get_all_device_ids()?;
        let mut result = HashMap::with_capacity(device_ids.len());
        for device_id in device_ids {
            result.insert(device_id.clone(), self.read_device_rows(&device_id)?);
        }
        Ok(result)
    }

    pub fn query_devices(
        &mut self,
        device_ids: &[String],
        measurements: &[String],
        time_range: Option<TimeRange>,
    ) -> TsFileResult<ResultSet> {
        let mut rows = Vec::new();
        for device_id in device_ids {
            let device_rows = self.read_rows(device_id, measurements)?;
            for mut row in device_rows {
                if time_range.is_some_and(|range| !range.contains_time(row.timestamp)) {
                    continue;
                }
                row.fields.insert(0, Field::text(crate::utils::read_write_io_utils::Binary::from_str(device_id)));
                rows.push(row);
            }
        }
        rows.sort_by_key(|row| row.timestamp);

        let mut columns = Vec::with_capacity(measurements.len() + 1);
        columns.push("device_id".to_string());
        columns.extend(measurements.iter().cloned());
        Ok(ResultSet::new(columns, rows))
    }

    pub fn query_all(&mut self) -> TsFileResult<ResultSet> {
        self.query_all_by_time_range(None)
    }

    pub fn query_all_by_time_range(
        &mut self,
        time_range: Option<TimeRange>,
    ) -> TsFileResult<ResultSet> {
        let device_ids = self.get_all_device_ids()?;
        let mut measurements = BTreeSet::new();
        for device_id in &device_ids {
            for measurement in self.get_measurements_for_device(device_id)? {
                measurements.insert(measurement);
            }
        }
        let measurement_list: Vec<String> = measurements.into_iter().collect();
        self.query_devices(&device_ids, &measurement_list, time_range)
    }

    pub fn get_table_devices(&mut self, table_name: &str) -> TsFileResult<Vec<String>> {
        self.sequence_reader.get_table_devices(table_name)
    }

    pub fn get_table_schema(&mut self, table_name: &str) -> TsFileResult<Option<TableSchema>> {
        self.sequence_reader.get_table_schema(table_name)
    }

    pub fn get_all_table_schemas(&mut self) -> TsFileResult<Vec<TableSchema>> {
        self.sequence_reader.get_all_table_schemas()
    }

    pub fn get_table_schema_map(&mut self) -> TsFileResult<HashMap<String, TableSchema>> {
        self.sequence_reader.get_table_schema_map()
    }

    pub fn close(self) -> TsFileResult<()> {
        self.sequence_reader.close()
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

fn build_rows_from_measurements(
    device_map: &HashMap<String, Vec<TimeValuePair>>,
    measurements: &[String],
) -> Vec<RowRecord> {
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
    rows
}
