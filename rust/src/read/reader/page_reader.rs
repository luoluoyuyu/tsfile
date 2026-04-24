// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! PageReader: decodes one TsFile page into time-value pairs.

use crate::common::enums::{TSDataType, TSEncoding};
use crate::encoding::decoder::{create_decoder, read_next_value, DecodedValue};
use crate::error::{TsFileError, TsFileResult};
use crate::read::time_value_pair::{TimeValue, TimeValuePair};
use crate::utils::ReadWriteIOUtils;

pub struct PageReader {
    data_type: TSDataType,
    value_encoding: TSEncoding,
    page_data: Vec<u8>,
}

impl PageReader {
    pub fn new(data_type: TSDataType, value_encoding: TSEncoding, page_data: Vec<u8>) -> Self {
        PageReader {
            data_type,
            value_encoding,
            page_data,
        }
    }

    pub fn read_all(&self) -> TsFileResult<Vec<TimeValuePair>> {
        let mut page_cursor = std::io::Cursor::new(&self.page_data);
        let time_len = ReadWriteIOUtils::read_i32(&mut page_cursor)? as usize;
        let time_data_start = page_cursor.position() as usize;
        let time_data_end = time_data_start.checked_add(time_len).ok_or_else(|| {
            TsFileError::DecodingError("Time data length overflow".to_string())
        })?;
        if time_data_end > self.page_data.len() {
            return Err(TsFileError::DecodingError(
                "Time data exceeds page boundary".to_string(),
            ));
        }

        let time_data = &self.page_data[time_data_start..time_data_end];
        let value_data = &self.page_data[time_data_end..];
        let mut time_decoder = create_decoder(TSDataType::Int64, TSEncoding::Ts2diff);
        let mut value_decoder = create_decoder(self.data_type, self.value_encoding);
        time_decoder.init(time_data)?;
        value_decoder.init(value_data)?;

        let mut results = Vec::new();
        while time_decoder.has_next() && value_decoder.has_next() {
            let timestamp = time_decoder.read_i64()?;
            let value = time_value_from_decoded(read_next_value(value_decoder.as_mut(), self.data_type)?);
            results.push(TimeValuePair::new(timestamp, value));
        }
        Ok(results)
    }
}

fn time_value_from_decoded(value: DecodedValue) -> TimeValue {
    match value {
        DecodedValue::Boolean(value) => TimeValue::Boolean(value),
        DecodedValue::Int32(value) => TimeValue::Int32(value),
        DecodedValue::Int64(value) => TimeValue::Int64(value),
        DecodedValue::Float(value) => TimeValue::Float(value),
        DecodedValue::Double(value) => TimeValue::Double(value),
        DecodedValue::Binary(value) => TimeValue::Text(value),
        DecodedValue::Null => TimeValue::Null,
    }
}
