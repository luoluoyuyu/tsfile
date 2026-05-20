// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Encoder trait and implementations.
//!
//! Mirrors Java's IEncoder hierarchy.

use std::io::Write;

use crate::common::enums::{TSDataType, TSEncoding};
use crate::error::{TsFileError, TsFileResult};
use crate::utils::read_write_io_utils::Binary;

#[derive(Debug, Clone, PartialEq)]
pub enum EncodableValue {
    Boolean(bool),
    Int32(i32),
    Int64(i64),
    Float(f32),
    Double(f64),
    Binary(Binary),
}

fn write_unsigned_var_long(mut value: u64, writer: &mut dyn Write) -> TsFileResult<()> {
    while (value & !0x7F) != 0 {
        writer.write_all(&[((value & 0x7F) | 0x80) as u8])?;
        value >>= 7;
    }
    writer.write_all(&[(value & 0x7F) as u8])?;
    Ok(())
}

fn zigzag_i64(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}

/// Trait for encoding time series values into bytes.
pub trait Encoder: Send + Sync {
    /// Encode a boolean value.
    fn encode_bool(&mut self, _value: bool, _writer: &mut dyn Write) -> TsFileResult<()> {
        Err(TsFileError::UnsupportedOperation(
            "encode_bool not supported for this encoder".to_string(),
        ))
    }

    /// Encode an i32 value.
    fn encode_i32(&mut self, _value: i32, _writer: &mut dyn Write) -> TsFileResult<()> {
        Err(TsFileError::UnsupportedOperation(
            "encode_i32 not supported for this encoder".to_string(),
        ))
    }

    /// Encode an i64 value.
    fn encode_i64(&mut self, _value: i64, _writer: &mut dyn Write) -> TsFileResult<()> {
        Err(TsFileError::UnsupportedOperation(
            "encode_i64 not supported for this encoder".to_string(),
        ))
    }

    /// Encode a f32 value.
    fn encode_f32(&mut self, _value: f32, _writer: &mut dyn Write) -> TsFileResult<()> {
        Err(TsFileError::UnsupportedOperation(
            "encode_f32 not supported for this encoder".to_string(),
        ))
    }

    /// Encode a f64 value.
    fn encode_f64(&mut self, _value: f64, _writer: &mut dyn Write) -> TsFileResult<()> {
        Err(TsFileError::UnsupportedOperation(
            "encode_f64 not supported for this encoder".to_string(),
        ))
    }

    /// Encode a binary value.
    fn encode_binary(&mut self, _value: &Binary, _writer: &mut dyn Write) -> TsFileResult<()> {
        Err(TsFileError::UnsupportedOperation(
            "encode_binary not supported for this encoder".to_string(),
        ))
    }

    /// Flush any remaining buffered data to writer.
    fn flush(&mut self, writer: &mut dyn Write) -> TsFileResult<()>;

    /// Encode a dynamically typed value.
    fn encode_value(&mut self, value: EncodableValue, writer: &mut dyn Write) -> TsFileResult<()> {
        match value {
            EncodableValue::Boolean(value) => self.encode_bool(value, writer),
            EncodableValue::Int32(value) => self.encode_i32(value, writer),
            EncodableValue::Int64(value) => self.encode_i64(value, writer),
            EncodableValue::Float(value) => self.encode_f32(value, writer),
            EncodableValue::Double(value) => self.encode_f64(value, writer),
            EncodableValue::Binary(value) => self.encode_binary(&value, writer),
        }
    }

    /// Reset the encoder state.
    fn reset(&mut self);

    /// Get the encoding type.
    fn encoding(&self) -> TSEncoding;

    /// Estimate the current size of buffered data.
    fn estimated_size(&self) -> usize {
        0
    }
}

pub struct UnsupportedEncoder {
    data_type: TSDataType,
    encoding: TSEncoding,
}

impl UnsupportedEncoder {
    pub fn new(data_type: TSDataType, encoding: TSEncoding) -> Self {
        UnsupportedEncoder { data_type, encoding }
    }
}

impl Encoder for UnsupportedEncoder {
    fn flush(&mut self, _writer: &mut dyn Write) -> TsFileResult<()> {
        Err(TsFileError::UnsupportedOperation(format!(
            "Encoding {} is not implemented for data type {}",
            self.encoding, self.data_type
        )))
    }

    fn reset(&mut self) {}

    fn encoding(&self) -> TSEncoding {
        self.encoding
    }
}

