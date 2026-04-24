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

//! Example: write a TsFile with multiple devices and measurements.

use tsfile::common::enums::{CompressionType, TSDataType, TSEncoding};
use tsfile::write::record::{DataPoint, TSRecord};
use tsfile::write::schema::MeasurementSchema;
use tsfile::write::TsFileWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let output_path = "/tmp/example_write.tsfile";
    println!("Writing TsFile to: {}", output_path);

    // 1. Create writer
    let mut writer = TsFileWriter::new(output_path)?;

    // 2. Register schemas for device "device1"
    writer.register_timeseries(
        "device1".to_string(),
        MeasurementSchema::new(
            "temperature".to_string(),
            TSDataType::Float,
            TSEncoding::Gorilla,
            CompressionType::Snappy,
        ),
    )?;

    writer.register_timeseries(
        "device1".to_string(),
        MeasurementSchema::new(
            "humidity".to_string(),
            TSDataType::Double,
            TSEncoding::Gorilla,
            CompressionType::Snappy,
        ),
    )?;

    writer.register_timeseries(
        "device1".to_string(),
        MeasurementSchema::new(
            "status".to_string(),
            TSDataType::Boolean,
            TSEncoding::Rle,
            CompressionType::Uncompressed,
        ),
    )?;

    // Register schema for device "device2"
    writer.register_timeseries(
        "device2".to_string(),
        MeasurementSchema::new(
            "voltage".to_string(),
            TSDataType::Int32,
            TSEncoding::Ts2diff,
            CompressionType::Gzip,
        ),
    )?;

    writer.register_timeseries(
        "device2".to_string(),
        MeasurementSchema::new(
            "current".to_string(),
            TSDataType::Int64,
            TSEncoding::Ts2diff,
            CompressionType::Gzip,
        ),
    )?;

    // 3. Write data for device1
    println!("Writing data for device1...");
    for i in 0..10 {
        let timestamp = 1_000_000 + i as i64 * 1000;
        let mut record = TSRecord::new(timestamp, "device1".to_string());
        record.add_tuple(DataPoint::new_f32(
            "temperature".to_string(),
            25.0 + i as f32 * 0.5,
        ));
        record.add_tuple(DataPoint::new_f64(
            "humidity".to_string(),
            60.0 + i as f64 * 0.2,
        ));
        record.add_tuple(DataPoint::new_bool("status".to_string(), i % 2 == 0));
        writer.write(record)?;
    }

    // 4. Write data for device2
    println!("Writing data for device2...");
    for i in 0..10 {
        let timestamp = 1_000_000 + i as i64 * 1000;
        let mut record = TSRecord::new(timestamp, "device2".to_string());
        record.add_tuple(DataPoint::new_i32("voltage".to_string(), 220 + i as i32));
        record.add_tuple(DataPoint::new_i64("current".to_string(), 1000 + i as i64 * 10));
        writer.write(record)?;
    }

    // 5. Close (flushes all data and writes footer)
    writer.close()?;
    println!("TsFile written successfully to: {}", output_path);

    Ok(())
}
