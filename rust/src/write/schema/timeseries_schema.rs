use crate::write::schema::MeasurementSchema;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeseriesSchema {
    pub device_id: String,
    pub measurement_schema: MeasurementSchema,
}

impl TimeseriesSchema {
    pub fn new(device_id: String, measurement_schema: MeasurementSchema) -> Self {
        TimeseriesSchema { device_id, measurement_schema }
    }
}
