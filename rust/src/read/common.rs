// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Common read-side data structures mirroring Java's `read.common` package.

use std::fmt;

use crate::common::enums::TSDataType;
use crate::file::header::ChunkHeader;
use crate::file::metadata::device_id::DeviceId;
use crate::file::metadata::statistics::Statistics;
use crate::read::time_value_pair::TimeValue;
use crate::utils::read_write_io_utils::Binary;

/// One typed value in a row record.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub data_type: Option<TSDataType>,
    pub value: TimeValue,
}

impl Field {
    pub fn null() -> Self {
        Field {
            data_type: None,
            value: TimeValue::Null,
        }
    }

    pub fn new(data_type: TSDataType, value: TimeValue) -> Self {
        Field {
            data_type: Some(data_type),
            value,
        }
    }

    pub fn boolean(value: bool) -> Self {
        Self::new(TSDataType::Boolean, TimeValue::Boolean(value))
    }

    pub fn int32(value: i32) -> Self {
        Self::new(TSDataType::Int32, TimeValue::Int32(value))
    }

    pub fn int64(value: i64) -> Self {
        Self::new(TSDataType::Int64, TimeValue::Int64(value))
    }

    pub fn float(value: f32) -> Self {
        Self::new(TSDataType::Float, TimeValue::Float(value))
    }

    pub fn double(value: f64) -> Self {
        Self::new(TSDataType::Double, TimeValue::Double(value))
    }

    pub fn text(value: Binary) -> Self {
        Self::new(TSDataType::Text, TimeValue::Text(value))
    }

    pub fn is_null(&self) -> bool {
        self.data_type.is_none() || self.value.is_null()
    }

    pub fn string_value(&self) -> String {
        match &self.value {
            TimeValue::Boolean(value) => value.to_string(),
            TimeValue::Int32(value) => value.to_string(),
            TimeValue::Int64(value) => value.to_string(),
            TimeValue::Float(value) => value.to_string(),
            TimeValue::Double(value) => value.to_string(),
            TimeValue::Text(value) => value.to_string(),
            TimeValue::Null => "null".to_string(),
        }
    }
}

impl fmt::Display for Field {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.string_value())
    }
}

/// A timestamp plus a list of fields.
#[derive(Debug, Clone, PartialEq)]
pub struct RowRecord {
    pub timestamp: i64,
    pub fields: Vec<Field>,
    has_null_field: bool,
    all_null: bool,
}

impl RowRecord {
    pub fn new(timestamp: i64) -> Self {
        RowRecord {
            timestamp,
            fields: Vec::new(),
            has_null_field: false,
            all_null: true,
        }
    }

    pub fn with_fields(timestamp: i64, fields: Vec<Field>) -> Self {
        let mut record = RowRecord::new(timestamp);
        for field in fields {
            record.add_field(field);
        }
        record
    }

    pub fn add_field(&mut self, field: Field) {
        if field.is_null() {
            self.has_null_field = true;
        } else {
            self.all_null = false;
        }
        self.fields.push(field);
    }

    pub fn set_field(&mut self, index: usize, field: Field) {
        self.fields[index] = field;
        self.recompute_null_flags();
    }

    pub fn field(&self, index: usize) -> Option<&Field> {
        self.fields.get(index)
    }

    pub fn has_null_field(&self) -> bool {
        self.has_null_field
    }

    pub fn is_all_null(&self) -> bool {
        self.all_null
    }

    pub fn reset_null_flag(&mut self) {
        self.has_null_field = false;
        self.all_null = true;
    }

    fn recompute_null_flags(&mut self) {
        self.has_null_field = self.fields.iter().any(Field::is_null);
        self.all_null = self.fields.iter().all(Field::is_null);
    }
}

impl fmt::Display for RowRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.timestamp)?;
        for field in &self.fields {
            write!(formatter, "\t{}", field)?;
        }
        Ok(())
    }
}

/// Closed/open time interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeRange {
    pub min: i64,
    pub max: i64,
    pub left_closed: bool,
    pub right_closed: bool,
}

impl TimeRange {
    pub fn new(min: i64, max: i64) -> Self {
        TimeRange::with_bounds(min, max, true, true)
    }

    pub fn with_bounds(min: i64, max: i64, left_closed: bool, right_closed: bool) -> Self {
        assert!(min <= max, "min must not be larger than max");
        TimeRange {
            min,
            max,
            left_closed,
            right_closed,
        }
    }

    pub fn contains_time(&self, time: i64) -> bool {
        let left = if self.left_closed { time >= self.min } else { time > self.min };
        let right = if self.right_closed { time <= self.max } else { time < self.max };
        left && right
    }

    pub fn contains_range(&self, other: TimeRange) -> bool {
        self.contains_time(other.min) && self.contains_time(other.max)
    }

    pub fn intersects(&self, other: TimeRange) -> bool {
        self.contains_time(other.min)
            || self.contains_time(other.max)
            || other.contains_time(self.min)
            || other.contains_time(self.max)
    }
}

/// A full path composed of device and measurement.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path {
    pub device: String,
    pub measurement: String,
}

/// A timeseries path represented by device id plus measurement name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimeSeries {
    pub device_id: DeviceId,
    pub measurement_name: String,
}

impl TimeSeries {
    pub fn new(device_id: DeviceId, measurement_name: String) -> Self {
        TimeSeries {
            device_id,
            measurement_name,
        }
    }

    pub fn path_list(device_id: DeviceId, measurements: &[String]) -> Vec<TimeSeries> {
        measurements
            .iter()
            .cloned()
            .map(|measurement| TimeSeries::new(device_id.clone(), measurement))
            .collect()
    }
}

/// A raw chunk and associated metadata used by readers.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub header: ChunkHeader,
    pub data: Vec<u8>,
    pub delete_interval_list: Vec<TimeRange>,
    pub statistics: Option<Statistics>,
}

impl Chunk {
    pub fn new(header: ChunkHeader, data: Vec<u8>) -> Self {
        Chunk {
            header,
            data,
            delete_interval_list: Vec::new(),
            statistics: None,
        }
    }

    pub fn with_statistics(
        header: ChunkHeader,
        data: Vec<u8>,
        delete_interval_list: Vec<TimeRange>,
        statistics: Statistics,
    ) -> Self {
        Chunk {
            header,
            data,
            delete_interval_list,
            statistics: Some(statistics),
        }
    }
}

/// Ordered batch of time-value pairs for one series.
#[derive(Debug, Clone, Default)]
pub struct BatchData {
    values: Vec<(i64, TimeValue)>,
    cursor: usize,
}

impl BatchData {
    pub fn new() -> Self {
        BatchData::default()
    }

    pub fn put(&mut self, timestamp: i64, value: TimeValue) {
        self.values.push((timestamp, value));
    }

    pub fn sort_by_time(&mut self) {
        self.values.sort_by_key(|(timestamp, _)| *timestamp);
    }

    pub fn has_current(&self) -> bool {
        self.cursor < self.values.len()
    }

    pub fn current_time(&self) -> Option<i64> {
        self.values.get(self.cursor).map(|(timestamp, _)| *timestamp)
    }

    pub fn current_value(&self) -> Option<&TimeValue> {
        self.values.get(self.cursor).map(|(_, value)| value)
    }

    pub fn next(&mut self) -> bool {
        if self.cursor < self.values.len() {
            self.cursor += 1;
        }
        self.has_current()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl Path {
    pub fn new(device: String, measurement: String) -> Self {
        Path { device, measurement }
    }

    pub fn full_path(&self) -> String {
        format!("{}.{}", self.device, self.measurement)
    }
}
