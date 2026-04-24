// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Public page writer abstraction for building encoded TsFile pages.

use crate::common::enums::{TSDataType, TSEncoding};
use crate::encoding::encoder::{create_encoder, EncodableValue, Encoder};
use crate::error::TsFileResult;
use crate::file::metadata::statistics::Statistics;
use crate::utils::read_write_io_utils::Binary;

/// Encoded page payload and statistics.
#[derive(Debug, Clone)]
pub struct EncodedPage {
    pub data: Vec<u8>,
    pub statistics: Statistics,
    pub point_count: usize,
}

/// Encodes time and value columns for one page.
pub struct PageWriter {
    time_buffer: Vec<u8>,
    value_buffer: Vec<u8>,
    time_encoder: Box<dyn Encoder>,
    value_encoder: Box<dyn Encoder>,
    statistics: Statistics,
    point_count: usize,
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

    pub fn point_count(&self) -> usize {
        self.point_count
    }

    pub fn estimated_size(&self) -> usize {
        self.time_buffer.len() + self.value_buffer.len()
    }

    pub fn is_full(&self) -> bool {
        self.point_count >= self.max_points
    }

    pub fn write_bool(&mut self, timestamp: i64, value: bool) -> TsFileResult<()> {
        self.encode_time(timestamp)?;
        self.value_encoder
            .encode_value(EncodableValue::Boolean(value), &mut self.value_buffer)?;
        self.statistics.update_bool(timestamp, value);
        self.point_count += 1;
        Ok(())
    }

    pub fn write_i32(&mut self, timestamp: i64, value: i32) -> TsFileResult<()> {
        self.encode_time(timestamp)?;
        self.value_encoder
            .encode_value(EncodableValue::Int32(value), &mut self.value_buffer)?;
        self.statistics.update_i32(timestamp, value);
        self.point_count += 1;
        Ok(())
    }

    pub fn write_i64(&mut self, timestamp: i64, value: i64) -> TsFileResult<()> {
        self.encode_time(timestamp)?;
        self.value_encoder
            .encode_value(EncodableValue::Int64(value), &mut self.value_buffer)?;
        self.statistics.update_i64(timestamp, value);
        self.point_count += 1;
        Ok(())
    }

    pub fn write_f32(&mut self, timestamp: i64, value: f32) -> TsFileResult<()> {
        self.encode_time(timestamp)?;
        self.value_encoder
            .encode_value(EncodableValue::Float(value), &mut self.value_buffer)?;
        self.statistics.update_f32(timestamp, value);
        self.point_count += 1;
        Ok(())
    }

    pub fn write_f64(&mut self, timestamp: i64, value: f64) -> TsFileResult<()> {
        self.encode_time(timestamp)?;
        self.value_encoder
            .encode_value(EncodableValue::Double(value), &mut self.value_buffer)?;
        self.statistics.update_f64(timestamp, value);
        self.point_count += 1;
        Ok(())
    }

    pub fn write_binary(&mut self, timestamp: i64, value: Binary) -> TsFileResult<()> {
        self.encode_time(timestamp)?;
        self.value_encoder
            .encode_value(EncodableValue::Binary(value.clone()), &mut self.value_buffer)?;
        self.statistics.update_binary(timestamp, value);
        self.point_count += 1;
        Ok(())
    }

    fn encode_time(&mut self, timestamp: i64) -> TsFileResult<()> {
        self.time_encoder
            .encode_value(EncodableValue::Int64(timestamp), &mut self.time_buffer)
    }

    pub fn flush(&mut self) -> TsFileResult<Option<EncodedPage>> {
        if self.point_count == 0 {
            return Ok(None);
        }
        self.time_encoder.flush(&mut self.time_buffer)?;
        self.value_encoder.flush(&mut self.value_buffer)?;

        let mut data = Vec::new();
        data.extend_from_slice(&(self.time_buffer.len() as i32).to_be_bytes());
        data.extend_from_slice(&self.time_buffer);
        data.extend_from_slice(&self.value_buffer);

        let data_type = self.statistics.typed.data_type();
        let statistics = std::mem::replace(&mut self.statistics, Statistics::new(data_type));
        let point_count = self.point_count;
        self.time_buffer.clear();
        self.value_buffer.clear();
        self.point_count = 0;
        self.time_encoder.reset();
        self.value_encoder.reset();

        Ok(Some(EncodedPage {
            data,
            statistics,
            point_count,
        }))
    }
}
