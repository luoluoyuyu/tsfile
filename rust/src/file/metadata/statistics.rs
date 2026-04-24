// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Statistics for various data types, mirroring Java's Statistics hierarchy.
//! Uses an enum-based approach for dyn-compatibility in Rust.

#[path = "statistics/binary_statistics.rs"]
pub mod binary_statistics;
#[path = "statistics/blob_statistics.rs"]
pub mod blob_statistics;
#[path = "statistics/boolean_statistics.rs"]
pub mod boolean_statistics;
#[path = "statistics/date_statistics.rs"]
pub mod date_statistics;
#[path = "statistics/double_statistics.rs"]
pub mod double_statistics;
#[path = "statistics/float_statistics.rs"]
pub mod float_statistics;
#[path = "statistics/integer_statistics.rs"]
pub mod integer_statistics;
#[path = "statistics/long_statistics.rs"]
pub mod long_statistics;
#[path = "statistics/object_statistics.rs"]
pub mod object_statistics;
#[path = "statistics/string_statistics.rs"]
pub mod string_statistics;
#[path = "statistics/time_statistics.rs"]
pub mod time_statistics;
#[path = "statistics/timestamp_statistics.rs"]
pub mod timestamp_statistics;

use std::io::{Read, Write};

use crate::common::enums::TSDataType;
use crate::error::TsFileResult;
use crate::utils::read_write_io_utils::Binary;
use crate::utils::{ReadWriteForEncodingUtils, ReadWriteIOUtils};

// =========================================================================
// TypedStats enum - replaces dyn StatisticsValue for Rust compatibility
// =========================================================================

/// Type-specific statistics, stored as an enum for Rust dyn-safety.
#[derive(Debug, Clone)]
pub enum TypedStats {
    Boolean(BooleanStats),
    Integer(IntegerStats),
    Long(LongStats),
    Float(FloatStats),
    Double(DoubleStats),
    Binary(BinaryStats),
    Time(TimeStats),
}

impl TypedStats {
    pub fn data_type(&self) -> TSDataType {
        match self {
            TypedStats::Boolean(_) => TSDataType::Boolean,
            TypedStats::Integer(_) => TSDataType::Int32,
            TypedStats::Long(_) => TSDataType::Int64,
            TypedStats::Float(_) => TSDataType::Float,
            TypedStats::Double(_) => TSDataType::Double,
            TypedStats::Binary(_) => TSDataType::Text,
            TypedStats::Time(_) => TSDataType::Vector,
        }
    }

    pub fn stats_size(&self) -> usize {
        match self {
            TypedStats::Boolean(_) => 2,
            TypedStats::Integer(_) => 4 + 4 + 4 + 4 + 8,
            TypedStats::Long(_) => 8 + 8 + 8 + 8 + 8,
            TypedStats::Float(_) => 4 + 4 + 4 + 4 + 8,
            TypedStats::Double(_) => 8 + 8 + 8 + 8 + 8,
            TypedStats::Binary(s) => 4 + s.first_value.len() + 4 + s.last_value.len(),
            TypedStats::Time(_) => 0,
        }
    }

