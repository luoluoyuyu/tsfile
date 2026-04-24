use crate::write::record::DataPoint;
#[derive(Debug, Clone)]
pub struct FloatDataPoint { inner: DataPoint }
impl FloatDataPoint { pub fn new(measurement_id: String, value: f32) -> Self { Self { inner: DataPoint::new_f32(measurement_id, value) } } pub fn into_data_point(self) -> DataPoint { self.inner } pub fn inner(&self) -> &DataPoint { &self.inner } }
