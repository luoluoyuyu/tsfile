// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Java-compatible metadata/enums/CompressionType module.

pub use crate::common::enums::CompressionType;

pub const UNCOMPRESSED: CompressionType = CompressionType::Uncompressed;
pub const SNAPPY: CompressionType = CompressionType::Snappy;
pub const GZIP: CompressionType = CompressionType::Gzip;
pub const LZO: CompressionType = CompressionType::Lzo;
pub const SDT: CompressionType = CompressionType::Sdt;
pub const PAA: CompressionType = CompressionType::Paa;
pub const PLA: CompressionType = CompressionType::Pla;
pub const LZ4: CompressionType = CompressionType::Lz4;
pub const ZSTD: CompressionType = CompressionType::Zstd;
