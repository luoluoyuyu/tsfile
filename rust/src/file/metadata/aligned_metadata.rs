// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Aligned chunk and timeseries metadata, corresponding to Java's aligned metadata classes.

use crate::common::enums::TSDataType;
use crate::error::{TsFileError, TsFileResult};
use crate::file::metadata::chunk_metadata::ChunkMetadata;
use crate::file::metadata::statistics::Statistics;
use crate::file::metadata::timeseries_metadata::TimeseriesMetadata;
use crate::file::metadata::traits::{Metadata, TimeSeriesMetadataView};

/// Common behavior for aligned chunk metadata.
pub trait AlignedChunkMetadataView: Metadata {
    fn time_chunk_metadata(&self) -> &ChunkMetadata;
    fn value_chunk_metadata_list(&self) -> &[Option<ChunkMetadata>];

    fn measurement_count(&self) -> usize {
        self.value_chunk_metadata_list().len()
    }

    fn has_null_value(&self, value_index: usize) -> bool {
        let time_count = self.time_chunk_metadata().statistics.count;
        self.value_chunk_metadata_list()
            .get(value_index)
            .and_then(|metadata| metadata.as_ref())
            .is_none_or(|metadata| metadata.statistics.count < time_count)
    }

    fn time_all_selected(&self) -> bool {
        (0..AlignedChunkMetadataView::measurement_count(self)).any(|index| !AlignedChunkMetadataView::has_null_value(self, index))
    }
}

/// Metadata for one aligned chunk: one time chunk plus N value chunks.
#[derive(Debug, Clone)]
pub struct AlignedChunkMetadata {
    pub time_chunk_metadata: ChunkMetadata,
    pub value_chunk_metadata_list: Vec<Option<ChunkMetadata>>,
    pub chunk_loader_set: bool,
}

impl AlignedChunkMetadata {
    pub fn new(
        time_chunk_metadata: ChunkMetadata,
        value_chunk_metadata_list: Vec<Option<ChunkMetadata>>,
    ) -> Self {
        AlignedChunkMetadata {
            time_chunk_metadata,
            value_chunk_metadata_list,
            chunk_loader_set: false,
        }
    }

    pub fn create_new_chunk_metadata(
        &self,
        time_chunk_metadata: ChunkMetadata,
        value_chunk_metadata_list: Vec<Option<ChunkMetadata>>,
    ) -> Self {
        Self::new(time_chunk_metadata, value_chunk_metadata_list)
    }

    pub fn statistics(&self) -> &Statistics {
        if self.value_chunk_metadata_list.len() == 1 {
            if let Some(Some(metadata)) = self.value_chunk_metadata_list.first() {
                return &metadata.statistics;
            }
        }
        &self.time_chunk_metadata.statistics
    }

    pub fn time_statistics_ref(&self) -> &Statistics {
        &self.time_chunk_metadata.statistics
    }

    pub fn measurement_statistics(&self, measurement_index: usize) -> Option<&Statistics> {
        self.value_chunk_metadata_list
            .get(measurement_index)
            .and_then(|metadata| metadata.as_ref())
            .map(|metadata| &metadata.statistics)
    }

    pub fn value_metadata(&self, value_index: usize) -> Option<&ChunkMetadata> {
        self.value_chunk_metadata_list
            .get(value_index)
            .and_then(|metadata| metadata.as_ref())
    }

    pub fn value_metadata_mut(&mut self, value_index: usize) -> Option<&mut ChunkMetadata> {
        self.value_chunk_metadata_list
            .get_mut(value_index)
            .and_then(|metadata| metadata.as_mut())
    }

    pub fn is_modified(&self) -> bool {
        self.time_chunk_metadata.is_modified()
    }

    pub fn set_modified(&mut self, modified: bool) {
        self.time_chunk_metadata.set_modified(modified);
        for metadata in self.value_chunk_metadata_list.iter_mut().flatten() {
            metadata.set_modified(modified);
        }
    }

    pub fn is_seq(&self) -> bool {
        self.time_chunk_metadata.is_seq()
    }

    pub fn set_seq(&mut self, seq: bool) {
        self.time_chunk_metadata.set_seq(seq);
        for metadata in self.value_chunk_metadata_list.iter_mut().flatten() {
            metadata.set_seq(seq);
        }
    }

