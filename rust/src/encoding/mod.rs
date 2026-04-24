// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Encoding module: encoders and decoders for various data types.

pub mod encoder;
pub mod decoder;

pub use encoder::{
    create_encoder, DictionaryEncoder, EncodableValue, Encoder, GorillaEncoder, PlainEncoder,
    RleEncoder, Ts2diffEncoder, ZigzagEncoder,
};
pub use decoder::{
    create_decoder, read_next_value, DecodedValue, Decoder, DictionaryDecoder, GorillaDecoder,
    PlainDecoder, RleDecoder, Ts2diffDecoder, ZigzagDecoder,
};
