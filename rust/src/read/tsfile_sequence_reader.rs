// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TsFileSequenceReader: sequential scan of a TsFile.
//!
//! Mirrors Java's TsFileSequenceReader.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use crate::common::constant::TsFileConstant;
use crate::error::{TsFileError, TsFileResult};
use crate::file::header::{ChunkGroupHeader, ChunkHeader};
use crate::file::meta_marker::MetaMarker;
use crate::file::metadata::tsfile_metadata::TsFileMetadata;
use crate::read::reader::ChunkReader;
use crate::read::time_value_pair::TimeValuePair;

/// Information about a chunk during sequential reading.
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub device_id: String,
    pub chunk_header: ChunkHeader,
    pub chunk_data: Vec<u8>,
}

/// Sequential reader for TsFile format.
pub struct TsFileSequenceReader {
    reader: BufReader<File>,
    file_size: u64,
    /// Cached file metadata.
    pub file_metadata: Option<TsFileMetadata>,
}

impl TsFileSequenceReader {
    /// Open a TsFile for reading.
    pub fn new<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        let file = File::open(path.as_ref())?;
        let file_size = file.metadata()?.len();
        let reader = BufReader::new(file);
        let mut seq_reader = TsFileSequenceReader {
            reader,
            file_size,
            file_metadata: None,
        };
        seq_reader.verify_magic()?;
        Ok(seq_reader)
    }

    /// Verify magic string and version.
    fn verify_magic(&mut self) -> TsFileResult<()> {
        let magic_len = TsFileConstant::MAGIC_STRING.len();
        let mut magic_buf = vec![0u8; magic_len];
        self.reader.read_exact(&mut magic_buf)?;
        if magic_buf != TsFileConstant::MAGIC_STRING.as_bytes() {
            return Err(TsFileError::InvalidMagicString);
        }
        // Read version byte
        let mut version_buf = [0u8; 1];
        self.reader.read_exact(&mut version_buf)?;
        // Version check is lenient (accept any version)
        Ok(())
    }

    /// Read the TsFile footer metadata.
    pub fn read_file_metadata(&mut self) -> TsFileResult<&TsFileMetadata> {
        if self.file_metadata.is_some() {
            return Ok(self.file_metadata.as_ref().unwrap());
        }

        let magic_len = TsFileConstant::MAGIC_STRING.len() as i64;

        // Seek to read metadata size (last 4+magic_len bytes)
        self.reader
            .seek(SeekFrom::End(-(magic_len + 4)))?;

        let mut size_buf = [0u8; 4];
        self.reader.read_exact(&mut size_buf)?;
        let meta_size = i32::from_be_bytes(size_buf) as usize;

        // Verify closing magic
        let mut closing_magic = vec![0u8; magic_len as usize];
        self.reader.read_exact(&mut closing_magic)?;
        if closing_magic != TsFileConstant::MAGIC_STRING.as_bytes() {
            return Err(TsFileError::InvalidMagicString);
        }

        // Seek to metadata start
        let meta_offset = self.file_size as i64 - magic_len - 4 - meta_size as i64;
        self.reader.seek(SeekFrom::Start(meta_offset as u64))?;

        let mut meta_buf = vec![0u8; meta_size];
        self.reader.read_exact(&mut meta_buf)?;

        let metadata = TsFileMetadata::deserialize(&mut std::io::Cursor::new(&meta_buf))?;
        self.file_metadata = Some(metadata);
        Ok(self.file_metadata.as_ref().unwrap())
    }

    /// Scan the file sequentially, returning chunks in order.
    /// Reads file metadata first to determine the data region boundary.
    pub fn read_all_chunks(
        &mut self,
    ) -> TsFileResult<Vec<(String, ChunkHeader, Vec<u8>)>> {
        // First, read the file metadata to get the meta_offset boundary.
        // meta_offset is the position right after all chunk data (before TimeseriesMetadata).
        let meta_offset = {
            let metadata = self.read_file_metadata()?;
            metadata.meta_offset as u64
        };

        // Seek past magic + version
        let start = TsFileConstant::MAGIC_STRING.len() + 1;
        self.reader.seek(SeekFrom::Start(start as u64))?;

        let mut results = Vec::new();
        let mut current_device = String::new();

        loop {
            // Stop if we've reached the metadata region
            let current_pos = self.reader.seek(SeekFrom::Current(0))?;
            if current_pos >= meta_offset {
                break;
            }

            let mut marker_buf = [0u8; 1];
            match self.reader.read_exact(&mut marker_buf) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(TsFileError::Io(e)),
            }
            let marker = marker_buf[0];

            match marker {
                MetaMarker::CHUNK_GROUP_HEADER => {
                    let header = ChunkGroupHeader::deserialize(&mut self.reader)?;
                    current_device = header.device_id;
                }
                MetaMarker::CHUNK_HEADER
                | MetaMarker::ONLY_ONE_PAGE_CHUNK_HEADER
                | MetaMarker::TIME_CHUNK_HEADER
                | MetaMarker::VALUE_CHUNK_HEADER
                | MetaMarker::ONLY_ONE_PAGE_TIME_CHUNK_HEADER
                | MetaMarker::ONLY_ONE_PAGE_VALUE_CHUNK_HEADER => {
                    let chunk_header = ChunkHeader::deserialize(&mut self.reader, marker)?;
                    let data_size = chunk_header.data_size as usize;
                    let mut chunk_data = vec![0u8; data_size];
                    self.reader.read_exact(&mut chunk_data)?;
                    results.push((current_device.clone(), chunk_header, chunk_data));
                }
                MetaMarker::SEPARATOR => {
                    // SEPARATOR marks the boundary between chunk data and metadata section.
                    // In Java format, there is only ONE SEPARATOR, written just before the metadata.
                    // Stop scanning here.
                    break;
                }
                MetaMarker::OPERATION_INDEX_RANGE => {
                    // Skip 16 bytes (two longs)
                    let mut skip_buf = [0u8; 16];
                    self.reader.read_exact(&mut skip_buf)?;
                }
                _ => {
                    // Unknown marker, stop scanning
                    break;
                }
            }
        }

        Ok(results)
    }

    /// Read all chunks as structured `ChunkInfo` records.
    pub fn read_chunk_infos(&mut self) -> TsFileResult<Vec<ChunkInfo>> {
        Ok(self
            .read_all_chunks()?
            .into_iter()
            .map(|(device_id, chunk_header, chunk_data)| ChunkInfo {
                device_id,
                chunk_header,
                chunk_data,
            })
            .collect())
    }

    /// Read and decode all points, organized as device -> measurement -> points.
    pub fn read_all_data(&mut self) -> TsFileResult<DeviceChunkMap> {
        let mut data_map: DeviceChunkMap = HashMap::new();
        for chunk_info in self.read_chunk_infos()? {
            let measurement_id = chunk_info.chunk_header.measurement_id.clone();
            let points = Self::read_chunk_data(&chunk_info.chunk_header, &chunk_info.chunk_data)?;
            data_map
                .entry(chunk_info.device_id)
                .or_default()
                .entry(measurement_id)
                .or_default()
                .extend(points);
        }
        for measurement_map in data_map.values_mut() {
            for points in measurement_map.values_mut() {
                points.sort_by_key(|point| point.timestamp);
            }
        }
        Ok(data_map)
    }

    /// Read all time-value pairs from a chunk.
    pub fn read_chunk_data(
        chunk_header: &ChunkHeader,
        chunk_data: &[u8],
    ) -> TsFileResult<Vec<TimeValuePair>> {
        ChunkReader::new(chunk_header.clone(), chunk_data.to_vec()).read_all()
    }
}

/// Organizes chunk data per device and measurement.
pub type DeviceChunkMap = HashMap<String, HashMap<String, Vec<TimeValuePair>>>;
