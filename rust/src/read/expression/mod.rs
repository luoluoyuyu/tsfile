// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Query expressions and executor.

use crate::error::TsFileResult;
use crate::read::filter::Filter;
use crate::read::result_set::ResultSet;
use crate::read::tsfile_reader::TsFileReader;

#[derive(Debug, Clone)]
pub enum Expression {
    GlobalTime(Filter),
    SingleSeries { path: String, filter: Filter },
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
}

#[derive(Debug, Clone)]
pub struct QueryExecutor {
    pub device_id: String,
    pub measurements: Vec<String>,
    pub filter: Option<Filter>,
}

impl QueryExecutor {
    pub fn new(device_id: String, measurements: Vec<String>) -> Self {
        QueryExecutor { device_id, measurements, filter: None }
    }

    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn execute(&self, reader: &mut TsFileReader) -> TsFileResult<ResultSet> {
        let mut rows = reader.read_rows(&self.device_id, &self.measurements)?;
        if let Some(filter) = &self.filter {
            rows.retain(|row| row.fields.iter().any(|field| filter.satisfy(row.timestamp, &field.value)));
        }
        Ok(ResultSet::new(self.measurements.clone(), rows))
    }
}
