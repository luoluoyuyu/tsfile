// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Writer API traits mirroring Java write interfaces.

use crate::error::TsFileResult;
use crate::write::record::TSRecord;
use crate::write::schema::MeasurementSchema;
use crate::write::tablet::Tablet;

pub trait DataWriter {
    fn write_record(&mut self, record: TSRecord) -> TsFileResult<bool>;
    fn write_tablet_batch(&mut self, tablet: &Tablet) -> TsFileResult<usize>;
}

pub trait TsFileWriteApi: DataWriter {
    fn register_timeseries(
        &mut self,
        device_id: String,
        schema: MeasurementSchema,
    ) -> TsFileResult<()>;

    fn close(self) -> TsFileResult<()>;
}
