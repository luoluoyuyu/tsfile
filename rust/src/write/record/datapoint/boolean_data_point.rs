use crate::write::record::DataPoint;
#[derive(Debug, Clone)]
pub struct BooleanDataPoint { inner: DataPoint }
impl BooleanDataPoint { pub fn new(measurement_id: String, value: bool) -> Self { Self { inner: DataPoint::new_bool(measurement_id, value) } } pub fn into_data_point(self) -> DataPoint { self.inner } pub fn inner(&self) -> &DataPoint { &self.inner } }
