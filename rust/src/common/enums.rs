// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Core enum types: TSDataType, CompressionType, TSEncoding.

use crate::error::{TsFileError, TsFileResult};

/// Time series data types, mirroring Java's TSDataType.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TSDataType {
    Boolean = 0,
    Int32 = 1,
    Int64 = 2,
    Float = 3,
    Double = 4,
    Text = 5,
    /// Vector (aligned time series time column).
    Vector = 6,
    NullType = 7,
    /// Unix timestamp (i64), treated like INT64.
    Timestamp = 8,
    /// Date stored as i32 (days since epoch).
    Date = 9,
    /// Blob (raw binary).
    Blob = 10,
    /// String (UTF-8 text with statistics).
    String = 11,
}

impl TSDataType {
    /// Serialized size in bytes (always 1).
    pub const fn serialized_size() -> usize {
        1
    }

    /// Deserialize from byte.
    pub fn deserialize(byte: u8) -> TsFileResult<Self> {
        match byte {
            0 => Ok(TSDataType::Boolean),
            1 => Ok(TSDataType::Int32),
            2 => Ok(TSDataType::Int64),
            3 => Ok(TSDataType::Float),
            4 => Ok(TSDataType::Double),
            5 => Ok(TSDataType::Text),
            6 => Ok(TSDataType::Vector),
            7 => Ok(TSDataType::NullType),
            8 => Ok(TSDataType::Timestamp),
            9 => Ok(TSDataType::Date),
            10 => Ok(TSDataType::Blob),
            11 => Ok(TSDataType::String),
            _ => Err(TsFileError::InvalidDataType(format!(
                "Unknown data type byte: {}",
                byte
            ))),
        }
    }

    /// Serialize to byte.
    pub fn serialize(self) -> u8 {
        self as u8
    }

    /// Check if another type is compatible (can be merged) with this type.
    pub fn is_compatible(&self, other: &TSDataType) -> bool {
        if self == other {
            return true;
        }
        // INT32 <-> DATE, INT64 <-> TIMESTAMP are compatible
        matches!(
            (self, other),
            (TSDataType::Int32, TSDataType::Date)
                | (TSDataType::Date, TSDataType::Int32)
                | (TSDataType::Int64, TSDataType::Timestamp)
                | (TSDataType::Timestamp, TSDataType::Int64)
                | (TSDataType::Text, TSDataType::Blob)
                | (TSDataType::Blob, TSDataType::Text)
        )
    }
}

impl std::fmt::Display for TSDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TSDataType::Boolean => write!(f, "BOOLEAN"),
            TSDataType::Int32 => write!(f, "INT32"),
            TSDataType::Int64 => write!(f, "INT64"),
            TSDataType::Float => write!(f, "FLOAT"),
            TSDataType::Double => write!(f, "DOUBLE"),
            TSDataType::Text => write!(f, "TEXT"),
            TSDataType::Vector => write!(f, "VECTOR"),
            TSDataType::NullType => write!(f, "NULL"),
            TSDataType::Timestamp => write!(f, "TIMESTAMP"),
            TSDataType::Date => write!(f, "DATE"),
            TSDataType::Blob => write!(f, "BLOB"),
            TSDataType::String => write!(f, "STRING"),
        }
    }
}

/// Compression types supported by TsFile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompressionType {
    /// No compression.
    Uncompressed,
    /// Snappy compression.
    Snappy,
    /// GZIP compression.
    Gzip,
    /// LZO compression.
    Lzo,
    /// SDT compression.
    Sdt,
    /// PAA compression.
    Paa,
    /// PLA compression.
    Pla,
    /// LZ4 compression.
    Lz4,
    /// Zstandard compression.
    Zstd,
    /// LZMA2 compression.
    Lzma2,
}

impl CompressionType {
    /// Serialized size in bytes.
    pub const fn serialized_size() -> usize {
        1
    }

