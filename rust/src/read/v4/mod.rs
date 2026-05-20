// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Table-model/tree-model reader facades.

use std::path::{Path, PathBuf};

use crate::error::TsFileResult;
use crate::file::metadata::table_schema::TableSchema;
use crate::read::common::{RowRecord, TimeRange};
use crate::read::query::{
    query_data_set_to_tsblock_reader, tree_result_set_from_query_data_set, QueryDataSet,
    TableResultSet, TreeResultSet,
};
use crate::read::result_set::{QueryExpression, ResultSet};
use crate::read::tsfile_reader::TsFileReader;
use crate::read::time_value_pair::TimeValuePair;
use crate::write::schema::MeasurementSchema;

pub trait ITsFileReader {
    fn query(&mut self, expression: QueryExpression) -> TsFileResult<ResultSet>;
}

pub struct TsFileTreeReader {
    inner: TsFileReader,
}

impl TsFileTreeReader {
    pub fn new<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        Ok(TsFileTreeReader { inner: TsFileReader::new(path)? })
    }

    pub fn query_tree(
        &mut self,
        device_ids: &[String],
        measurement_names: &[String],
        start_time: i64,
        end_time: i64,
    ) -> TsFileResult<ResultSet> {
        self.inner.query_devices(
            device_ids,
            measurement_names,
            Some(TimeRange::new(start_time, end_time)),
        )
    }

    pub fn query_all(&mut self) -> TsFileResult<ResultSet> {
        self.inner.query_all()
    }

    pub fn query_tree_result_set(
        &mut self,
        device_ids: &[String],
        measurement_names: &[String],
        start_time: i64,
        end_time: i64,
    ) -> TsFileResult<TreeResultSet> {
        let result_set = self.inner.query_devices(
            device_ids,
            measurement_names,
            Some(TimeRange::new(start_time, end_time)),
        )?;
        Ok(tree_result_set_from_query_data_set(
            QueryDataSet::from_result_set(result_set),
            measurement_names.to_vec(),
        ))
    }

    pub fn query_all_by_time_range(
        &mut self,
        start_time: i64,
        end_time: i64,
    ) -> TsFileResult<ResultSet> {
        self.inner
            .query_all_by_time_range(Some(TimeRange::new(start_time, end_time)))
    }

    pub fn get_all_device_ids(&mut self) -> TsFileResult<Vec<String>> {
        self.inner.get_all_device_ids()
    }

    pub fn get_device_schema(&mut self, device_id: &str) -> TsFileResult<Vec<MeasurementSchema>> {
        self.inner.get_measurement(device_id)
    }

    pub fn get_all_measurements(
        &mut self,
    ) -> TsFileResult<std::collections::HashMap<String, crate::common::enums::TSDataType>> {
        self.inner.get_all_measurements()
    }

    pub fn get_device_measurements_map(
        &mut self,
    ) -> TsFileResult<std::collections::HashMap<String, Vec<String>>> {
        self.inner.get_device_measurements_map()
    }

    pub fn get_full_path_data_type_map(
        &mut self,
    ) -> TsFileResult<std::collections::HashMap<String, crate::common::enums::TSDataType>> {
        self.inner.get_full_path_data_type_map()
    }

    pub fn read_device(
        &mut self,
        device_id: &str,
    ) -> TsFileResult<std::collections::HashMap<String, Vec<TimeValuePair>>> {
        self.inner.read_device(device_id)
    }

    pub fn read_device_rows(&mut self, device_id: &str) -> TsFileResult<Vec<RowRecord>> {
        self.inner.read_device_rows(device_id)
    }

    pub fn read_all(
        &mut self,
    ) -> TsFileResult<&std::collections::HashMap<String, std::collections::HashMap<String, Vec<TimeValuePair>>>> {
        self.inner.read_all()
    }

    pub fn close(self) -> TsFileResult<()> {
        self.inner.close()
    }

    pub fn inner_mut(&mut self) -> &mut TsFileReader {
        &mut self.inner
    }
}

impl ITsFileReader for TsFileTreeReader {
    fn query(&mut self, expression: QueryExpression) -> TsFileResult<ResultSet> {
        self.inner.query(expression)
    }
}

pub struct DeviceTableModelReader {
    inner: TsFileReader,
}