    pub fn serialize<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        match self {
            TypedStats::Boolean(s) => {
                let mut n = 0;
                n += ReadWriteIOUtils::write_bool(s.first_value, writer)?;
                n += ReadWriteIOUtils::write_bool(s.last_value, writer)?;
                Ok(n)
            }
            TypedStats::Integer(s) => {
                let mut n = 0;
                n += ReadWriteIOUtils::write_i32(s.min_value, writer)?;
                n += ReadWriteIOUtils::write_i32(s.max_value, writer)?;
                n += ReadWriteIOUtils::write_i32(s.first_value, writer)?;
                n += ReadWriteIOUtils::write_i32(s.last_value, writer)?;
                n += ReadWriteIOUtils::write_i64(s.sum_value, writer)?;
                Ok(n)
            }
            TypedStats::Long(s) => {
                let mut n = 0;
                n += ReadWriteIOUtils::write_i64(s.min_value, writer)?;
                n += ReadWriteIOUtils::write_i64(s.max_value, writer)?;
                n += ReadWriteIOUtils::write_i64(s.first_value, writer)?;
                n += ReadWriteIOUtils::write_i64(s.last_value, writer)?;
                n += ReadWriteIOUtils::write_i64(s.sum_value, writer)?;
                Ok(n)
            }
            TypedStats::Float(s) => {
                let mut n = 0;
                n += ReadWriteIOUtils::write_f32(s.min_value, writer)?;
                n += ReadWriteIOUtils::write_f32(s.max_value, writer)?;
                n += ReadWriteIOUtils::write_f32(s.first_value, writer)?;
                n += ReadWriteIOUtils::write_f32(s.last_value, writer)?;
                n += ReadWriteIOUtils::write_f64(s.sum_value, writer)?;
                Ok(n)
            }
            TypedStats::Double(s) => {
                let mut n = 0;
                n += ReadWriteIOUtils::write_f64(s.min_value, writer)?;
                n += ReadWriteIOUtils::write_f64(s.max_value, writer)?;
                n += ReadWriteIOUtils::write_f64(s.first_value, writer)?;
                n += ReadWriteIOUtils::write_f64(s.last_value, writer)?;
                n += ReadWriteIOUtils::write_f64(s.sum_value, writer)?;
                Ok(n)
            }
            TypedStats::Binary(s) => {
                let mut n = 0;
                n += ReadWriteIOUtils::write_binary(&s.first_value, writer)?;
                n += ReadWriteIOUtils::write_binary(&s.last_value, writer)?;
                Ok(n)
            }
            TypedStats::Time(_) => Ok(0),
        }
    }

    pub fn deserialize<R: Read>(&mut self, reader: &mut R) -> TsFileResult<()> {
        match self {
            TypedStats::Boolean(s) => {
                s.first_value = ReadWriteIOUtils::read_bool(reader)?;
                s.last_value = ReadWriteIOUtils::read_bool(reader)?;
            }
            TypedStats::Integer(s) => {
                s.min_value = ReadWriteIOUtils::read_i32(reader)?;
                s.max_value = ReadWriteIOUtils::read_i32(reader)?;
                s.first_value = ReadWriteIOUtils::read_i32(reader)?;
                s.last_value = ReadWriteIOUtils::read_i32(reader)?;
                s.sum_value = ReadWriteIOUtils::read_i64(reader)?;
            }
            TypedStats::Long(s) => {
                s.min_value = ReadWriteIOUtils::read_i64(reader)?;
                s.max_value = ReadWriteIOUtils::read_i64(reader)?;
                s.first_value = ReadWriteIOUtils::read_i64(reader)?;
                s.last_value = ReadWriteIOUtils::read_i64(reader)?;
                s.sum_value = ReadWriteIOUtils::read_i64(reader)?;
            }
            TypedStats::Float(s) => {
                s.min_value = ReadWriteIOUtils::read_f32(reader)?;
                s.max_value = ReadWriteIOUtils::read_f32(reader)?;
                s.first_value = ReadWriteIOUtils::read_f32(reader)?;
                s.last_value = ReadWriteIOUtils::read_f32(reader)?;
                s.sum_value = ReadWriteIOUtils::read_f64(reader)?;
            }
            TypedStats::Double(s) => {
                s.min_value = ReadWriteIOUtils::read_f64(reader)?;
                s.max_value = ReadWriteIOUtils::read_f64(reader)?;
                s.first_value = ReadWriteIOUtils::read_f64(reader)?;
                s.last_value = ReadWriteIOUtils::read_f64(reader)?;
                s.sum_value = ReadWriteIOUtils::read_f64(reader)?;
            }
            TypedStats::Binary(s) => {
                s.first_value = ReadWriteIOUtils::read_binary(reader)?;
                s.last_value = ReadWriteIOUtils::read_binary(reader)?;
            }
            TypedStats::Time(_) => {}
        }
        Ok(())
    }
}

/// Statistics metadata for a chunk or page.
///
/// Mirrors Java's Statistics<T>.
#[derive(Debug, Clone)]
pub struct Statistics {
    /// Is the statistics empty (no data added yet).
    pub is_empty: bool,
    /// Number of data points.
    pub count: u32,
    /// Start timestamp (inclusive).
    pub start_time: i64,
    /// End timestamp (inclusive).
    pub end_time: i64,
    /// Type-specific stats (min, max, first, last, sum, etc.).
    pub typed: TypedStats,
}

impl Statistics {
    /// Create a new empty Statistics for the given data type.
    pub fn new(data_type: TSDataType) -> Self {
        Statistics {
            is_empty: true,
            count: 0,
            start_time: i64::MAX,
            end_time: i64::MIN,
            typed: Self::create_typed(data_type),
        }
    }

