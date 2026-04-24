// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.
//
// Simple test to generate a TsFile for Java verification

use tsfile::common::enums::{CompressionType, TSDataType, TSEncoding};
use tsfile::write::record::{DataPoint, TSRecord};
use tsfile::write::schema::MeasurementSchema;
use tsfile::write::TsFileWriter;

fn main() {
    let path = "/tmp/rust_generated.tsfile";
    
    println!("Generating TsFile at: {}", path);
    
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
            record.add_tuple(DataPoint::new_i32("sensor1".to_string(), i as i32 * 10));
            writer.write(record).unwrap();
        }
        writer.close().unwrap();
    }
    
    println!("✓ TsFile generated successfully");
    
    // Verify by reading back
    {
        use tsfile::read::TsFileReader;
        let mut reader = TsFileReader::new(path).unwrap();
        let pairs = reader.read_timeseries("device1", "sensor1").unwrap();
        println!("✓ Verified: read {} data points", pairs.len());
        
        for (idx, pair) in pairs.iter().enumerate() {
            println!("  [{}] timestamp={:?}, value={:?}", idx, pair.timestamp, pair.value);
        }
    }
}
