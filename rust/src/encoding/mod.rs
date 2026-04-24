// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Encoding module: encoders and decoders for various data types.

pub mod encoder;
pub mod decoder;

pub use encoder::{Encoder, PlainEncoder};
pub use decoder::{Decoder, PlainDecoder};