    fn create_typed(data_type: TSDataType) -> TypedStats {
        match data_type {
            TSDataType::Boolean => TypedStats::Boolean(BooleanStats::default()),
            TSDataType::Int32 | TSDataType::Date => TypedStats::Integer(IntegerStats::default()),
            TSDataType::Int64 | TSDataType::Timestamp => TypedStats::Long(LongStats::default()),
            TSDataType::Float => TypedStats::Float(FloatStats::default()),
            TSDataType::Double => TypedStats::Double(DoubleStats::default()),
            TSDataType::Text | TSDataType::Blob | TSDataType::String => {
                TypedStats::Binary(BinaryStats::default())
            }
            TSDataType::Vector => TypedStats::Time(TimeStats),
            _ => TypedStats::Binary(BinaryStats::default()),
        }
    }

    /// Update with a boolean value.
    pub fn update_bool(&mut self, timestamp: i64, value: bool) {
        self.update_time(timestamp);
        if let TypedStats::Boolean(s) = &mut self.typed {
            s.update(value);
        }
    }

    /// Update with an i32 value.
    pub fn update_i32(&mut self, timestamp: i64, value: i32) {
        self.update_time(timestamp);
        if let TypedStats::Integer(s) = &mut self.typed {
            s.update(value);
        }
    }

    /// Update with an i64 value.
    pub fn update_i64(&mut self, timestamp: i64, value: i64) {
        self.update_time(timestamp);
        if let TypedStats::Long(s) = &mut self.typed {
            s.update(value);
        }
    }

    /// Update with a f32 value.
    pub fn update_f32(&mut self, timestamp: i64, value: f32) {
        self.update_time(timestamp);
        if let TypedStats::Float(s) = &mut self.typed {
            s.update(value);
        }
    }

    /// Update with a f64 value.
    pub fn update_f64(&mut self, timestamp: i64, value: f64) {
        self.update_time(timestamp);
        if let TypedStats::Double(s) = &mut self.typed {
            s.update(value);
        }
    }

    /// Update with a binary value.
    pub fn update_binary(&mut self, timestamp: i64, value: Binary) {
        self.update_time(timestamp);
        if let TypedStats::Binary(s) = &mut self.typed {
            s.update(value);
        }
    }

    fn update_time(&mut self, timestamp: i64) {
        if timestamp < self.start_time {
            self.start_time = timestamp;
        }
        if timestamp > self.end_time {
            self.end_time = timestamp;
        }
        self.count += 1;
        self.is_empty = false;
    }

    /// Get serialized size.
    pub fn serialized_size(&self) -> usize {
        ReadWriteForEncodingUtils::u_var_int_size(self.count)
            + 16 // start_time + end_time
            + self.typed.stats_size()
    }

    /// Serialize statistics.
    pub fn serialize<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        let mut written = 0;
        written += ReadWriteForEncodingUtils::write_unsigned_var_int(self.count, writer)?;
        written += ReadWriteIOUtils::write_i64(self.start_time, writer)?;
        written += ReadWriteIOUtils::write_i64(self.end_time, writer)?;
        written += self.typed.serialize(writer)?;
        Ok(written)
    }

    /// Deserialize statistics.
    pub fn deserialize<R: Read>(reader: &mut R, data_type: TSDataType) -> TsFileResult<Self> {
        let count = ReadWriteForEncodingUtils::read_unsigned_var_int(reader)?;
        let start_time = ReadWriteIOUtils::read_i64(reader)?;
        let end_time = ReadWriteIOUtils::read_i64(reader)?;
        let mut typed = Self::create_typed(data_type);
        typed.deserialize(reader)?;
        Ok(Statistics {
            is_empty: false,
            count,
            start_time,
            end_time,
            typed,
        })
    }

    /// Merge another statistics into this one.
    pub fn merge(&mut self, other: &Statistics) {
        if !other.is_empty {
            if other.start_time < self.start_time {
                self.start_time = other.start_time;
            }
            if other.end_time > self.end_time {
                self.end_time = other.end_time;
            }
            self.count += other.count;
            self.is_empty = false;
        }
    }

    /// Get the data type of the typed statistics.
    pub fn data_type(&self) -> TSDataType {
        self.typed.data_type()
    }
}

// =========================================================================
// Boolean Statistics
// =========================================================================

#[derive(Debug, Clone, Default)]
pub struct BooleanStats {
    pub first_value: bool,
    pub last_value: bool,
    pub is_first: bool,
}

