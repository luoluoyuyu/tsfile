// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use crate::file::metadata::device_id::DeviceId;
use crate::file::metadata::id_column::{extract_prefix, DatabaseNameExtractor};

#[derive(Debug, Clone, Copy)]
pub struct FourOrHigherLevelDBExtractor {
    pub level: usize,
}

impl FourOrHigherLevelDBExtractor {
    pub fn new(level: usize) -> Self {
        FourOrHigherLevelDBExtractor { level: level.max(4) }
    }
}

impl Default for FourOrHigherLevelDBExtractor {
    fn default() -> Self {
        FourOrHigherLevelDBExtractor::new(4)
    }
}

impl DatabaseNameExtractor for FourOrHigherLevelDBExtractor {
    fn extract(&self, device_id: &DeviceId) -> Option<String> {
        extract_prefix(device_id, self.level)
    }
}
