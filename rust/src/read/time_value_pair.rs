// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TimeValuePair - a single timestamp+value pair from reading.

use crate::utils::read_write_io_utils::Binary;

/// A time-value pair read from a timeseries.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeValuePair {
    /// Timestamp.
    pub timestamp: i64,
    /// Value.
    pub value: TimeValue,
}

/// Value in a TimeValuePair.
#[derive(Debug, Clone, PartialEq)]
pub enum TimeValue {
    Boolean(bool),
    Int32(i32),
    Int64(i64),
    Float(f32),
    Double(f64),
    Text(Binary),
    Null,
}

impl TimeValuePair {
    pub fn new(timestamp: i64, value: TimeValue) -> Self {
        TimeValuePair { timestamp, value }
    }
}
