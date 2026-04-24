use std::collections::HashMap;
use crate::common::enums::{CompressionType, TSDataType, TSEncoding};

pub trait IMeasurementSchema {
    fn measurement_id(&self) -> &str;
    fn data_type(&self) -> TSDataType;
    fn encoding_type(&self) -> TSEncoding;
    fn compression_type(&self) -> CompressionType;
    fn props(&self) -> &HashMap<String, String>;
}

impl IMeasurementSchema for crate::write::schema::MeasurementSchema {
    fn measurement_id(&self) -> &str { &self.measurement_id }
    fn data_type(&self) -> TSDataType { self.data_type }
    fn encoding_type(&self) -> TSEncoding { self.encoding }
    fn compression_type(&self) -> CompressionType { self.compression }
    fn props(&self) -> &HashMap<String, String> { &self.props }
}
