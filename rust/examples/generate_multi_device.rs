// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.
//
use tsfile::common::enums::{CompressionType, TSDataType, TSEncoding};
use tsfile::write::record::{DataPoint, TSRecord};
use tsfile::write::schema::MeasurementSchema;
use tsfile::write::TsFileWriter;

fn main() {
    let path = "/tmp/multi_device_test.tsfile";
    let mut writer = TsFileWriter::new(path).unwrap();
    
    writer.register_timeseries(
        "device_a".to_string(),
        MeasurementSchema::new(
            "x".to_string(),
            TSDataType::Int64,
            TSEncoding::Plain,
            CompressionType::Uncompressed,
        ),
    ).unwrap();
    
    writer.register_timeseries(
        "device_b".to_string(),
        MeasurementSchema::new(
            "y".to_string(),
            TSDataType::Double,
            TSEncoding::Plain,
            CompressionType::Uncompressed,
        ),
    ).unwrap();
    
    for i in 0..3 {
        let mut r1 = TSRecord::new(i as i64 * 1000, "device_a".to_string());
        r1.add_tuple(DataPoint::new_i64("x".to_string(), i as i64 * 100));
        writer.write(r1).unwrap();
        
        let mut r2 = TSRecord::new(i as i64 * 1000, "device_b".to_string());
        r2.add_tuple(DataPoint::new_f64("y".to_string(), i as f64 * 3.14));
        writer.write(r2).unwrap();
    }
    writer.close().unwrap();
    
    println!("✓ Multi-device file generated at: {}", path);
}
