// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Low-level IO utilities for reading and writing primitive types in TsFile format.
//!
//! Mirrors Java's ReadWriteIOUtils.

use std::io::{Read, Write};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::common::enums::{CompressionType, TSDataType, TSEncoding};
use crate::error::{TsFileError, TsFileResult};
use crate::utils::ReadWriteForEncodingUtils;

/// A Binary value (raw bytes) equivalent to Java's Binary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Binary(pub Vec<u8>);

impl Binary {
    pub fn new(data: Vec<u8>) -> Self {
        Binary(data)
    }

    pub fn from_str(s: &str) -> Self {
        Binary(s.as_bytes().to_vec())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for Binary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match std::str::from_utf8(&self.0) {
            Ok(s) => write!(f, "{}", s),
            Err(_) => write!(f, "<binary {:?}>", self.0),
        }
    }
}

/// A value that can be stored in a TsFile time series.
#[derive(Debug, Clone, PartialEq)]
pub enum TsValue {
    Boolean(bool),
    Int32(i32),
    Int64(i64),
    Float(f32),
    Double(f64),
    Text(Binary),
    Null,
}

/// Utility struct for reading/writing primitives in TsFile format.
pub struct ReadWriteIOUtils;

impl ReadWriteIOUtils {
    // =========================================================================
    // Write primitives
    // =========================================================================

    /// Write a bool (1 byte).
    pub fn write_bool<W: Write>(value: bool, writer: &mut W) -> TsFileResult<usize> {
        writer.write_u8(if value { 1 } else { 0 })?;
        Ok(1)
    }

    /// Write a byte (1 byte).
    pub fn write_byte<W: Write>(value: u8, writer: &mut W) -> TsFileResult<usize> {
        writer.write_u8(value)?;
        Ok(1)
    }

    /// Write i16 big-endian (2 bytes).
    pub fn write_i16<W: Write>(value: i16, writer: &mut W) -> TsFileResult<usize> {
        writer.write_i16::<BigEndian>(value)?;
        Ok(2)
    }

    /// Write i32 big-endian (4 bytes).
    pub fn write_i32<W: Write>(value: i32, writer: &mut W) -> TsFileResult<usize> {
        writer.write_i32::<BigEndian>(value)?;
        Ok(4)
    }

    /// Write i64 big-endian (8 bytes).
    pub fn write_i64<W: Write>(value: i64, writer: &mut W) -> TsFileResult<usize> {
        writer.write_i64::<BigEndian>(value)?;
        Ok(8)
    }

    /// Write f32 big-endian (4 bytes).
    pub fn write_f32<W: Write>(value: f32, writer: &mut W) -> TsFileResult<usize> {
        writer.write_f32::<BigEndian>(value)?;
        Ok(4)
    }

    /// Write f64 big-endian (8 bytes).
    pub fn write_f64<W: Write>(value: f64, writer: &mut W) -> TsFileResult<usize> {
        writer.write_f64::<BigEndian>(value)?;
        Ok(8)
    }

    /// Write raw bytes.
    pub fn write_bytes<W: Write>(data: &[u8], writer: &mut W) -> TsFileResult<usize> {
        writer.write_all(data)?;
        Ok(data.len())
    }

    /// Write a string with i32 length prefix (big-endian).
    pub fn write_string<W: Write>(s: &str, writer: &mut W) -> TsFileResult<usize> {
        let bytes = s.as_bytes();
        let len = bytes.len() as i32;
        let mut written = Self::write_i32(len, writer)?;
        writer.write_all(bytes)?;
        written += bytes.len();
        Ok(written)
    }

    /// Write a string with variable-length length prefix (signed varint with zigzag encoding).
    /// Matches Java's ReadWriteIOUtils.writeVar(String, OutputStream).
    pub fn write_var_int_string<W: Write>(s: &str, writer: &mut W) -> TsFileResult<usize> {
        let bytes = s.as_bytes();
        let len = bytes.len() as i32;
        let mut written = ReadWriteForEncodingUtils::write_var_int(len, writer)?;
        writer.write_all(bytes)?;
        written += bytes.len();
        Ok(written)
    }

