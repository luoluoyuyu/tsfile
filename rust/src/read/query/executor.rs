// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Simplified query executors mirroring Java's `read.query.executor` package.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::path::{Path as FsPath, PathBuf};

use crate::common::enums::TSDataType;
use crate::error::{TsFileError, TsFileResult};
use crate::read::common::{Field, Path as SeriesPath, RowRecord, TimeRange};
use crate::read::controller::{
    CachedChunkLoaderImpl, IChunkLoader, IMetadataQuerier, MetadataQuerierByFileImpl,
};
use crate::read::expression::Expression;
use crate::read::query::dataset::{
    device_id_field, row_record_to_column_values, DataSetWithTimeGenerator,
    DataSetWithoutTimeGenerator, QueryDataSet, TableResultSet, TreeResultSet,
};
use crate::read::reader::VecTsBlockReader;
use crate::read::result_set::QueryExpression;
use crate::read::time_value_pair::{TimeValue, TimeValuePair};
use crate::read::{TsBlock, TsBlockBuilder, TsFileReader};

pub struct TsFileExecutor {
    metadata_querier: MetadataQuerierByFileImpl,
    chunk_loader: CachedChunkLoaderImpl,
}

impl TsFileExecutor {
    pub fn new<P: AsRef<FsPath>>(path: P) -> TsFileResult<Self> {
        Ok(TsFileExecutor {
            metadata_querier: MetadataQuerierByFileImpl::new(path.as_ref())?,
            chunk_loader: CachedChunkLoaderImpl::new(path)?,
        })
    }

    pub fn execute(&mut self, query_expression: QueryExpression) -> TsFileResult<QueryDataSet> {
        Ok(self
            .execute_without_time_generator(query_expression)?
            .into_query_data_set())
    }

    pub fn execute_without_time_generator(
        &mut self,
        query_expression: QueryExpression,
    ) -> TsFileResult<DataSetWithoutTimeGenerator> {
        let (paths, data_types, series_data) = self.load_series_data(
            &query_expression.device_id,
            &query_expression.measurements,
            query_expression.time_range,
        )?;
        Ok(DataSetWithoutTimeGenerator::new(paths, data_types, series_data))
    }

    pub fn execute_with_time_generator(
        &mut self,
        device_id: &str,
        measurements: &[String],
        expression: &Expression,
    ) -> TsFileResult<DataSetWithTimeGenerator> {
        let (paths, data_types, series_data) =
            self.load_series_data(device_id, measurements, None)?;
        let timestamps = generate_timestamps_from_expression(expression, &paths, &series_data);
        Ok(DataSetWithTimeGenerator::new(
            paths,
            data_types,
            series_data,
            timestamps,
        ))
    }

    pub fn execute_with_space_partition(
        &mut self,
        query_expression: QueryExpression,
        space_partition_start_pos: u64,
        space_partition_end_pos: u64,
    ) -> TsFileResult<QueryDataSet> {
        let paths = query_expression
            .measurements
            .iter()
            .cloned()
            .map(|measurement| SeriesPath::new(query_expression.device_id.clone(), measurement))
            .collect::<Vec<_>>();
        let time_partitions = self.metadata_querier.convert_space_to_time_partition(
            &paths,
            space_partition_start_pos,
            space_partition_end_pos,
        )?;
        if time_partitions.is_empty() {
            return Ok(QueryDataSet::new(
                query_expression.measurements.clone(),
                Vec::new(),
            ));
        }

        let (paths, data_types, mut series_data) = self.load_series_data(
            &query_expression.device_id,
            &query_expression.measurements,
            query_expression.time_range,
        )?;
        for points in &mut series_data {
            points.retain(|point| {
                time_partitions
                    .iter()
                    .any(|range| range.contains_time(point.timestamp))
            });
        }
        Ok(DataSetWithoutTimeGenerator::new(paths, data_types, series_data).into_query_data_set())
    }

    pub fn close(self) -> TsFileResult<()> {
        self.chunk_loader.close()?;
        self.metadata_querier.close()
    }

