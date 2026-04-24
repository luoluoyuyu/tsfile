// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! ChunkWriter: writes a single chunk of time series data.

use std::io::Write;

use crate::common::enums::{CompressionType, TSDataType, TSEncoding};
use crate::compress::{create_compressor, Compressor};
use crate::encoding::encoder::{create_encoder, Encoder};
use crate::error::TsFileResult;
use crate::file::header::PageHeader;
use crate::file::metadata::statistics::Statistics;
use crate::utils::read_write_io_utils::Binary;

/// A page buffer under construction.
struct PageWriter {
    /// The encoded time data.
    time_buffer: Vec<u8>,
    /// The encoded value data.
    value_buffer: Vec<u8>,
    /// Time encoder.
    time_encoder: Box<dyn Encoder>,
    /// Value encoder.
    value_encoder: Box<dyn Encoder>,
    /// Statistics for this page.
    statistics: Statistics,
    /// Number of data points in this page.
    point_count: usize,
    /// Max number of points per page.
    max_points: usize,
}

impl PageWriter {
    pub fn new(data_type: TSDataType, encoding: TSEncoding, max_points: usize) -> Self {
        PageWriter {
            time_buffer: Vec::new(),
            value_buffer: Vec::new(),
            time_encoder: create_encoder(TSDataType::Int64, TSEncoding::Ts2diff),
            value_encoder: create_encoder(data_type, encoding),
            statistics: Statistics::new(data_type),
            point_count: 0,
            max_points,
        }
    }

    pub fn is_full(&self) -> bool {
        self.point_count >= self.max_points
    }

    pub fn point_count(&self) -> usize {
        self.point_count
    }

    pub fn write_bool(&mut self, timestamp: i64, value: bool) -> TsFileResult<()> {
        self.time_encoder
            .encode_i64(timestamp, &mut self.time_buffer)?;
        self.value_encoder
            .encode_bool(value, &mut self.value_buffer)?;
        self.statistics.update_bool(timestamp, value);
        self.point_count += 1;
        Ok(())
    }

    pub fn write_i32(&mut self, timestamp: i64, value: i32) -> TsFileResult<()> {
        self.time_encoder
            .encode_i64(timestamp, &mut self.time_buffer)?;
        self.value_encoder
            .encode_i32(value, &mut self.value_buffer)?;
        self.statistics.update_i32(timestamp, value);
        self.point_count += 1;
        Ok(())
    }

    pub fn write_i64(&mut self, timestamp: i64, value: i64) -> TsFileResult<()> {
        self.time_encoder
            .encode_i64(timestamp, &mut self.time_buffer)?;
        self.value_encoder
            .encode_i64(value, &mut self.value_buffer)?;
        self.statistics.update_i64(timestamp, value);
        self.point_count += 1;
        Ok(())
    }

    pub fn write_f32(&mut self, timestamp: i64, value: f32) -> TsFileResult<()> {
        self.time_encoder
            .encode_i64(timestamp, &mut self.time_buffer)?;
        self.value_encoder
            .encode_f32(value, &mut self.value_buffer)?;
        self.statistics.update_f32(timestamp, value);
        self.point_count += 1;
        Ok(())
    }

    pub fn write_f64(&mut self, timestamp: i64, value: f64) -> TsFileResult<()> {
        self.time_encoder
            .encode_i64(timestamp, &mut self.time_buffer)?;
        self.value_encoder
            .encode_f64(value, &mut self.value_buffer)?;
        self.statistics.update_f64(timestamp, value);
        self.point_count += 1;
        Ok(())
    }

    pub fn write_binary(&mut self, timestamp: i64, value: Binary) -> TsFileResult<()> {
        self.time_encoder
            .encode_i64(timestamp, &mut self.time_buffer)?;
        self.value_encoder
            .encode_binary(&value, &mut self.value_buffer)?;
        self.statistics.update_binary(timestamp, value);
        self.point_count += 1;
        Ok(())
    }

    /// Flush all buffered data into the page byte buffer.
    /// Returns (uncompressed_bytes, statistics).
    pub fn flush_to_page_bytes(&mut self) -> TsFileResult<(Vec<u8>, Statistics)> {
        self.time_encoder.flush(&mut self.time_buffer)?;
        self.value_encoder.flush(&mut self.value_buffer)?;

        let mut page_bytes = Vec::new();
        // Write time data length as u32 big-endian, then time data
        let time_len = self.time_buffer.len() as u32;
        page_bytes.extend_from_slice(&time_len.to_be_bytes());
        page_bytes.extend_from_slice(&self.time_buffer);
        // Write value data
        page_bytes.extend_from_slice(&self.value_buffer);

        let data_type = self.statistics.typed.data_type();
        let stats = std::mem::replace(
            &mut self.statistics,
            Statistics::new(data_type),
        );

        self.time_buffer.clear();
        self.value_buffer.clear();
        self.point_count = 0;
        self.time_encoder.reset();
        self.value_encoder.reset();

        Ok((page_bytes, stats))
    }
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
    /// All compressed pages collected so far (header + data).
    page_buffer: Vec<u8>,
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
            page_buffer: Vec::new(),
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
            || self.page_writer.time_buffer.len() + self.page_writer.value_buffer.len()
                >= self.page_size_threshold
        {
            self.flush_current_page()?;
        }
        Ok(())
    }

    fn flush_current_page(&mut self) -> TsFileResult<()> {
        if self.page_writer.point_count() == 0 {
            return Ok(());
        }
        let (uncompressed_bytes, page_stats) = self.page_writer.flush_to_page_bytes()?;
        let uncompressed_size = uncompressed_bytes.len() as u32;
        let compressed_bytes = self.compressor.compress(&uncompressed_bytes)?;
        let compressed_size = compressed_bytes.len() as u32;

        // Match Java's convention:
        // - If this is the only page in the chunk (num_pages == 0 before this flush),
        //   the page header does NOT include statistics.
        // - If this is a multi-page chunk, page headers include statistics.
        let has_statistics = self.num_pages > 0;
        let page_header = PageHeader::new(
            uncompressed_size,
            compressed_size,
            if has_statistics {
                Some(page_stats.clone())
            } else {
                None
            },
        );
        page_header.serialize(&mut self.page_buffer)?;
        self.page_buffer.extend_from_slice(&compressed_bytes);

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

        let data_size = self.page_buffer.len() as u32;

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
        writer.write_all(&self.page_buffer)?;
        let data_size_usize = self.page_buffer.len();

        // Reset for next chunk
        self.page_buffer.clear();
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
        !self.page_buffer.is_empty() || self.page_writer.point_count() > 0
    }
}
