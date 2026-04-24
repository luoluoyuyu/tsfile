// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TsFile constants.

/// Constant values for TsFile format.
pub struct TsFileConstant;

impl TsFileConstant {
    /// The magic string at the beginning and end of a TsFile.
    pub const MAGIC_STRING: &'static str = "TsFile";

    /// The version number byte of TsFile.
    pub const VERSION_NUMBER: u8 = 3;

    /// Bit mask indicating a time chunk (aligned series).
    pub const TIME_COLUMN_MASK: u8 = 0x80;

    /// Bit mask indicating a value chunk (aligned series).
    pub const VALUE_COLUMN_MASK: u8 = 0x40;

    /// Default page size: 64KB.
    pub const PAGE_SIZE_IN_BYTE: usize = 64 * 1024;

    /// Maximum number of points in a page.
    pub const MAX_NUMBER_OF_POINTS_IN_PAGE: usize = 1_000_000;

    /// Default group size threshold: 128MB.
    pub const GROUP_SIZE_IN_BYTE: usize = 128 * 1024 * 1024;

    /// Default bloom filter error rate.
    pub const BLOOM_FILTER_ERROR_RATE: f64 = 0.05;

    /// TsFile suffix.
    pub const TSFILE_SUFFIX: &'static str = ".tsfile";

    /// Chunk metadata temporary file suffix.
    pub const CHUNK_METADATA_TEMP_FILE_SUFFIX: &'static str = ".meta";
}
