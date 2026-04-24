// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! ChunkReader: parses page headers and delegates page decoding.

use std::io::{Cursor, Read};

use crate::compress::create_decompressor;
use crate::error::TsFileResult;
use crate::file::header::{ChunkHeader, PageHeader};
use crate::file::meta_marker::MetaMarker;
use crate::read::reader::page_reader::PageReader;
use crate::read::time_value_pair::TimeValuePair;

pub struct ChunkReader {
    header: ChunkHeader,
    chunk_data: Vec<u8>,
}

impl ChunkReader {
    pub fn new(header: ChunkHeader, chunk_data: Vec<u8>) -> Self {
        ChunkReader { header, chunk_data }
    }

    pub fn read_all(&self) -> TsFileResult<Vec<TimeValuePair>> {
        let has_multiple_pages = (self.header.chunk_type & 0x3F) == MetaMarker::CHUNK_HEADER;
        let has_page_statistics = has_multiple_pages;
        let decompressor = create_decompressor(self.header.compression_type);
        let mut cursor = Cursor::new(self.chunk_data.as_slice());
        let mut results = Vec::new();

        while (cursor.position() as usize) < self.chunk_data.len() {
            let page_header = PageHeader::deserialize(
                &mut cursor,
                self.header.data_type,
                has_page_statistics,
            )?;
            if page_header.uncompressed_size == 0 {
                continue;
            }

            let compressed_size = page_header.compressed_size as usize;
            let uncompressed_size = page_header.uncompressed_size as usize;
            let mut compressed = vec![0u8; compressed_size];
            cursor.read_exact(&mut compressed)?;
            let page_data = decompressor.decompress(&compressed, uncompressed_size)?;
            let page_reader = PageReader::new(
                self.header.data_type,
                self.header.encoding_type,
                page_data,
            );
            results.extend(page_reader.read_all()?);
        }
        Ok(results)
    }
}
