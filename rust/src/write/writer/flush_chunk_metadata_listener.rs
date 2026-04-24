use crate::error::TsFileResult;
use crate::file::metadata::chunk_metadata::ChunkMetadata;

pub trait FlushChunkMetadataListener {
    fn on_chunk_metadata_flushed(&mut self, _device_id: &str, _metadata: &ChunkMetadata) -> TsFileResult<()> {
        Ok(())
    }
}
