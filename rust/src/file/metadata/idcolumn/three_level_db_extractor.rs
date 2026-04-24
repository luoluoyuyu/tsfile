// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use crate::file::metadata::device_id::DeviceId;
use crate::file::metadata::id_column::{extract_prefix, DatabaseNameExtractor};

#[derive(Debug, Clone, Copy, Default)]
pub struct ThreeLevelDBExtractor;

impl DatabaseNameExtractor for ThreeLevelDBExtractor {
    fn extract(&self, device_id: &DeviceId) -> Option<String> {
        extract_prefix(device_id, 3)
    }
}
