// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TsFileMetadata - the file-level footer metadata.

use std::collections::HashMap;
use std::io::{Read, Write};

use crate::common::enums::MetadataIndexNodeType;
use crate::error::TsFileResult;
use crate::file::metadata::metadata_index_node::MetadataIndexNode;
use crate::file::metadata::table_schema::TableSchema;
use crate::utils::bloom_filter::BloomFilter;
use crate::utils::{ReadWriteForEncodingUtils, ReadWriteIOUtils};

/// File-level metadata stored in the TsFile footer.
///
/// Mirrors Java's TsFileMetadata.
#[derive(Debug)]
pub struct TsFileMetadata {
    /// Map from table/device name to MetadataIndexNode root.
    pub table_metadata_index_node_map: HashMap<String, MetadataIndexNode>,
    /// Bloom filter for timeseries paths.
    pub bloom_filter: Option<BloomFilter>,
    /// Map from table name to table schema.
    pub table_schema_map: HashMap<String, TableSchema>,
    /// Offset of the SEPARATOR marker.
    pub meta_offset: i64,
    /// TsFile properties (encryption, etc.).
    pub ts_file_properties: HashMap<String, String>,
}

impl TsFileMetadata {
    pub fn new() -> Self {
        TsFileMetadata {
            table_metadata_index_node_map: HashMap::new(),
            bloom_filter: None,
            table_schema_map: HashMap::new(),
            meta_offset: 0,
            ts_file_properties: HashMap::new(),
        }
    }

    /// Add a metadata index node for a table/device group.
    pub fn add_table_metadata_index_node(
        &mut self,
        table_name: String,
        node: MetadataIndexNode,
    ) {
        self.table_metadata_index_node_map.insert(table_name, node);
    }

    /// Add a property.
    pub fn add_property(&mut self, key: String, value: String) {
        self.ts_file_properties.insert(key, value);
    }

    pub fn metadata_index_node(&self, table_name: &str) -> Option<&MetadataIndexNode> {
        self.table_metadata_index_node_map
            .get(table_name)
            .or_else(|| self.table_metadata_index_node_map.get(""))
    }

    pub fn may_contain_path(&self, full_path: &str) -> bool {
        self.bloom_filter
            .as_ref()
            .is_none_or(|bloom_filter| bloom_filter.contains(full_path))
    }

    pub fn property(&self, key: &str) -> Option<&str> {
        self.ts_file_properties.get(key).map(String::as_str)
    }

    pub fn add_table_schema(&mut self, table_name: String, schema: TableSchema) {
        self.table_schema_map.insert(table_name.to_lowercase(), schema);
    }

    /// Serialize to writer using the current Java TsFileMetadata footer format.
    pub fn serialize<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        let mut written = 0;

        let mut table_entries: Vec<_> = self.table_metadata_index_node_map.iter().collect();
        table_entries.sort_by(|a, b| a.0.cmp(b.0));
        written += ReadWriteForEncodingUtils::write_unsigned_var_int(
            table_entries.len() as u32,
            writer,
        )?;
        for (table_name, node) in table_entries {
            written += ReadWriteIOUtils::write_var_int_string(table_name, writer)?;
            written += node.serialize(writer)?;
        }

        let mut schema_entries: Vec<_> = self.table_schema_map.iter().collect();
        schema_entries.sort_by(|a, b| a.0.cmp(b.0));
        written += ReadWriteForEncodingUtils::write_unsigned_var_int(
            schema_entries.len() as u32,
            writer,
        )?;
        for (table_name, schema) in schema_entries {
            written += ReadWriteIOUtils::write_var_int_string(table_name, writer)?;
            written += schema.serialize(writer)?;
        }
        written += ReadWriteIOUtils::write_i64(self.meta_offset, writer)?;
        match &self.bloom_filter {
            Some(bf) => written += bf.serialize(writer)?,
            None => written += ReadWriteForEncodingUtils::write_unsigned_var_int(0, writer)?,
        }

