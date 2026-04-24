// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Table-model device metadata wrappers.

use crate::common::enums::TSDataType;
use crate::file::metadata::aligned_metadata::{
    AlignedChunkMetadata, AlignedChunkMetadataView, AlignedTimeSeriesMetadata,
    AlignedTimeSeriesMetadataView,
};
use crate::file::metadata::chunk_metadata::ChunkMetadata;
use crate::file::metadata::device_id::DeviceId;
use crate::file::metadata::statistics::Statistics;
use crate::file::metadata::timeseries_metadata::TimeseriesMetadata;
use crate::file::metadata::traits::{Metadata, TimeSeriesMetadataView};

/// Table device chunk metadata: one table-model device's time chunk plus field chunks.
#[derive(Debug, Clone)]
pub struct TableDeviceChunkMetadata {
    pub device_id: DeviceId,
    pub aligned_metadata: AlignedChunkMetadata,
}

impl TableDeviceChunkMetadata {
    pub fn new(
        device_id: DeviceId,
        time_chunk_metadata: ChunkMetadata,
        value_chunk_metadata_list: Vec<Option<ChunkMetadata>>,
    ) -> Self {
        TableDeviceChunkMetadata {
            device_id,
            aligned_metadata: AlignedChunkMetadata::new(
                time_chunk_metadata,
                value_chunk_metadata_list,
            ),
        }
    }

    pub fn create_new_chunk_metadata(
        &self,
        time_chunk_metadata: ChunkMetadata,
        value_chunk_metadata_list: Vec<Option<ChunkMetadata>>,
    ) -> Self {
        Self::new(self.device_id.clone(), time_chunk_metadata, value_chunk_metadata_list)
    }

    pub fn table_name(&self) -> String {
        self.device_id.table_name()
    }

    /// Java TableDeviceChunkMetadata always exposes time statistics as effective statistics.
    pub fn statistics(&self) -> &Statistics {
        &self.aligned_metadata.time_chunk_metadata.statistics
    }

    pub fn value_metadata(&self, value_index: usize) -> Option<&ChunkMetadata> {
        self.aligned_metadata.value_metadata(value_index)
    }

    pub fn measurement_statistics(&self, measurement_index: usize) -> Option<&Statistics> {
        self.aligned_metadata.measurement_statistics(measurement_index)
    }

    pub fn set_modified(&mut self, modified: bool) {
        self.aligned_metadata.set_modified(modified);
    }

    pub fn set_seq(&mut self, seq: bool) {
        self.aligned_metadata.set_seq(seq);
    }

    pub fn set_version(&mut self, version: i64) {
        self.aligned_metadata.set_version(version);
    }

    pub fn set_closed(&mut self, closed: bool) {
        self.aligned_metadata.set_closed(closed);
    }

    pub fn type_match(&mut self, data_types: &[TSDataType]) -> bool {
        self.aligned_metadata.type_match(data_types)
    }
}

impl Metadata for TableDeviceChunkMetadata {
    fn statistics(&self) -> &Statistics { self.statistics() }
    fn time_statistics(&self) -> Option<&Statistics> { self.aligned_metadata.time_statistics() }
    fn measurement_statistics(&self, measurement_index: usize) -> Option<&Statistics> { self.measurement_statistics(measurement_index) }
    fn has_null_value(&self, measurement_index: usize) -> bool { AlignedChunkMetadataView::has_null_value(self, measurement_index) }
    fn time_all_selected(&self) -> bool { AlignedChunkMetadataView::time_all_selected(self) }
    fn measurement_count(&self) -> usize { self.aligned_metadata.value_chunk_metadata_list.len() }
}

impl AlignedChunkMetadataView for TableDeviceChunkMetadata {
    fn time_chunk_metadata(&self) -> &ChunkMetadata {
        &self.aligned_metadata.time_chunk_metadata
    }

    fn value_chunk_metadata_list(&self) -> &[Option<ChunkMetadata>] {
        &self.aligned_metadata.value_chunk_metadata_list
    }
}

/// Table device timeseries metadata used by table-model readers.
#[derive(Debug, Clone)]
pub struct TableDeviceTimeSeriesMetadata {
    pub device_id: DeviceId,
    pub aligned_metadata: AlignedTimeSeriesMetadata,
}

