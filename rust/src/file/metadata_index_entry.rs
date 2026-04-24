// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Metadata index entry trait, corresponding to Java's `IMetadataIndexEntry`.

use std::io::Write;

use crate::error::TsFileResult;
use crate::file::metadata::device_id::DeviceId;
use crate::file::metadata::metadata_index_node::{
    DeviceMetadataIndexEntry, MetadataIndexEntry,
};

pub trait MetadataIndexEntryView {
    fn name(&self) -> String;
    fn offset(&self) -> i64;
    fn serialize_to<W: Write>(&self, writer: &mut W) -> TsFileResult<usize>;
}

impl MetadataIndexEntryView for MetadataIndexEntry {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn offset(&self) -> i64 {
        self.offset
    }

    fn serialize_to<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        self.serialize(writer)
    }
}

impl MetadataIndexEntryView for DeviceMetadataIndexEntry {
    fn name(&self) -> String {
        self.device_id.to_string()
    }

    fn offset(&self) -> i64 {
        self.offset
    }

    fn serialize_to<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        self.serialize(writer)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataIndexEntryKind {
    Device(DeviceId, i64),
    Measurement(String, i64),
}

impl MetadataIndexEntryKind {
    pub fn offset(&self) -> i64 {
        match self {
            MetadataIndexEntryKind::Device(_, offset)
            | MetadataIndexEntryKind::Measurement(_, offset) => *offset,
        }
    }

    pub fn key(&self) -> String {
        match self {
            MetadataIndexEntryKind::Device(device_id, _) => device_id.to_string(),
            MetadataIndexEntryKind::Measurement(name, _) => name.clone(),
        }
    }
}