        let mut property_entries: Vec<_> = self.ts_file_properties.iter().collect();
        property_entries.sort_by(|a, b| a.0.cmp(b.0));
        written += ReadWriteForEncodingUtils::write_var_int(
            property_entries.len() as i32,
            writer,
        )?;
        for (key, value) in property_entries {
            written += ReadWriteIOUtils::write_var_int_string(key, writer)?;
            written += ReadWriteIOUtils::write_var_int_string(value, writer)?;
        }

        Ok(written)
    }

    /// Deserialize from reader. Supports the current Java footer format and the earlier
    /// Rust V3-compatible footer used by this crate during development.
    pub fn deserialize<R: Read>(reader: &mut R) -> TsFileResult<Self> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;

        match Self::deserialize_current(&bytes) {
            Ok(metadata) => Ok(metadata),
            Err(_) => Self::deserialize_legacy_v3(&bytes),
        }
    }

    fn deserialize_current(bytes: &[u8]) -> TsFileResult<Self> {
        let mut metadata = TsFileMetadata::new();
        let mut cursor = std::io::Cursor::new(bytes);

        let table_count = ReadWriteForEncodingUtils::read_unsigned_var_int(&mut cursor)? as usize;
        for _ in 0..table_count {
            let table_name = ReadWriteIOUtils::read_var_int_string(&mut cursor)?;
            let node = MetadataIndexNode::deserialize(&mut cursor)?;
            metadata.table_metadata_index_node_map.insert(table_name, node);
        }

        let table_schema_count =
            ReadWriteForEncodingUtils::read_unsigned_var_int(&mut cursor)? as usize;
        for _ in 0..table_schema_count {
            let table_name = ReadWriteIOUtils::read_var_int_string(&mut cursor)?;
            let schema = TableSchema::deserialize(&mut cursor, table_name.clone())?;
            metadata.table_schema_map.insert(table_name, schema);
        }

        metadata.meta_offset = ReadWriteIOUtils::read_i64(&mut cursor)?;

        if (cursor.position() as usize) < bytes.len() {
            let bloom = BloomFilter::deserialize(&mut cursor)?;
            if !bloom.is_empty() {
                metadata.bloom_filter = Some(bloom);
            }
        }

        if (cursor.position() as usize) < bytes.len() {
            let property_count = ReadWriteForEncodingUtils::read_var_int(&mut cursor)? as usize;
            for _ in 0..property_count {
                let key = ReadWriteIOUtils::read_var_int_string(&mut cursor)?;
                let value = ReadWriteIOUtils::read_var_int_string(&mut cursor)?;
                metadata.ts_file_properties.insert(key, value);
            }
        }

        Ok(metadata)
    }

    fn deserialize_legacy_v3(bytes: &[u8]) -> TsFileResult<Self> {
        let mut cursor = std::io::Cursor::new(bytes);
        let mut metadata = TsFileMetadata::new();
        let node = MetadataIndexNode::deserialize(&mut cursor)?;
        if !matches!(
            node.node_type,
            MetadataIndexNodeType::LeafDevice | MetadataIndexNodeType::InternalDevice
        ) {
            return Err(crate::error::TsFileError::InvalidFileFormat(
                "invalid legacy metadata index node type".to_string(),
            ));
        }
        metadata.table_metadata_index_node_map.insert(String::new(), node);
        metadata.meta_offset = ReadWriteIOUtils::read_i64(&mut cursor)?;
        if (cursor.position() as usize) < bytes.len() {
            let bloom = BloomFilter::deserialize(&mut cursor)?;
            if !bloom.is_empty() {
                metadata.bloom_filter = Some(bloom);
            }
        }
        Ok(metadata)
    }
}

impl Default for TsFileMetadata {
    fn default() -> Self {
        TsFileMetadata::new()
    }
}
