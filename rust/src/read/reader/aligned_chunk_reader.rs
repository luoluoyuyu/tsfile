// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Aligned chunk reader placeholder with concrete column assembly from decoded chunks.

use crate::error::TsFileResult;
use crate::read::block::{ColumnValue, TsBlock, TsBlockBuilder};
use crate::read::reader::ChunkReader;
use crate::read::time_value_pair::TimeValuePair;

pub struct AlignedChunkReader {
    time_points: Vec<TimeValuePair>,
    value_chunks: Vec<ChunkReader>,
}

impl AlignedChunkReader {
    pub fn new(time_points: Vec<TimeValuePair>, value_chunks: Vec<ChunkReader>) -> Self {
        AlignedChunkReader { time_points, value_chunks }
    }

    pub fn read_tsblock(&self) -> TsFileResult<TsBlock> {
        let value_series: Vec<Vec<TimeValuePair>> = self
            .value_chunks
            .iter()
            .map(ChunkReader::read_all)
            .collect::<TsFileResult<Vec<_>>>()?;
        let data_types = value_series
            .iter()
            .map(|series| infer_column_type(series))
            .collect();
        let mut builder = TsBlockBuilder::new(data_types);
        for time_pair in &self.time_points {
            let values = value_series
                .iter()
                .map(|series| {
                    series
                        .iter()
                        .find(|pair| pair.timestamp == time_pair.timestamp)
                        .map(|pair| pair.value.clone().into())
                        .unwrap_or(ColumnValue::Null)
                })
                .collect();
            builder.declare_position(time_pair.timestamp, values);
        }
        Ok(builder.build())
    }
}

fn infer_column_type(series: &[TimeValuePair]) -> crate::common::enums::TSDataType {
    series
        .iter()
        .find_map(|pair| match pair.value {
            crate::read::time_value_pair::TimeValue::Boolean(_) => Some(crate::common::enums::TSDataType::Boolean),
            crate::read::time_value_pair::TimeValue::Int32(_) => Some(crate::common::enums::TSDataType::Int32),
            crate::read::time_value_pair::TimeValue::Int64(_) => Some(crate::common::enums::TSDataType::Int64),
            crate::read::time_value_pair::TimeValue::Float(_) => Some(crate::common::enums::TSDataType::Float),
            crate::read::time_value_pair::TimeValue::Double(_) => Some(crate::common::enums::TSDataType::Double),
            crate::read::time_value_pair::TimeValue::Text(_) => Some(crate::common::enums::TSDataType::Text),
            crate::read::time_value_pair::TimeValue::Null => None,
        })
        .unwrap_or(crate::common::enums::TSDataType::NullType)
}
