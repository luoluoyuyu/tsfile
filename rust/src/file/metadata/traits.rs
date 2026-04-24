// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Shared metadata traits mirroring Java's metadata interfaces.

use crate::common::enums::TSDataType;
use crate::file::metadata::statistics::Statistics;

/// Common metadata object exposing time statistics.
pub trait Metadata {
    fn time_statistics(&self) -> Option<&Statistics>;
}

/// Common chunk metadata view.
pub trait ChunkMetadataView: Metadata {
    fn measurement_uid(&self) -> &str;
    fn data_type(&self) -> TSDataType;
    fn offset_of_chunk_header(&self) -> i64;
}

/// Common timeseries metadata view.
pub trait TimeSeriesMetadataView: Metadata {
    fn measurement_id(&self) -> &str;
    fn data_type(&self) -> TSDataType;
}