    fn load_series_data(
        &mut self,
        device_id: &str,
        measurements: &[String],
        time_range: Option<TimeRange>,
    ) -> TsFileResult<(Vec<SeriesPath>, Vec<TSDataType>, Vec<Vec<TimeValuePair>>)> {
        let paths = measurements
            .iter()
            .cloned()
            .map(|measurement| SeriesPath::new(device_id.to_string(), measurement))
            .collect::<Vec<_>>();
        self.metadata_querier.load_chunk_metadata(&paths)?;

        let mut data_types = Vec::with_capacity(paths.len());
        let mut series_data = Vec::with_capacity(paths.len());
        for path in &paths {
            data_types.push(
                self.metadata_querier
                    .get_data_type(path)?
                    .ok_or_else(|| {
                        TsFileError::MeasurementNotFound(path.full_path())
                    })?,
            );
            let chunk_metadata = self.metadata_querier.get_chunk_metadata_list(path)?;
            let mut points = Vec::new();
            for chunk in &chunk_metadata {
                points.extend(self.chunk_loader.load_points(chunk, None)?);
            }
            points.sort_by_key(|point| point.timestamp);
            if let Some(time_range) = time_range {
                points.retain(|point| time_range.contains_time(point.timestamp));
            }
            series_data.push(points);
        }
        Ok((paths, data_types, series_data))
    }
}

pub struct ExecutorWithTimeGenerator {
    inner: TsFileExecutor,
}

impl ExecutorWithTimeGenerator {
    pub fn new<P: AsRef<FsPath>>(path: P) -> TsFileResult<Self> {
        Ok(ExecutorWithTimeGenerator {
            inner: TsFileExecutor::new(path)?,
        })
    }

    pub fn execute(
        &mut self,
        device_id: &str,
        measurements: &[String],
        expression: &Expression,
    ) -> TsFileResult<DataSetWithTimeGenerator> {
        self.inner
            .execute_with_time_generator(device_id, measurements, expression)
    }

