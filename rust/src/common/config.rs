// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TsFile configuration.

use crate::common::enums::{CompressionType, TSEncoding};

/// TsFile configuration parameters, mirroring Java's TSFileConfig.
#[derive(Debug, Clone)]
pub struct TsFileConfig {
    /// Bloom filter error rate (default: 0.05).
    pub bloom_filter_error_rate: f64,

    /// Number of points per group size check (default: 100).
    pub group_size_in_byte: usize,

    /// Page size in bytes (default: 64KB).
    pub page_size_in_byte: usize,

    /// Max number of points in a page (default: 1_000_000).
    pub max_number_of_points_in_page: usize,

    /// Float precision (default: 2).
    pub float_precision: u32,

    /// Default encoding for Boolean type.
    pub default_boolean_encoding: TSEncoding,

    /// Default encoding for INT32 type.
    pub default_int32_encoding: TSEncoding,

    /// Default encoding for INT64 type.
    pub default_int64_encoding: TSEncoding,

    /// Default encoding for FLOAT type.
    pub default_float_encoding: TSEncoding,

    /// Default encoding for DOUBLE type.
    pub default_double_encoding: TSEncoding,

    /// Default encoding for TEXT type.
    pub default_text_encoding: TSEncoding,

    /// Default compression type.
    pub compressor: CompressionType,

    /// Max metadata size before flushing to disk (default: 100MB).
    pub max_metadata_size: usize,
}

impl Default for TsFileConfig {
    fn default() -> Self {
        TsFileConfig {
            bloom_filter_error_rate: 0.05,
            group_size_in_byte: 128 * 1024 * 1024, // 128MB
            page_size_in_byte: 64 * 1024,           // 64KB
            max_number_of_points_in_page: 1_000_000,
            float_precision: 2,
            default_boolean_encoding: TSEncoding::Rle,
            default_int32_encoding: TSEncoding::Ts2diff,
            default_int64_encoding: TSEncoding::Ts2diff,
            default_float_encoding: TSEncoding::Gorilla,
            default_double_encoding: TSEncoding::Gorilla,
            default_text_encoding: TSEncoding::Plain,
            compressor: CompressionType::Snappy,
            max_metadata_size: 100 * 1024 * 1024, // 100MB
        }
    }
}

impl TsFileConfig {
    /// Create a new TsFileConfig with default values.
    pub fn new() -> Self {
        TsFileConfig::default()
    }
}

// Global TsFileConfig singleton - thread-local for Rust safety.
thread_local! {
    static INSTANCE: TsFileConfig = TsFileConfig::default();
}

/// Get a copy of the default global config.
pub fn get_config() -> TsFileConfig {
    INSTANCE.with(|c| c.clone())
}
