// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Read/Write encoding utilities: VarInt encoding/decoding.
//!
//! Mirrors Java's ReadWriteForEncodingUtils.

use std::io::{Read, Write};

use crate::error::{TsFileError, TsFileResult};

/// Utilities for variable-length integer encoding/decoding.
pub struct ReadWriteForEncodingUtils;

impl ReadWriteForEncodingUtils {
    // =========================================================================
    // Unsigned VarInt (u32)
    // =========================================================================

    /// Write an unsigned varint to the writer. Returns number of bytes written.
    pub fn write_unsigned_var_int<W: Write>(value: u32, writer: &mut W) -> TsFileResult<usize> {
        let mut v = value;
        let mut count = 0;
        while (v & 0xFFFFFF80) != 0 {
            writer.write_all(&[((v & 0x7F) | 0x80) as u8])?;
            v >>= 7;
            count += 1;
        }
        writer.write_all(&[(v & 0x7F) as u8])?;
        count += 1;
        Ok(count)
    }

    /// Write an unsigned varint to a byte vector. Returns number of bytes written.
    pub fn write_unsigned_var_int_to_vec(value: u32, buf: &mut Vec<u8>) -> usize {
        let mut v = value;
        let mut count = 0;
        while (v & 0xFFFFFF80) != 0 {
            buf.push(((v & 0x7F) | 0x80) as u8);
            v >>= 7;
            count += 1;
        }
        buf.push((v & 0x7F) as u8);
        count + 1
    }

    /// Read an unsigned varint from the reader.
    pub fn read_unsigned_var_int<R: Read>(reader: &mut R) -> TsFileResult<u32> {
        let mut value: u32 = 0;
        let mut shift = 0u32;
        let mut byte = [0u8; 1];
        loop {
            reader.read_exact(&mut byte)?;
            let b = byte[0] as u32;
            value |= (b & 0x7F) << shift;
            shift += 7;
            if (b & 0x80) == 0 {
                break;
            }
            if shift >= 35 {
                return Err(TsFileError::DecodingError(
                    "VarInt too large".to_string(),
                ));
            }
        }
        Ok(value)
    }

    /// Read an unsigned varint from a byte slice. Returns (value, bytes_consumed).
    pub fn read_unsigned_var_int_from_slice(data: &[u8]) -> TsFileResult<(u32, usize)> {
        let mut value: u32 = 0;
        let mut shift = 0u32;
        let mut pos = 0;
        loop {
            if pos >= data.len() {
                return Err(TsFileError::DecodingError(
                    "Unexpected end of data reading VarInt".to_string(),
                ));
            }
            let b = data[pos] as u32;
            pos += 1;
            value |= (b & 0x7F) << shift;
            shift += 7;
            if (b & 0x80) == 0 {
                break;
            }
            if shift >= 35 {
                return Err(TsFileError::DecodingError(
                    "VarInt too large".to_string(),
                ));
            }
        }
        Ok((value, pos))
    }

    // =========================================================================
    // Signed VarInt (i32) using zigzag encoding
    // =========================================================================

    /// Write a signed varint (zigzag encoded) to the writer.
    pub fn write_var_int<W: Write>(value: i32, writer: &mut W) -> TsFileResult<usize> {
        let uvalue = Self::zigzag_encode(value);
        Self::write_unsigned_var_int(uvalue, writer)
    }

    /// Write a signed varint to a byte vector.
    pub fn write_var_int_to_vec(value: i32, buf: &mut Vec<u8>) -> usize {
        let uvalue = Self::zigzag_encode(value);
        Self::write_unsigned_var_int_to_vec(uvalue, buf)
    }

    /// Read a signed varint (zigzag decoded).
    pub fn read_var_int<R: Read>(reader: &mut R) -> TsFileResult<i32> {
        let uvalue = Self::read_unsigned_var_int(reader)?;
        Ok(Self::zigzag_decode(uvalue))
    }

    /// Read a signed varint from a byte slice.
    pub fn read_var_int_from_slice(data: &[u8]) -> TsFileResult<(i32, usize)> {
        let (uvalue, len) = Self::read_unsigned_var_int_from_slice(data)?;
        Ok((Self::zigzag_decode(uvalue), len))
    }

    // =========================================================================
    // Size calculations
    // =========================================================================

