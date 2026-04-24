// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use crate::common::enums::TSDataType;
use crate::read::block::column::{Column, ColumnBuilder, ColumnValue};
use crate::read::common::RowRecord;
use crate::error::{TsFileError, TsFileResult};
use crate::utils::ReadWriteIOUtils;

#[derive(Debug, Clone)]
pub struct TsBlock {
    time_column: Vec<i64>,
    value_columns: Vec<Column>,
}

impl TsBlock {
    pub fn new(time_column: Vec<i64>, value_columns: Vec<Column>) -> Self {
        TsBlock {
            time_column,
            value_columns,
        }
    }

    pub fn position_count(&self) -> usize {
        self.time_column.len()
    }

    pub fn time_by_index(&self, index: usize) -> Option<i64> {
        self.time_column.get(index).copied()
    }

    pub fn column(&self, index: usize) -> Option<&Column> {
        self.value_columns.get(index)
    }

    pub fn columns(&self) -> &[Column] {
        &self.value_columns
    }

    pub fn times(&self) -> &[i64] {
        &self.time_column
    }

    pub fn value_column_count(&self) -> usize {
        self.value_columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.time_column.is_empty()
    }

    pub fn region(&self, offset: usize, length: usize) -> TsFileResult<TsBlock> {
        let end = offset.checked_add(length).ok_or_else(|| {
            TsFileError::InvalidFileFormat("TsBlock region offset overflow".to_string())
        })?;
        if end > self.time_column.len() {
            return Err(TsFileError::InvalidFileFormat(format!(
                "TsBlock region out of bounds: offset={}, length={}, size={}",
                offset,
                length,
                self.time_column.len()
            )));
        }
        let columns = self
            .value_columns
            .iter()
            .map(|column| column.region(offset, length))
            .collect::<TsFileResult<Vec<_>>>()?;
        Ok(TsBlock::new(self.time_column[offset..end].to_vec(), columns))
    }

    pub fn row(&self, index: usize) -> Option<Vec<&ColumnValue>> {
        if index >= self.position_count() {
            return None;
        }
        Some(self.value_columns.iter().filter_map(|column| column.get(index)).collect())
    }

    pub fn serialize<W: std::io::Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        let mut written = ReadWriteIOUtils::write_i32(self.time_column.len() as i32, writer)?;
        for timestamp in &self.time_column {
            written += ReadWriteIOUtils::write_i64(*timestamp, writer)?;
        }
        written += ReadWriteIOUtils::write_i32(self.value_columns.len() as i32, writer)?;
        for column in &self.value_columns {
            written += column.serialize(writer)?;
        }
        Ok(written)
    }

    pub fn deserialize<R: std::io::Read>(reader: &mut R) -> TsFileResult<Self> {
        let position_count = ReadWriteIOUtils::read_i32(reader)?;
        if position_count < 0 {
            return Err(TsFileError::InvalidFileFormat(format!(
                "Negative TsBlock position count: {}",
                position_count
            )));
        }
        let mut times = Vec::with_capacity(position_count as usize);
        for _ in 0..position_count {
            times.push(ReadWriteIOUtils::read_i64(reader)?);
        }
        let column_count = ReadWriteIOUtils::read_i32(reader)?;
        if column_count < 0 {
            return Err(TsFileError::InvalidFileFormat(format!(
                "Negative TsBlock column count: {}",
                column_count
            )));
        }
        let mut columns = Vec::with_capacity(column_count as usize);
        for _ in 0..column_count {
            columns.push(Column::deserialize(reader)?);
        }
        Ok(TsBlock::new(times, columns))
    }
}

pub struct TsBlockBuilder {
    time_column: Vec<i64>,
    value_builders: Vec<ColumnBuilder>,
}

impl TsBlockBuilder {
    pub fn new(data_types: Vec<TSDataType>) -> Self {
        TsBlockBuilder {
            time_column: Vec::new(),
            value_builders: data_types.into_iter().map(ColumnBuilder::new).collect(),
        }
    }

    pub fn declare_position(&mut self, timestamp: i64, values: Vec<ColumnValue>) {
        self.time_column.push(timestamp);
        for (builder, value) in self.value_builders.iter_mut().zip(values.into_iter()) {
            builder.append_value(value);
        }
    }

    pub fn append_row_record(&mut self, row: &RowRecord) {
        self.time_column.push(row.timestamp);
        for (builder, field) in self.value_builders.iter_mut().zip(row.fields.iter()) {
            builder.append_value(field.value.clone().into());
        }
    }

    pub fn build(self) -> TsBlock {
        TsBlock::new(
            self.time_column,
            self.value_builders.into_iter().map(ColumnBuilder::build).collect(),
        )
    }

    pub fn position_count(&self) -> usize {
        self.time_column.len()
    }
}
