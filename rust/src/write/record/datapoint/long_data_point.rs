use crate::write::record::DataPoint;
#[derive(Debug, Clone)]
pub struct LongDataPoint { inner: DataPoint }
impl LongDataPoint { pub fn new(measurement_id: String, value: i64) -> Self { Self { inner: DataPoint::new_i64(measurement_id, value) } } pub fn into_data_point(self) -> DataPoint { self.inner } pub fn inner(&self) -> &DataPoint { &self.inner } }
