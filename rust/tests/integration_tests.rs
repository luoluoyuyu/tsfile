// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use tempfile::NamedTempFile;

use tsfile::common::enums::{CompressionType, TSDataType, TSEncoding};
use tsfile::read::TsFileReader;
use tsfile::write::record::{DataPoint, TSRecord};
use tsfile::write::schema::MeasurementSchema;
use tsfile::write::TsFileWriter;

/// Helper: create a temp file path.
fn temp_path() -> String {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();
    // Close so the writer can open it fresh
    drop(tmp);
    path
}

#[test]
fn test_write_and_read_int32() {
    let path = temp_path();

    // Write
    {
        let mut writer = TsFileWriter::new(&path).unwrap();
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

    // Read back
    {
        let mut reader = TsFileReader::new(&path).unwrap();
        let pairs = reader.read_timeseries("device1", "sensor1").unwrap();
        assert_eq!(pairs.len(), 5, "Expected 5 time-value pairs");
        for (idx, pair) in pairs.iter().enumerate() {
            assert_eq!(pair.timestamp, idx as i64 * 1000);
        }
    }
}

#[test]
fn test_write_and_read_float() {
    let path = temp_path();

    {
        let mut writer = TsFileWriter::new(&path).unwrap();
        writer
            .register_timeseries(
                "sensor_device".to_string(),
                MeasurementSchema::new(
                    "temperature".to_string(),
                    TSDataType::Float,
                    TSEncoding::Plain,
                    CompressionType::Snappy,
                ),
            )
            .unwrap();

        for i in 0..8 {
            let mut record = TSRecord::new(i as i64 * 500, "sensor_device".to_string());
            record.add_tuple(DataPoint::new_f32(
                "temperature".to_string(),
                20.0 + i as f32,
            ));
            writer.write(record).unwrap();
        }
        writer.close().unwrap();
    }

    {
        let mut reader = TsFileReader::new(&path).unwrap();
        let pairs = reader
            .read_timeseries("sensor_device", "temperature")
            .unwrap();
        assert_eq!(pairs.len(), 8);
    }
}

#[test]
fn test_multiple_devices() {
    let path = temp_path();

    {
        let mut writer = TsFileWriter::new(&path).unwrap();

        writer
            .register_timeseries(
                "device_a".to_string(),
                MeasurementSchema::new(
                    "x".to_string(),
                    TSDataType::Int64,
                    TSEncoding::Plain,
                    CompressionType::Uncompressed,
                ),
            )
            .unwrap();

        writer
            .register_timeseries(
                "device_b".to_string(),
                MeasurementSchema::new(
                    "y".to_string(),
                    TSDataType::Double,
                    TSEncoding::Plain,
                    CompressionType::Uncompressed,
                ),
            )
            .unwrap();

        for i in 0..3 {
            let mut r1 = TSRecord::new(i as i64 * 1000, "device_a".to_string());
            r1.add_tuple(DataPoint::new_i64("x".to_string(), i as i64 * 100));
            writer.write(r1).unwrap();

            let mut r2 = TSRecord::new(i as i64 * 1000, "device_b".to_string());
            r2.add_tuple(DataPoint::new_f64("y".to_string(), i as f64 * 3.14));
            writer.write(r2).unwrap();
        }
        writer.close().unwrap();
    }

    {
        let mut reader = TsFileReader::new(&path).unwrap();
        let devices = reader.get_all_devices().unwrap();
        assert_eq!(devices.len(), 2);

        let a_data = reader.read_timeseries("device_a", "x").unwrap();
        assert_eq!(a_data.len(), 3);

        let b_data = reader.read_timeseries("device_b", "y").unwrap();
        assert_eq!(b_data.len(), 3);
    }
}

#[test]
fn test_varint_encoding_roundtrip() {
    use tsfile::utils::ReadWriteForEncodingUtils;
    use std::io::Cursor;

    let test_values = [0u32, 1, 127, 128, 300, 16383, 16384, 1_000_000];
    for &v in &test_values {
        let mut buf = Vec::new();
        let written = ReadWriteForEncodingUtils::write_unsigned_var_int(v, &mut buf).unwrap();
        assert!(written > 0);

        let mut cursor = Cursor::new(&buf);
        let read = ReadWriteForEncodingUtils::read_unsigned_var_int(&mut cursor).unwrap();
        assert_eq!(v, read, "VarInt roundtrip failed for {}", v);
    }
}

#[test]
fn test_compression_roundtrip() {
    use tsfile::compress::{create_compressor, create_decompressor};

    let data = b"Hello TsFile Rust! This is test data for compression roundtrip test.";

    let types = [
        CompressionType::Uncompressed,
        CompressionType::Snappy,
        CompressionType::Gzip,
        CompressionType::Lz4,
    ];

    for &compression in &types {
        let compressor = create_compressor(compression);
        let compressed = compressor.compress(data).unwrap();
        let decompressor = create_decompressor(compression);
        let decompressed = decompressor.decompress(&compressed, data.len()).unwrap();
        assert_eq!(
            data.as_ref(),
            decompressed.as_slice(),
            "Roundtrip failed for {:?}",
            compression
        );
    }
}

#[test]
fn test_read_java_generated_tsfile() {
    // Test interoperability: Java generates TsFile, Rust reads it
    // The Java-generated file should be at /tmp/tsfile_interop/interop_int32.tsfile
    // Generated by: org.apache.tsfile.InteropTestWrite.writeInt32File()
    // Contains: device1/sensor1 (INT32, PLAIN, UNCOMPRESSED), 5 data points
    
    let java_tsfile = "/tmp/tsfile_interop/interop_int32.tsfile";
    
    // Check if the file exists (it should have been generated by Java)
    if !std::path::Path::new(java_tsfile).exists() {
        eprintln!("Warning: Java-generated TsFile not found at {}", java_tsfile);
        eprintln!("Please run: java org.apache.tsfile.InteropTestWrite /tmp/tsfile_interop");
        return; // Skip this test if file doesn't exist
    }
    
    let mut reader = TsFileReader::new(java_tsfile).unwrap();
    
    // Read the data
    let pairs = reader.read_timeseries("device1", "sensor1").unwrap();
    
    // Verify we got 5 data points (i=0..5, timestamp=i*1000, value=i*10)
    assert_eq!(pairs.len(), 5, "Expected 5 time-value pairs from Java file");
    
    for (idx, pair) in pairs.iter().enumerate() {
        let expected_ts = idx as i64 * 1000;
        assert_eq!(
            pair.timestamp, expected_ts,
            "Timestamp mismatch at index {}", idx
        );
    }
    
    println!("✓ Successfully read Java-generated TsFile: {} pairs", pairs.len());
}
