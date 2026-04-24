// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Filter operators for time/value predicates.

use crate::file::metadata::statistics::{Statistics, TypedStats};
use crate::read::time_value_pair::TimeValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorType {
    Eq,
    NotEq,
    Gt,
    GtEq,
    Lt,
    LtEq,
}

#[derive(Debug, Clone)]
pub enum Filter {
    True,
    False,
    Time { op: OperatorType, value: i64 },
    TimeBetween { min: i64, max: i64 },
    Value { op: OperatorType, value: TimeValue },
    ValueIsNull,
    ValueIsNotNull,
    And(Box<Filter>, Box<Filter>),
    Or(Box<Filter>, Box<Filter>),
    Not(Box<Filter>),
}

impl Filter {
    pub fn satisfy(&self, timestamp: i64, value: &TimeValue) -> bool {
        match self {
            Filter::True => true,
            Filter::False => false,
            Filter::Time { op, value } => compare_i64(timestamp, *op, *value),
            Filter::TimeBetween { min, max } => timestamp >= *min && timestamp <= *max,
            Filter::Value { op, value: rhs } => compare_value(value, *op, rhs),
            Filter::ValueIsNull => value.is_null(),
            Filter::ValueIsNotNull => !value.is_null(),
            Filter::And(left, right) => left.satisfy(timestamp, value) && right.satisfy(timestamp, value),
            Filter::Or(left, right) => left.satisfy(timestamp, value) || right.satisfy(timestamp, value),
            Filter::Not(filter) => !filter.satisfy(timestamp, value),
        }
    }

    pub fn and(self, other: Filter) -> Filter {
        Filter::And(Box::new(self), Box::new(other))
    }

    pub fn or(self, other: Filter) -> Filter {
        Filter::Or(Box::new(self), Box::new(other))
    }

    pub fn negate(self) -> Filter {
        Filter::Not(Box::new(self))
    }

    /// Return true when statistics prove that at least one value may satisfy this filter.
    /// This is intentionally conservative: unknown or unsupported value ranges return true.
    pub fn satisfy_statistics(&self, statistics: &Statistics) -> bool {
        if statistics.is_empty || statistics.count == 0 {
            return false;
        }
        match self {
            Filter::True => true,
            Filter::False => false,
            Filter::Time { op, value } => compare_range_i64(statistics.start_time, statistics.end_time, *op, *value),
            Filter::TimeBetween { min, max } => statistics.end_time >= *min && statistics.start_time <= *max,
            Filter::Value { op, value } => value_may_satisfy(statistics, *op, value),
            Filter::ValueIsNull => true,
            Filter::ValueIsNotNull => statistics.count > 0,
            Filter::And(left, right) => left.satisfy_statistics(statistics) && right.satisfy_statistics(statistics),
            Filter::Or(left, right) => left.satisfy_statistics(statistics) || right.satisfy_statistics(statistics),
            Filter::Not(_) => true,
        }
    }
}

pub struct TimeFilterApi;

impl TimeFilterApi {
    pub fn eq(value: i64) -> Filter { Filter::Time { op: OperatorType::Eq, value } }
    pub fn not_eq(value: i64) -> Filter { Filter::Time { op: OperatorType::NotEq, value } }
    pub fn gt(value: i64) -> Filter { Filter::Time { op: OperatorType::Gt, value } }
    pub fn gt_eq(value: i64) -> Filter { Filter::Time { op: OperatorType::GtEq, value } }
    pub fn lt(value: i64) -> Filter { Filter::Time { op: OperatorType::Lt, value } }
    pub fn lt_eq(value: i64) -> Filter { Filter::Time { op: OperatorType::LtEq, value } }
    pub fn between(min: i64, max: i64) -> Filter { Filter::TimeBetween { min, max } }
}

pub struct ValueFilterApi;

impl ValueFilterApi {
    pub fn eq(value: TimeValue) -> Filter { Filter::Value { op: OperatorType::Eq, value } }
    pub fn not_eq(value: TimeValue) -> Filter { Filter::Value { op: OperatorType::NotEq, value } }
    pub fn gt(value: TimeValue) -> Filter { Filter::Value { op: OperatorType::Gt, value } }
    pub fn gt_eq(value: TimeValue) -> Filter { Filter::Value { op: OperatorType::GtEq, value } }
    pub fn lt(value: TimeValue) -> Filter { Filter::Value { op: OperatorType::Lt, value } }
    pub fn lt_eq(value: TimeValue) -> Filter { Filter::Value { op: OperatorType::LtEq, value } }
    pub fn is_null() -> Filter { Filter::ValueIsNull }
    pub fn is_not_null() -> Filter { Filter::ValueIsNotNull }
}

