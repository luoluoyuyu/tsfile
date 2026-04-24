// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use crate::write::record::{DataPoint, TSRecord as InnerTSRecord};

#[derive(Debug, Clone)]
pub struct TSRecord {
    inner: InnerTSRecord,
}

impl TSRecord {
    pub fn new(timestamp: i64, device_id: String) -> Self { TSRecord { inner: InnerTSRecord::new(timestamp, device_id) } }
    pub fn add_tuple(&mut self, data_point: DataPoint) { self.inner.add_tuple(data_point); }
    pub fn add_data_point(&mut self, data_point: DataPoint) { self.inner.add_data_point(data_point); }
    pub fn inner(&self) -> &InnerTSRecord { &self.inner }
    pub fn inner_mut(&mut self) -> &mut InnerTSRecord { &mut self.inner }
    pub fn into_inner(self) -> InnerTSRecord { self.inner }
}
