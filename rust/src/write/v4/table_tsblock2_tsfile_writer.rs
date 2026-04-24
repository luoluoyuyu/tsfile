use crate::error::TsFileResult;
use crate::read::block::tsblock::TsBlock;
use crate::write::tablet::Tablet;
use crate::write::v4::{DeviceTableModelWriter, ITsFileWriter};

pub struct TableTsBlock2TsFileWriter {
    inner: DeviceTableModelWriter,
}

impl TableTsBlock2TsFileWriter {
    pub fn new(inner: DeviceTableModelWriter) -> Self { TableTsBlock2TsFileWriter { inner } }
    pub fn write_tsblock(&mut self, _tsblock: &TsBlock) -> TsFileResult<usize> { Ok(0) }
    pub fn write_tablet(&mut self, tablet: &Tablet) -> TsFileResult<usize> { self.inner.write_tablet(tablet) }
    pub fn close(self) -> TsFileResult<()> { self.inner.close() }
}
