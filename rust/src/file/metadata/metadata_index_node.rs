// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Metadata index node for the B-tree-like index structure.

use std::io::{Read, Write};

use crate::common::enums::MetadataIndexNodeType;
use crate::error::TsFileResult;
use crate::utils::{ReadWriteForEncodingUtils, ReadWriteIOUtils};

/// A single entry in a MetadataIndexNode.
#[derive(Debug, Clone)]
pub struct MetadataIndexEntry {
    /// Name (device or measurement).
    pub name: String,
    /// Offset in the TsFile.
    pub offset: i64,
}

impl MetadataIndexEntry {
    pub fn new(name: String, offset: i64) -> Self {
        MetadataIndexEntry { name, offset }
    }

    pub fn serialize<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        let mut written = 0;
        written += ReadWriteIOUtils::write_var_int_string(&self.name, writer)?;
        written += ReadWriteIOUtils::write_i64(self.offset, writer)?;
        Ok(written)
    }

    pub fn deserialize<R: Read>(reader: &mut R) -> TsFileResult<Self> {
        let name = ReadWriteIOUtils::read_var_int_string(reader)?;
        let offset = ReadWriteIOUtils::read_i64(reader)?;
        Ok(MetadataIndexEntry { name, offset })
    }
}

/// A node in the metadata B-tree index.
///
/// Mirrors Java's MetadataIndexNode.
#[derive(Debug, Clone)]
pub struct MetadataIndexNode {
    /// Child entries.
    pub children: Vec<MetadataIndexEntry>,
    /// End offset of the last child.
    pub end_offset: i64,
    /// Node type (internal/leaf, device/measurement).
    pub node_type: MetadataIndexNodeType,
}

impl MetadataIndexNode {
    pub fn new(node_type: MetadataIndexNodeType, end_offset: i64) -> Self {
        MetadataIndexNode {
            children: Vec::new(),
            end_offset,
            node_type,
        }
    }

    pub fn add_child(&mut self, entry: MetadataIndexEntry) {
        self.children.push(entry);
    }

    pub fn serialize<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        let mut written = 0;
        written +=
            ReadWriteForEncodingUtils::write_unsigned_var_int(self.children.len() as u32, writer)?;
        for child in &self.children {
            written += child.serialize(writer)?;
        }
        written += ReadWriteIOUtils::write_i64(self.end_offset, writer)?;
        written += ReadWriteIOUtils::write_byte(self.node_type.serialize(), writer)?;
        Ok(written)
    }

    pub fn deserialize<R: Read>(reader: &mut R) -> TsFileResult<Self> {
        let count = ReadWriteForEncodingUtils::read_unsigned_var_int(reader)? as usize;
        let mut children = Vec::with_capacity(count);
        for _ in 0..count {
            children.push(MetadataIndexEntry::deserialize(reader)?);
        }
        let end_offset = ReadWriteIOUtils::read_i64(reader)?;
        let node_type_byte = ReadWriteIOUtils::read_byte(reader)?;
        let node_type = MetadataIndexNodeType::deserialize(node_type_byte)?;
        Ok(MetadataIndexNode {
            children,
            end_offset,
            node_type,
        })
    }
}
