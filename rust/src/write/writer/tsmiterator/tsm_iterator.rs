use crate::error::TsFileResult;
use crate::file::metadata::timeseries_metadata::TimeseriesMetadata;

pub trait TSMIterator {
    fn next_metadata(&mut self) -> TsFileResult<Option<TimeseriesMetadata>>;
}
