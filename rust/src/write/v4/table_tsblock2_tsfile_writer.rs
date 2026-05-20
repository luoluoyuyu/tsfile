use std::collections::HashSet;

use crate::common::enums::TSDataType;
use crate::error::{TsFileError, TsFileResult};
use crate::file::metadata::ColumnCategory;
use crate::read::block::column::ColumnValue;
use crate::read::block::tsblock::TsBlock;
use crate::write::record::{DataPoint, TSRecord};
use crate::write::tablet::Tablet;
use crate::write::v4::{DeviceTableModelWriter, ITsFileWriter};

const NULL_DEVICE_SEGMENT: &str = "__null__";

pub struct TableTsBlock2TsFileWriter {
    inner: DeviceTableModelWriter,
    row_count: usize,
    devices: HashSet<String>,
}

impl TableTsBlock2TsFileWriter {
    pub fn new(inner: DeviceTableModelWriter) -> Self {
        TableTsBlock2TsFileWriter {
            inner,
            row_count: 0,
            devices: HashSet::new(),
        }
    }

    pub fn write_tsblock(&mut self, tsblock: &TsBlock) -> TsFileResult<usize> {
        if tsblock.is_empty() {
            return Ok(0);
        }

        let table_schema = self.inner.table_schema();
        if tsblock.value_column_count() != table_schema.measurement_schemas.len() {
            return Err(TsFileError::WriteError(format!(
                "TsBlock column count mismatch for table {}: expected {}, got {}",
                table_schema.table_name,
                table_schema.measurement_schemas.len(),
                tsblock.value_column_count()
            )));
        }

        let table_name = table_schema.table_name.clone();
        let measurement_schemas = table_schema.measurement_schemas.clone();
        let column_categories = table_schema.column_categories.clone();
        let mut written = 0usize;

        for row_index in 0..tsblock.position_count() {
            let timestamp = tsblock.time_by_index(row_index).ok_or_else(|| {
                TsFileError::WriteError(format!(
                    "TsBlock time column missing row {} for table {}",
                    row_index, table_name
                ))
            })?;
            let device_id = build_device_id(
                &table_name,
                &measurement_schemas,
                &column_categories,
                tsblock,
                row_index,
            )?;
            let mut record = TSRecord::new(timestamp, device_id.clone());

            for (column_index, measurement_schema) in measurement_schemas.iter().enumerate() {
                let column = tsblock.column(column_index).ok_or_else(|| {
                    TsFileError::WriteError(format!(
                        "TsBlock value column {} missing for table {}",
                        column_index, table_name
                    ))
                })?;
                let Some(value) = column.get(row_index) else {
                    return Err(TsFileError::WriteError(format!(
                        "TsBlock row {} missing value for column {}",
                        row_index, measurement_schema.measurement_id
                    )));
                };
                if let Some(data_point) =
                    column_value_to_data_point(&measurement_schema.measurement_id, measurement_schema.data_type, value)?
                {
                    record.add_tuple(data_point);
                }
            }

            if self.inner.write_record(record)? {
                self.row_count += 1;
                self.devices.insert(device_id);
                written += 1;
            }
        }

        Ok(written)
    }

    pub fn write_tablet(&mut self, tablet: &Tablet) -> TsFileResult<usize> {
        let written = self.inner.write_tablet(tablet)?;
        if written > 0 {
            self.row_count += written;
            self.devices.insert(tablet.device_id.clone());
        }
        Ok(written)
    }

    pub fn write_record(&mut self, record: TSRecord) -> TsFileResult<bool> {
        let device_id = record.device_id.clone();
        let written = self.inner.write_record(record)?;
        if written {
            self.row_count += 1;
            self.devices.insert(device_id);
        }
        Ok(written)
    }

    pub fn row_count(&self) -> usize {
        self.row_count
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn flush(&mut self) -> TsFileResult<()> {
        self.inner.flush()
    }

    pub fn close(self) -> TsFileResult<()> {
        self.inner.close()
    }
}

fn build_device_id(
    table_name: &str,
    measurement_schemas: &[crate::write::schema::MeasurementSchema],
    column_categories: &[ColumnCategory],
    tsblock: &TsBlock,
    row_index: usize,
) -> TsFileResult<String> {
    let mut segments = vec![table_name.to_string()];
    for (column_index, column_category) in column_categories.iter().enumerate() {
        if *column_category != ColumnCategory::Tag {
            continue;
        }
        let column = tsblock.column(column_index).ok_or_else(|| {
            TsFileError::WriteError(format!(
                "Missing tag column {} ({}) in TsBlock",
                column_index, measurement_schemas[column_index].measurement_id
            ))
        })?;
        let segment = match column.get(row_index) {
            Some(ColumnValue::Null) | None => NULL_DEVICE_SEGMENT.to_string(),
            Some(ColumnValue::Binary(value)) => value.to_string(),
            Some(other) => {
                return Err(TsFileError::TypeMismatch {
                    expected: "TEXT".to_string(),
                    got: format!("{:?}", other),
                });
            }
        };
        segments.push(segment);
    }
    Ok(segments.join("."))
}

fn column_value_to_data_point(
    measurement_id: &str,
    data_type: TSDataType,
    value: &ColumnValue,
) -> TsFileResult<Option<DataPoint>> {
    let data_point = match (data_type, value) {
        (_, ColumnValue::Null) => return Ok(None),
        (TSDataType::Boolean, ColumnValue::Boolean(value)) => {
            DataPoint::new_bool(measurement_id.to_string(), *value)
        }
        (TSDataType::Int32 | TSDataType::Date, ColumnValue::Int32(value)) => {
            DataPoint::new_i32(measurement_id.to_string(), *value)
        }
        (TSDataType::Int64 | TSDataType::Timestamp, ColumnValue::Int64(value)) => {
            DataPoint::new_i64(measurement_id.to_string(), *value)
        }
        (TSDataType::Float, ColumnValue::Float(value)) => {
            DataPoint::new_f32(measurement_id.to_string(), *value)
        }
        (TSDataType::Double, ColumnValue::Double(value)) => {
            DataPoint::new_f64(measurement_id.to_string(), *value)
        }
        (TSDataType::Text | TSDataType::Blob | TSDataType::String, ColumnValue::Binary(value)) => {
            DataPoint::new_text(measurement_id.to_string(), value.clone())
        }
        _ => {
            return Err(TsFileError::TypeMismatch {
                expected: data_type.to_string(),
                got: format!("{:?}", value),
            });
        }
    };
    Ok(Some(data_point))
}
