use crate::write::record::DataPoint;
#[derive(Debug, Clone)]
pub struct StringDataPoint { inner: DataPoint }
impl StringDataPoint { pub fn new(measurement_id: String, value: String) -> Self { Self { inner: DataPoint::new_string(measurement_id, value) } } pub fn into_data_point(self) -> DataPoint { self.inner } pub fn inner(&self) -> &DataPoint { &self.inner } }
