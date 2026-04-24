// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TsFileReader: high-level API for reading TsFiles.
//!
//! Mirrors Java's TsFileReader.

use std::collections::HashMap;
use std::path::Path;

use crate::error::TsFileResult;
use crate::read::time_value_pair::TimeValuePair;
use crate::read::tsfile_sequence_reader::TsFileSequenceReader;

/// High-level TsFile reader.
///
/// Usage:
/// ```no_run
/// use tsfile::read::TsFileReader;
///
/// let mut reader = TsFileReader::new("data.tsfile").unwrap();
/// let data = reader.read_timeseries("device1", "temperature").unwrap();
/// for pair in data {
///     println!("ts={}, val={:?}", pair.timestamp, pair.value);
/// }
/// ```
pub struct TsFileReader {
    /// Sequence reader for low-level access.
    sequence_reader: TsFileSequenceReader,
    /// Cached data: device -> measurement -> time-value pairs.
    cache: Option<HashMap<String, HashMap<String, Vec<TimeValuePair>>>>,
}

impl TsFileReader {
    /// Open a TsFile for reading.
    pub fn new<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        let sequence_reader = TsFileSequenceReader::new(path)?;
        Ok(TsFileReader {
            sequence_reader,
            cache: None,
        })
    }

    /// Load all data into memory cache.
    fn load_cache(&mut self) -> TsFileResult<()> {
        if self.cache.is_some() {
            return Ok(());
        }

        let mut data_map: HashMap<String, HashMap<String, Vec<TimeValuePair>>> = HashMap::new();
        let chunks = self.sequence_reader.read_all_chunks()?;

        for (device_id, chunk_header, chunk_data) in chunks {
            let measurement_id = chunk_header.measurement_id.clone();
            let pairs =
                TsFileSequenceReader::read_chunk_data(&chunk_header, &chunk_data)?;

            data_map
                .entry(device_id)
                .or_default()
                .entry(measurement_id)
                .or_default()
                .extend(pairs);
        }

        // Sort all time-value pairs by timestamp
        for device_map in data_map.values_mut() {
            for pairs in device_map.values_mut() {
                pairs.sort_by_key(|p| p.timestamp);
            }
        }

        self.cache = Some(data_map);
        Ok(())
    }

    /// Read all time-value pairs for a specific device and measurement.
    pub fn read_timeseries(
        &mut self,
        device_id: &str,
        measurement_id: &str,
    ) -> TsFileResult<Vec<TimeValuePair>> {
        self.load_cache()?;
        let cache = self.cache.as_ref().unwrap();
        Ok(cache
            .get(device_id)
            .and_then(|d| d.get(measurement_id))
            .cloned()
            .unwrap_or_default())
    }

    /// Get all device IDs in the file.
    pub fn get_all_devices(&mut self) -> TsFileResult<Vec<String>> {
        self.load_cache()?;
        let cache = self.cache.as_ref().unwrap();
        Ok(cache.keys().cloned().collect())
    }

    /// Get all measurement IDs for a device.
    pub fn get_measurements_for_device(
        &mut self,
        device_id: &str,
    ) -> TsFileResult<Vec<String>> {
        self.load_cache()?;
        let cache = self.cache.as_ref().unwrap();
        Ok(cache
            .get(device_id)
            .map(|d| d.keys().cloned().collect())
            .unwrap_or_default())
    }

    /// Read all data in the file.
    pub fn read_all(
        &mut self,
    ) -> TsFileResult<&HashMap<String, HashMap<String, Vec<TimeValuePair>>>> {
        self.load_cache()?;
        Ok(self.cache.as_ref().unwrap())
    }
}
