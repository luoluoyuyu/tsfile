// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Decoder trait and implementations.

use crate::common::enums::{TSDataType, TSEncoding};
use crate::error::{TsFileError, TsFileResult};
use crate::utils::read_write_io_utils::Binary;

fn read_unsigned_var_long(data: &[u8], pos: &mut usize) -> TsFileResult<u64> {
    let mut value = 0u64;
    let mut shift = 0u32;
    loop {
        if *pos >= data.len() {
            return Err(TsFileError::DecodingError("VarInt truncated".to_string()));
        }
        let byte = data[*pos] as u64;
        *pos += 1;
        value |= (byte & 0x7F) << shift;
        if (byte & 0x80) == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            return Err(TsFileError::DecodingError("VarInt too large".to_string()));
        }
    }
    Ok(value)
}

fn unzigzag_i64(value: u64) -> i64 {
    ((value >> 1) as i64) ^ -((value & 1) as i64)
}

/// A decoded value from a timeseries.
#[derive(Debug, Clone, PartialEq)]
pub enum DecodedValue {
    Boolean(bool),
    Int32(i32),
    Int64(i64),
    Float(f32),
    Double(f64),
    Binary(Binary),
    Null,
}

/// Trait for decoding page data.
pub trait Decoder: Send + Sync {
    /// Initialize decoder from page data.
    fn init(&mut self, data: &[u8]) -> TsFileResult<()>;

    /// Check if there are more values.
    fn has_next(&self) -> bool;

    /// Read next boolean value.
    fn read_bool(&mut self) -> TsFileResult<bool> {
        Err(TsFileError::UnsupportedOperation(
            "read_bool not supported".to_string(),
        ))
    }

    /// Read next i32 value.
    fn read_i32(&mut self) -> TsFileResult<i32> {
        Err(TsFileError::UnsupportedOperation(
            "read_i32 not supported".to_string(),
        ))
    }

    /// Read next i64 value.
    fn read_i64(&mut self) -> TsFileResult<i64> {
        Err(TsFileError::UnsupportedOperation(
            "read_i64 not supported".to_string(),
        ))
    }

    /// Read next f32 value.
    fn read_f32(&mut self) -> TsFileResult<f32> {
        Err(TsFileError::UnsupportedOperation(
            "read_f32 not supported".to_string(),
        ))
    }

    /// Read next f64 value.
    fn read_f64(&mut self) -> TsFileResult<f64> {
        Err(TsFileError::UnsupportedOperation(
            "read_f64 not supported".to_string(),
        ))
    }

    /// Read next binary value.
    fn read_binary(&mut self) -> TsFileResult<Binary> {
        Err(TsFileError::UnsupportedOperation(
            "read_binary not supported".to_string(),
        ))
    }

    /// Reset decoder state.
    fn reset(&mut self);
}

pub struct UnsupportedDecoder {
    data_type: TSDataType,
    encoding: TSEncoding,
}

impl UnsupportedDecoder {
    pub fn new(data_type: TSDataType, encoding: TSEncoding) -> Self {
        UnsupportedDecoder { data_type, encoding }
    }
}

impl Decoder for UnsupportedDecoder {
    fn init(&mut self, _data: &[u8]) -> TsFileResult<()> {
        Err(TsFileError::UnsupportedOperation(format!(
            "Decoding {} is not implemented for data type {}",
            self.encoding, self.data_type
        )))
    }

    fn has_next(&self) -> bool {
        false
    }

    fn reset(&mut self) {}
}

