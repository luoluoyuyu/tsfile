// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Java-style named statistics wrappers around Rust's enum-backed `Statistics`.

pub use crate::file::metadata::statistics::{
    BinaryStats as BinaryStatistics, BinaryStats as BlobStatistics,
    BinaryStats as ObjectStatistics, BinaryStats as StringStatistics,
    BooleanStats as BooleanStatistics, DoubleStats as DoubleStatistics,
    FloatStats as FloatStatistics, IntegerStats as DateStatistics,
    IntegerStats as IntegerStatistics, LongStats as LongStatistics,
    LongStats as TimestampStatistics, Statistics, TimeStats as TimeStatistics,
};

use crate::common::enums::TSDataType;
use crate::file::metadata::statistics::{Statistics as GenericStatistics, TypedStats};

pub fn statistics_for_type(data_type: TSDataType) -> GenericStatistics {
    GenericStatistics::new(data_type)
}

pub fn typed_statistics_name(statistics: &GenericStatistics) -> &'static str {
    match &statistics.typed {
        TypedStats::Boolean(_) => "BooleanStatistics",
        TypedStats::Integer(_) if statistics.data_type() == TSDataType::Date => "DateStatistics",
        TypedStats::Integer(_) => "IntegerStatistics",
        TypedStats::Long(_) if statistics.data_type() == TSDataType::Timestamp => "TimestampStatistics",
        TypedStats::Long(_) => "LongStatistics",
        TypedStats::Float(_) => "FloatStatistics",
        TypedStats::Double(_) => "DoubleStatistics",
        TypedStats::Binary(_) => "BinaryStatistics",
        TypedStats::Time(_) => "TimeStatistics",
    }
}
