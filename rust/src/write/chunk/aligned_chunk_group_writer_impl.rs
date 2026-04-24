// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use crate::write::chunk::AlignedChunkWriter;

pub struct AlignedChunkGroupWriterImpl {
    device_id: String,
    aligned_writers: Vec<AlignedChunkWriter>,
}

impl AlignedChunkGroupWriterImpl {
    pub fn new(device_id: String) -> Self {
        AlignedChunkGroupWriterImpl { device_id, aligned_writers: Vec::new() }
    }
    pub fn device_id(&self) -> &str { &self.device_id }
    pub fn add_aligned_writer(&mut self, writer: AlignedChunkWriter) { self.aligned_writers.push(writer); }
    pub fn aligned_writers(&self) -> &[AlignedChunkWriter] { &self.aligned_writers }
    pub fn into_aligned_writers(self) -> Vec<AlignedChunkWriter> { self.aligned_writers }
}