/// Create a decoder for the given data type and encoding.
pub fn create_decoder(data_type: TSDataType, encoding: TSEncoding) -> Box<dyn Decoder> {
    match encoding {
        TSEncoding::Plain => Box::new(PlainDecoder::new(data_type)),
        TSEncoding::Dictionary => Box::new(DictionaryDecoder::new()),
        TSEncoding::Rle => Box::new(RleDecoder::new(data_type)),
        TSEncoding::Zigzag => Box::new(ZigzagDecoder::new(data_type)),
        TSEncoding::Ts2diff => Box::new(Ts2diffDecoder::new(data_type)),
        TSEncoding::Gorilla | TSEncoding::GorillaV1 => {
            Box::new(GorillaDecoder::new(data_type))
        }
        _ => Box::new(UnsupportedDecoder::new(data_type, encoding)),
    }
}

// =========================================================================
// Dictionary Decoder
// =========================================================================

pub struct DictionaryDecoder {
    dictionary: Vec<Binary>,
    ids: Vec<u32>,
    index: usize,
}

impl DictionaryDecoder {
    pub fn new() -> Self {
        DictionaryDecoder {
            dictionary: Vec::new(),
            ids: Vec::new(),
            index: 0,
        }
    }
}

impl Decoder for DictionaryDecoder {
    fn init(&mut self, data: &[u8]) -> TsFileResult<()> {
        self.dictionary.clear();
        self.ids.clear();
        self.index = 0;
        let mut pos = 0usize;
        let dictionary_len = read_unsigned_var_long(data, &mut pos)? as usize;
        for _ in 0..dictionary_len {
            let len = read_unsigned_var_long(data, &mut pos)? as usize;
            if pos + len > data.len() {
                return Err(TsFileError::DecodingError(
                    "Dictionary entry truncated".to_string(),
                ));
            }
            self.dictionary.push(Binary(data[pos..pos + len].to_vec()));
            pos += len;
        }
        let id_len = read_unsigned_var_long(data, &mut pos)? as usize;
        for _ in 0..id_len {
            self.ids.push(read_unsigned_var_long(data, &mut pos)? as u32);
        }
        Ok(())
    }

    fn has_next(&self) -> bool {
        self.index < self.ids.len()
    }

    fn read_binary(&mut self) -> TsFileResult<Binary> {
        if !self.has_next() {
            return Err(TsFileError::DecodingError("No more dictionary ids".to_string()));
        }
        let id = self.ids[self.index] as usize;
        self.index += 1;
        self.dictionary.get(id).cloned().ok_or_else(|| {
            TsFileError::DecodingError(format!("Dictionary id out of range: {}", id))
        })
    }

    fn reset(&mut self) {
        self.index = 0;
    }
}

// =========================================================================
// Zigzag Decoder
// =========================================================================

pub struct ZigzagDecoder {
    _data_type: TSDataType,
    data: Vec<u8>,
    pos: usize,
}

impl ZigzagDecoder {
    pub fn new(data_type: TSDataType) -> Self {
        ZigzagDecoder {
            _data_type: data_type,
            data: Vec::new(),
            pos: 0,
        }
    }

    fn read_value(&mut self) -> TsFileResult<i64> {
        read_unsigned_var_long(&self.data, &mut self.pos).map(unzigzag_i64)
    }
}

impl Decoder for ZigzagDecoder {
    fn init(&mut self, data: &[u8]) -> TsFileResult<()> {
        self.data = data.to_vec();
        self.pos = 0;
        Ok(())
    }

    fn has_next(&self) -> bool {
        self.pos < self.data.len()
    }

    fn read_i32(&mut self) -> TsFileResult<i32> {
        Ok(self.read_value()? as i32)
    }

    fn read_i64(&mut self) -> TsFileResult<i64> {
        self.read_value()
    }

    fn reset(&mut self) {
        self.pos = 0;
    }
}

// =========================================================================
// Plain Decoder
// =========================================================================

/// Plain decoder: reads values as-is in big-endian.
pub struct PlainDecoder {
    data_type: TSDataType,
    data: Vec<u8>,
    pos: usize,
}

impl PlainDecoder {
    pub fn new(data_type: TSDataType) -> Self {
        PlainDecoder {
            data_type,
            data: Vec::new(),
            pos: 0,
        }
    }
}

