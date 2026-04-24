// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! # TsFile Rust Implementation
//!
//! A Rust implementation of the Apache TsFile format - a columnar storage file format
//! designed for time series data.
//!
//! ## Quick Start
//!
//! ### Writing a TsFile
//! ```no_run
//! use tsfile::write::{TsFileWriter, schema::MeasurementSchema};
//! use tsfile::write::record::TSRecord;
//! use tsfile::common::enums::{TSDataType, TSEncoding, CompressionType};
//!
//! let mut writer = TsFileWriter::new("test.tsfile").unwrap();
//! let schema = MeasurementSchema::new(
//!     "temperature".to_string(),
//!     TSDataType::Float,
//!     TSEncoding::Gorilla,
//!     CompressionType::Snappy,
//! );
//! writer.register_timeseries("device1".to_string(), schema).unwrap();
//! // write records...
//! writer.close().unwrap();
//! ```
//!
//! ### Reading a TsFile
//! ```no_run
//! use tsfile::read::TsFileReader;
//!
//! let reader = TsFileReader::new("test.tsfile").unwrap();
//! // read data...
//! ```

pub mod common;
pub mod compress;
pub mod encoding;
pub mod error;
pub mod file;
pub mod read;
pub mod utils;
pub mod write;

pub use error::{TsFileError, TsFileResult};
