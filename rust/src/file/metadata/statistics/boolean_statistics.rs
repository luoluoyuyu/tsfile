// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use crate::common::enums::TSDataType;
use crate::file::metadata::statistics::{BooleanStats, Statistics, TypedStats};

#[derive(Debug, Clone, Default)]
pub struct BooleanStatistics(pub BooleanStats);

impl BooleanStatistics {
    pub fn new() -> Self { Self(BooleanStats::default()) }
    pub fn update(&mut self, value: bool) { self.0.update(value); }
    pub fn into_statistics(self, count: u32, start_time: i64, end_time: i64) -> Statistics {
        Statistics { is_empty: count == 0, count, start_time, end_time, typed: TypedStats::Boolean(self.0) }
    }
    pub fn data_type(&self) -> TSDataType { TSDataType::Boolean }
}