impl Decoder for PlainDecoder {
    fn init(&mut self, data: &[u8]) -> TsFileResult<()> {
        self.data = data.to_vec();
        self.pos = 0;
        Ok(())
    }

    fn has_next(&self) -> bool {
        match self.data_type {
            TSDataType::Boolean => self.pos < self.data.len(),
            TSDataType::Int32 | TSDataType::Float | TSDataType::Date => {
                self.pos + 4 <= self.data.len()
            }
            TSDataType::Int64
            | TSDataType::Double
            | TSDataType::Timestamp => {
                self.pos + 8 <= self.data.len()
            }
            TSDataType::Text | TSDataType::Blob | TSDataType::String => {
                if self.pos + 4 > self.data.len() {
                    return false;
                }
                let len = i32::from_be_bytes([
                    self.data[self.pos],
                    self.data[self.pos + 1],
                    self.data[self.pos + 2],
                    self.data[self.pos + 3],
                ]);
                len >= 0 && self.pos + 4 + len as usize <= self.data.len()
            }
            _ => false,
        }
    }

    fn read_bool(&mut self) -> TsFileResult<bool> {
        if self.pos >= self.data.len() {
            return Err(TsFileError::DecodingError("No more data".to_string()));
        }
        let v = self.data[self.pos] == 1;
        self.pos += 1;
        Ok(v)
    }

    fn read_i32(&mut self) -> TsFileResult<i32> {
        if self.pos + 4 > self.data.len() {
            return Err(TsFileError::DecodingError("No more data".to_string()));
        }
        let v = i32::from_be_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(v)
    }

    fn read_i64(&mut self) -> TsFileResult<i64> {
        if self.pos + 8 > self.data.len() {
            return Err(TsFileError::DecodingError("No more data".to_string()));
        }
        let v = i64::from_be_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
            self.data[self.pos + 4],
            self.data[self.pos + 5],
            self.data[self.pos + 6],
            self.data[self.pos + 7],
        ]);
        self.pos += 8;
        Ok(v)
    }

    fn read_f32(&mut self) -> TsFileResult<f32> {
        let bits = self.read_i32()? as u32;
        Ok(f32::from_bits(bits))
    }

    fn read_f64(&mut self) -> TsFileResult<f64> {
        let bits = self.read_i64()? as u64;
        Ok(f64::from_bits(bits))
    }

    fn read_binary(&mut self) -> TsFileResult<Binary> {
        if self.pos + 4 > self.data.len() {
            return Err(TsFileError::DecodingError("No more data".to_string()));
        }
        let len = i32::from_be_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]) as usize;
        self.pos += 4;
        if self.pos + len > self.data.len() {
            return Err(TsFileError::DecodingError(
                "Binary data truncated".to_string(),
            ));
        }
        let bytes = self.data[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Ok(Binary(bytes))
    }

    fn reset(&mut self) {
        self.pos = 0;
    }
}

pub fn read_next_value(decoder: &mut dyn Decoder, data_type: TSDataType) -> TsFileResult<DecodedValue> {
    match data_type {
        TSDataType::Boolean => Ok(DecodedValue::Boolean(decoder.read_bool()?)),
        TSDataType::Int32 | TSDataType::Date => Ok(DecodedValue::Int32(decoder.read_i32()?)),
        TSDataType::Int64 | TSDataType::Timestamp => Ok(DecodedValue::Int64(decoder.read_i64()?)),
        TSDataType::Float => Ok(DecodedValue::Float(decoder.read_f32()?)),
        TSDataType::Double => Ok(DecodedValue::Double(decoder.read_f64()?)),
        TSDataType::Text | TSDataType::Blob | TSDataType::String => {
            Ok(DecodedValue::Binary(decoder.read_binary()?))
        }
        _ => Ok(DecodedValue::Null),
    }
}

// =========================================================================
// RLE Decoder
// =========================================================================

