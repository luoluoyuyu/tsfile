// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Reader API traits and builder matching Java reader abstractions.

use std::path::{Path, PathBuf};

use crate::error::TsFileResult;
use crate::read::result_set::{QueryExpression, ResultSet};
use crate::read::time_value_pair::TimeValuePair;
use crate::read::TsFileReader;

pub trait TsFileReadApi {
    fn read_timeseries(
        &mut self,
        device_id: &str,
        measurement_id: &str,
    ) -> TsFileResult<Vec<TimeValuePair>>;

    fn query(&mut self, expression: QueryExpression) -> TsFileResult<ResultSet>;
}

#[derive(Debug, Clone)]
pub struct TsFileReaderBuilder {
    path: PathBuf,
}

impl TsFileReaderBuilder {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        TsFileReaderBuilder {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn build(self) -> TsFileResult<TsFileReader> {
        TsFileReader::new(self.path)
    }
}
