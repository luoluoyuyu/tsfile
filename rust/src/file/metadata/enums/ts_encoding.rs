// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Java-compatible metadata/enums/TSEncoding module.

pub use crate::common::enums::TSEncoding;

pub const PLAIN: TSEncoding = TSEncoding::Plain;
pub const DICTIONARY: TSEncoding = TSEncoding::Dictionary;
pub const RLE: TSEncoding = TSEncoding::Rle;
pub const DIFF: TSEncoding = TSEncoding::Diff;
pub const TS_2DIFF: TSEncoding = TSEncoding::Ts2diff;
pub const BITMAP: TSEncoding = TSEncoding::Bitmap;
pub const GORILLA: TSEncoding = TSEncoding::Gorilla;
pub const ZIGZAG: TSEncoding = TSEncoding::Zigzag;
pub const CHIMP: TSEncoding = TSEncoding::Chimp;
pub const SPRINTZ: TSEncoding = TSEncoding::Sprintz;
pub const RLBE: TSEncoding = TSEncoding::Rlbe;
pub const CAMEL: TSEncoding = TSEncoding::Camel;
