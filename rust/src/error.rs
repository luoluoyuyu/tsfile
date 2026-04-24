// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.

//! Error types for TsFile operations.

use thiserror::Error;

/// The main error type for TsFile operations.
#[derive(Debug, Error)]
pub enum TsFileError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid data type: {0}")]
    InvalidDataType(String),

    #[error("Invalid compression type: {0}")]
    InvalidCompressionType(u8),

    #[error("Invalid encoding type: {0}")]
    InvalidEncodingType(u8),

    #[error("Invalid marker byte: {0}")]
    InvalidMarker(u8),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Decoding error: {0}")]
    DecodingError(String),

    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Decompression error: {0}")]
    DecompressionError(String),

    #[error("Write error: {0}")]
    WriteError(String),

    #[error("Read error: {0}")]
    ReadError(String),

    #[error("Schema error: {0}")]
    SchemaError(String),

    #[error("Statistics error: {0}")]
    StatisticsError(String),

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Invalid magic string")]
    InvalidMagicString,

    #[error("Invalid file format: {0}")]
    InvalidFileFormat(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Measurement not found: {0}")]
    MeasurementNotFound(String),

    #[error("Type mismatch: expected {expected}, got {got}")]
    TypeMismatch { expected: String, got: String },

    #[error("Data out of order: timestamp {timestamp} is not greater than last timestamp {last_timestamp}")]
    OutOfOrderData {
        timestamp: i64,
        last_timestamp: i64,
    },

    #[error("Bloom filter error: {0}")]
    BloomFilterError(String),
}

/// A convenient Result type using TsFileError.
pub type TsFileResult<T> = Result<T, TsFileError>;
