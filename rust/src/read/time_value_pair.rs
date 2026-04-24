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

impl TimeValue {
    pub fn is_null(&self) -> bool {
        matches!(self, TimeValue::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            TimeValue::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            TimeValue::Int32(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            TimeValue::Int64(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match self {
            TimeValue::Float(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            TimeValue::Double(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_binary(&self) -> Option<&Binary> {
        match self {
            TimeValue::Text(value) => Some(value),
            _ => None,
        }
    }
}
