// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Table-model/tree-model reader facades.

use std::path::Path;

use crate::error::TsFileResult;
use crate::read::result_set::{QueryExpression, ResultSet};
use crate::read::tsfile_reader::TsFileReader;

pub trait ITsFileReader {
    fn query(&mut self, expression: QueryExpression) -> TsFileResult<ResultSet>;
}

pub struct TsFileTreeReader {
    inner: TsFileReader,
}

impl TsFileTreeReader {
    pub fn new<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        Ok(TsFileTreeReader { inner: TsFileReader::new(path)? })
    }

    pub fn inner_mut(&mut self) -> &mut TsFileReader {
        &mut self.inner
    }
}

impl ITsFileReader for TsFileTreeReader {
    fn query(&mut self, expression: QueryExpression) -> TsFileResult<ResultSet> {
        self.inner.query(expression)
    }
}

pub struct DeviceTableModelReader {
    inner: TsFileReader,
}

impl DeviceTableModelReader {
    pub fn new<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        Ok(DeviceTableModelReader { inner: TsFileReader::new(path)? })
    }
}

impl ITsFileReader for DeviceTableModelReader {
    fn query(&mut self, expression: QueryExpression) -> TsFileResult<ResultSet> {
        self.inner.query(expression)
    }
}