    /// Write a Binary with i32 length prefix.
    pub fn write_binary<W: Write>(binary: &Binary, writer: &mut W) -> TsFileResult<usize> {
        let bytes = binary.as_bytes();
        let len = bytes.len() as i32;
        let mut written = Self::write_i32(len, writer)?;
        writer.write_all(bytes)?;
        written += bytes.len();
        Ok(written)
    }

    /// Write a TSDataType byte.
    pub fn write_data_type<W: Write>(data_type: TSDataType, writer: &mut W) -> TsFileResult<usize> {
        writer.write_u8(data_type.serialize())?;
        Ok(1)
    }

    /// Write a CompressionType byte.
    pub fn write_compression_type<W: Write>(
        compression: CompressionType,
        writer: &mut W,
    ) -> TsFileResult<usize> {
        writer.write_u8(compression.serialize())?;
        Ok(1)
    }

    /// Write a TSEncoding byte.
    pub fn write_encoding<W: Write>(encoding: TSEncoding, writer: &mut W) -> TsFileResult<usize> {
        writer.write_u8(encoding.serialize())?;
        Ok(1)
    }

    // =========================================================================
    // Read primitives
    // =========================================================================

    /// Read a bool (1 byte).
    pub fn read_bool<R: Read>(reader: &mut R) -> TsFileResult<bool> {
        let b = reader.read_u8()?;
        Ok(b == 1)
    }

    /// Read a byte (u8).
    pub fn read_byte<R: Read>(reader: &mut R) -> TsFileResult<u8> {
        Ok(reader.read_u8()?)
    }

    /// Read i16 big-endian.
    pub fn read_i16<R: Read>(reader: &mut R) -> TsFileResult<i16> {
        Ok(reader.read_i16::<BigEndian>()?)
    }

    /// Read i32 big-endian.
    pub fn read_i32<R: Read>(reader: &mut R) -> TsFileResult<i32> {
        Ok(reader.read_i32::<BigEndian>()?)
    }

    /// Read i64 big-endian.
    pub fn read_i64<R: Read>(reader: &mut R) -> TsFileResult<i64> {
        Ok(reader.read_i64::<BigEndian>()?)
    }

    /// Read f32 big-endian.
    pub fn read_f32<R: Read>(reader: &mut R) -> TsFileResult<f32> {
        Ok(reader.read_f32::<BigEndian>()?)
    }

    /// Read f64 big-endian.
    pub fn read_f64<R: Read>(reader: &mut R) -> TsFileResult<f64> {
        Ok(reader.read_f64::<BigEndian>()?)
    }

