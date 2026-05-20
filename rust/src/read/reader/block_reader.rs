// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! TsBlock readers mirroring Java's block-reader layer.

use crate::error::TsFileResult;
use crate::read::block::TsBlock;

pub trait TsBlockReader {
    fn has_next(&self) -> bool;
    fn next(&mut self) -> TsFileResult<Option<TsBlock>>;

    fn close(&mut self) -> TsFileResult<()> {
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EmptyTsBlockReader;

impl TsBlockReader for EmptyTsBlockReader {
    fn has_next(&self) -> bool {
        false
    }

    fn next(&mut self) -> TsFileResult<Option<TsBlock>> {
        Ok(None)
    }
}

#[derive(Debug, Clone)]
pub struct VecTsBlockReader {
    blocks: Vec<TsBlock>,
    cursor: usize,
}

impl VecTsBlockReader {
    pub fn new(blocks: Vec<TsBlock>) -> Self {
        VecTsBlockReader { blocks, cursor: 0 }
    }

    pub fn blocks(&self) -> &[TsBlock] {
        &self.blocks
    }
}

impl TsBlockReader for VecTsBlockReader {
    fn has_next(&self) -> bool {
        self.cursor < self.blocks.len()
    }

    fn next(&mut self) -> TsFileResult<Option<TsBlock>> {
        if !self.has_next() {
            return Ok(None);
        }
        let block = self.blocks[self.cursor].clone();
        self.cursor += 1;
        Ok(Some(block))
    }
}
