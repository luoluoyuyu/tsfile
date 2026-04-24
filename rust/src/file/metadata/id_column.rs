// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Database/table name extractors for device ids, matching Java's idcolumn helpers.

use crate::file::metadata::device_id::DeviceId;

pub trait DatabaseNameExtractor {
    fn extract(&self, device_id: &DeviceId) -> Option<String>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TwoLevelDBExtractor;

#[derive(Debug, Clone, Copy, Default)]
pub struct ThreeLevelDBExtractor;

#[derive(Debug, Clone, Copy, Default)]
pub struct FourOrHigherLevelDBExtractor {
    pub level: usize,
}

impl FourOrHigherLevelDBExtractor {
    pub fn new(level: usize) -> Self {
        FourOrHigherLevelDBExtractor { level: level.max(4) }
    }
}

impl DatabaseNameExtractor for TwoLevelDBExtractor {
    fn extract(&self, device_id: &DeviceId) -> Option<String> {
        extract_prefix(device_id, 2)
    }
}

impl DatabaseNameExtractor for ThreeLevelDBExtractor {
    fn extract(&self, device_id: &DeviceId) -> Option<String> {
        extract_prefix(device_id, 3)
    }
}

impl DatabaseNameExtractor for FourOrHigherLevelDBExtractor {
    fn extract(&self, device_id: &DeviceId) -> Option<String> {
        extract_prefix(device_id, self.level)
    }
}

pub fn extract_prefix(device_id: &DeviceId, level: usize) -> Option<String> {
    let segments = device_id.segments();
    (segments.len() >= level).then(|| segments[..level].join("."))
}