impl TableDeviceTimeSeriesMetadata {
    pub fn new(
        device_id: DeviceId,
        timeseries_metadata: TimeseriesMetadata,
        value_timeseries_metadata_list: Vec<Option<TimeseriesMetadata>>,
    ) -> Self {
        TableDeviceTimeSeriesMetadata {
            device_id,
            aligned_metadata: AlignedTimeSeriesMetadata::new(
                timeseries_metadata,
                value_timeseries_metadata_list,
            ),
        }
    }

    pub fn table_name(&self) -> String {
        self.device_id.table_name()
    }

    /// Java TableDeviceTimeSeriesMetadata always exposes time-column statistics.
    pub fn statistics(&self) -> &Statistics {
        &self.aligned_metadata.timeseries_metadata.statistics
    }

    pub fn value_metadata(&self, value_index: usize) -> Option<&TimeseriesMetadata> {
        self.aligned_metadata.value_metadata(value_index)
    }

    /// Java table model always constructs a TableDeviceChunkMetadata for every time chunk.
    pub fn construct_table_device_chunk_metadata(&self) -> Vec<TableDeviceChunkMetadata> {
        self.aligned_metadata
            .construct_aligned_chunk_metadata_with_mode(true)
            .unwrap_or_default()
            .into_iter()
            .map(|metadata| TableDeviceChunkMetadata {
                device_id: self.device_id.clone(),
                aligned_metadata: metadata,
            })
            .collect()
    }

    pub fn construct_only_time_chunk_metadata(&self) -> Vec<TableDeviceChunkMetadata> {
        self.aligned_metadata
            .construct_only_time_chunk_metadata()
            .into_iter()
            .map(|metadata| TableDeviceChunkMetadata {
                device_id: self.device_id.clone(),
                aligned_metadata: metadata,
            })
            .collect()
    }

    pub fn set_modified(&mut self, modified: bool) {
        self.aligned_metadata.set_modified(modified);
    }

    pub fn type_match(&mut self, data_types: &[TSDataType]) -> bool {
        self.aligned_metadata.type_match(data_types)
    }
}

impl Metadata for TableDeviceTimeSeriesMetadata {
    fn statistics(&self) -> &Statistics { self.statistics() }
    fn time_statistics(&self) -> Option<&Statistics> { self.aligned_metadata.time_statistics() }
    fn measurement_statistics(&self, measurement_index: usize) -> Option<&Statistics> {
        self.aligned_metadata.value_metadata(measurement_index).map(|metadata| &metadata.statistics)
    }
    fn has_null_value(&self, measurement_index: usize) -> bool { AlignedTimeSeriesMetadataView::has_null_value(self, measurement_index) }
    fn time_all_selected(&self) -> bool { AlignedTimeSeriesMetadataView::time_all_selected(self) }
    fn measurement_count(&self) -> usize { self.aligned_metadata.value_timeseries_metadata_list.len() }
}

impl TimeSeriesMetadataView for TableDeviceTimeSeriesMetadata {
    fn measurement_id(&self) -> &str { self.aligned_metadata.measurement_id() }
    fn data_type(&self) -> TSDataType { self.aligned_metadata.data_type() }
    fn is_modified(&self) -> bool { self.aligned_metadata.is_modified() }
    fn set_modified(&mut self, modified: bool) { self.aligned_metadata.set_modified(modified); }
    fn is_data_type_modified_and_cannot_use_statistics(&self) -> bool { self.aligned_metadata.is_data_type_modified_and_cannot_use_statistics() }
    fn set_data_type_modified_and_cannot_use_statistics(&mut self, value: bool) { self.aligned_metadata.set_data_type_modified_and_cannot_use_statistics(value); }
    fn is_seq(&self) -> bool { self.aligned_metadata.timeseries_metadata.chunk_metadata_list.iter().all(|chunk| chunk.is_seq()) }
    fn set_seq(&mut self, seq: bool) {
        for chunk in &mut self.aligned_metadata.timeseries_metadata.chunk_metadata_list { chunk.set_seq(seq); }
    }
    fn load_chunk_metadata_list(&self) -> Vec<ChunkMetadata> { self.aligned_metadata.timeseries_metadata.chunk_metadata_list.clone() }
    fn type_match(&mut self, data_types: &[TSDataType]) -> bool { self.type_match(data_types) }
}

impl AlignedTimeSeriesMetadataView for TableDeviceTimeSeriesMetadata {
    fn time_timeseries_metadata(&self) -> &TimeseriesMetadata {
        &self.aligned_metadata.timeseries_metadata
    }

    fn value_timeseries_metadata_list(&self) -> &[Option<TimeseriesMetadata>] {
        &self.aligned_metadata.value_timeseries_metadata_list
    }
}
