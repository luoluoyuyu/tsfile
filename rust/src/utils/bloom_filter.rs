// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Bloom filter implementation, mirroring Java's BloomFilter.

use crate::error::TsFileResult;
use crate::utils::ReadWriteForEncodingUtils;
use std::io::{Read, Write};

/// A Bloom filter for path membership testing.
#[derive(Debug, Clone)]
pub struct BloomFilter {
    bit_set: Vec<u8>,
    size: usize,
    hash_function_size: u32,
}

impl BloomFilter {
    /// Create a BloomFilter for the expected item count and error rate.
    pub fn new(error_rate: f64, max_items: usize) -> Self {
        let m = Self::compute_m(error_rate, max_items);
        let k = Self::compute_k(error_rate);
        let byte_size = (m + 7) / 8;
        BloomFilter {
            bit_set: vec![0u8; byte_size],
            size: m,
            hash_function_size: k,
        }
    }

    /// Create an empty BloomFilter for cases where no paths are tracked.
    pub fn empty() -> Self {
        BloomFilter {
            bit_set: Vec::new(),
            size: 0,
            hash_function_size: 0,
        }
    }

    fn compute_m(error_rate: f64, max_items: usize) -> usize {
        let n = max_items as f64;
        let m = (-n * error_rate.ln() / (2.0_f64.ln() * 2.0_f64.ln())).ceil() as usize;
        m.max(8)
    }

    fn compute_k(error_rate: f64) -> u32 {
        let k = (-(error_rate.ln()) / 2.0_f64.ln()).ceil() as u32;
        k.max(1)
    }

    /// Add a path string to the bloom filter.
    pub fn add(&mut self, path: &str) {
        let hash = Self::murmur_hash(path.as_bytes(), 0);
        let hash2 = Self::murmur_hash(path.as_bytes(), hash as u32);
        for i in 0..self.hash_function_size {
            let combined = hash.wrapping_add((i as i64).wrapping_mul(hash2));
            let index = (combined.wrapping_abs() as usize) % self.size;
            let byte_index = index / 8;
            let bit_index = index % 8;
            self.bit_set[byte_index] |= 1 << bit_index;
        }
    }

    /// Test if a path might be in the set.
    pub fn contains(&self, path: &str) -> bool {
        if self.bit_set.is_empty() || self.size == 0 || self.hash_function_size == 0 {
            return false;
        }
        let hash = Self::murmur_hash(path.as_bytes(), 0);
        let hash2 = Self::murmur_hash(path.as_bytes(), hash as u32);
        for i in 0..self.hash_function_size {
            let combined = hash.wrapping_add((i as i64).wrapping_mul(hash2));
            let index = (combined.wrapping_abs() as usize) % self.size;
            let byte_index = index / 8;
            let bit_index = index % 8;
            if (self.bit_set[byte_index] & (1 << bit_index)) == 0 {
                return false;
            }
        }
        true
    }

    pub fn is_empty(&self) -> bool {
        self.bit_set.is_empty() || self.size == 0 || self.hash_function_size == 0
    }

