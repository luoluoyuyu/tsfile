// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Query dataset abstractions mirroring Java's `read.query.dataset` package.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::common::enums::TSDataType;
use crate::read::block::column::ColumnValue;
use crate::read::common::{Field, Path as SeriesPath, RowRecord};
use crate::read::reader::TsBlockReader;
use crate::read::result_set::ResultSet;
use crate::read::time_value_pair::{TimeValue, TimeValuePair};
use crate::utils::read_write_io_utils::Binary;
use crate::write::record::{DataPoint, TSRecord};

#[derive(Debug, Clone)]
pub struct QueryDataSet {
    columns: Vec<String>,
    rows: Vec<RowRecord>,
    cursor: usize,
    row_limit: usize,
    row_offset: usize,
    already_returned_row_num: usize,
    without_any_null: bool,
    without_all_null: bool,
    without_null_columns_index: Option<HashSet<usize>>,
    fetch_size: usize,
    ascending: bool,
}

impl QueryDataSet {
    pub fn new(columns: Vec<String>, rows: Vec<RowRecord>) -> Self {
        QueryDataSet {
            columns,
            rows,
            cursor: 0,
            row_limit: 0,
            row_offset: 0,
            already_returned_row_num: 0,
            without_any_null: false,
            without_all_null: false,
            without_null_columns_index: None,
            fetch_size: 10_000,
            ascending: true,
        }
    }

    pub fn from_result_set(result_set: ResultSet) -> Self {
        QueryDataSet::new(result_set.columns().to_vec(), result_set.rows().to_vec())
    }

    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    pub fn rows(&self) -> &[RowRecord] {
        &self.rows
    }

    pub fn set_row_limit(&mut self, row_limit: usize) {
        self.row_limit = row_limit;
    }

    pub fn set_row_offset(&mut self, row_offset: usize) {
        self.row_offset = row_offset;
    }

    pub fn row_limit(&self) -> usize {
        self.row_limit
    }

    pub fn row_offset(&self) -> usize {
        self.row_offset
    }

    pub fn has_limit(&self) -> bool {
        self.row_limit > 0
    }

    pub fn set_fetch_size(&mut self, fetch_size: usize) {
        self.fetch_size = fetch_size;
    }

    pub fn fetch_size(&self) -> usize {
        self.fetch_size
    }

    pub fn set_ascending(&mut self, ascending: bool) {
        self.ascending = ascending;
    }

    pub fn ascending(&self) -> bool {
        self.ascending
    }

    pub fn set_without_any_null(&mut self, without_any_null: bool) {
        self.without_any_null = without_any_null;
    }

    pub fn set_without_all_null(&mut self, without_all_null: bool) {
        self.without_all_null = without_all_null;
    }

    pub fn set_without_null_columns_index(
        &mut self,
        without_null_columns_index: Option<HashSet<usize>>,
    ) {
        self.without_null_columns_index = without_null_columns_index;
    }

    pub fn has_next(&mut self) -> bool {
        if self.row_limit > 0 && self.already_returned_row_num >= self.row_limit {
            return false;
        }

        let mut probe_cursor = self.cursor;
        let mut probe_offset = self.row_offset;
        while probe_cursor < self.rows.len() {
            let row = &self.rows[probe_cursor];
            probe_cursor += 1;
            if self.without_null_filter(row) {
                continue;
            }
            if probe_offset > 0 {
                probe_offset -= 1;
                continue;
            }
            return true;
        }
        false
    }

    pub fn next(&mut self) -> Option<&RowRecord> {
        while self.cursor < self.rows.len() {
            let index = self.cursor;
            self.cursor += 1;
            let row = &self.rows[index];
            if self.without_null_filter(row) {
                continue;
            }
            if self.row_offset > 0 {
                self.row_offset -= 1;
                continue;
            }
            if self.row_limit > 0 && self.already_returned_row_num >= self.row_limit {
                return None;
            }
            self.already_returned_row_num += 1;
            return Some(row);
        }
        None
    }

    pub fn reset(&mut self) {
        self.cursor = 0;
        self.already_returned_row_num = 0;
    }

    pub fn into_result_set(self) -> ResultSet {
        ResultSet::new(self.columns, self.rows)
    }

    fn without_null_filter(&self, row: &RowRecord) -> bool {
        let mut any_null_flag = row.has_null_field();
        let mut all_null_flag = row.is_all_null();

        if let Some(indexes) = &self.without_null_columns_index {
            if indexes.is_empty() {
                any_null_flag = row.has_null_field();
                all_null_flag = row.is_all_null();
            } else {
                any_null_flag = false;
                all_null_flag = true;
                for index in indexes {
                    let is_null = row.field(*index).is_none_or(|field| field.is_null());
                    any_null_flag |= is_null;
                    all_null_flag &= is_null;
                }
            }
        }

        (self.without_any_null && any_null_flag) || (self.without_all_null && all_null_flag)
    }
}

#[derive(Debug, Clone)]
pub struct DataSetWithoutTimeGenerator {
    paths: Vec<SeriesPath>,
    data_types: Vec<TSDataType>,
    inner: QueryDataSet,
}

