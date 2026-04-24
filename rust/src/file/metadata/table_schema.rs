// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Table schema metadata for TsFile table model.

use std::collections::HashMap;
use std::io::{Read, Write};

use crate::common::enums::{CompressionType, TSDataType, TSEncoding};
use crate::error::{TsFileError, TsFileResult};
use crate::utils::{ReadWriteForEncodingUtils, ReadWriteIOUtils};
use crate::write::schema::MeasurementSchema;

/// Column category in Java's `ColumnCategory` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColumnCategory {
    Tag,
    Field,
    Attribute,
    Time,
}

impl Default for ColumnCategory {
    fn default() -> Self {
        ColumnCategory::Field
    }
}

impl ColumnCategory {
    pub fn serialize(self) -> i32 {
        match self {
            ColumnCategory::Tag => 0,
            ColumnCategory::Field => 1,
            ColumnCategory::Attribute => 2,
            ColumnCategory::Time => 3,
        }
    }

    pub fn deserialize(value: i32) -> TsFileResult<Self> {
        match value {
            0 => Ok(ColumnCategory::Tag),
            1 => Ok(ColumnCategory::Field),
            2 => Ok(ColumnCategory::Attribute),
            3 => Ok(ColumnCategory::Time),
            _ => Err(TsFileError::InvalidFileFormat(format!(
                "Unknown ColumnCategory ordinal: {}",
                value
            ))),
        }
    }
}

/// Public column schema, matching Java's `ColumnSchema` API shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnSchema {
    pub column_name: String,
    pub data_type: TSDataType,
    pub column_category: ColumnCategory,
}

impl ColumnSchema {
    pub fn new(
        column_name: String,
        data_type: TSDataType,
        column_category: ColumnCategory,
    ) -> Self {
        ColumnSchema {
            column_name: column_name.to_lowercase(),
            data_type,
            column_category,
        }
    }
}

/// Fluent builder matching Java's `ColumnSchemaBuilder`.
#[derive(Debug, Clone, Default)]
pub struct ColumnSchemaBuilder {
    column_name: Option<String>,
    data_type: Option<TSDataType>,
    column_category: ColumnCategory,
}

impl ColumnSchemaBuilder {
    pub fn new() -> Self {
        ColumnSchemaBuilder {
            column_name: None,
            data_type: None,
            column_category: ColumnCategory::Field,
        }
    }

    pub fn name(mut self, column_name: String) -> Self {
        self.column_name = Some(column_name);
        self
    }

    pub fn data_type(mut self, data_type: TSDataType) -> Self {
        self.data_type = Some(data_type);
        self
    }

    pub fn category(mut self, column_category: ColumnCategory) -> Self {
        self.column_category = column_category;
        self
    }

    pub fn build(self) -> TsFileResult<ColumnSchema> {
        let column_name = self.column_name.ok_or_else(|| {
            TsFileError::SchemaError("Column name must be set before building".to_string())
        })?;
        if column_name.is_empty() {
            return Err(TsFileError::SchemaError(
                "Column name must be a non empty string".to_string(),
            ));
        }
        let data_type = self.data_type.ok_or_else(|| {
            TsFileError::SchemaError("Column data type must be set before building".to_string())
        })?;
        Ok(ColumnSchema::new(
            column_name,
            data_type,
            self.column_category,
        ))
    }
}

/// Table schema stored in `TsFileMetadata.tableSchemaMap`.
#[derive(Debug, Clone)]
pub struct TableSchema {
    pub table_name: String,
    pub measurement_schemas: Vec<MeasurementSchema>,
    pub column_categories: Vec<ColumnCategory>,
    column_pos_index: HashMap<String, usize>,
    id_column_order: HashMap<String, usize>,
    updatable: bool,
}

impl TableSchema {
    pub fn new(table_name: String) -> Self {
        TableSchema {
            table_name: table_name.to_lowercase(),
            measurement_schemas: Vec::new(),
            column_categories: Vec::new(),
            column_pos_index: HashMap::new(),
            id_column_order: HashMap::new(),
            updatable: true,
        }
    }

