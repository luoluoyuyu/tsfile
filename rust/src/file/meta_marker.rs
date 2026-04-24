// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! MetaMarker - denotes the type of headers and footers in TsFile.
//!
//! Mirrors Java's MetaMarker.

use crate::common::constant::TsFileConstant;
use crate::error::{TsFileError, TsFileResult};

/// Marker bytes for TsFile format.
pub struct MetaMarker;

impl MetaMarker {
    /// Marker for chunk group header.
    pub const CHUNK_GROUP_HEADER: u8 = 0;

    /// Chunk header: chunk has more than one page.
    pub const CHUNK_HEADER: u8 = 1;

    /// Separator marker (end of all chunk groups).
    pub const SEPARATOR: u8 = 2;

    /// Operation index range marker.
    pub const OPERATION_INDEX_RANGE: u8 = 4;

    /// Chunk header: chunk has only one page.
    pub const ONLY_ONE_PAGE_CHUNK_HEADER: u8 = 5;

    /// Time chunk header: more than one page.
    pub const TIME_CHUNK_HEADER: u8 =
        Self::CHUNK_HEADER | TsFileConstant::TIME_COLUMN_MASK;

    /// Value chunk header: more than one page.
    pub const VALUE_CHUNK_HEADER: u8 =
        Self::CHUNK_HEADER | TsFileConstant::VALUE_COLUMN_MASK;

    /// Time chunk header: only one page.
    pub const ONLY_ONE_PAGE_TIME_CHUNK_HEADER: u8 =
        Self::ONLY_ONE_PAGE_CHUNK_HEADER | TsFileConstant::TIME_COLUMN_MASK;

    /// Value chunk header: only one page.
    pub const ONLY_ONE_PAGE_VALUE_CHUNK_HEADER: u8 =
        Self::ONLY_ONE_PAGE_CHUNK_HEADER | TsFileConstant::VALUE_COLUMN_MASK;

    /// Handle unexpected marker bytes.
    pub fn handle_unexpected_marker(marker: u8) -> TsFileResult<()> {
        Err(TsFileError::InvalidMarker(marker))
    }
}