    /// Read exactly `n` bytes.
    pub fn read_bytes<R: Read>(reader: &mut R, n: usize) -> TsFileResult<Vec<u8>> {
        let mut buf = vec![0u8; n];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Read a string with i32 length prefix.
    pub fn read_string<R: Read>(reader: &mut R) -> TsFileResult<String> {
        let len = Self::read_i32(reader)?;
        if len < 0 {
            return Err(TsFileError::ReadError(format!(
                "Negative string length: {}",
                len
            )));
        }
        let bytes = Self::read_bytes(reader, len as usize)?;
        String::from_utf8(bytes)
            .map_err(|e| TsFileError::ReadError(format!("Invalid UTF-8: {}", e)))
    }

    /// Read a string with variable-length length prefix (signed varint with zigzag decoding).
    /// Matches Java's ReadWriteIOUtils.readVarIntString.
    pub fn read_var_int_string<R: Read>(reader: &mut R) -> TsFileResult<String> {
        let len = ReadWriteForEncodingUtils::read_var_int(reader)?;
        if len < 0 {
            return Err(TsFileError::ReadError(
                "Negative string length in varint string".to_string(),
            ));
        }
        let bytes = Self::read_bytes(reader, len as usize)?;
        String::from_utf8(bytes)
            .map_err(|e| TsFileError::ReadError(format!("Invalid UTF-8: {}", e)))
    }

    /// Read a Binary with i32 length prefix.
    pub fn read_binary<R: Read>(reader: &mut R) -> TsFileResult<Binary> {
        let len = Self::read_i32(reader)?;
        if len < 0 {
            return Err(TsFileError::ReadError(format!(
                "Negative binary length: {}",
                len
            )));
        }
        let bytes = Self::read_bytes(reader, len as usize)?;
        Ok(Binary(bytes))
    }

    /// Read a TSDataType byte.
    pub fn read_data_type<R: Read>(reader: &mut R) -> TsFileResult<TSDataType> {
        let b = reader.read_u8()?;
        TSDataType::deserialize(b)
    }

    /// Read a CompressionType byte.
    pub fn read_compression_type<R: Read>(reader: &mut R) -> TsFileResult<CompressionType> {
        let b = reader.read_u8()?;
        CompressionType::deserialize(b)
    }

    /// Read a TSEncoding byte.
    pub fn read_encoding<R: Read>(reader: &mut R) -> TsFileResult<TSEncoding> {
        let b = reader.read_u8()?;
        TSEncoding::deserialize(b)
    }

    /// Skip n bytes.
    pub fn skip<R: Read>(reader: &mut R, n: usize) -> TsFileResult<()> {
        let mut buf = vec![0u8; n];
        reader.read_exact(&mut buf)?;
        Ok(())
    }

    // =========================================================================
    // Read from byte slice (zero-copy slice operations)
    // =========================================================================

    /// Read i32 big-endian from slice. Returns (value, bytes_consumed).
    pub fn read_i32_from_slice(data: &[u8]) -> TsFileResult<(i32, usize)> {
        if data.len() < 4 {
            return Err(TsFileError::ReadError(
                "Not enough bytes to read i32".to_string(),
            ));
        }
        let value = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        Ok((value, 4))
    }

    /// Read i64 big-endian from slice.
    pub fn read_i64_from_slice(data: &[u8]) -> TsFileResult<(i64, usize)> {
        if data.len() < 8 {
            return Err(TsFileError::ReadError(
                "Not enough bytes to read i64".to_string(),
            ));
        }
        let value = i64::from_be_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]);
        Ok((value, 8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_bool_roundtrip() {
        let mut buf = Vec::new();
        ReadWriteIOUtils::write_bool(true, &mut buf).unwrap();
        ReadWriteIOUtils::write_bool(false, &mut buf).unwrap();
        let mut cursor = Cursor::new(&buf);
        assert_eq!(ReadWriteIOUtils::read_bool(&mut cursor).unwrap(), true);
        assert_eq!(ReadWriteIOUtils::read_bool(&mut cursor).unwrap(), false);
    }

    #[test]
    fn test_i32_roundtrip() {
        let values = [0i32, 1, -1, i32::MAX, i32::MIN];
        for &v in &values {
            let mut buf = Vec::new();
            ReadWriteIOUtils::write_i32(v, &mut buf).unwrap();
            let mut cursor = Cursor::new(&buf);
            let read = ReadWriteIOUtils::read_i32(&mut cursor).unwrap();
            assert_eq!(v, read);
        }
    }

    #[test]
    fn test_string_roundtrip() {
        let s = "hello, TsFile!";
        let mut buf = Vec::new();
        ReadWriteIOUtils::write_string(s, &mut buf).unwrap();
        let mut cursor = Cursor::new(&buf);
        let read = ReadWriteIOUtils::read_string(&mut cursor).unwrap();
        assert_eq!(s, read);
    }

    #[test]
    fn test_var_int_string_roundtrip() {
        let s = "test_measurement_id";
        let mut buf = Vec::new();
        ReadWriteIOUtils::write_var_int_string(s, &mut buf).unwrap();
        let mut cursor = Cursor::new(&buf);
        let read = ReadWriteIOUtils::read_var_int_string(&mut cursor).unwrap();
        assert_eq!(s, read);
    }
}