    pub fn from_columns(table_name: String, columns: Vec<ColumnSchema>) -> TsFileResult<Self> {
        let mut schema = TableSchema::new(table_name);
        schema.updatable = false;
        for column in columns {
            schema.add_column(
                MeasurementSchema::new(
                    column.column_name,
                    column.data_type,
                    TSEncoding::Plain,
                    CompressionType::Uncompressed,
                ),
                column.column_category,
            )?;
        }
        Ok(schema)
    }


    pub fn columns(&self) -> Vec<ColumnSchema> {
        self.measurement_schemas
            .iter()
            .zip(self.column_categories.iter())
            .map(|(schema, category)| {
                ColumnSchema::new(schema.measurement_id.clone(), schema.data_type, *category)
            })
            .collect()
    }

    pub fn add_column(
        &mut self,
        mut measurement_schema: MeasurementSchema,
        column_category: ColumnCategory,
    ) -> TsFileResult<()> {
        measurement_schema.measurement_id = measurement_schema.measurement_id.to_lowercase();
        if self
            .column_pos_index
            .contains_key(&measurement_schema.measurement_id)
        {
            return Err(TsFileError::SchemaError(format!(
                "Duplicate column in table {}: {}",
                self.table_name, measurement_schema.measurement_id
            )));
        }
        let index = self.measurement_schemas.len();
        self.column_pos_index
            .insert(measurement_schema.measurement_id.clone(), index);
        if column_category == ColumnCategory::Tag {
            self.id_column_order
                .insert(measurement_schema.measurement_id.clone(), self.id_column_order.len());
        }
        self.measurement_schemas.push(measurement_schema);
        self.column_categories.push(column_category);
        Ok(())
    }

    pub fn find_column_index(&mut self, column_name: &str) -> Option<usize> {
        let column_name = column_name.to_lowercase();
        if let Some(index) = self.column_pos_index.get(&column_name) {
            return Some(*index);
        }
        let index = self
            .measurement_schemas
            .iter()
            .position(|schema| schema.measurement_id == column_name)?;
        self.column_pos_index.insert(column_name, index);
        Some(index)
    }

    pub fn find_id_column_order(&mut self, column_name: &str) -> Option<usize> {
        let column_name = column_name.to_lowercase();
        if let Some(index) = self.id_column_order.get(&column_name) {
            return Some(*index);
        }
        let mut order = 0usize;
        for (index, schema) in self.measurement_schemas.iter().enumerate() {
            if self.column_categories[index] == ColumnCategory::Tag {
                if schema.measurement_id == column_name {
                    self.id_column_order.insert(column_name, order);
                    return Some(order);
                }
                order += 1;
            }
        }
        None
    }

    pub fn tag_column_count(&self) -> usize {
        self.column_categories
            .iter()
            .filter(|category| **category == ColumnCategory::Tag)
            .count()
    }

    pub fn is_updatable(&self) -> bool {
        self.updatable
    }

    pub fn set_updatable(&mut self, updatable: bool) {
        self.updatable = updatable;
    }

    pub fn serialize<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        let mut written = ReadWriteForEncodingUtils::write_unsigned_var_int(
            self.measurement_schemas.len() as u32,
            writer,
        )?;
        for (schema, category) in self
            .measurement_schemas
            .iter()
            .zip(self.column_categories.iter())
        {
            written += schema.serialize(writer)?;
            written += ReadWriteIOUtils::write_i32(category.serialize(), writer)?;
        }
        Ok(written)
    }

    pub fn deserialize<R: Read>(reader: &mut R, table_name: String) -> TsFileResult<Self> {
        let column_count = ReadWriteForEncodingUtils::read_unsigned_var_int(reader)? as usize;
        let mut schema = TableSchema::new(table_name);
        schema.updatable = false;
        for _ in 0..column_count {
            let measurement_schema = MeasurementSchema::deserialize(reader)?;
            let category = ColumnCategory::deserialize(ReadWriteIOUtils::read_i32(reader)?)?;
            schema.add_column(measurement_schema, category)?;
        }
        Ok(schema)
    }
}
