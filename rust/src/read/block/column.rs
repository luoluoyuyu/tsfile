// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use crate::common::enums::TSDataType;
use crate::error::{TsFileError, TsFileResult};
use crate::read::time_value_pair::TimeValue;
use crate::utils::read_write_io_utils::Binary;
use crate::utils::ReadWriteIOUtils;

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnValue {
    Boolean(bool),
    Int32(i32),
    Int64(i64),
    Float(f32),
    Double(f64),
    Binary(Binary),
    Null,
}

impl From<TimeValue> for ColumnValue {
    fn from(value: TimeValue) -> Self {
        match value {
            TimeValue::Boolean(value) => ColumnValue::Boolean(value),
            TimeValue::Int32(value) => ColumnValue::Int32(value),
            TimeValue::Int64(value) => ColumnValue::Int64(value),
            TimeValue::Float(value) => ColumnValue::Float(value),
            TimeValue::Double(value) => ColumnValue::Double(value),
            TimeValue::Text(value) => ColumnValue::Binary(value),
            TimeValue::Null => ColumnValue::Null,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Column {
    data_type: TSDataType,
    values: Vec<ColumnValue>,
}

impl Column {
    pub fn new(data_type: TSDataType, values: Vec<ColumnValue>) -> Self {
        Column { data_type, values }
    }

    pub fn data_type(&self) -> TSDataType {
        self.data_type
    }

    pub fn position_count(&self) -> usize {
        self.values.len()
    }

    pub fn is_null(&self, position: usize) -> bool {
        matches!(self.values.get(position), Some(ColumnValue::Null) | None)
    }

    pub fn get(&self, position: usize) -> Option<&ColumnValue> {
        self.values.get(position)
    }

    pub fn values(&self) -> &[ColumnValue] {
        &self.values
    }

    pub fn null_count(&self) -> usize {
        self.values.iter().filter(|value| matches!(value, ColumnValue::Null)).count()
    }

    pub fn region(&self, offset: usize, length: usize) -> TsFileResult<Column> {
        let end = offset.checked_add(length).ok_or_else(|| {
            TsFileError::InvalidFileFormat("Column region offset overflow".to_string())
        })?;
        if end > self.values.len() {
            return Err(TsFileError::InvalidFileFormat(format!(
                "Column region out of bounds: offset={}, length={}, size={}",
                offset,
                length,
                self.values.len()
            )));
        }
        Ok(Column::new(self.data_type, self.values[offset..end].to_vec()))
    }

    pub fn get_bool(&self, position: usize) -> Option<bool> {
        match self.get(position) {
            Some(ColumnValue::Boolean(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn get_i32(&self, position: usize) -> Option<i32> {
        match self.get(position) {
            Some(ColumnValue::Int32(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn get_i64(&self, position: usize) -> Option<i64> {
        match self.get(position) {
            Some(ColumnValue::Int64(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn get_f32(&self, position: usize) -> Option<f32> {
        match self.get(position) {
            Some(ColumnValue::Float(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn get_f64(&self, position: usize) -> Option<f64> {
        match self.get(position) {
            Some(ColumnValue::Double(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn get_binary(&self, position: usize) -> Option<&Binary> {
        match self.get(position) {
            Some(ColumnValue::Binary(value)) => Some(value),
            _ => None,
        }
    }

    pub fn serialize<W: std::io::Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        let mut written = ReadWriteIOUtils::write_byte(self.data_type.serialize(), writer)?;
        written += ReadWriteIOUtils::write_i32(self.values.len() as i32, writer)?;
        for value in &self.values {
            match value {
                ColumnValue::Null => written += ReadWriteIOUtils::write_bool(true, writer)?,
                ColumnValue::Boolean(value) => {
                    written += ReadWriteIOUtils::write_bool(false, writer)?;
                    written += ReadWriteIOUtils::write_bool(*value, writer)?;
                }
                ColumnValue::Int32(value) => {
                    written += ReadWriteIOUtils::write_bool(false, writer)?;
                    written += ReadWriteIOUtils::write_i32(*value, writer)?;
                }
                ColumnValue::Int64(value) => {
                    written += ReadWriteIOUtils::write_bool(false, writer)?;
                    written += ReadWriteIOUtils::write_i64(*value, writer)?;
                }
                ColumnValue::Float(value) => {
                    written += ReadWriteIOUtils::write_bool(false, writer)?;
                    written += ReadWriteIOUtils::write_f32(*value, writer)?;
                }
                ColumnValue::Double(value) => {
                    written += ReadWriteIOUtils::write_bool(false, writer)?;
                    written += ReadWriteIOUtils::write_f64(*value, writer)?;
                }
                ColumnValue::Binary(value) => {
                    written += ReadWriteIOUtils::write_bool(false, writer)?;
                    written += ReadWriteIOUtils::write_binary(value, writer)?;
                }
            }
        }
        Ok(written)
    }

    pub fn deserialize<R: std::io::Read>(reader: &mut R) -> TsFileResult<Self> {
        let data_type = TSDataType::deserialize(ReadWriteIOUtils::read_byte(reader)?)?;
        let count = ReadWriteIOUtils::read_i32(reader)?;
        if count < 0 {
            return Err(TsFileError::InvalidFileFormat(format!(
                "Negative column value count: {}",
                count
            )));
        }
        let mut values = Vec::with_capacity(count as usize);
        for _ in 0..count {
            if ReadWriteIOUtils::read_bool(reader)? {
                values.push(ColumnValue::Null);
                continue;
            }
            let value = match data_type {
                TSDataType::Boolean => ColumnValue::Boolean(ReadWriteIOUtils::read_bool(reader)?),
                TSDataType::Int32 | TSDataType::Date => ColumnValue::Int32(ReadWriteIOUtils::read_i32(reader)?),
                TSDataType::Int64 | TSDataType::Timestamp => ColumnValue::Int64(ReadWriteIOUtils::read_i64(reader)?),
                TSDataType::Float => ColumnValue::Float(ReadWriteIOUtils::read_f32(reader)?),
                TSDataType::Double => ColumnValue::Double(ReadWriteIOUtils::read_f64(reader)?),
                TSDataType::Text | TSDataType::Blob | TSDataType::String => ColumnValue::Binary(ReadWriteIOUtils::read_binary(reader)?),
                _ => ColumnValue::Null,
            };
            values.push(value);
        }
        Ok(Column::new(data_type, values))
    }
}

#[derive(Debug, Clone)]
pub struct ColumnBuilder {
    data_type: TSDataType,
    values: Vec<ColumnValue>,
}

impl ColumnBuilder {
    pub fn new(data_type: TSDataType) -> Self {
        ColumnBuilder {
            data_type,
            values: Vec::new(),
        }
    }

    pub fn append_null(&mut self) {
        self.values.push(ColumnValue::Null);
    }

    pub fn write_bool(&mut self, value: bool) {
        self.values.push(ColumnValue::Boolean(value));
    }

    pub fn write_i32(&mut self, value: i32) {
        self.values.push(ColumnValue::Int32(value));
    }

    pub fn write_i64(&mut self, value: i64) {
        self.values.push(ColumnValue::Int64(value));
    }

    pub fn write_f32(&mut self, value: f32) {
        self.values.push(ColumnValue::Float(value));
    }

    pub fn write_f64(&mut self, value: f64) {
        self.values.push(ColumnValue::Double(value));
    }

    pub fn write_binary(&mut self, value: Binary) {
        self.values.push(ColumnValue::Binary(value));
    }

    pub fn append_value(&mut self, value: ColumnValue) {
        self.values.push(value);
    }

    pub fn build(self) -> Column {
        Column::new(self.data_type, self.values)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}