    pub fn close(self) -> TsFileResult<()> {
        self.inner.close()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableQueryOrdering {
    Device,
    Time,
}

#[derive(Debug, Clone)]
pub struct DeviceQueryTask {
    pub device_id: String,
    pub column_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DeviceTaskIterator {
    tasks: VecDeque<DeviceQueryTask>,
}

impl DeviceTaskIterator {
    pub fn new(device_ids: &[String], column_names: &[String]) -> Self {
        DeviceTaskIterator {
            tasks: device_ids
                .iter()
                .cloned()
                .map(|device_id| DeviceQueryTask {
                    device_id,
                    column_names: column_names.to_vec(),
                })
                .collect(),
        }
    }
}

impl Iterator for DeviceTaskIterator {
    type Item = DeviceQueryTask;

    fn next(&mut self) -> Option<Self::Item> {
        self.tasks.pop_front()
    }
}

pub struct TableQueryExecutor {
    reader: TsFileReader,
    ordering: TableQueryOrdering,
}

impl TableQueryExecutor {
    pub fn new<P: AsRef<FsPath>>(path: P) -> TsFileResult<Self> {
        Self::with_ordering(path, TableQueryOrdering::Device)
    }

    pub fn with_ordering<P: AsRef<FsPath>>(
        path: P,
        ordering: TableQueryOrdering,
    ) -> TsFileResult<Self> {
        Ok(TableQueryExecutor {
            reader: TsFileReader::new(path)?,
            ordering,
        })
    }

    pub fn query(
        &mut self,
        table_name: &str,
        column_names: &[String],
        time_range: Option<TimeRange>,
    ) -> TsFileResult<QueryDataSet> {
        let device_ids = self.reader.get_table_devices(table_name)?;
        if device_ids.is_empty() {
            return Ok(QueryDataSet::new(Vec::new(), Vec::new()));
        }

        let result_set = self.reader.query_devices(&device_ids, column_names, time_range)?;
        let mut rows = result_set.rows().to_vec();
        match self.ordering {
            TableQueryOrdering::Device => rows.sort_by(|left, right| {
                let left_device = left.field(0).map(|field| field.string_value()).unwrap_or_default();
                let right_device =
                    right.field(0).map(|field| field.string_value()).unwrap_or_default();
                left_device
                    .cmp(&right_device)
                    .then(left.timestamp.cmp(&right.timestamp))
            }),
            TableQueryOrdering::Time => rows.sort_by_key(|row| row.timestamp),
        }

        Ok(QueryDataSet::new(result_set.columns().to_vec(), rows))
    }

    pub fn query_all_columns(
        &mut self,
        table_name: &str,
        time_range: Option<TimeRange>,
    ) -> TsFileResult<QueryDataSet> {
        let Some(schema) = self.reader.get_table_schema(table_name)? else {
            return Ok(QueryDataSet::new(Vec::new(), Vec::new()));
        };
        let columns: Vec<String> = schema
            .measurement_schemas
            .iter()
            .map(|measurement| measurement.measurement_id.clone())
            .collect();
        self.query(table_name, &columns, time_range)
    }

    pub fn query_tsblocks(
        &mut self,
        table_name: &str,
        column_names: &[String],
        time_range: Option<TimeRange>,
        block_size: usize,
    ) -> TsFileResult<VecTsBlockReader> {
        let Some(schema) = self.reader.get_table_schema(table_name)? else {
            return Ok(VecTsBlockReader::new(Vec::new()));
        };

        let mut device_ids = self.reader.get_table_devices(table_name)?;
        if self.ordering == TableQueryOrdering::Time {
            let data_set = self.query(table_name, column_names, time_range)?;
            let column_types = table_column_types(&schema, column_names)?;
            return query_data_set_to_tsblock_reader(&data_set, &column_types, block_size);
        }

        device_ids.sort();
        let task_iterator = DeviceTaskIterator::new(&device_ids, column_names);
        let mut blocks = Vec::new();
        let column_types = table_column_types(&schema, column_names)?;
        for task in task_iterator {
            let result_set = self.reader.query_devices(
                std::slice::from_ref(&task.device_id),
                &task.column_names,
                time_range,
            )?;
            let data_set = QueryDataSet::from_result_set(result_set);
            blocks.extend(query_data_set_to_tsblocks(
                &data_set,
                &column_types,
                block_size,
            )?);
        }
        Ok(VecTsBlockReader::new(blocks))
    }

    pub fn query_table_result_set(
        &mut self,
        table_name: &str,
        column_names: &[String],
        time_range: Option<TimeRange>,
        block_size: usize,
    ) -> TsFileResult<TableResultSet> {
        let reader = self.query_tsblocks(table_name, column_names, time_range, block_size)?;
        let schema = self
            .reader
            .get_table_schema(table_name)?
            .ok_or_else(|| TsFileError::MeasurementNotFound(table_name.to_string()))?;
        let column_types = table_column_types(&schema, column_names)?;
        let mut names = Vec::with_capacity(column_names.len() + 1);
        names.push("device_id".to_string());
        names.extend(column_names.iter().cloned());
        Ok(TableResultSet::new(
            Box::new(reader),
            names,
            column_types,
            table_name.to_string(),
        ))
    }

    pub fn query_all_columns_result_set(
        &mut self,
        table_name: &str,
        time_range: Option<TimeRange>,
        block_size: usize,
    ) -> TsFileResult<TableResultSet> {
        let schema = self
            .reader
            .get_table_schema(table_name)?
            .ok_or_else(|| TsFileError::MeasurementNotFound(table_name.to_string()))?;
        let columns = schema
            .measurement_schemas
            .iter()
            .map(|measurement| measurement.measurement_id.clone())
            .collect::<Vec<_>>();
        self.query_table_result_set(table_name, &columns, time_range, block_size)
    }

    pub fn close(self) -> TsFileResult<()> {
        self.reader.close()
    }
}

pub struct QueryExecutorBuilder {
    path: PathBuf,
}

impl QueryExecutorBuilder {
    pub fn new<P: AsRef<FsPath>>(path: P) -> Self {
        QueryExecutorBuilder {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn build_tree_executor(self) -> TsFileResult<TsFileExecutor> {
        TsFileExecutor::new(self.path)
    }

    pub fn build_table_executor(self) -> TsFileResult<TableQueryExecutor> {
        TableQueryExecutor::new(self.path)
    }

    pub fn build_time_generator_executor(self) -> TsFileResult<ExecutorWithTimeGenerator> {
        ExecutorWithTimeGenerator::new(self.path)
    }
}

pub fn query_data_set_to_tsblock(
    query_data_set: &QueryDataSet,
    column_types: &[TSDataType],
) -> TsFileResult<TsBlock> {
    if query_data_set.columns().len() != column_types.len() {
        return Err(TsFileError::InvalidFileFormat(format!(
            "Column type count mismatch: {} columns vs {} types",
            query_data_set.columns().len(),
            column_types.len()
        )));
    }

    let mut builder = TsBlockBuilder::new(column_types.to_vec());
    for row in query_data_set.rows() {
        builder.declare_position(row.timestamp, row_record_to_column_values(row));
    }
    Ok(builder.build())
}

pub fn query_data_set_to_tsblocks(
    query_data_set: &QueryDataSet,
    column_types: &[TSDataType],
    block_size: usize,
) -> TsFileResult<Vec<TsBlock>> {
    if block_size == 0 {
        return Err(TsFileError::InvalidFileFormat(
            "block_size must be greater than zero".to_string(),
        ));
    }

    let mut blocks = Vec::new();
    for chunk_rows in query_data_set.rows().chunks(block_size) {
        let chunk = QueryDataSet::new(query_data_set.columns().to_vec(), chunk_rows.to_vec());
        blocks.push(query_data_set_to_tsblock(&chunk, column_types)?);
    }
    Ok(blocks)
}

pub fn query_data_set_to_tsblock_reader(
    query_data_set: &QueryDataSet,
    column_types: &[TSDataType],
    block_size: usize,
) -> TsFileResult<VecTsBlockReader> {
    Ok(VecTsBlockReader::new(query_data_set_to_tsblocks(
        query_data_set,
        column_types,
        block_size,
    )?))
}

pub fn build_full_table_query_data_set(
    reader: &mut TsFileReader,
    device_ids: &[String],
) -> TsFileResult<QueryDataSet> {
    let mut measurements = BTreeSet::new();
    for device_id in device_ids {
        for measurement in reader.get_measurements_for_device(device_id)? {
            measurements.insert(measurement);
        }
    }
    let measurement_names: Vec<String> = measurements.into_iter().collect();
    let result_set = reader.query_devices(device_ids, &measurement_names, None)?;
    Ok(QueryDataSet::from_result_set(result_set))
}

pub fn tree_result_set_from_query_data_set(
    query_data_set: QueryDataSet,
    measurement_names: Vec<String>,
) -> TreeResultSet {
    TreeResultSet::new(query_data_set, measurement_names)
}

pub fn prepend_device_column(
    columns: &[String],
    rows: &[RowRecord],
    device_id: &str,
) -> QueryDataSet {
    let mut result_columns = Vec::with_capacity(columns.len() + 1);
    result_columns.push("device_id".to_string());
    result_columns.extend(columns.iter().cloned());

    let mut result_rows = Vec::with_capacity(rows.len());
    for row in rows {
        let mut next_row = row.clone();
        next_row.fields.insert(0, device_id_field(device_id));
        result_rows.push(next_row);
    }

    QueryDataSet::new(result_columns, result_rows)
}

fn generate_timestamps_from_expression(
    expression: &Expression,
    paths: &[SeriesPath],
    series_data: &[Vec<TimeValuePair>],
) -> Vec<i64> {
    let series_value_map = paths
        .iter()
        .zip(series_data.iter())
        .map(|(path, points)| {
            (
                path.full_path(),
                points
                    .iter()
                    .map(|point| (point.timestamp, point.value.clone()))
                    .collect::<HashMap<_, _>>(),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut timestamps = BTreeSet::new();
    for points in series_data {
        timestamps.extend(points.iter().map(|point| point.timestamp));
    }
    timestamps
        .into_iter()
        .filter(|timestamp| evaluate_expression(expression, *timestamp, &series_value_map))
        .collect()
}

fn evaluate_expression(
    expression: &Expression,
    timestamp: i64,
    series_value_map: &HashMap<String, HashMap<i64, TimeValue>>,
) -> bool {
    match expression {
        Expression::GlobalTime(filter) => filter.satisfy(timestamp, &TimeValue::Null),
        Expression::SingleSeries { path, filter } => resolve_series_value(path, timestamp, series_value_map)
            .is_some_and(|value| filter.satisfy(timestamp, value)),
        Expression::And(left, right) => {
            evaluate_expression(left, timestamp, series_value_map)
                && evaluate_expression(right, timestamp, series_value_map)
        }
        Expression::Or(left, right) => {
            evaluate_expression(left, timestamp, series_value_map)
                || evaluate_expression(right, timestamp, series_value_map)
        }
    }
}

fn resolve_series_value<'a>(
    path: &str,
    timestamp: i64,
    series_value_map: &'a HashMap<String, HashMap<i64, TimeValue>>,
) -> Option<&'a TimeValue> {
    if let Some(values) = series_value_map.get(path) {
        return values.get(&timestamp);
    }
    series_value_map
        .iter()
        .find(|(candidate, _)| candidate.ends_with(path))
        .and_then(|(_, values)| values.get(&timestamp))
}

fn table_column_types(
    schema: &crate::file::metadata::TableSchema,
    column_names: &[String],
) -> TsFileResult<Vec<TSDataType>> {
    let mut column_types = Vec::with_capacity(column_names.len() + 1);
    column_types.push(TSDataType::Text);
    for column_name in column_names {
        let data_type = schema
            .measurement_schemas
            .iter()
            .find(|measurement| measurement.measurement_id == *column_name)
            .map(|measurement| measurement.data_type)
            .ok_or_else(|| TsFileError::MeasurementNotFound(column_name.clone()))?;
        column_types.push(data_type);
    }
    Ok(column_types)
}