    pub fn version(&self) -> i64 {
        self.time_chunk_metadata.version()
    }

    pub fn set_version(&mut self, version: i64) {
        self.time_chunk_metadata.set_version(version);
        for metadata in self.value_chunk_metadata_list.iter_mut().flatten() {
            metadata.set_version(version);
        }
    }

    pub fn offset_of_chunk_header(&self) -> i64 {
        self.time_chunk_metadata.offset_of_chunk_header
    }

    pub fn start_time(&self) -> i64 {
        self.time_chunk_metadata.start_time()
    }

    pub fn end_time(&self) -> i64 {
        self.time_chunk_metadata.end_time()
    }

    pub fn need_set_chunk_loader(&self) -> bool {
        !self.chunk_loader_set
    }

    pub fn set_chunk_loader_set(&mut self, value: bool) {
        self.chunk_loader_set = value;
    }

    pub fn set_closed(&mut self, closed: bool) {
        self.time_chunk_metadata.set_closed(closed);
        for metadata in self.value_chunk_metadata_list.iter_mut().flatten() {
            metadata.set_closed(closed);
        }
    }

    pub fn data_type(&self) -> TSDataType {
        self.time_chunk_metadata.data_type
    }

    pub fn measurement_uid(&self) -> &str {
        &self.time_chunk_metadata.measurement_uid
    }

    pub fn set_new_type(&mut self, _data_type: TSDataType) -> TsFileResult<()> {
        Err(TsFileError::UnsupportedOperation(
            "AlignedChunkMetadata doesn't support set_new_type".to_string(),
        ))
    }

    pub fn set_measurement_uid(&mut self, _measurement_uid: String) -> TsFileResult<()> {
        Err(TsFileError::UnsupportedOperation(
            "AlignedChunkMetadata doesn't support set_measurement_uid".to_string(),
        ))
    }

    pub fn merge_statistics(&self) -> Statistics {
        self.statistics().clone()
    }

    pub fn selected_value_metadata(&self, value_indices: &[usize]) -> Vec<Option<ChunkMetadata>> {
        value_indices
            .iter()
            .map(|index| self.value_chunk_metadata_list.get(*index).cloned().unwrap_or(None))
            .collect()
    }

    pub fn select_value_columns(&self, value_indices: &[usize]) -> Self {
        Self::new(
            self.time_chunk_metadata.clone(),
            self.selected_value_metadata(value_indices),
        )
    }

    pub fn type_match(&mut self, data_types: &[TSDataType]) -> bool {
        if data_types.is_empty() {
            return true;
        }
        for (index, data_type) in data_types.iter().enumerate() {
            if let Some(Some(metadata)) = self.value_chunk_metadata_list.get_mut(index) {
                if !metadata.type_match(*data_type) {
                    self.value_chunk_metadata_list[index] = None;
                }
            }
        }
        true
    }
}

impl Metadata for AlignedChunkMetadata {
    fn statistics(&self) -> &Statistics { self.statistics() }
    fn time_statistics(&self) -> Option<&Statistics> { Some(&self.time_chunk_metadata.statistics) }
    fn measurement_statistics(&self, measurement_index: usize) -> Option<&Statistics> { self.measurement_statistics(measurement_index) }
    fn has_null_value(&self, measurement_index: usize) -> bool { AlignedChunkMetadataView::has_null_value(self, measurement_index) }
    fn time_all_selected(&self) -> bool { AlignedChunkMetadataView::time_all_selected(self) }
    fn measurement_count(&self) -> usize { self.value_chunk_metadata_list.len() }
}

impl AlignedChunkMetadataView for AlignedChunkMetadata {
    fn time_chunk_metadata(&self) -> &ChunkMetadata {
        &self.time_chunk_metadata
    }

    fn value_chunk_metadata_list(&self) -> &[Option<ChunkMetadata>] {
        &self.value_chunk_metadata_list
    }
}

/// Common behavior for aligned timeseries metadata.
pub trait AlignedTimeSeriesMetadataView: Metadata {
    fn time_timeseries_metadata(&self) -> &TimeseriesMetadata;
    fn value_timeseries_metadata_list(&self) -> &[Option<TimeseriesMetadata>];