    /// Deserialize from byte.
    pub fn deserialize(byte: u8) -> TsFileResult<Self> {
        match byte {
            0 => Ok(CompressionType::Uncompressed),
            1 => Ok(CompressionType::Snappy),
            2 => Ok(CompressionType::Gzip),
            3 => Ok(CompressionType::Lzo),
            4 => Ok(CompressionType::Sdt),
            5 => Ok(CompressionType::Paa),
            6 => Ok(CompressionType::Pla),
            7 => Ok(CompressionType::Lz4),
            8 => Ok(CompressionType::Zstd),
            9 => Ok(CompressionType::Lzma2),
            _ => Err(TsFileError::InvalidCompressionType(byte)),
        }
    }

    /// Serialize to byte.
    pub fn serialize(self) -> u8 {
        match self {
            CompressionType::Uncompressed => 0,
            CompressionType::Snappy => 1,
            CompressionType::Gzip => 2,
            CompressionType::Lzo => 3,
            CompressionType::Sdt => 4,
            CompressionType::Paa => 5,
            CompressionType::Pla => 6,
            CompressionType::Lz4 => 7,
            CompressionType::Zstd => 8,
            CompressionType::Lzma2 => 9,
        }
    }

    /// Get the file extension for this compression type.
    pub fn extension(&self) -> &'static str {
        match self {
            CompressionType::Uncompressed => "",
            CompressionType::Snappy => ".snappy",
            CompressionType::Gzip => ".gzip",
            CompressionType::Lzo => ".lzo",
            CompressionType::Sdt => ".sdt",
            CompressionType::Paa => ".paa",
            CompressionType::Pla => ".pla",
            CompressionType::Lz4 => ".lz4",
            CompressionType::Zstd => ".zstd",
            CompressionType::Lzma2 => ".lzma2",
        }
    }
}

impl std::fmt::Display for CompressionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionType::Uncompressed => write!(f, "UNCOMPRESSED"),
            CompressionType::Snappy => write!(f, "SNAPPY"),
            CompressionType::Gzip => write!(f, "GZIP"),
            CompressionType::Lzo => write!(f, "LZO"),
            CompressionType::Sdt => write!(f, "SDT"),
            CompressionType::Paa => write!(f, "PAA"),
            CompressionType::Pla => write!(f, "PLA"),
            CompressionType::Lz4 => write!(f, "LZ4"),
            CompressionType::Zstd => write!(f, "ZSTD"),
            CompressionType::Lzma2 => write!(f, "LZMA2"),
        }
    }
}

/// Encoding types for time series data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TSEncoding {
    Plain,
    Dictionary,
    Rle,
    Diff,
    Ts2diff,
    Bitmap,
    GorillaV1,
    Regular,
    Gorilla,
    Zigzag,
    Chimp,
    Sprintz,
    Rlbe,
    Camel,
}

impl TSEncoding {
    /// Serialized size in bytes.
    pub const fn serialized_size() -> usize {
        1
    }

    /// Deserialize from byte.
    pub fn deserialize(byte: u8) -> TsFileResult<Self> {
        match byte {
            0 => Ok(TSEncoding::Plain),
            1 => Ok(TSEncoding::Dictionary),
            2 => Ok(TSEncoding::Rle),
            3 => Ok(TSEncoding::Diff),
            4 => Ok(TSEncoding::Ts2diff),
            5 => Ok(TSEncoding::Bitmap),
            6 => Ok(TSEncoding::GorillaV1),
            7 => Ok(TSEncoding::Regular),
            8 => Ok(TSEncoding::Gorilla),
            9 => Ok(TSEncoding::Zigzag),
            11 => Ok(TSEncoding::Chimp),
            12 => Ok(TSEncoding::Sprintz),
            13 => Ok(TSEncoding::Rlbe),
            14 => Ok(TSEncoding::Camel),
            _ => Err(TsFileError::InvalidEncodingType(byte)),
        }
    }

