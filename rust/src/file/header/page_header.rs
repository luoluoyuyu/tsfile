// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! PageHeader - metadata header for a data page.

use std::io::{Read, Write};

use crate::common::enums::TSDataType;
use crate::error::TsFileResult;
use crate::file::metadata::statistics::Statistics;
use crate::utils::ReadWriteForEncodingUtils;

/// The header for a data page.
///
/// Format:
///   uncompressedSize(uvarint) + compressedSize(uvarint) + [statistics]
///
/// Mirrors Java's PageHeader.
#[derive(Debug, Clone)]
pub struct PageHeader {
    /// Uncompressed page data size.
    pub uncompressed_size: u32,
    /// Compressed page data size.
    pub compressed_size: u32,
    /// Statistics for this page (may be None for single-page chunks).
    pub statistics: Option<Statistics>,
    /// Whether this page has been marked as modified/deleted.
    pub modified: bool,
}

impl PageHeader {
    pub fn new(
        uncompressed_size: u32,
        compressed_size: u32,
        statistics: Option<Statistics>,
    ) -> Self {
        PageHeader {
            uncompressed_size,
            compressed_size,
            statistics,
            modified: false,
        }
    }

    /// Create an empty page header.
    pub fn empty() -> Self {
        PageHeader {
            uncompressed_size: 0,
            compressed_size: 0,
            statistics: None,
            modified: false,
        }
    }

    /// Maximum page header size without statistics (2 * 5 bytes for 2 uvartints).
    pub fn estimate_max_page_header_size_without_statistics() -> usize {
        2 * (4 + 1)
    }

    /// Get the serialized size of this page header.
    pub fn serialized_size(&self) -> usize {
        if self.uncompressed_size == 0 {
            return ReadWriteForEncodingUtils::u_var_int_size(0);
        }
        ReadWriteForEncodingUtils::u_var_int_size(self.uncompressed_size)
            + ReadWriteForEncodingUtils::u_var_int_size(self.compressed_size)
            + self
                .statistics
                .as_ref()
                .map_or(0, |s| s.serialized_size())
    }

    /// Total size of this page including data.
    pub fn page_size(&self) -> usize {
        if self.uncompressed_size == 0 {
            return ReadWriteForEncodingUtils::u_var_int_size(0);
        }
        ReadWriteForEncodingUtils::u_var_int_size(self.uncompressed_size)
            + ReadWriteForEncodingUtils::u_var_int_size(self.compressed_size)
            + self
                .statistics
                .as_ref()
                .map_or(0, |s| s.serialized_size())
            + self.compressed_size as usize
    }

    /// Serialize to writer.
    pub fn serialize<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        let mut written = 0;
        written +=
            ReadWriteForEncodingUtils::write_unsigned_var_int(self.uncompressed_size, writer)?;
        if self.uncompressed_size == 0 {
            return Ok(written);
        }
        written +=
            ReadWriteForEncodingUtils::write_unsigned_var_int(self.compressed_size, writer)?;
        if let Some(stats) = &self.statistics {
            written += stats.serialize(writer)?;
        }
        Ok(written)
    }

    /// Deserialize from reader. `has_statistics` determines if statistics follow.
    pub fn deserialize<R: Read>(
        reader: &mut R,
        data_type: TSDataType,
        has_statistics: bool,
    ) -> TsFileResult<Self> {
        let uncompressed_size = ReadWriteForEncodingUtils::read_unsigned_var_int(reader)?;
        if uncompressed_size == 0 {
            return Ok(PageHeader::empty());
        }
        let compressed_size = ReadWriteForEncodingUtils::read_unsigned_var_int(reader)?;
        let statistics = if has_statistics {
            Some(Statistics::deserialize(reader, data_type)?)
        } else {
            None
        };
        Ok(PageHeader {
            uncompressed_size,
            compressed_size,
            statistics,
            modified: false,
        })
    }

    pub fn start_time(&self) -> Option<i64> {
        self.statistics.as_ref().map(|s| s.start_time)
    }

    pub fn end_time(&self) -> Option<i64> {
        self.statistics.as_ref().map(|s| s.end_time)
    }

    pub fn num_of_values(&self) -> Option<u32> {
        self.statistics.as_ref().map(|s| s.count)
    }
}

impl std::fmt::Display for PageHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PageHeader{{uncompressedSize={}, compressedSize={}, statistics={:?}}}",
            self.uncompressed_size, self.compressed_size, self.statistics
        )
    }
}