    fn measurement_count(&self) -> usize {
        self.value_timeseries_metadata_list().len()
    }

    fn has_null_value(&self, value_index: usize) -> bool {
        let time_count = self.time_timeseries_metadata().statistics.count;
        self.value_timeseries_metadata_list()
            .get(value_index)
            .and_then(|metadata| metadata.as_ref())
            .is_none_or(|metadata| metadata.statistics.count < time_count)
    }

    fn time_all_selected(&self) -> bool {
        (0..AlignedTimeSeriesMetadataView::measurement_count(self)).any(|index| !AlignedTimeSeriesMetadataView::has_null_value(self, index))
    }
}

/// Metadata for an aligned vector series: time column metadata plus value column metadata.
#[derive(Debug, Clone)]
pub struct AlignedTimeSeriesMetadata {
    pub timeseries_metadata: TimeseriesMetadata,
    pub value_timeseries_metadata_list: Vec<Option<TimeseriesMetadata>>,
    pub chunk_metadata_loaded: bool,
}

impl AlignedTimeSeriesMetadata {
    pub fn new(
        timeseries_metadata: TimeseriesMetadata,
        value_timeseries_metadata_list: Vec<Option<TimeseriesMetadata>>,
    ) -> Self {
        let chunk_metadata_loaded = !timeseries_metadata.chunk_metadata_list.is_empty();
        AlignedTimeSeriesMetadata {
            timeseries_metadata,
            value_timeseries_metadata_list,
            chunk_metadata_loaded,
        }
    }

    pub fn statistics(&self) -> &Statistics {
        if self.value_timeseries_metadata_list.len() == 1 {
            if let Some(Some(metadata)) = self.value_timeseries_metadata_list.first() {
                return &metadata.statistics;
            }
        }
        &self.timeseries_metadata.statistics
    }

    pub fn effective_statistics(&self) -> &Statistics {
        self.statistics()
    }

    pub fn value_metadata(&self, value_index: usize) -> Option<&TimeseriesMetadata> {
        self.value_timeseries_metadata_list
            .get(value_index)
            .and_then(|metadata| metadata.as_ref())
    }

    pub fn value_metadata_mut(&mut self, value_index: usize) -> Option<&mut TimeseriesMetadata> {
        self.value_timeseries_metadata_list
            .get_mut(value_index)
            .and_then(|metadata| metadata.as_mut())
    }

    pub fn is_modified(&self) -> bool {
        self.timeseries_metadata.is_modified()
    }

    pub fn set_modified(&mut self, modified: bool) {
        self.timeseries_metadata.set_modified(modified);
        for metadata in self.value_timeseries_metadata_list.iter_mut().flatten() {
            metadata.set_modified(modified);
        }
    }

    pub fn is_data_type_modified_and_cannot_use_statistics(&self) -> bool {
        self.timeseries_metadata
            .is_data_type_modified_and_cannot_use_statistics()
    }

    pub fn set_data_type_modified_and_cannot_use_statistics(&mut self, value: bool) {
        self.timeseries_metadata
            .set_data_type_modified_and_cannot_use_statistics(value);
    }

    pub fn set_chunk_metadata_loaded(&mut self, loaded: bool) {
        self.chunk_metadata_loaded = loaded;
    }

    pub fn is_chunk_metadata_loaded(&self) -> bool {
        self.chunk_metadata_loaded
    }

    pub fn construct_aligned_chunk_metadata(&self) -> TsFileResult<Vec<AlignedChunkMetadata>> {
        self.construct_aligned_chunk_metadata_with_mode(false)
    }

    pub fn construct_only_time_chunk_metadata(&self) -> Vec<AlignedChunkMetadata> {
        self.timeseries_metadata
            .chunk_metadata_list
            .iter()
            .cloned()
            .map(|time_chunk| AlignedChunkMetadata::new(time_chunk, Vec::new()))
            .collect()
    }

