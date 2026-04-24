use crate::common::enums::{CompressionType, TSDataType, TSEncoding};
use crate::write::schema::MeasurementSchema;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorMeasurementSchema {
    pub measurement_id: String,
    pub sub_measurements: Vec<MeasurementSchema>,
}

impl VectorMeasurementSchema {
    pub fn new(measurement_id: String, sub_measurements: Vec<MeasurementSchema>) -> Self {
        VectorMeasurementSchema { measurement_id, sub_measurements }
    }

    pub fn from_names(measurement_id: String, names: Vec<String>, data_types: Vec<TSDataType>, encodings: Vec<TSEncoding>, compression: CompressionType) -> Self {
        let sub_measurements = names.into_iter().enumerate().map(|(index, name)| {
            MeasurementSchema::new(
                name,
                data_types.get(index).copied().unwrap_or(TSDataType::Text),
                encodings.get(index).copied().unwrap_or(TSEncoding::Plain),
                compression,
            )
        }).collect();
        VectorMeasurementSchema { measurement_id, sub_measurements }
    }
}
