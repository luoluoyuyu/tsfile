use std::io::Write;
use crate::error::TsFileResult;
use crate::file::metadata::statistics::Statistics;
use crate::utils::read_write_io_utils::Binary;

pub trait IChunkWriter {
    fn write_bool(&mut self, timestamp: i64, value: bool) -> TsFileResult<()>;
    fn write_i32(&mut self, timestamp: i64, value: i32) -> TsFileResult<()>;
    fn write_i64(&mut self, timestamp: i64, value: i64) -> TsFileResult<()>;
    fn write_f32(&mut self, timestamp: i64, value: f32) -> TsFileResult<()>;
    fn write_f64(&mut self, timestamp: i64, value: f64) -> TsFileResult<()>;
    fn write_binary(&mut self, timestamp: i64, value: Binary) -> TsFileResult<()>;
    fn write_to<W: Write>(&mut self, writer: &mut W) -> TsFileResult<(usize, usize)>;
    fn statistics(&self) -> &Statistics;
    fn has_data(&self) -> bool;
}

impl IChunkWriter for crate::write::chunk::ChunkWriter {
    fn write_bool(&mut self, timestamp: i64, value: bool) -> TsFileResult<()> { self.write_bool(timestamp, value) }
    fn write_i32(&mut self, timestamp: i64, value: i32) -> TsFileResult<()> { self.write_i32(timestamp, value) }
    fn write_i64(&mut self, timestamp: i64, value: i64) -> TsFileResult<()> { self.write_i64(timestamp, value) }
    fn write_f32(&mut self, timestamp: i64, value: f32) -> TsFileResult<()> { self.write_f32(timestamp, value) }
    fn write_f64(&mut self, timestamp: i64, value: f64) -> TsFileResult<()> { self.write_f64(timestamp, value) }
    fn write_binary(&mut self, timestamp: i64, value: Binary) -> TsFileResult<()> { self.write_binary(timestamp, value) }
    fn write_to<W: Write>(&mut self, writer: &mut W) -> TsFileResult<(usize, usize)> { self.write_to(writer) }
    fn statistics(&self) -> &Statistics { self.statistics() }
    fn has_data(&self) -> bool { self.has_data() }
}
