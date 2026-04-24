// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Shared metadata traits mirroring Java's metadata interfaces.

use std::io::Write;

use crate::common::enums::TSDataType;
use crate::error::TsFileResult;
use crate::file::metadata::chunk_metadata::ChunkMetadata;
use crate::file::metadata::statistics::Statistics;

/// Common metadata object exposing statistics.
pub trait Metadata {
    fn statistics(&self) -> &Statistics;

    fn time_statistics(&self) -> Option<&Statistics> {
        Some(self.statistics())
    }

    fn measurement_statistics(&self, measurement_index: usize) -> Option<&Statistics> {
        (measurement_index == 0).then(|| self.statistics())
    }

    fn has_null_value(&self, measurement_index: usize) -> bool {
        self.measurement_statistics(measurement_index).is_none()
    }

    fn time_all_selected(&self) -> bool {
        true
    }

    fn measurement_count(&self) -> usize {
        1
    }
}

/// Common chunk metadata view, corresponding to Java's `IChunkMetadata`.
pub trait ChunkMetadataView: Metadata {
    fn is_modified(&self) -> bool;
    fn set_modified(&mut self, modified: bool);
    fn is_data_type_modified_and_cannot_use_statistics(&self) -> bool;
    fn set_data_type_modified_and_cannot_use_statistics(&mut self, value: bool);
    fn is_seq(&self) -> bool;
    fn set_seq(&mut self, seq: bool);
    fn version(&self) -> i64;
    fn set_version(&mut self, version: i64);
    fn offset_of_chunk_header(&self) -> i64;
    fn start_time(&self) -> i64;
    fn end_time(&self) -> i64;
    fn need_set_chunk_loader(&self) -> bool;
    fn set_closed(&mut self, closed: bool);
    fn data_type(&self) -> TSDataType;
    fn new_type(&self) -> Option<TSDataType>;
    fn set_new_type(&mut self, new_type: TSDataType) -> TsFileResult<()>;
    fn measurement_uid(&self) -> &str;
    fn set_measurement_uid(&mut self, measurement_uid: String) -> TsFileResult<()>;
    fn serialize_to<W: Write>(&self, writer: &mut W, serialize_statistics: bool) -> TsFileResult<usize>;
    fn mask(&self) -> u8;
}

/// Common timeseries metadata view, corresponding to Java's `ITimeSeriesMetadata`.
pub trait TimeSeriesMetadataView: Metadata {
    fn measurement_id(&self) -> &str;
    fn data_type(&self) -> TSDataType;
    fn is_modified(&self) -> bool;
    fn set_modified(&mut self, modified: bool);
    fn is_data_type_modified_and_cannot_use_statistics(&self) -> bool;
    fn set_data_type_modified_and_cannot_use_statistics(&mut self, value: bool);
    fn is_seq(&self) -> bool;
    fn set_seq(&mut self, seq: bool);
    fn load_chunk_metadata_list(&self) -> Vec<ChunkMetadata>;
    fn type_match(&mut self, data_types: &[TSDataType]) -> bool;
}
