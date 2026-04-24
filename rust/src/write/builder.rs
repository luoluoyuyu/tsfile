// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TsFile writer builder matching Java's builder-style construction.

use std::path::{Path, PathBuf};

use crate::error::TsFileResult;
use crate::write::schema::Schema;
use crate::write::tsfile_writer::TsFileWriter;

#[derive(Debug)]
pub struct TsFileWriterBuilder {
    path: PathBuf,
    schema: Option<Schema>,
    chunk_group_size_threshold: Option<usize>,
    is_unseq: bool,
}

impl TsFileWriterBuilder {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        TsFileWriterBuilder {
            path: path.as_ref().to_path_buf(),
            schema: None,
            chunk_group_size_threshold: None,
            is_unseq: false,
        }
    }

    pub fn with_schema(mut self, schema: Schema) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn with_chunk_group_size_threshold(mut self, threshold: usize) -> Self {
        self.chunk_group_size_threshold = Some(threshold);
        self
    }

    pub fn with_unseq(mut self, is_unseq: bool) -> Self {
        self.is_unseq = is_unseq;
        self
    }

    pub fn build(self) -> TsFileResult<TsFileWriter> {
        let mut writer = if let Some(schema) = self.schema {
            TsFileWriter::new_with_schema(self.path, schema)?
        } else {
            TsFileWriter::new(self.path)?
        };
        if let Some(threshold) = self.chunk_group_size_threshold {
            writer.set_chunk_group_size_threshold(threshold);
        }
        writer.set_unseq(self.is_unseq);
        Ok(writer)
    }
}
