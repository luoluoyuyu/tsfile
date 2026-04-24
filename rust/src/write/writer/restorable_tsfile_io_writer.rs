// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Restorable and force-append writer helpers.

use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::common::constant::TsFileConstant;
use crate::error::{TsFileError, TsFileResult};
use crate::write::writer::TsFileIOWriter;

pub struct RestorableTsFileIOWriter {
    path: PathBuf,
}

impl RestorableTsFileIOWriter {
    pub fn new<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            let _writer = TsFileIOWriter::new(&path)?;
        }
        Ok(RestorableTsFileIOWriter { path })
    }

    pub fn into_inner(self) -> TsFileResult<TsFileIOWriter> {
        TsFileIOWriter::new(self.path)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub struct ForceAppendTsFileWriter {
    path: PathBuf,
}

impl ForceAppendTsFileWriter {
    pub fn new<P: AsRef<Path>>(path: P) -> TsFileResult<Self> {
        let path = path.as_ref().to_path_buf();
        validate_tsfile_magic(&path)?;
        Ok(ForceAppendTsFileWriter { path })
    }

    pub fn truncate_footer(&self) -> TsFileResult<u64> {
        let mut file = OpenOptions::new().read(true).write(true).open(&self.path)?;
        let len = file.metadata()?.len();
        let magic_len = TsFileConstant::MAGIC_STRING.len() as u64;
        if len < magic_len + 4 {
            return Err(TsFileError::InvalidFileFormat(
                "TsFile is too small to contain footer".to_string(),
            ));
        }
        file.seek(SeekFrom::End(-((magic_len + 4) as i64)))?;
        let mut size_buf = [0u8; 4];
        std::io::Read::read_exact(&mut file, &mut size_buf)?;
        let metadata_size = i32::from_be_bytes(size_buf) as u64;
        let truncate_position = len
            .checked_sub(magic_len + 4 + metadata_size)
            .ok_or_else(|| TsFileError::InvalidFileFormat("Invalid footer size".to_string()))?;
        file.set_len(truncate_position)?;
        file.seek(SeekFrom::Start(truncate_position))?;
        file.flush()?;
        Ok(truncate_position)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn validate_tsfile_magic(path: &Path) -> TsFileResult<()> {
    let mut file = OpenOptions::new().read(true).open(path)?;
    let mut magic = vec![0u8; TsFileConstant::MAGIC_STRING.len()];
    std::io::Read::read_exact(&mut file, &mut magic)?;
    if magic != TsFileConstant::MAGIC_STRING.as_bytes() {
        return Err(TsFileError::InvalidMagicString);
    }
    Ok(())
}