impl BooleanStats {
    pub fn update(&mut self, value: bool) {
        if self.is_first {
            self.first_value = value;
            self.is_first = false;
        }
        self.last_value = value;
    }
}

// =========================================================================
// Integer Statistics
// =========================================================================

#[derive(Debug, Clone)]
pub struct IntegerStats {
    pub min_value: i32,
    pub max_value: i32,
    pub first_value: i32,
    pub last_value: i32,
    pub sum_value: i64,
}

impl Default for IntegerStats {
    fn default() -> Self {
        IntegerStats {
            min_value: i32::MAX,
            max_value: i32::MIN,
            first_value: 0,
            last_value: 0,
            sum_value: 0,
        }
    }
}

impl IntegerStats {
    pub fn update(&mut self, value: i32) {
        if value < self.min_value {
            self.min_value = value;
        }
        if value > self.max_value {
            self.max_value = value;
        }
        self.last_value = value;
        self.sum_value += value as i64;
    }
}

// =========================================================================
// Long Statistics
// =========================================================================

#[derive(Debug, Clone)]
pub struct LongStats {
    pub min_value: i64,
    pub max_value: i64,
    pub first_value: i64,
    pub last_value: i64,
    pub sum_value: i64,
}

impl Default for LongStats {
    fn default() -> Self {
        LongStats {
            min_value: i64::MAX,
            max_value: i64::MIN,
            first_value: 0,
            last_value: 0,
            sum_value: 0,
        }
    }
}

impl LongStats {
    pub fn update(&mut self, value: i64) {
        if value < self.min_value {
            self.min_value = value;
        }
        if value > self.max_value {
            self.max_value = value;
        }
        self.last_value = value;
        self.sum_value = self.sum_value.wrapping_add(value);
    }
}

// =========================================================================
// Float Statistics
// =========================================================================

#[derive(Debug, Clone)]
pub struct FloatStats {
    pub min_value: f32,
    pub max_value: f32,
    pub first_value: f32,
    pub last_value: f32,
    pub sum_value: f64,
}

impl Default for FloatStats {
    fn default() -> Self {
        FloatStats {
            min_value: f32::MAX,
            max_value: f32::MIN,
            first_value: 0.0,
            last_value: 0.0,
            sum_value: 0.0,
        }
    }
}

impl FloatStats {
    pub fn update(&mut self, value: f32) {
        if value < self.min_value {
            self.min_value = value;
        }
        if value > self.max_value {
            self.max_value = value;
        }
        self.last_value = value;
        self.sum_value += value as f64;
    }
}

// =========================================================================
// Double Statistics
// =========================================================================

#[derive(Debug, Clone)]
pub struct DoubleStats {
    pub min_value: f64,
    pub max_value: f64,
    pub first_value: f64,
    pub last_value: f64,
    pub sum_value: f64,
}

impl Default for DoubleStats {
    fn default() -> Self {
        DoubleStats {
            min_value: f64::MAX,
            max_value: f64::MIN,
            first_value: 0.0,
            last_value: 0.0,
            sum_value: 0.0,
        }
    }
}

impl DoubleStats {
    pub fn update(&mut self, value: f64) {
        if value < self.min_value {
            self.min_value = value;
        }
        if value > self.max_value {
            self.max_value = value;
        }
        self.last_value = value;
        self.sum_value += value;
    }
}

// =========================================================================
// Binary Statistics (TEXT, BLOB, STRING)
// =========================================================================

#[derive(Debug, Clone, Default)]
pub struct BinaryStats {
    pub first_value: Binary,
    pub last_value: Binary,
}

impl BinaryStats {
    pub fn update(&mut self, value: Binary) {
        if self.first_value.is_empty() {
            self.first_value = value.clone();
        }
        self.last_value = value;
    }
}

// =========================================================================
// Time Statistics (aligned time column VECTOR)
// =========================================================================

#[derive(Debug, Clone, Default)]
pub struct TimeStats;

pub use binary_statistics::BinaryStatistics;
pub use blob_statistics::BlobStatistics;
pub use boolean_statistics::BooleanStatistics;
pub use date_statistics::DateStatistics;
pub use double_statistics::DoubleStatistics;
pub use float_statistics::FloatStatistics;
pub use integer_statistics::IntegerStatistics;
pub use long_statistics::LongStatistics;
pub use object_statistics::ObjectStatistics;
pub use string_statistics::StringStatistics;
pub use time_statistics::TimeStatistics;
pub use timestamp_statistics::TimestampStatistics;
