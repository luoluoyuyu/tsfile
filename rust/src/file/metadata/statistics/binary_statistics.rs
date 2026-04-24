// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use crate::common::enums::TSDataType;
use crate::file::metadata::statistics::{BinaryStats, Statistics, TypedStats};
use crate::utils::read_write_io_utils::Binary;

#[derive(Debug, Clone, Default)]
pub struct BinaryStatistics(pub BinaryStats);

impl BinaryStatistics {
    pub fn new() -> Self { Self(BinaryStats::default()) }
    pub fn update(&mut self, value: Binary) { self.0.update(value); }
    pub fn into_statistics(self, count: u32, start_time: i64, end_time: i64) -> Statistics {
        Statistics { is_empty: count == 0, count, start_time, end_time, typed: TypedStats::Binary(self.0) }
    }
    pub fn data_type(&self) -> TSDataType { TSDataType::Text }
}