pub struct RleDecoder {
    _data_type: TSDataType,
    data: Vec<u8>,
    pos: usize,
    current_run_value: i64,
    current_run_remaining: u32,
}

impl RleDecoder {
    pub fn new(data_type: TSDataType) -> Self {
        RleDecoder {
            _data_type: data_type,
            data: Vec::new(),
            pos: 0,
            current_run_value: 0,
            current_run_remaining: 0,
        }
    }

    fn load_next_run(&mut self) -> TsFileResult<()> {
        if self.pos + 12 > self.data.len() {
            return Err(TsFileError::DecodingError("No more runs".to_string()));
        }
        let run_len = u32::from_be_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        let value = i64::from_be_bytes([
            self.data[self.pos + 4],
            self.data[self.pos + 5],
            self.data[self.pos + 6],
            self.data[self.pos + 7],
            self.data[self.pos + 8],
            self.data[self.pos + 9],
            self.data[self.pos + 10],
            self.data[self.pos + 11],
        ]);
        self.pos += 12;
        self.current_run_value = value;
        self.current_run_remaining = run_len;
        Ok(())
    }
}

impl Decoder for RleDecoder {
    fn init(&mut self, data: &[u8]) -> TsFileResult<()> {
        self.data = data.to_vec();
        self.pos = 0;
        self.current_run_remaining = 0;
        Ok(())
    }

    fn has_next(&self) -> bool {
        self.current_run_remaining > 0 || self.pos + 12 <= self.data.len()
    }

    fn read_i64(&mut self) -> TsFileResult<i64> {
        if self.current_run_remaining == 0 {
            self.load_next_run()?;
        }
        self.current_run_remaining -= 1;
        Ok(self.current_run_value)
    }

    fn read_i32(&mut self) -> TsFileResult<i32> {
        Ok(self.read_i64()? as i32)
    }

    fn read_bool(&mut self) -> TsFileResult<bool> {
        Ok(self.read_i64()? != 0)
    }

    fn read_f32(&mut self) -> TsFileResult<f32> {
        Ok(f32::from_bits(self.read_i64()? as u32))
    }

    fn read_f64(&mut self) -> TsFileResult<f64> {
        Ok(f64::from_bits(self.read_i64()? as u64))
    }

    fn reset(&mut self) {
        self.pos = 0;
        self.current_run_remaining = 0;
    }
}

// =========================================================================
// TS_2DIFF Decoder
// =========================================================================

pub struct Ts2diffDecoder {
    _data_type: TSDataType,
    data: Vec<u8>,
    pos: usize,
    last_value: i64,
    last_delta: i64,
    is_first: bool,
}

impl Ts2diffDecoder {
    pub fn new(data_type: TSDataType) -> Self {
        Ts2diffDecoder {
            _data_type: data_type,
            data: Vec::new(),
            pos: 0,
            last_value: 0,
            last_delta: 0,
            is_first: true,
        }
    }

    fn read_value(&mut self) -> TsFileResult<i64> {
        if self.is_first {
            if self.pos + 8 > self.data.len() {
                return Err(TsFileError::DecodingError("Not enough data".to_string()));
            }
            let v = i64::from_be_bytes([
                self.data[self.pos],
                self.data[self.pos + 1],
                self.data[self.pos + 2],
                self.data[self.pos + 3],
                self.data[self.pos + 4],
                self.data[self.pos + 5],
                self.data[self.pos + 6],
                self.data[self.pos + 7],
            ]);
            self.pos += 8;
            self.last_value = v;
            self.is_first = false;
            Ok(v)
        } else {
            // Read zigzag varint
            let mut zigzag: u64 = 0;
            let mut shift = 0u32;
            loop {
                if self.pos >= self.data.len() {
                    return Err(TsFileError::DecodingError("VarInt truncated".to_string()));
                }
                let b = self.data[self.pos] as u64;
                self.pos += 1;
                zigzag |= (b & 0x7F) << shift;
                shift += 7;
                if (b & 0x80) == 0 {
                    break;
                }
            }
            let delta2 = ((zigzag >> 1) as i64) ^ -((zigzag & 1) as i64);
            let delta = delta2 + self.last_delta;
            let value = delta + self.last_value;
            self.last_delta = delta;
            self.last_value = value;
            Ok(value)
        }
    }
}

