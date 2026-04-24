use crate::error::TsFileResult;
use crate::write::record::TSRecord;
use crate::write::schema::MeasurementSchema;

pub trait IChunkGroupWriter {
    fn device_id(&self) -> &str;
    fn register_schema(&mut self, schema: MeasurementSchema);
    fn write(&mut self, record: &TSRecord, schemas: &[MeasurementSchema]) -> TsFileResult<()>;
}

impl IChunkGroupWriter for crate::write::chunk::ChunkGroupWriter {
    fn device_id(&self) -> &str { self.device_id() }
    fn register_schema(&mut self, schema: MeasurementSchema) { self.register_schema(schema); }
    fn write(&mut self, record: &TSRecord, schemas: &[MeasurementSchema]) -> TsFileResult<()> { self.write(record, schemas) }
}