/// Create an encoder for the given data type and encoding.
pub fn create_encoder(data_type: TSDataType, encoding: TSEncoding) -> Box<dyn Encoder> {
    match encoding {
        TSEncoding::Plain => Box::new(PlainEncoder::new(data_type)),
        TSEncoding::Dictionary => Box::new(DictionaryEncoder::new()),
        TSEncoding::Rle => Box::new(RleEncoder::new(data_type)),
        TSEncoding::Zigzag => Box::new(ZigzagEncoder::new(data_type)),
        TSEncoding::Ts2diff => Box::new(Ts2diffEncoder::new(data_type)),
        TSEncoding::Gorilla | TSEncoding::GorillaV1 => Box::new(GorillaEncoder::new(data_type)),
        _ => Box::new(UnsupportedEncoder::new(data_type, encoding)),
    }
}

// =========================================================================
// Dictionary Encoder
// =========================================================================

/// Dictionary encoder for binary/text values.
///
/// Format used by this Rust implementation: dictionary length, dictionary entries,
/// id stream length, then id stream encoded with unsigned varints.
pub struct DictionaryEncoder {
    dictionary: Vec<Binary>,
    ids: Vec<u32>,
}

impl DictionaryEncoder {
    pub fn new() -> Self {
        DictionaryEncoder {
            dictionary: Vec::new(),
            ids: Vec::new(),
        }
    }
}

impl Encoder for DictionaryEncoder {
    fn encode_binary(&mut self, value: &Binary, _writer: &mut dyn Write) -> TsFileResult<()> {
        let id = match self.dictionary.iter().position(|item| item == value) {
            Some(index) => index as u32,
            None => {
                self.dictionary.push(value.clone());
                (self.dictionary.len() - 1) as u32
            }
        };
        self.ids.push(id);
        Ok(())
    }

    fn flush(&mut self, writer: &mut dyn Write) -> TsFileResult<()> {
        write_unsigned_var_long(self.dictionary.len() as u64, writer)?;
        for value in &self.dictionary {
            write_unsigned_var_long(value.len() as u64, writer)?;
            writer.write_all(value.as_bytes())?;
        }
        write_unsigned_var_long(self.ids.len() as u64, writer)?;
        for id in &self.ids {
            write_unsigned_var_long(*id as u64, writer)?;
        }
        self.reset();
        Ok(())
    }

    fn reset(&mut self) {
        self.dictionary.clear();
        self.ids.clear();
    }

    fn encoding(&self) -> TSEncoding {
        TSEncoding::Dictionary
    }

    fn estimated_size(&self) -> usize {
        self.dictionary.iter().map(Binary::len).sum::<usize>() + self.ids.len() * 2
    }
}

// =========================================================================
// Zigzag Encoder
// =========================================================================

/// Zigzag varint encoder for signed integer values.
pub struct ZigzagEncoder {
    _data_type: TSDataType,
    buffer: Vec<u8>,
}

impl ZigzagEncoder {
    pub fn new(data_type: TSDataType) -> Self {
        ZigzagEncoder {
            _data_type: data_type,
            buffer: Vec::new(),
        }
    }

    fn encode_value(&mut self, value: i64) -> TsFileResult<()> {
        write_unsigned_var_long(zigzag_i64(value), &mut self.buffer)
    }
}

impl Encoder for ZigzagEncoder {
    fn encode_i32(&mut self, value: i32, _writer: &mut dyn Write) -> TsFileResult<()> {
        self.encode_value(value as i64)
    }

    fn encode_i64(&mut self, value: i64, _writer: &mut dyn Write) -> TsFileResult<()> {
        self.encode_value(value)
    }

    fn flush(&mut self, writer: &mut dyn Write) -> TsFileResult<()> {
        writer.write_all(&self.buffer)?;
        self.buffer.clear();
        Ok(())
    }

    fn reset(&mut self) {
        self.buffer.clear();
    }

    fn encoding(&self) -> TSEncoding {
        TSEncoding::Zigzag
    }

    fn estimated_size(&self) -> usize {
        self.buffer.len()
    }
}

// =========================================================================
// Plain Encoder - stores values as-is in big-endian
// =========================================================================

/// Plain encoder: stores values without any compression encoding.
pub struct PlainEncoder {
    _data_type: TSDataType,
}

impl PlainEncoder {
    pub fn new(data_type: TSDataType) -> Self {
        PlainEncoder { _data_type: data_type }
    }
}

impl Encoder for PlainEncoder {
    fn encode_bool(&mut self, value: bool, writer: &mut dyn Write) -> TsFileResult<()> {
        writer.write_all(&[if value { 1u8 } else { 0u8 }])?;
        Ok(())
    }

