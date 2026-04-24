// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use crate::common::enums::TSDataType;
use crate::file::metadata::statistics::{Statistics, TimeStats, TypedStats};

#[derive(Debug, Clone, Default)]
pub struct TimeStatistics(pub TimeStats);

impl TimeStatistics {
    pub fn new() -> Self { Self(TimeStats) }
    pub fn into_statistics(self, count: u32, start_time: i64, end_time: i64) -> Statistics {
        Statistics { is_empty: count == 0, count, start_time, end_time, typed: TypedStats::Time(self.0) }
    }
    pub fn data_type(&self) -> TSDataType { TSDataType::Vector }
}