    /// Serialize to byte.
    pub fn serialize(self) -> u8 {
        match self {
            TSEncoding::Plain => 0,
            TSEncoding::Dictionary => 1,
            TSEncoding::Rle => 2,
            TSEncoding::Diff => 3,
            TSEncoding::Ts2diff => 4,
            TSEncoding::Bitmap => 5,
            TSEncoding::GorillaV1 => 6,
            TSEncoding::Regular => 7,
            TSEncoding::Gorilla => 8,
            TSEncoding::Zigzag => 9,
            TSEncoding::Chimp => 11,
            TSEncoding::Sprintz => 12,
            TSEncoding::Rlbe => 13,
            TSEncoding::Camel => 14,
        }
    }

    /// Check if this encoding is supported for the given data type.
    pub fn is_supported_for(&self, data_type: &TSDataType) -> bool {
        match data_type {
            TSDataType::Boolean => matches!(self, TSEncoding::Plain | TSEncoding::Rle),
            TSDataType::Int32
            | TSDataType::Int64
            | TSDataType::Timestamp
            | TSDataType::Date => matches!(
                self,
                TSEncoding::Plain
                    | TSEncoding::Rle
                    | TSEncoding::Ts2diff
                    | TSEncoding::Gorilla
                    | TSEncoding::Zigzag
            ),
            TSDataType::Float => matches!(
                self,
                TSEncoding::Plain
                    | TSEncoding::Rle
                    | TSEncoding::Ts2diff
                    | TSEncoding::GorillaV1
                    | TSEncoding::Gorilla
            ),
            TSDataType::Double => matches!(
                self,
                TSEncoding::Plain
                    | TSEncoding::Rle
                    | TSEncoding::Ts2diff
                    | TSEncoding::GorillaV1
                    | TSEncoding::Gorilla
            ),
            TSDataType::Text | TSDataType::String => {
                matches!(self, TSEncoding::Plain | TSEncoding::Dictionary)
            }
            TSDataType::Blob => matches!(self, TSEncoding::Plain),
            _ => false,
        }
    }
}

impl std::fmt::Display for TSEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TSEncoding::Plain => write!(f, "PLAIN"),
            TSEncoding::Dictionary => write!(f, "DICTIONARY"),
            TSEncoding::Rle => write!(f, "RLE"),
            TSEncoding::Diff => write!(f, "DIFF"),
            TSEncoding::Ts2diff => write!(f, "TS_2DIFF"),
            TSEncoding::Bitmap => write!(f, "BITMAP"),
            TSEncoding::GorillaV1 => write!(f, "GORILLA_V1"),
            TSEncoding::Regular => write!(f, "REGULAR"),
            TSEncoding::Gorilla => write!(f, "GORILLA"),
            TSEncoding::Zigzag => write!(f, "ZIGZAG"),
            TSEncoding::Chimp => write!(f, "CHIMP"),
            TSEncoding::Sprintz => write!(f, "SPRINTZ"),
            TSEncoding::Rlbe => write!(f, "RLBE"),
            TSEncoding::Camel => write!(f, "CAMEL"),
        }
    }
}

/// MetadataIndexNodeType for metadata index tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataIndexNodeType {
    InternalDevice,
    LeafDevice,
    InternalMeasurement,
    LeafMeasurement,
}

impl MetadataIndexNodeType {
    pub fn deserialize(byte: u8) -> TsFileResult<Self> {
        match byte {
            0 => Ok(MetadataIndexNodeType::InternalDevice),
            1 => Ok(MetadataIndexNodeType::LeafDevice),
            2 => Ok(MetadataIndexNodeType::InternalMeasurement),
            3 => Ok(MetadataIndexNodeType::LeafMeasurement),
            _ => Err(TsFileError::InvalidFileFormat(format!(
                "Unknown MetadataIndexNodeType: {}",
                byte
            ))),
        }
    }

    pub fn serialize(self) -> u8 {
        match self {
            MetadataIndexNodeType::InternalDevice => 0,
            MetadataIndexNodeType::LeafDevice => 1,
            MetadataIndexNodeType::InternalMeasurement => 2,
            MetadataIndexNodeType::LeafMeasurement => 3,
        }
    }
}