impl DeviceTableModelReader {
    pub fn new<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        Ok(DeviceTableModelReader { inner: TsFileReader::new(path)? })
    }

    pub fn query_table(
        &mut self,
        table_name: &str,
        column_names: &[String],
        start_time: i64,
        end_time: i64,
    ) -> TsFileResult<ResultSet> {
        let device_ids = self.inner.get_table_devices(table_name)?;
        self.inner.query_devices(
            &device_ids,
            column_names,
            Some(TimeRange::new(start_time, end_time)),
        )
    }

    pub fn query_all_columns(
        &mut self,
        table_name: &str,
        start_time: i64,
        end_time: i64,
    ) -> TsFileResult<ResultSet> {
        let Some(schema) = self.inner.get_table_schema(table_name)? else {
            return Ok(ResultSet::new(Vec::new(), Vec::new()));
        };
        let columns: Vec<String> = schema
            .measurement_schemas
            .iter()
            .map(|measurement| measurement.measurement_id.clone())
            .collect();
        self.query_table(table_name, &columns, start_time, end_time)
    }

    pub fn query_table_result_set(
        &mut self,
        table_name: &str,
        column_names: &[String],
        start_time: i64,
        end_time: i64,
        block_size: usize,
    ) -> TsFileResult<TableResultSet> {
        let result_set = self.query_table(table_name, column_names, start_time, end_time)?;
        let schema = self
            .inner
            .get_table_schema(table_name)?
            .ok_or_else(|| crate::error::TsFileError::MeasurementNotFound(table_name.to_string()))?;
        let mut column_types = Vec::with_capacity(column_names.len() + 1);
        column_types.push(crate::common::enums::TSDataType::Text);
        for column_name in column_names {
            let data_type = schema
                .measurement_schemas
                .iter()
                .find(|measurement| measurement.measurement_id == *column_name)
                .map(|measurement| measurement.data_type)
                .ok_or_else(|| crate::error::TsFileError::MeasurementNotFound(column_name.clone()))?;
            column_types.push(data_type);
        }

        let query_data_set = QueryDataSet::from_result_set(result_set);
        let ts_block_reader =
            query_data_set_to_tsblock_reader(&query_data_set, &column_types, block_size)?;
        let mut names = Vec::with_capacity(column_names.len() + 1);
        names.push("device_id".to_string());
        names.extend(column_names.iter().cloned());
        Ok(TableResultSet::new(
            Box::new(ts_block_reader),
            names,
            column_types,
            table_name.to_string(),
        ))
    }

    pub fn query_all_columns_result_set(
        &mut self,
        table_name: &str,
        start_time: i64,
        end_time: i64,
        block_size: usize,
    ) -> TsFileResult<TableResultSet> {
        let schema = self
            .inner
            .get_table_schema(table_name)?
            .ok_or_else(|| crate::error::TsFileError::MeasurementNotFound(table_name.to_string()))?;
        let columns = schema
            .measurement_schemas
            .iter()
            .map(|measurement| measurement.measurement_id.clone())
            .collect::<Vec<_>>();
        self.query_table_result_set(
            table_name,
            &columns,
            start_time,
            end_time,
            block_size,
        )
    }

    pub fn get_table_schema(&mut self, table_name: &str) -> TsFileResult<Option<TableSchema>> {
        self.inner.get_table_schema(table_name)
    }

    pub fn get_all_table_schemas(&mut self) -> TsFileResult<Vec<TableSchema>> {
        self.inner.get_all_table_schemas()
    }

    pub fn get_table_devices(&mut self, table_name: &str) -> TsFileResult<Vec<String>> {
        self.inner.get_table_devices(table_name)
    }

    pub fn inner_mut(&mut self) -> &mut TsFileReader {
        &mut self.inner
    }

    pub fn close(self) -> TsFileResult<()> {
        self.inner.close()
    }
}

impl ITsFileReader for DeviceTableModelReader {
    fn query(&mut self, expression: QueryExpression) -> TsFileResult<ResultSet> {
        self.inner.query(expression)
    }
}

#[derive(Debug, Clone)]
pub struct TsFileTreeReaderBuilder {
    path: PathBuf,
}

impl TsFileTreeReaderBuilder {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        TsFileTreeReaderBuilder {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn build(self) -> TsFileResult<TsFileTreeReader> {
        TsFileTreeReader::new(self.path)
    }
}

#[derive(Debug, Clone)]
pub struct DeviceTableModelReaderBuilder {
    path: PathBuf,
}

impl DeviceTableModelReaderBuilder {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        DeviceTableModelReaderBuilder {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn build(self) -> TsFileResult<DeviceTableModelReader> {
        DeviceTableModelReader::new(self.path)
    }
}
