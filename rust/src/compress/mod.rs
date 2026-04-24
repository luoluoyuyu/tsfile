// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Compression and decompression module.
//!
//! Mirrors Java's ICompressor and IUnCompressor.

use crate::common::enums::CompressionType;
use crate::error::{TsFileError, TsFileResult};

/// Trait for compressors.
pub trait Compressor: Send + Sync {
    /// Compress data.
    fn compress(&self, data: &[u8]) -> TsFileResult<Vec<u8>>;
    /// Get the compression type.
    fn compression_type(&self) -> CompressionType;
    /// Estimate the max compressed size for given input size.
    fn max_compressed_size(&self, input_len: usize) -> usize;
}

/// Trait for decompressors.
pub trait Decompressor: Send + Sync {
    /// Decompress data. `output_len` is the expected uncompressed size.
    fn decompress(&self, data: &[u8], output_len: usize) -> TsFileResult<Vec<u8>>;
    /// Get the compression type.
    fn compression_type(&self) -> CompressionType;
}

/// Create a compressor for the given compression type.
pub fn create_compressor(compression: CompressionType) -> Box<dyn Compressor> {
    match compression {
        CompressionType::Uncompressed => Box::new(UncompressedCompressor),
        CompressionType::Snappy => Box::new(SnappyCompressor),
        CompressionType::Gzip => Box::new(GzipCompressor),
        CompressionType::Lz4 => Box::new(Lz4Compressor),
        CompressionType::Zstd => {
            log::warn!("ZSTD compression not supported, using uncompressed");
            Box::new(UncompressedCompressor)
        }
        CompressionType::Lzma2 => {
            log::warn!("LZMA2 compression not supported, using uncompressed");
            Box::new(UncompressedCompressor)
        }
    }
}

/// Create a decompressor for the given compression type.
pub fn create_decompressor(compression: CompressionType) -> Box<dyn Decompressor> {
    match compression {
        CompressionType::Uncompressed => Box::new(UncompressedDecompressor),
        CompressionType::Snappy => Box::new(SnappyDecompressor),
        CompressionType::Gzip => Box::new(GzipDecompressor),
        CompressionType::Lz4 => Box::new(Lz4Decompressor),
        CompressionType::Zstd => {
            log::warn!("ZSTD decompression not supported, assuming uncompressed");
            Box::new(UncompressedDecompressor)
        }
        CompressionType::Lzma2 => {
            log::warn!("LZMA2 decompression not supported, assuming uncompressed");
            Box::new(UncompressedDecompressor)
        }
    }
}

// =========================================================================
// Uncompressed
// =========================================================================

pub struct UncompressedCompressor;

impl Compressor for UncompressedCompressor {
    fn compress(&self, data: &[u8]) -> TsFileResult<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn compression_type(&self) -> CompressionType {
        CompressionType::Uncompressed
    }

    fn max_compressed_size(&self, input_len: usize) -> usize {
        input_len
    }
}

pub struct UncompressedDecompressor;

impl Decompressor for UncompressedDecompressor {
    fn decompress(&self, data: &[u8], _output_len: usize) -> TsFileResult<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn compression_type(&self) -> CompressionType {
        CompressionType::Uncompressed
    }
}

// =========================================================================
// Snappy
// =========================================================================

pub struct SnappyCompressor;

impl Compressor for SnappyCompressor {
    fn compress(&self, data: &[u8]) -> TsFileResult<Vec<u8>> {
        let mut encoder = snap::raw::Encoder::new();
        encoder
            .compress_vec(data)
            .map_err(|e| TsFileError::CompressionError(format!("Snappy compress: {}", e)))
    }

    fn compression_type(&self) -> CompressionType {
        CompressionType::Snappy
    }

    fn max_compressed_size(&self, input_len: usize) -> usize {
        snap::raw::max_compress_len(input_len)
    }
}

pub struct SnappyDecompressor;

impl Decompressor for SnappyDecompressor {
    fn decompress(&self, data: &[u8], _output_len: usize) -> TsFileResult<Vec<u8>> {
        let mut decoder = snap::raw::Decoder::new();
        decoder
            .decompress_vec(data)
            .map_err(|e| TsFileError::DecompressionError(format!("Snappy decompress: {}", e)))
    }

    fn compression_type(&self) -> CompressionType {
        CompressionType::Snappy
    }
}

// =========================================================================
// GZIP
// =========================================================================

pub struct GzipCompressor;

impl Compressor for GzipCompressor {
    fn compress(&self, data: &[u8]) -> TsFileResult<Vec<u8>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(data)
            .map_err(|e| TsFileError::CompressionError(format!("GZIP write: {}", e)))?;
        encoder
            .finish()
            .map_err(|e| TsFileError::CompressionError(format!("GZIP finish: {}", e)))
    }

    fn compression_type(&self) -> CompressionType {
        CompressionType::Gzip
    }

    fn max_compressed_size(&self, input_len: usize) -> usize {
        input_len + 32 // GZIP overhead estimate
    }
}

pub struct GzipDecompressor;

impl Decompressor for GzipDecompressor {
    fn decompress(&self, data: &[u8], _output_len: usize) -> TsFileResult<Vec<u8>> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(data);
        let mut out = Vec::new();
        decoder
            .read_to_end(&mut out)
            .map_err(|e| TsFileError::DecompressionError(format!("GZIP decompress: {}", e)))?;
        Ok(out)
    }

    fn compression_type(&self) -> CompressionType {
        CompressionType::Gzip
    }
}

// =========================================================================
// LZ4
// =========================================================================

pub struct Lz4Compressor;

impl Compressor for Lz4Compressor {
    fn compress(&self, data: &[u8]) -> TsFileResult<Vec<u8>> {
        // lz4_flex::compress_prepend_size returns Vec<u8> directly (infallible)
        Ok(lz4_flex::compress_prepend_size(data))
    }

    fn compression_type(&self) -> CompressionType {
        CompressionType::Lz4
    }

    fn max_compressed_size(&self, input_len: usize) -> usize {
        lz4_flex::block::get_maximum_output_size(input_len) + 4
    }
}

pub struct Lz4Decompressor;

impl Decompressor for Lz4Decompressor {
    fn decompress(&self, data: &[u8], _output_len: usize) -> TsFileResult<Vec<u8>> {
        lz4_flex::decompress_size_prepended(data)
            .map_err(|e| TsFileError::DecompressionError(format!("LZ4 decompress: {}", e)))
    }

    fn compression_type(&self) -> CompressionType {
        CompressionType::Lz4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip_test(compression: CompressionType, data: &[u8]) {
        let compressor = create_compressor(compression);
        let compressed = compressor.compress(data).unwrap();
        let decompressor = create_decompressor(compression);
        let decompressed = decompressor.decompress(&compressed, data.len()).unwrap();
        assert_eq!(data, decompressed.as_slice());
    }

    #[test]
    fn test_uncompressed_roundtrip() {
        roundtrip_test(
            CompressionType::Uncompressed,
            b"Hello, TsFile Rust!",
        );
    }

    #[test]
    fn test_snappy_roundtrip() {
        roundtrip_test(CompressionType::Snappy, b"Hello, TsFile Rust! snappy test with repeated repeated repeated data.");
    }

    #[test]
    fn test_gzip_roundtrip() {
        roundtrip_test(CompressionType::Gzip, b"Hello, TsFile Rust! gzip test.");
    }

    #[test]
    fn test_lz4_roundtrip() {
        roundtrip_test(CompressionType::Lz4, b"Hello, TsFile Rust! lz4 test data data data data.");
    }
}
