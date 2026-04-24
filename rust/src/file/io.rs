// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TsFile input/output abstractions mirroring Java's TsFileInput/TsFileOutput.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::error::TsFileResult;

pub trait TsFileInput: Read + Seek {
    fn len(&mut self) -> TsFileResult<u64>;
    fn position(&mut self) -> TsFileResult<u64> {
        Ok(self.stream_position()?)
    }
    fn read_exact_at(&mut self, position: u64, buffer: &mut [u8]) -> TsFileResult<()> {
        let current = self.stream_position()?;
        self.seek(SeekFrom::Start(position))?;
        self.read_exact(buffer)?;
        self.seek(SeekFrom::Start(current))?;
        Ok(())
    }
}

pub trait TsFileOutput: Write + Seek {
    fn position(&mut self) -> TsFileResult<u64> {
        Ok(self.stream_position()?)
    }
    fn truncate(&mut self, size: u64) -> TsFileResult<()>;
}

pub struct LocalTsFileInput {
    file: File,
}

impl LocalTsFileInput {
    pub fn open<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        Ok(LocalTsFileInput { file: File::open(path)? })
    }
}

impl Read for LocalTsFileInput {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for LocalTsFileInput {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl TsFileInput for LocalTsFileInput {
    fn len(&mut self) -> TsFileResult<u64> {
        Ok(self.file.metadata()?.len())
    }
}

pub struct LocalTsFileOutput {
    file: File,
}

impl LocalTsFileOutput {
    pub fn create<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        Ok(LocalTsFileOutput { file: File::create(path)? })
    }
}

impl Write for LocalTsFileOutput {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

impl Seek for LocalTsFileOutput {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl TsFileOutput for LocalTsFileOutput {
    fn truncate(&mut self, size: u64) -> TsFileResult<()> {
        self.file.set_len(size)?;
        Ok(())
    }
}
