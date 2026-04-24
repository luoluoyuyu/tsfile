use crate::error::TsFileResult;
use crate::file::metadata::table_schema::TableSchema;
use crate::write::tablet::Tablet;
use crate::write::v4::{DeviceTableModelWriter, ITsFileWriter};

pub trait AbstractTableModelTsFileWriter: ITsFileWriter {
    fn table_schema(&self) -> &TableSchema;
    fn write_table_model_tablet(&mut self, tablet: &Tablet) -> TsFileResult<usize> {
        self.write_tablet(tablet)
    }
}

impl AbstractTableModelTsFileWriter for DeviceTableModelWriter {
    fn table_schema(&self) -> &TableSchema { self.table_schema() }
}
