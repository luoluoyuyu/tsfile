// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Device id metadata model, corresponding to Java's `IDeviceID` hierarchy.

use std::cmp::Ordering;
use std::fmt;
use std::io::{Read, Write};

use crate::error::{TsFileError, TsFileResult};
use crate::utils::ReadWriteIOUtils;

/// Device identifier used by metadata index entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeviceId {
    /// Legacy path device id, serialized as a single string in v3-compatible files.
    Plain(String),
    /// Table-model tuple device id. The first segment is the table name.
    StringArray(Vec<String>),
}

impl DeviceId {
    pub fn plain(device_id: impl Into<String>) -> Self {
        DeviceId::Plain(device_id.into())
    }

    pub fn string_array(segments: Vec<String>) -> TsFileResult<Self> {
        if segments.is_empty() {
            return Err(TsFileError::InvalidFileFormat(
                "DeviceId must contain at least one segment".to_string(),
            ));
        }
        Ok(DeviceId::StringArray(segments))
    }

    pub fn is_empty(&self) -> bool {
        match self {
            DeviceId::Plain(value) => value.is_empty(),
            DeviceId::StringArray(segments) => segments.is_empty(),
        }
    }

    pub fn is_table_model(&self) -> bool {
        matches!(self, DeviceId::StringArray(_))
    }

    pub fn table_name(&self) -> String {
        match self {
            DeviceId::Plain(value) => table_name_from_path(value),
            DeviceId::StringArray(segments) => segments.first().cloned().unwrap_or_default(),
        }
    }

    pub fn segment_num(&self) -> usize {
        match self {
            DeviceId::Plain(value) => value.split('.').count(),
            DeviceId::StringArray(segments) => segments.len(),
        }
    }

    pub fn segment(&self, index: usize) -> Option<String> {
        match self {
            DeviceId::Plain(value) => value.split('.').nth(index).map(ToOwned::to_owned),
            DeviceId::StringArray(segments) => segments.get(index).cloned(),
        }
    }

    pub fn segments(&self) -> Vec<String> {
        match self {
            DeviceId::Plain(value) => value.split('.').map(ToOwned::to_owned).collect(),
            DeviceId::StringArray(segments) => segments.clone(),
        }
    }

    pub fn starts_with(&self, prefix: &str, match_entire_segment: bool) -> bool {
        let full = self.to_string();
        if !match_entire_segment {
            return full.starts_with(prefix);
        }
        full == prefix || full.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('.'))
    }

    pub fn match_database_name(&self, database_name: &str) -> bool {
        let table_name = self.table_name();
        table_name == database_name
            || table_name
                .strip_prefix(database_name)
                .is_some_and(|rest| rest.starts_with('.'))
            || self.starts_with(database_name, true)
    }

    pub fn serialize<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        match self {
            DeviceId::Plain(value) => ReadWriteIOUtils::write_var_int_string(value, writer),
            DeviceId::StringArray(segments) => {
                let mut written = ReadWriteIOUtils::write_i32(segments.len() as i32, writer)?;
                for segment in segments {
                    written += ReadWriteIOUtils::write_var_int_string(segment, writer)?;
                }
                Ok(written)
            }
        }
    }

    pub fn deserialize_plain<R: Read>(reader: &mut R) -> TsFileResult<Self> {
        Ok(DeviceId::Plain(ReadWriteIOUtils::read_var_int_string(reader)?))
    }

    pub fn deserialize_string_array<R: Read>(reader: &mut R) -> TsFileResult<Self> {
        let segment_count = ReadWriteIOUtils::read_i32(reader)?;
        if segment_count < 0 {
            return Err(TsFileError::InvalidFileFormat(format!(
                "Negative DeviceId segment count: {}",
                segment_count
            )));
        }
        let mut segments = Vec::with_capacity(segment_count as usize);
        for _ in 0..segment_count {
            segments.push(ReadWriteIOUtils::read_var_int_string(reader)?);
        }
        DeviceId::string_array(segments)
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceId::Plain(value) => formatter.write_str(value),
            DeviceId::StringArray(segments) => formatter.write_str(&segments.join(".")),
        }
    }
}

impl PartialOrd for DeviceId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DeviceId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

fn table_name_from_path(path: &str) -> String {
    let mut parts = path.rsplitn(2, '.');
    let _measurement_or_leaf = parts.next();
    parts.next().unwrap_or(path).to_string()
}
