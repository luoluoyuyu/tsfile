use crate::write::record::DataPoint;
#[derive(Debug, Clone)]
pub struct DoubleDataPoint { inner: DataPoint }
impl DoubleDataPoint { pub fn new(measurement_id: String, value: f64) -> Self { Self { inner: DataPoint::new_f64(measurement_id, value) } } pub fn into_data_point(self) -> DataPoint { self.inner } pub fn inner(&self) -> &DataPoint { &self.inner } }
