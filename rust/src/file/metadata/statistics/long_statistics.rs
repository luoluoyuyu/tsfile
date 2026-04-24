// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use crate::common::enums::TSDataType;
use crate::file::metadata::statistics::{LongStats, Statistics, TypedStats};

#[derive(Debug, Clone, Default)]
pub struct LongStatistics(pub LongStats);

impl LongStatistics {
    pub fn new() -> Self { Self(LongStats::default()) }
    pub fn update(&mut self, value: i64) { self.0.update(value); }
    pub fn into_statistics(self, count: u32, start_time: i64, end_time: i64) -> Statistics {
        Statistics { is_empty: count == 0, count, start_time, end_time, typed: TypedStats::Long(self.0) }
    }
    pub fn data_type(&self) -> TSDataType { TSDataType::Int64 }
}