    /// Serialize to writer.
    pub fn serialize<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        let mut written = 0;
        written += ReadWriteForEncodingUtils::write_unsigned_var_int(
            self.bit_set.len() as u32,
            writer,
        )?;
        if !self.bit_set.is_empty() {
            writer.write_all(&self.bit_set)?;
            written += self.bit_set.len();
            written += ReadWriteForEncodingUtils::write_unsigned_var_int(
                self.size as u32,
                writer,
            )?;
            written += ReadWriteForEncodingUtils::write_unsigned_var_int(
                self.hash_function_size,
                writer,
            )?;
        }
        Ok(written)
    }

    /// Serialize with self-description length (V3 format).
    /// Format: [bytes.length (varint)] [bytes...] [size (varint)] [hashFunctionSize (varint)]
    pub fn serialize_with_self_description_length<W: Write>(&self, writer: &mut W) -> TsFileResult<usize> {
        let mut written = 0;
        // Write byte array with self-description length
        written += ReadWriteForEncodingUtils::write_unsigned_var_int(
            self.bit_set.len() as u32,
            writer,
        )?;
        writer.write_all(&self.bit_set)?;
        written += self.bit_set.len();
        // Write size and hashFunctionSize
        written += ReadWriteForEncodingUtils::write_unsigned_var_int(
            self.size as u32,
            writer,
        )?;
        written += ReadWriteForEncodingUtils::write_unsigned_var_int(
            self.hash_function_size,
            writer,
        )?;
        Ok(written)
    }

    /// Deserialize from reader.
    pub fn deserialize<R: Read>(reader: &mut R) -> TsFileResult<Self> {
        let byte_len = ReadWriteForEncodingUtils::read_unsigned_var_int(reader)? as usize;
        if byte_len == 0 {
            return Ok(BloomFilter::empty());
        }
        let mut bit_set = vec![0u8; byte_len];
        reader.read_exact(&mut bit_set)?;
        let size = ReadWriteForEncodingUtils::read_unsigned_var_int(reader)? as usize;
        let hash_function_size =
            ReadWriteForEncodingUtils::read_unsigned_var_int(reader)?;
        Ok(BloomFilter {
            bit_set,
            size,
            hash_function_size,
        })
    }

    /// Murmur hash used by Java's BloomFilter implementation.
    fn murmur_hash(data: &[u8], seed: u32) -> i64 {
        let mut h1: u32 = seed;
        let mut h2: u32 = seed;
        let c1: u32 = 0xcc9e2d51;
        let c2: u32 = 0x1b873593;
        let len = data.len();
        let nblocks = len / 4;

        for i in 0..nblocks {
            let mut k1 = u32::from_le_bytes([
                data[i * 4],
                data[i * 4 + 1],
                data[i * 4 + 2],
                data[i * 4 + 3],
            ]);
            k1 = k1.wrapping_mul(c1);
            k1 = k1.rotate_left(15);
            k1 = k1.wrapping_mul(c2);
            h1 ^= k1;
            h1 = h1.rotate_left(13);
            h1 = h1.wrapping_mul(5).wrapping_add(0xe6546b64);
        }

        let tail = &data[nblocks * 4..];
        let mut k1: u32 = 0;
        match tail.len() {
            3 => {
                k1 ^= (tail[2] as u32) << 16;
                k1 ^= (tail[1] as u32) << 8;
                k1 ^= tail[0] as u32;
                k1 = k1.wrapping_mul(c1);
                k1 = k1.rotate_left(15);
                k1 = k1.wrapping_mul(c2);
                h1 ^= k1;
            }
            2 => {
                k1 ^= (tail[1] as u32) << 8;
                k1 ^= tail[0] as u32;
                k1 = k1.wrapping_mul(c1);
                k1 = k1.rotate_left(15);
                k1 = k1.wrapping_mul(c2);
                h1 ^= k1;
            }
            1 => {
                k1 ^= tail[0] as u32;
                k1 = k1.wrapping_mul(c1);
                k1 = k1.rotate_left(15);
                k1 = k1.wrapping_mul(c2);
                h1 ^= k1;
            }
            _ => {}
        }

        h1 ^= len as u32;
        h2 ^= len as u32;
        h1 = h1.wrapping_add(h2);
        h2 = h2.wrapping_add(h1);
        h1 = Self::fmix(h1);
        h2 = Self::fmix(h2);
        h1 = h1.wrapping_add(h2);
        h2 = h2.wrapping_add(h1);

        (((h2 as u64) << 32) | (h1 as u64)) as i64
    }

    fn fmix(mut h: u32) -> u32 {
        h ^= h >> 16;
        h = h.wrapping_mul(0x85ebca6b);
        h ^= h >> 13;
        h = h.wrapping_mul(0xc2b2ae35);
        h ^= h >> 16;
        h
    }
}