fn compare_i64(left: i64, op: OperatorType, right: i64) -> bool {
    match op {
        OperatorType::Eq => left == right,
        OperatorType::NotEq => left != right,
        OperatorType::Gt => left > right,
        OperatorType::GtEq => left >= right,
        OperatorType::Lt => left < right,
        OperatorType::LtEq => left <= right,
    }
}

fn compare_value(left: &TimeValue, op: OperatorType, right: &TimeValue) -> bool {
    match (left, right) {
        (TimeValue::Boolean(left), TimeValue::Boolean(right)) => compare_bool(*left, op, *right),
        (TimeValue::Int32(left), TimeValue::Int32(right)) => compare_i64(*left as i64, op, *right as i64),
        (TimeValue::Int64(left), TimeValue::Int64(right)) => compare_i64(*left, op, *right),
        (TimeValue::Float(left), TimeValue::Float(right)) => compare_f64(*left as f64, op, *right as f64),
        (TimeValue::Double(left), TimeValue::Double(right)) => compare_f64(*left, op, *right),
        (TimeValue::Text(left), TimeValue::Text(right)) => compare_str(&left.to_string(), op, &right.to_string()),
        (TimeValue::Null, TimeValue::Null) => matches!(op, OperatorType::Eq | OperatorType::GtEq | OperatorType::LtEq),
        _ => matches!(op, OperatorType::NotEq),
    }
}

fn compare_bool(left: bool, op: OperatorType, right: bool) -> bool {
    compare_i64(left as i64, op, right as i64)
}

fn compare_f64(left: f64, op: OperatorType, right: f64) -> bool {
    match op {
        OperatorType::Eq => left == right,
        OperatorType::NotEq => left != right,
        OperatorType::Gt => left > right,
        OperatorType::GtEq => left >= right,
        OperatorType::Lt => left < right,
        OperatorType::LtEq => left <= right,
    }
}

fn compare_str(left: &str, op: OperatorType, right: &str) -> bool {
    match op {
        OperatorType::Eq => left == right,
        OperatorType::NotEq => left != right,
        OperatorType::Gt => left > right,
        OperatorType::GtEq => left >= right,
        OperatorType::Lt => left < right,
        OperatorType::LtEq => left <= right,
    }
}


fn compare_range_i64(min_value: i64, max_value: i64, op: OperatorType, rhs: i64) -> bool {
    match op {
        OperatorType::Eq => rhs >= min_value && rhs <= max_value,
        OperatorType::NotEq => min_value != max_value || min_value != rhs,
        OperatorType::Gt => max_value > rhs,
        OperatorType::GtEq => max_value >= rhs,
        OperatorType::Lt => min_value < rhs,
        OperatorType::LtEq => min_value <= rhs,
    }
}

fn compare_range_f64(min_value: f64, max_value: f64, op: OperatorType, rhs: f64) -> bool {
    match op {
        OperatorType::Eq => rhs >= min_value && rhs <= max_value,
        OperatorType::NotEq => min_value != max_value || min_value != rhs,
        OperatorType::Gt => max_value > rhs,
        OperatorType::GtEq => max_value >= rhs,
        OperatorType::Lt => min_value < rhs,
        OperatorType::LtEq => min_value <= rhs,
    }
}

fn value_may_satisfy(statistics: &Statistics, op: OperatorType, rhs: &TimeValue) -> bool {
    match (&statistics.typed, rhs) {
        (TypedStats::Boolean(_), TimeValue::Boolean(_)) => match op {
            OperatorType::Eq | OperatorType::GtEq | OperatorType::LtEq => true,
            OperatorType::NotEq | OperatorType::Gt | OperatorType::Lt => true,
        },
        (TypedStats::Integer(stats), TimeValue::Int32(value)) => {
            compare_range_i64(stats.min_value as i64, stats.max_value as i64, op, *value as i64)
        }
        (TypedStats::Long(stats), TimeValue::Int64(value)) => {
            compare_range_i64(stats.min_value, stats.max_value, op, *value)
        }
        (TypedStats::Float(stats), TimeValue::Float(value)) => {
            compare_range_f64(stats.min_value as f64, stats.max_value as f64, op, *value as f64)
        }
        (TypedStats::Double(stats), TimeValue::Double(value)) => {
            compare_range_f64(stats.min_value, stats.max_value, op, *value)
        }
        (TypedStats::Binary(_), TimeValue::Text(_)) => true,
        (_, TimeValue::Null) => true,
        _ => true,
    }
}
