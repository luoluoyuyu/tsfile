use crate::write::record::DataPoint;
#[derive(Debug, Clone)]
pub struct DateDataPoint { inner: DataPoint }
impl DateDataPoint { pub fn new(measurement_id: String, value: i32) -> Self { Self { inner: DataPoint::new_i32(measurement_id, value) } } pub fn into_data_point(self) -> DataPoint { self.inner } pub fn inner(&self) -> &DataPoint { &self.inner } }
