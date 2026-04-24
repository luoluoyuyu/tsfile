use std::collections::VecDeque;
use crate::error::TsFileResult;
use crate::file::metadata::timeseries_metadata::TimeseriesMetadata;
use crate::write::writer::tsmiterator::tsm_iterator::TSMIterator;

pub struct DiskTSMIterator {
    queue: VecDeque<TimeseriesMetadata>,
}

impl DiskTSMIterator {
    pub fn new(metadata: Vec<TimeseriesMetadata>) -> Self {
        DiskTSMIterator { queue: metadata.into() }
    }
}

impl TSMIterator for DiskTSMIterator {
    fn next_metadata(&mut self) -> TsFileResult<Option<TimeseriesMetadata>> {
        Ok(self.queue.pop_front())
    }
}
