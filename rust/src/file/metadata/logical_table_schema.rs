// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Logical table schema for path-based device ids.

use std::io::{Read, Write};

use crate::common::enums::{CompressionType, TSDataType, TSEncoding};
use crate::error::TsFileResult;
use crate::file::metadata::chunk_metadata::ChunkGroupMetadata;
use crate::file::metadata::table_schema::{ColumnCategory, TableSchema};
use crate::write::schema::MeasurementSchema;

#[derive(Debug, Clone)]
pub struct LogicalTableSchema {
    pub table_schema: TableSchema,
    pub max_level: usize,
}

impl LogicalTableSchema {
    pub fn new(table_name: String) -> Self {
        LogicalTableSchema {
            table_schema: TableSchema::new(table_name),
            max_level: 0,
        }
    }

    pub fn from_table_schema(table_schema: TableSchema) -> Self {
        LogicalTableSchema {
            table_schema,
            max_level: 0,
        }
    }

    pub fn update(&mut self, chunk_group_metadata: &ChunkGroupMetadata) {
        self.max_level = self
            .max_level
            .max(chunk_group_metadata.device_id.split('.').count());
        for chunk_metadata in &chunk_group_metadata.chunk_metadata_list {
            if self
                .table_schema
                .find_column_index(&chunk_metadata.measurement_uid)
                .is_none()
            {
                let _ = self.table_schema.add_column(
                    MeasurementSchema::new(
                        chunk_metadata.measurement_uid.clone(),
                        chunk_metadata.data_type,
                        TSEncoding::Plain,
                        CompressionType::Uncompressed,
                    ),
                    ColumnCategory::Field,
                );
            }
        }
    }

    pub fn generate_id_columns(&self) -> Vec<MeasurementSchema> {
        (1..self.max_level)
            .map(|level| {
                MeasurementSchema::new(
                    format!("__level{}", level),
                    TSDataType::String,
                    TSEncoding::Plain,
                    CompressionType::Uncompressed,
                )
            })
            .collect()
    }

    pub fn finalize_column_schema(&mut self) -> TsFileResult<()> {
        if !self.table_schema.is_updatable() {
            return Ok(());
        }

        let existing_columns = self.table_schema.measurement_schemas.clone();
        let existing_categories = self.table_schema.column_categories.clone();
        let mut finalized = TableSchema::new(self.table_schema.table_name.clone());
        for id_column in self.generate_id_columns() {
            finalized.add_column(id_column, ColumnCategory::Tag)?;
        }
        for (schema, category) in existing_columns.into_iter().zip(existing_categories) {
            finalized.add_column(schema, category)?;
        }
        finalized.set_updatable(false);
        self.table_schema = finalized;
        Ok(())
    }

    pub fn serialize<W: Write>(&mut self, writer: &mut W) -> TsFileResult<usize> {
        self.finalize_column_schema()?;
        self.table_schema.serialize(writer)
    }

    pub fn deserialize<R: Read>(reader: &mut R, table_name: String) -> TsFileResult<Self> {
        Ok(LogicalTableSchema::from_table_schema(TableSchema::deserialize(
            reader, table_name,
        )?))
    }
}

impl std::ops::Deref for LogicalTableSchema {
    type Target = TableSchema;

    fn deref(&self) -> &Self::Target {
        &self.table_schema
    }
}

impl std::ops::DerefMut for LogicalTableSchema {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.table_schema
    }
}