impl DataSetWithoutTimeGenerator {
    pub fn new(
        paths: Vec<SeriesPath>,
        data_types: Vec<TSDataType>,
        series_data: Vec<Vec<TimeValuePair>>,
    ) -> Self {
        let columns = paths
            .iter()
            .map(|path| path.measurement.clone())
            .collect::<Vec<_>>();
        let rows = rows_from_series_data(&paths, &series_data, None);
        DataSetWithoutTimeGenerator {
            paths,
            data_types,
            inner: QueryDataSet::new(columns, rows),
        }
    }

    pub fn paths(&self) -> &[SeriesPath] {
        &self.paths
    }

    pub fn data_types(&self) -> &[TSDataType] {
        &self.data_types
    }

    pub fn query_data_set(&self) -> &QueryDataSet {
        &self.inner
    }

    pub fn query_data_set_mut(&mut self) -> &mut QueryDataSet {
        &mut self.inner
    }

    pub fn into_query_data_set(self) -> QueryDataSet {
        self.inner
    }
}

#[derive(Debug, Clone)]
pub struct DataSetWithTimeGenerator {
    paths: Vec<SeriesPath>,
    data_types: Vec<TSDataType>,
    generated_timestamps: Vec<i64>,
    inner: QueryDataSet,
}

impl DataSetWithTimeGenerator {
    pub fn new(
        paths: Vec<SeriesPath>,
        data_types: Vec<TSDataType>,
        series_data: Vec<Vec<TimeValuePair>>,
        generated_timestamps: Vec<i64>,
    ) -> Self {
        let columns = paths
            .iter()
            .map(|path| path.measurement.clone())
            .collect::<Vec<_>>();
        let rows = rows_from_series_data(&paths, &series_data, Some(&generated_timestamps));
        DataSetWithTimeGenerator {
            paths,
            data_types,
            generated_timestamps,
            inner: QueryDataSet::new(columns, rows),
        }
    }

    pub fn generated_timestamps(&self) -> &[i64] {
        &self.generated_timestamps
    }

    pub fn paths(&self) -> &[SeriesPath] {
        &self.paths
    }

    pub fn data_types(&self) -> &[TSDataType] {
        &self.data_types
    }

    pub fn query_data_set(&self) -> &QueryDataSet {
        &self.inner
    }

    pub fn query_data_set_mut(&mut self) -> &mut QueryDataSet {
        &mut self.inner
    }

    pub fn into_query_data_set(self) -> QueryDataSet {
        self.inner
    }
}

pub struct TreeResultSet {
    query_data_set: QueryDataSet,
    measurement_names: Vec<String>,
}

impl TreeResultSet {
    pub fn new(query_data_set: QueryDataSet, measurement_names: Vec<String>) -> Self {
        TreeResultSet {
            query_data_set,
            measurement_names,
        }
    }

    pub fn next_record(&mut self) -> Option<TSRecord> {
        while self.query_data_set.has_next() {
            let row = self.query_data_set.next()?.clone();
            if row.is_all_null() {
                continue;
            }
            let device_id = row
                .field(0)
                .map(Field::string_value)
                .unwrap_or_default();
            let mut record = TSRecord::new(row.timestamp, device_id);
            for (index, measurement) in self.measurement_names.iter().enumerate() {
                let field_index = index + 1;
                if let Some(data_point) = row
                    .field(field_index)
                    .and_then(|field| field_to_data_point(measurement, field))
                {
                    record.add_tuple(data_point);
                }
            }
            if !record.data_points.is_empty() {
                return Some(record);
            }
        }
        None
    }

    pub fn close(self) {}
}

impl Iterator for TreeResultSet {
    type Item = TSRecord;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_record()
    }
}

pub struct TableResultSet {
    ts_block_reader: Box<dyn TsBlockReader>,
    column_names: Vec<String>,
    data_types: Vec<TSDataType>,
    table_name: String,
    current_tsblock: Option<crate::read::TsBlock>,
    current_index: usize,
}

impl TableResultSet {
    pub fn new(
        ts_block_reader: Box<dyn TsBlockReader>,
        column_names: Vec<String>,
        data_types: Vec<TSDataType>,
        table_name: String,
    ) -> Self {
        TableResultSet {
            ts_block_reader,
            column_names,
            data_types,
            table_name,
            current_tsblock: None,
            current_index: 0,
        }
    }

    pub fn next_record(&mut self) -> Option<TSRecord> {
        loop {
            if let Some(tsblock) = &self.current_tsblock {
                if self.current_index < tsblock.position_count() {
                    let row_index = self.current_index;
                    self.current_index += 1;
                    return Some(self.record_from_current_block(row_index));
                }
            }

            let next_block = self.ts_block_reader.next().ok().flatten()?;
            self.current_tsblock = Some(next_block);
            self.current_index = 0;
        }
    }

