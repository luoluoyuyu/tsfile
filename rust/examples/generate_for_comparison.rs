// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.
//
// Temporary test to generate a Rust TsFile for comparison
use tsfile::common::enums::{CompressionType, TSDataType, TSEncoding};
use tsfile::write::record::{DataPoint, TSRecord};
use tsfile::write::schema::MeasurementSchema;
use tsfile::write::TsFileWriter;

fn main() {
    let path = "/tmp/rust_generated.tsfile";
    
    // Write
    {
        let mut writer = TsFileWriter::new(path).unwrap();
        writer
            .register_timeseries(
                "device1".to_string(),
                MeasurementSchema::new(
                    "sensor1".to_string(),
                    TSDataType::Int32,
                    TSEncoding::Plain,
                    CompressionType::Uncompressed,
                ),
            )
            .unwrap();

        for i in 0..5 {
            let mut record = TSRecord::new(i as i64 * 1000, "device1".to_string());
            record.add_tuple(DataPoint::new_i32("sensor1".to_string(), i * 10));
            writer.write(record).unwrap();
        }
        writer.close().unwrap();
    }
    
    println!("Rust TsFile written to: {}", path);
}