    fn encode_i32(&mut self, value: i32, writer: &mut dyn Write) -> TsFileResult<()> {
        writer.write_all(&value.to_be_bytes())?;
        Ok(())
    }

    fn encode_i64(&mut self, value: i64, writer: &mut dyn Write) -> TsFileResult<()> {
        writer.write_all(&value.to_be_bytes())?;
        Ok(())
    }

    fn encode_f32(&mut self, value: f32, writer: &mut dyn Write) -> TsFileResult<()> {
        writer.write_all(&value.to_bits().to_be_bytes())?;
        Ok(())
    }

    fn encode_f64(&mut self, value: f64, writer: &mut dyn Write) -> TsFileResult<()> {
        writer.write_all(&value.to_bits().to_be_bytes())?;
        Ok(())
    }

    fn encode_binary(&mut self, value: &Binary, writer: &mut dyn Write) -> TsFileResult<()> {
        let len = value.len() as i32;
        writer.write_all(&len.to_be_bytes())?;
        writer.write_all(value.as_bytes())?;
        Ok(())
    }

    fn flush(&mut self, _writer: &mut dyn Write) -> TsFileResult<()> {
        Ok(())
    }

    fn reset(&mut self) {}

    fn encoding(&self) -> TSEncoding {
        TSEncoding::Plain
    }
}

// =========================================================================
// RLE Encoder
// =========================================================================

/// Run-Length Encoding encoder.
pub struct RleEncoder {
    _data_type: TSDataType,
    buffer: Vec<u8>,
    /// Current run value and length.
    current_value: Option<i64>,
    run_length: u32,
}

impl RleEncoder {
    pub fn new(data_type: TSDataType) -> Self {
        RleEncoder {
            _data_type: data_type,
            buffer: Vec::new(),
            current_value: None,
            run_length: 0,
        }
    }

    fn flush_run(&mut self) {
        if let Some(value) = self.current_value {
            // Simple encoding: run_length(u32) + value(i64)
            self.buffer
                .extend_from_slice(&self.run_length.to_be_bytes());
            self.buffer.extend_from_slice(&value.to_be_bytes());
        }
        self.current_value = None;
        self.run_length = 0;
    }

    fn encode_value(&mut self, value: i64) {
        match self.current_value {
            Some(v) if v == value => {
                self.run_length += 1;
            }
            _ => {
                self.flush_run();
                self.current_value = Some(value);
                self.run_length = 1;
            }
        }
    }
}

impl Encoder for RleEncoder {
    fn encode_bool(&mut self, value: bool, _writer: &mut dyn Write) -> TsFileResult<()> {
        self.encode_value(if value { 1 } else { 0 });
        Ok(())
    }

    fn encode_i32(&mut self, value: i32, _writer: &mut dyn Write) -> TsFileResult<()> {
        self.encode_value(value as i64);
        Ok(())
    }

    fn encode_i64(&mut self, value: i64, _writer: &mut dyn Write) -> TsFileResult<()> {
        self.encode_value(value);
        Ok(())
    }

    fn encode_f32(&mut self, value: f32, _writer: &mut dyn Write) -> TsFileResult<()> {
        self.encode_value(value.to_bits() as i64);
        Ok(())
    }

    fn encode_f64(&mut self, value: f64, _writer: &mut dyn Write) -> TsFileResult<()> {
        self.encode_value(value.to_bits() as i64);
        Ok(())
    }

    fn flush(&mut self, writer: &mut dyn Write) -> TsFileResult<()> {
        self.flush_run();
        writer.write_all(&self.buffer)?;
        self.buffer.clear();
        Ok(())
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.current_value = None;
        self.run_length = 0;
    }

    fn encoding(&self) -> TSEncoding {
        TSEncoding::Rle
    }

    fn estimated_size(&self) -> usize {
        self.buffer.len()
    }
}

// =========================================================================
// TS_2DIFF Encoder
// =========================================================================

/// TS_2DIFF encoder: stores the delta of differences (second-order delta).
pub struct Ts2diffEncoder {
    _data_type: TSDataType,
    last_value: i64,
    last_delta: i64,
    buffer: Vec<u8>,
    is_first: bool,
}

impl Ts2diffEncoder {
    pub fn new(data_type: TSDataType) -> Self {
        Ts2diffEncoder {
            _data_type: data_type,
            last_value: 0,
            last_delta: 0,
            buffer: Vec::new(),
            is_first: true,
        }
    }