    /// Returns the number of bytes needed to encode an unsigned varint.
    pub fn u_var_int_size(value: u32) -> usize {
        let mut v = value;
        let mut size = 1;
        while (v & 0xFFFFFF80) != 0 {
            v >>= 7;
            size += 1;
        }
        size
    }

    /// Returns the number of bytes needed to encode a signed varint.
    pub fn var_int_size(value: i32) -> usize {
        let uvalue = Self::zigzag_encode(value);
        Self::u_var_int_size(uvalue)
    }

    // =========================================================================
    // Bit-width utilities (for encoding/decoding)
    // =========================================================================

    /// Find the max bit width needed to represent all integers in the list.
    pub fn get_int_max_bit_width(list: &[i32]) -> u32 {
        let mut max = 1u32;
        for &num in list {
            let bit_width = 32u32.saturating_sub(num.leading_zeros());
            max = max.max(bit_width);
        }
        max
    }

    /// Find the max bit width needed to represent all longs in the list.
    pub fn get_long_max_bit_width(list: &[i64]) -> u32 {
        let mut max = 1u32;
        for &num in list {
            let bit_width = 64u32.saturating_sub(num.leading_zeros());
            max = max.max(bit_width);
        }
        max
    }

    /// Write an integer in little-endian padded on bit width.
    pub fn write_int_little_endian_padded_on_bit_width<W: Write>(
        value: i32,
        writer: &mut W,
        bit_width: u32,
    ) -> TsFileResult<()> {
        let padded_byte_num = (bit_width + 7) / 8;
        if padded_byte_num > 4 {
            return Err(TsFileError::EncodingError(format!(
                "Padded byte num {} exceeds 4",
                padded_byte_num
            )));
        }
        let bytes = value.to_le_bytes();
        writer.write_all(&bytes[..padded_byte_num as usize])?;
        Ok(())
    }

    /// Write a long in little-endian padded on bit width.
    pub fn write_long_little_endian_padded_on_bit_width<W: Write>(
        value: i64,
        writer: &mut W,
        bit_width: u32,
    ) -> TsFileResult<()> {
        let padded_byte_num = (bit_width + 7) / 8;
        if padded_byte_num > 8 {
            return Err(TsFileError::EncodingError(format!(
                "Padded byte num {} exceeds 8",
                padded_byte_num
            )));
        }
        let bytes = value.to_le_bytes();
        writer.write_all(&bytes[..padded_byte_num as usize])?;
        Ok(())
    }

    /// Read an integer in little-endian padded on bit width.
    pub fn read_int_little_endian_padded_on_bit_width<R: Read>(
        reader: &mut R,
        bit_width: u32,
    ) -> TsFileResult<i32> {
        let padded_byte_num = (bit_width + 7) / 8;
        if padded_byte_num > 4 {
            return Err(TsFileError::DecodingError(format!(
                "Padded byte num {} exceeds 4",
                padded_byte_num
            )));
        }
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf[..padded_byte_num as usize])?;
        Ok(i32::from_le_bytes(buf))
    }

    // =========================================================================
    // Zigzag helpers
    // =========================================================================

    fn zigzag_encode(value: i32) -> u32 {
        ((value << 1) ^ (value >> 31)) as u32
    }

    fn zigzag_decode(value: u32) -> i32 {
        ((value >> 1) as i32) ^ -((value & 1) as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_unsigned_var_int_roundtrip() {
        let values = [0u32, 1, 127, 128, 16383, 16384, u32::MAX / 2];
        for &v in &values {
            let mut buf = Vec::new();
            let written = ReadWriteForEncodingUtils::write_unsigned_var_int(v, &mut buf).unwrap();
            let mut cursor = Cursor::new(&buf);
            let read = ReadWriteForEncodingUtils::read_unsigned_var_int(&mut cursor).unwrap();
            assert_eq!(v, read, "value={}", v);
            assert_eq!(written, buf.len());
        }
    }

    #[test]
    fn test_signed_var_int_roundtrip() {
        let values = [0i32, 1, -1, 100, -100, i32::MAX / 2, i32::MIN / 2];
        for &v in &values {
            let mut buf = Vec::new();
            ReadWriteForEncodingUtils::write_var_int(v, &mut buf).unwrap();
            let mut cursor = Cursor::new(&buf);
            let read = ReadWriteForEncodingUtils::read_var_int(&mut cursor).unwrap();
            assert_eq!(v, read, "value={}", v);
        }
    }
}
