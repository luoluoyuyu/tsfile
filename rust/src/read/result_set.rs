// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Result set and query expression support for high-level reads.

use crate::read::common::{RowRecord, TimeRange};

/// Query description for selecting measurements from one device.
#[derive(Debug, Clone)]
pub struct QueryExpression {
    pub device_id: String,
    pub measurements: Vec<String>,
    pub time_range: Option<TimeRange>,
}

impl QueryExpression {
    pub fn new(device_id: String, measurements: Vec<String>) -> Self {
        QueryExpression {
            device_id,
            measurements,
            time_range: None,
        }
    }

    pub fn with_time_range(mut self, time_range: TimeRange) -> Self {
        self.time_range = Some(time_range);
        self
    }
}

/// Row-oriented result set matching Java's ResultSet-style usage.
#[derive(Debug, Clone)]
pub struct ResultSet {
    columns: Vec<String>,
    rows: Vec<RowRecord>,
    cursor: usize,
}

impl ResultSet {
    pub fn new(columns: Vec<String>, rows: Vec<RowRecord>) -> Self {
        ResultSet {
            columns,
            rows,
            cursor: 0,
        }
    }

    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    pub fn has_next(&self) -> bool {
        self.cursor < self.rows.len()
    }

    pub fn next(&mut self) -> Option<&RowRecord> {
        if !self.has_next() {
            return None;
        }
        let index = self.cursor;
        self.cursor += 1;
        self.rows.get(index)
    }

    pub fn reset(&mut self) {
        self.cursor = 0;
    }

    pub fn rows(&self) -> &[RowRecord] {
        &self.rows
    }
}