    pub fn construct_aligned_chunk_metadata_with_mode(
        &self,
        include_empty_table_device_chunk: bool,
    ) -> TsFileResult<Vec<AlignedChunkMetadata>> {
        let time_chunks = &self.timeseries_metadata.chunk_metadata_list;
        if time_chunks.is_empty() {
            return Ok(Vec::new());
        }
        if self.value_timeseries_metadata_list.is_empty() {
            return Ok(self.construct_only_time_chunk_metadata());
        }

        let mut result = Vec::new();
        for (chunk_index, time_chunk) in time_chunks.iter().cloned().enumerate() {
            let mut value_chunks = Vec::with_capacity(self.value_timeseries_metadata_list.len());
            let mut exists = false;
            for value_metadata in &self.value_timeseries_metadata_list {
                let value_chunk = value_metadata.as_ref().and_then(|metadata| {
                    metadata
                        .chunk_metadata_list
                        .get(chunk_index)
                        .filter(|chunk| chunk.statistics.count > 0)
                        .cloned()
                });
                exists |= value_chunk.is_some();
                value_chunks.push(value_chunk);
            }
            if exists || include_empty_table_device_chunk {
                result.push(AlignedChunkMetadata::new(time_chunk, value_chunks));
            }
        }
        Ok(result)
    }

    pub fn type_match(&mut self, data_types: &[TSDataType]) -> bool {
        if data_types.is_empty() {
            return true;
        }
        for (index, data_type) in data_types.iter().enumerate() {
            if let Some(Some(metadata)) = self.value_timeseries_metadata_list.get_mut(index) {
                if !TimeseriesMetadata::type_match(metadata, *data_type) {
                    self.value_timeseries_metadata_list[index] = None;
                }
            }
        }
        true
    }
}

impl Metadata for AlignedTimeSeriesMetadata {
    fn statistics(&self) -> &Statistics { self.statistics() }
    fn time_statistics(&self) -> Option<&Statistics> { Some(&self.timeseries_metadata.statistics) }
    fn measurement_statistics(&self, measurement_index: usize) -> Option<&Statistics> {
        self.value_metadata(measurement_index).map(|metadata| &metadata.statistics)
    }
    fn has_null_value(&self, measurement_index: usize) -> bool { AlignedTimeSeriesMetadataView::has_null_value(self, measurement_index) }
    fn time_all_selected(&self) -> bool { AlignedTimeSeriesMetadataView::time_all_selected(self) }
    fn measurement_count(&self) -> usize { self.value_timeseries_metadata_list.len() }
}

impl TimeSeriesMetadataView for AlignedTimeSeriesMetadata {
    fn measurement_id(&self) -> &str { &self.timeseries_metadata.measurement_id }
    fn data_type(&self) -> TSDataType { self.timeseries_metadata.data_type }
    fn is_modified(&self) -> bool { self.is_modified() }
    fn set_modified(&mut self, modified: bool) { self.set_modified(modified); }
    fn is_data_type_modified_and_cannot_use_statistics(&self) -> bool { self.is_data_type_modified_and_cannot_use_statistics() }
    fn set_data_type_modified_and_cannot_use_statistics(&mut self, value: bool) { self.set_data_type_modified_and_cannot_use_statistics(value); }
    fn is_seq(&self) -> bool { self.timeseries_metadata.chunk_metadata_list.iter().all(|chunk| chunk.is_seq()) }
    fn set_seq(&mut self, seq: bool) {
        for chunk in &mut self.timeseries_metadata.chunk_metadata_list { chunk.set_seq(seq); }
        for metadata in self.value_timeseries_metadata_list.iter_mut().flatten() {
            for chunk in &mut metadata.chunk_metadata_list { chunk.set_seq(seq); }
        }
    }
    fn load_chunk_metadata_list(&self) -> Vec<ChunkMetadata> { self.timeseries_metadata.chunk_metadata_list.clone() }
    fn type_match(&mut self, data_types: &[TSDataType]) -> bool { self.type_match(data_types) }
}

impl AlignedTimeSeriesMetadataView for AlignedTimeSeriesMetadata {
    fn time_timeseries_metadata(&self) -> &TimeseriesMetadata {
        &self.timeseries_metadata
    }

    fn value_timeseries_metadata_list(&self) -> &[Option<TimeseriesMetadata>] {
        &self.value_timeseries_metadata_list
    }
}
