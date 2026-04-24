// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! ChunkWriter: writes a single chunk of time series data.

use std::io::Write;

use crate::common::enums::{CompressionType, TSDataType, TSEncoding};
use crate::compress::{create_compressor, Compressor};
use crate::error::TsFileResult;
use crate::file::header::PageHeader;
use crate::file::metadata::statistics::Statistics;
use crate::write::chunk::page_writer::PageWriter;
use crate::utils::read_write_io_utils::Binary;

struct BufferedPage {
    uncompressed_size: u32,
    compressed_bytes: Vec<u8>,
    statistics: Statistics,
}

/// ChunkWriter: manages writing pages and producing a chunk.
///
/// Mirrors Java's ChunkWriterImpl.
pub struct ChunkWriter {
    /// The measurement ID.
    pub measurement_id: String,
    /// Data type.
    pub data_type: TSDataType,
    /// Encoding.
    pub encoding: TSEncoding,
    /// Compression.
    pub compression: CompressionType,
    /// Compressor.
    compressor: Box<dyn Compressor>,
    /// Current page writer.
    page_writer: PageWriter,
    /// All compressed pages collected so far. Page headers are materialized at chunk flush time
    /// because Java writes page statistics iff the final chunk has more than one page.
    pages: Vec<BufferedPage>,
    /// Number of pages flushed.
    num_pages: u32,
    /// Chunk-level statistics.
    chunk_statistics: Statistics,
    /// Page size threshold in bytes.
    page_size_threshold: usize,
}

impl ChunkWriter {
    pub fn new(
        measurement_id: String,
        data_type: TSDataType,
        encoding: TSEncoding,
        compression: CompressionType,
    ) -> Self {
        let max_points = 1_000_000;
        let page_size_threshold = 64 * 1024;
        ChunkWriter {
            measurement_id: measurement_id.clone(),
            data_type,
            encoding,
            compression,
            compressor: create_compressor(compression),
            page_writer: PageWriter::new(data_type, encoding, max_points),
            pages: Vec::new(),
            num_pages: 0,
            chunk_statistics: Statistics::new(data_type),
            page_size_threshold,
        }
    }

    pub fn write_bool(&mut self, timestamp: i64, value: bool) -> TsFileResult<()> {
        self.page_writer.write_bool(timestamp, value)?;
        self.check_page_size_and_may_flush()?;
        Ok(())
    }

    pub fn write_i32(&mut self, timestamp: i64, value: i32) -> TsFileResult<()> {
        self.page_writer.write_i32(timestamp, value)?;
        self.check_page_size_and_may_flush()?;
        Ok(())
    }

    pub fn write_i64(&mut self, timestamp: i64, value: i64) -> TsFileResult<()> {
        self.page_writer.write_i64(timestamp, value)?;
        self.check_page_size_and_may_flush()?;
        Ok(())
    }

    pub fn write_f32(&mut self, timestamp: i64, value: f32) -> TsFileResult<()> {
        self.page_writer.write_f32(timestamp, value)?;
        self.check_page_size_and_may_flush()?;
        Ok(())
    }

    pub fn write_f64(&mut self, timestamp: i64, value: f64) -> TsFileResult<()> {
        self.page_writer.write_f64(timestamp, value)?;
        self.check_page_size_and_may_flush()?;
        Ok(())
    }

    pub fn write_binary(&mut self, timestamp: i64, value: Binary) -> TsFileResult<()> {
        self.page_writer.write_binary(timestamp, value)?;
        self.check_page_size_and_may_flush()?;
        Ok(())
    }

    fn check_page_size_and_may_flush(&mut self) -> TsFileResult<()> {
        if self.page_writer.is_full()
            || self.page_writer.estimated_size() >= self.page_size_threshold
        {
            self.flush_current_page()?;
        }
        Ok(())
    }

    fn flush_current_page(&mut self) -> TsFileResult<()> {
        if self.page_writer.point_count() == 0 {
            return Ok(());
        }
        let Some(encoded_page) = self.page_writer.flush()? else {
            return Ok(());
        };
        let uncompressed_bytes = encoded_page.data;
        let page_stats = encoded_page.statistics;
        let uncompressed_size = uncompressed_bytes.len() as u32;
        let compressed_bytes = self.compressor.compress(&uncompressed_bytes)?;
        self.pages.push(BufferedPage {
            uncompressed_size,
            compressed_bytes,
            statistics: page_stats.clone(),
        });

        self.chunk_statistics.merge(&page_stats);
        self.num_pages += 1;
        Ok(())
    }

    /// Write the chunk to the output writer.
    /// Returns (chunk_header_size, chunk_data_size).
    pub fn write_to<W: Write>(
        &mut self,
        writer: &mut W,
    ) -> TsFileResult<(usize, usize)> {
        // Flush remaining data in the current page
        self.flush_current_page()?;

        let has_page_statistics = self.num_pages > 1;
        let mut chunk_data = Vec::new();
        for page in &self.pages {
            let page_header = PageHeader::new(
                page.uncompressed_size,
                page.compressed_bytes.len() as u32,
                if has_page_statistics {
                    Some(page.statistics.clone())
                } else {
                    None
                },
            );
            page_header.serialize(&mut chunk_data)?;
            chunk_data.extend_from_slice(&page.compressed_bytes);
        }

        let data_size = chunk_data.len() as u32;

        // Write chunk header
        use crate::file::header::ChunkHeader;
        let header = ChunkHeader::new(
            self.measurement_id.clone(),
            data_size,
            self.data_type,
            self.compression,
            self.encoding,
            self.num_pages,
        );

        let mut header_bytes = Vec::new();
        header.serialize(&mut header_bytes)?;
        let header_size = header_bytes.len();
        writer.write_all(&header_bytes)?;

        // Write chunk data
        writer.write_all(&chunk_data)?;
        let data_size_usize = chunk_data.len();

        // Reset for next chunk
        self.pages.clear();
        self.num_pages = 0;
        self.chunk_statistics = Statistics::new(self.data_type);

        Ok((header_size, data_size_usize))
    }

    /// Get the current chunk statistics.
    pub fn statistics(&self) -> &Statistics {
        &self.chunk_statistics
    }

    /// Get the number of pages.
    pub fn num_pages(&self) -> u32 {
        self.num_pages
    }

    /// Check if there is data to write.
    pub fn has_data(&self) -> bool {
        !self.pages.is_empty() || self.page_writer.point_count() > 0
    }
}