    fn encode_value(&mut self, value: i64) {
        if self.is_first {
            // Store first value as-is
            self.buffer.extend_from_slice(&value.to_be_bytes());
            self.last_value = value;
            self.is_first = false;
        } else {
            let delta = value - self.last_value;
            let delta2 = delta - self.last_delta;
            // Zigzag encode delta2 and store as varint
            let zigzag = ((delta2 << 1) ^ (delta2 >> 63)) as u64;
            let mut v = zigzag;
            while (v & !0x7F) != 0 {
                self.buffer.push(((v & 0x7F) | 0x80) as u8);
                v >>= 7;
            }
            self.buffer.push((v & 0x7F) as u8);
            self.last_delta = delta;
            self.last_value = value;
        }
    }
}

impl Encoder for Ts2diffEncoder {
    fn encode_i32(&mut self, value: i32, _writer: &mut dyn Write) -> TsFileResult<()> {
        self.encode_value(value as i64);
        Ok(())
    }

    fn encode_i64(&mut self, value: i64, _writer: &mut dyn Write) -> TsFileResult<()> {
        self.encode_value(value);
        Ok(())
    }

    fn flush(&mut self, writer: &mut dyn Write) -> TsFileResult<()> {
        writer.write_all(&self.buffer)?;
        self.buffer.clear();
        Ok(())
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.last_value = 0;
        self.last_delta = 0;
        self.is_first = true;
    }

    fn encoding(&self) -> TSEncoding {
        TSEncoding::Ts2diff
    }

    fn estimated_size(&self) -> usize {
        self.buffer.len()
    }
}

// =========================================================================
// Gorilla Encoder
// =========================================================================

/// Gorilla encoder: XOR-based encoding for floating point or integer timestamps.
pub struct GorillaEncoder {
    _data_type: TSDataType,
    last_value: u64,
    buffer: Vec<u8>,
    bit_buf: u8,
    bit_count: u8,
    is_first: bool,
}

pub type ChimpEncoder = GorillaEncoder;
pub type SprintzEncoder = GorillaEncoder;
pub type CamelEncoder = GorillaEncoder;

impl GorillaEncoder {
    pub fn new(data_type: TSDataType) -> Self {
        GorillaEncoder {
            _data_type: data_type,
            last_value: 0,
            buffer: Vec::new(),
            bit_buf: 0,
            bit_count: 0,
            is_first: true,
        }
    }

    fn write_bits(&mut self, value: u64, num_bits: u8) {
        let mut remaining = num_bits;
        while remaining > 0 {
            let space = 8 - self.bit_count;
            let to_write = remaining.min(space);
            let shift = space - to_write;
            self.bit_buf |= ((value >> (remaining - to_write)) as u8) << shift;
            self.bit_count += to_write;
            remaining -= to_write;
            if self.bit_count == 8 {
                self.buffer.push(self.bit_buf);
                self.bit_buf = 0;
                self.bit_count = 0;
            }
        }
    }

    fn encode_value(&mut self, bits: u64) {
        if self.is_first {
            self.write_bits(bits, 64);
            self.last_value = bits;
            self.is_first = false;
        } else {
            let xor = bits ^ self.last_value;
            if xor == 0 {
                // Same value: write bit 0
                self.write_bits(0, 1);
            } else {
                // Different: write bit 1 + XOR value
                self.write_bits(1, 1);
                self.write_bits(xor, 64);
            }
            self.last_value = bits;
        }
    }
}

impl Encoder for GorillaEncoder {
    fn encode_i64(&mut self, value: i64, _writer: &mut dyn Write) -> TsFileResult<()> {
        self.encode_value(value as u64);
        Ok(())
    }

    fn encode_f32(&mut self, value: f32, _writer: &mut dyn Write) -> TsFileResult<()> {
        self.encode_value(value.to_bits() as u64);
        Ok(())
    }

    fn encode_f64(&mut self, value: f64, _writer: &mut dyn Write) -> TsFileResult<()> {
        self.encode_value(value.to_bits());
        Ok(())
    }

    fn flush(&mut self, writer: &mut dyn Write) -> TsFileResult<()> {
        if self.bit_count > 0 {
            self.buffer.push(self.bit_buf);
            self.bit_buf = 0;
            self.bit_count = 0;
        }
        writer.write_all(&self.buffer)?;
        self.buffer.clear();
        Ok(())
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.bit_buf = 0;
        self.bit_count = 0;
        self.last_value = 0;
        self.is_first = true;
    }

    fn encoding(&self) -> TSEncoding {
        TSEncoding::Gorilla
    }

    fn estimated_size(&self) -> usize {
        self.buffer.len()
    }
}