    fn record_from_current_block(&self, row_index: usize) -> TSRecord {
        let tsblock = self.current_tsblock.as_ref().expect("current block must exist");
        let timestamp = tsblock.time_by_index(row_index).unwrap_or_default();
        let device_id = if self
            .column_names
            .first()
            .is_some_and(|column| column == "device_id")
        {
            tsblock
                .column(0)
                .and_then(|column| column.get_binary(row_index))
                .map(|value| value.to_string())
                .unwrap_or_else(|| self.table_name.clone())
        } else {
            self.table_name.clone()
        };
        let mut record = TSRecord::new(timestamp, device_id);

        let start_index = usize::from(
            self.column_names
                .first()
                .is_some_and(|column| column == "device_id"),
        );
        for column_index in start_index..self.column_names.len() {
            let value_column = tsblock.column(column_index).unwrap();
            let value = value_column.get(row_index).unwrap_or(&ColumnValue::Null);
            if let Some(data_point) = column_value_to_data_point(
                &self.column_names[column_index],
                self.data_types[column_index],
                value,
            ) {
                record.add_tuple(data_point);
            }
        }

        record
    }

    pub fn close(&mut self) {
        let _ = self.ts_block_reader.close();
    }
}

impl Iterator for TableResultSet {
    type Item = TSRecord;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_record()
    }
}

fn rows_from_series_data(
    paths: &[SeriesPath],
    series_data: &[Vec<TimeValuePair>],
    selected_timestamps: Option<&[i64]>,
) -> Vec<RowRecord> {
    let mut timestamps = BTreeSet::new();
    if let Some(selected_timestamps) = selected_timestamps {
        timestamps.extend(selected_timestamps.iter().copied());
    } else {
        for points in series_data {
            for point in points {
                timestamps.insert(point.timestamp);
            }
        }
    }

    let point_maps = series_data
        .iter()
        .map(|points| {
            points
                .iter()
                .map(|point| (point.timestamp, point.value.clone()))
                .collect::<HashMap<_, _>>()
        })
        .collect::<Vec<_>>();

    let mut rows = Vec::with_capacity(timestamps.len());
    for timestamp in timestamps {
        let mut row = RowRecord::new(timestamp);
        for (path_index, _path) in paths.iter().enumerate() {
            let value = point_maps[path_index]
                .get(&timestamp)
                .cloned()
                .unwrap_or(TimeValue::Null);
            row.add_field(field_from_time_value(value));
        }
        rows.push(row);
    }
    rows
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

fn field_to_data_point(measurement_id: &str, field: &Field) -> Option<DataPoint> {
    match &field.value {
        TimeValue::Boolean(value) => Some(DataPoint::new_bool(measurement_id.to_string(), *value)),
        TimeValue::Int32(value) => Some(DataPoint::new_i32(measurement_id.to_string(), *value)),
        TimeValue::Int64(value) => Some(DataPoint::new_i64(measurement_id.to_string(), *value)),
        TimeValue::Float(value) => Some(DataPoint::new_f32(measurement_id.to_string(), *value)),
        TimeValue::Double(value) => Some(DataPoint::new_f64(measurement_id.to_string(), *value)),
        TimeValue::Text(value) => Some(DataPoint::new_text(
            measurement_id.to_string(),
            value.clone(),
        )),
        TimeValue::Null => None,
    }
}

fn column_value_to_data_point(
    measurement_id: &str,
    data_type: TSDataType,
    value: &ColumnValue,
) -> Option<DataPoint> {
    match (data_type, value) {
        (_, ColumnValue::Null) => None,
        (TSDataType::Boolean, ColumnValue::Boolean(value)) => {
            Some(DataPoint::new_bool(measurement_id.to_string(), *value))
        }
        (TSDataType::Int32 | TSDataType::Date, ColumnValue::Int32(value)) => {
            Some(DataPoint::new_i32(measurement_id.to_string(), *value))
        }
        (TSDataType::Int64 | TSDataType::Timestamp, ColumnValue::Int64(value)) => {
            Some(DataPoint::new_i64(measurement_id.to_string(), *value))
        }
        (TSDataType::Float, ColumnValue::Float(value)) => {
            Some(DataPoint::new_f32(measurement_id.to_string(), *value))
        }
        (TSDataType::Double, ColumnValue::Double(value)) => {
            Some(DataPoint::new_f64(measurement_id.to_string(), *value))
        }
        (TSDataType::Text | TSDataType::Blob | TSDataType::String, ColumnValue::Binary(value)) => {
            Some(DataPoint::new_text(measurement_id.to_string(), value.clone()))
        }
        _ => None,
    }
}

pub fn row_record_to_column_values(row: &RowRecord) -> Vec<ColumnValue> {
    row.fields
        .iter()
        .map(|field| match &field.value {
            TimeValue::Boolean(value) => ColumnValue::Boolean(*value),
            TimeValue::Int32(value) => ColumnValue::Int32(*value),
            TimeValue::Int64(value) => ColumnValue::Int64(*value),
            TimeValue::Float(value) => ColumnValue::Float(*value),
            TimeValue::Double(value) => ColumnValue::Double(*value),
            TimeValue::Text(value) => ColumnValue::Binary(value.clone()),
            TimeValue::Null => ColumnValue::Null,
        })
        .collect()
}

pub fn device_id_field(device_id: &str) -> Field {
    Field::text(Binary::from_str(device_id))
}