impl Decoder for Ts2diffDecoder {
    fn init(&mut self, data: &[u8]) -> TsFileResult<()> {
        self.data = data.to_vec();
        self.pos = 0;
        self.is_first = true;
        Ok(())
    }

    fn has_next(&self) -> bool {
        self.pos < self.data.len()
    }

    fn read_i64(&mut self) -> TsFileResult<i64> {
        self.read_value()
    }

    fn read_i32(&mut self) -> TsFileResult<i32> {
        Ok(self.read_value()? as i32)
    }

    fn reset(&mut self) {
        self.pos = 0;
        self.last_value = 0;
        self.last_delta = 0;
        self.is_first = true;
    }
}

// =========================================================================
// Gorilla Decoder
// =========================================================================

pub struct GorillaDecoder {
    _data_type: TSDataType,
    data: Vec<u8>,
    byte_pos: usize,
    bit_pos: u8,
    last_value: u64,
    is_first: bool,
}

pub type ChimpDecoder = GorillaDecoder;
pub type SprintzDecoder = GorillaDecoder;
pub type CamelDecoder = GorillaDecoder;

impl GorillaDecoder {
    pub fn new(data_type: TSDataType) -> Self {
        GorillaDecoder {
            _data_type: data_type,
            data: Vec::new(),
            byte_pos: 0,
            bit_pos: 0,
            last_value: 0,
            is_first: true,
        }
    }

    fn read_bit(&mut self) -> TsFileResult<u8> {
        if self.byte_pos >= self.data.len() {
            return Err(TsFileError::DecodingError("No more bits".to_string()));
        }
        let bit = (self.data[self.byte_pos] >> (7 - self.bit_pos)) & 1;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.byte_pos += 1;
            self.bit_pos = 0;
        }
        Ok(bit)
    }

    fn read_bits(&mut self, n: u8) -> TsFileResult<u64> {
        let mut result: u64 = 0;
        for _ in 0..n {
            result = (result << 1) | self.read_bit()? as u64;
        }
        Ok(result)
    }

    fn read_value(&mut self) -> TsFileResult<u64> {
        if self.is_first {
            let v = self.read_bits(64)?;
            self.last_value = v;
            self.is_first = false;
            Ok(v)
        } else {
            let flag = self.read_bit()?;
            if flag == 0 {
                Ok(self.last_value)
            } else {
                let xor = self.read_bits(64)?;
                let v = xor ^ self.last_value;
                self.last_value = v;
                Ok(v)
            }
        }
    }
}

impl Decoder for GorillaDecoder {
    fn init(&mut self, data: &[u8]) -> TsFileResult<()> {
        self.data = data.to_vec();
        self.byte_pos = 0;
        self.bit_pos = 0;
        self.is_first = true;
        Ok(())
    }

    fn has_next(&self) -> bool {
        self.byte_pos < self.data.len()
    }

    fn read_i64(&mut self) -> TsFileResult<i64> {
        Ok(self.read_value()? as i64)
    }

    fn read_f32(&mut self) -> TsFileResult<f32> {
        let bits = self.read_value()? as u32;
        Ok(f32::from_bits(bits))
    }

    fn read_f64(&mut self) -> TsFileResult<f64> {
        let bits = self.read_value()?;
        Ok(f64::from_bits(bits))
    }

    fn reset(&mut self) {
        self.byte_pos = 0;
        self.bit_pos = 0;
        self.last_value = 0;
        self.is_first = true;
    }
}
