// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

use tempfile::NamedTempFile;

use tsfile::common::constant::TsFileConstant;
use tsfile::common::enums::{CompressionType, TSDataType, TSEncoding};
use tsfile::file::metadata::{ColumnCategory, ColumnSchema, TableSchema};
use tsfile::read::{
    query_data_set_to_tsblock, CachedChunkLoaderImpl, ColumnValue, DeviceTableModelReader, Filter,
    IChunkLoader, IChunkMetadataLoader, IMetadataQuerier, MetadataQuerierByFileImpl,
    Path as SeriesPath, QueryExpression, QueryExecutorBuilder, SimpleChunkMetadataLoader,
    TimeRange, TsBlockBuilder, TsFileExecutor, TsFileReader, TsFileSequenceReader,
    TsFileTreeReader,
};
use tsfile::read::expression::Expression;
use tsfile::utils::read_write_io_utils::Binary;
use tsfile::write::record::{DataPoint, DataPointValue, TSRecord};
use tsfile::write::schema::MeasurementSchema;
use tsfile::write::{
    DeviceTableModelWriter, TableTsBlock2TsFileWriter, Tablet, TsFileTreeWriter, TsFileWriter,
};

/// Helper: create a temp file path.
fn temp_path() -> String {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();
    // Close so the writer can open it fresh
    drop(tmp);
    path
}

fn measurement_schema(measurement_id: &str, data_type: TSDataType) -> MeasurementSchema {
    MeasurementSchema::new(
        measurement_id.to_string(),
        data_type,
        TSEncoding::Plain,
        CompressionType::Uncompressed,
    )
}

fn field_text(row: &tsfile::read::RowRecord, index: usize) -> String {
    row.field(index)
        .unwrap()
        .value
        .as_binary()
        .unwrap()
        .to_string()
}

fn field_i32(row: &tsfile::read::RowRecord, index: usize) -> i32 {
    row.field(index).unwrap().value.as_i32().unwrap()
}

fn field_f64(row: &tsfile::read::RowRecord, index: usize) -> f64 {
    row.field(index).unwrap().value.as_f64().unwrap()
}

fn field_bool(row: &tsfile::read::RowRecord, index: usize) -> bool {
    row.field(index).unwrap().value.as_bool().unwrap()
}

fn field_is_null(row: &tsfile::read::RowRecord, index: usize) -> bool {
    row.field(index).unwrap().is_null()
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

#[test]
fn test_tree_writer_full_query_and_read_interfaces() {
    let path = temp_path();
    let d1_schemas = vec![
        measurement_schema("s1", TSDataType::Int32),
        measurement_schema("s2", TSDataType::Double),
    ];
    let d2_schemas = vec![
        measurement_schema("s1", TSDataType::Int32),
        measurement_schema("s3", TSDataType::Text),
    ];

    {
        let mut writer = TsFileTreeWriter::new(&path).unwrap();
        writer
            .register_timeseries_batch("root.sg.d1".to_string(), d1_schemas.clone())
            .unwrap();
        writer
            .register_aligned_timeseries("root.sg.d2".to_string(), d2_schemas.clone())
            .unwrap();

        let mut tablet = Tablet::new("root.sg.d1".to_string(), d1_schemas.clone(), 8);
        tablet
            .add_row(
                1,
                vec![
                    Some(DataPointValue::Int32(11)),
                    Some(DataPointValue::Double(1.5)),
                ],
            )
            .unwrap();
        tablet
            .add_row(
                2,
                vec![
                    Some(DataPointValue::Int32(12)),
                    Some(DataPointValue::Double(2.5)),
                ],
            )
            .unwrap();
        assert_eq!(writer.write_tablet_and_reset(&mut tablet).unwrap(), 2);
        assert_eq!(tablet.row_size(), 0);

        let mut record = TSRecord::new(1, "root.sg.d2".to_string());
        record.add_tuple(DataPoint::new_i32("s1".to_string(), 21));
        record.add_tuple(DataPoint::new_string("s3".to_string(), "alpha".to_string()));
        writer.write_record(record).unwrap();

        let mut record = TSRecord::new(3, "root.sg.d2".to_string());
        record.add_tuple(DataPoint::new_i32("s1".to_string(), 22));
        record.add_tuple(DataPoint::new_string("s3".to_string(), "beta".to_string()));
        writer.write_record(record).unwrap();

        writer.flush().unwrap();
        writer.close().unwrap();
    }

    {
        let mut reader = TsFileReader::new(&path).unwrap();
        assert_eq!(reader.file_version(), 4);
        assert!(reader.is_complete().unwrap());
        assert!(reader.contains_device("root.sg.d1").unwrap());
        assert!(reader.contains_timeseries("root.sg.d2", "s3").unwrap());
        assert_eq!(
            reader.get_all_device_ids().unwrap(),
            vec!["root.sg.d1".to_string(), "root.sg.d2".to_string()]
        );

        let d1_measurements = reader
            .get_measurement("root.sg.d1")
            .unwrap()
            .into_iter()
            .map(|schema| schema.measurement_id)
            .collect::<Vec<_>>();
        assert_eq!(d1_measurements, vec!["s1".to_string(), "s2".to_string()]);

        let d1_data = reader.read_device("root.sg.d1").unwrap();
        let d1_s1 = d1_data.get("s1").unwrap();
        assert_eq!(d1_s1.len(), 2);
        assert_eq!(d1_s1[0].value.as_i32(), Some(11));
        assert_eq!(d1_s1[1].value.as_i32(), Some(12));

        let measurement_map = reader.get_device_measurements_map().unwrap();
        assert_eq!(
            measurement_map.get("root.sg.d2").unwrap(),
            &vec!["s1".to_string(), "s3".to_string()]
        );

        let full_path_map = reader.get_full_path_data_type_map().unwrap();
        assert_eq!(
            full_path_map.get("root.sg.d1.s2"),
            Some(&TSDataType::Double)
        );
        assert_eq!(
            full_path_map.get("root.sg.d2.s3"),
            Some(&TSDataType::Text)
        );

        let all_rows = reader.read_all_rows().unwrap();
        assert_eq!(all_rows.get("root.sg.d1").unwrap().len(), 2);
        assert_eq!(all_rows.get("root.sg.d2").unwrap().len(), 2);

        let all_result = reader.query_all().unwrap();
        assert_eq!(
            all_result.columns(),
            &[
                "device_id".to_string(),
                "s1".to_string(),
                "s2".to_string(),
                "s3".to_string()
            ]
        );
        assert_eq!(all_result.rows().len(), 4);

        let first_row = &all_result.rows()[0];
        assert_eq!(first_row.timestamp, 1);
        assert_eq!(field_text(first_row, 0), "root.sg.d1");
        assert_eq!(field_i32(first_row, 1), 11);
        assert_eq!(field_f64(first_row, 2), 1.5);
        assert!(first_row.field(3).unwrap().is_null());

        let second_row = &all_result.rows()[1];
        assert_eq!(second_row.timestamp, 1);
        assert_eq!(field_text(second_row, 0), "root.sg.d2");
        assert_eq!(field_i32(second_row, 1), 21);
        assert!(second_row.field(2).unwrap().is_null());
        assert_eq!(field_text(second_row, 3), "alpha");

        let filtered = reader
            .query_all_by_time_range(Some(TimeRange::new(2, 3)))
            .unwrap();
        assert_eq!(filtered.rows().len(), 2);
    }

    {
        let mut reader = TsFileTreeReader::new(&path).unwrap();
        assert_eq!(
            reader.get_all_device_ids().unwrap(),
            vec!["root.sg.d1".to_string(), "root.sg.d2".to_string()]
        );
        assert_eq!(reader.read_device("root.sg.d2").unwrap().get("s3").unwrap().len(), 2);
        assert_eq!(reader.query_all().unwrap().rows().len(), 4);
        assert_eq!(reader.query_all_by_time_range(3, 3).unwrap().rows().len(), 1);
        reader.close().unwrap();
    }
}

#[test]
fn test_device_table_model_writer_and_reader_roundtrip() {
    let path = temp_path();
    let table_schema = TableSchema::from_columns(
        "metrics".to_string(),
        vec![
            ColumnSchema::new(
                "region".to_string(),
                TSDataType::Text,
                ColumnCategory::Tag,
            ),
            ColumnSchema::new(
                "temperature".to_string(),
                TSDataType::Double,
                ColumnCategory::Field,
            ),
            ColumnSchema::new(
                "status".to_string(),
                TSDataType::Boolean,
                ColumnCategory::Field,
            ),
        ],
    )
    .unwrap();
    let tablet_schemas = table_schema.measurement_schemas.clone();

    {
        let mut writer = DeviceTableModelWriter::new(&path, table_schema.clone()).unwrap();

        let mut record = TSRecord::new(1, "metrics.d1".to_string());
        record.add_tuple(DataPoint::new_string("region".to_string(), "north".to_string()));
        record.add_tuple(DataPoint::new_f64("temperature".to_string(), 21.5));
        record.add_tuple(DataPoint::new_bool("status".to_string(), true));
        writer.write_record(record).unwrap();

        let mut tablet = Tablet::new("metrics.d2".to_string(), tablet_schemas, 8);
        tablet
            .add_row(
                2,
                vec![
                    Some(DataPointValue::Text(Binary::from_str("south"))),
                    Some(DataPointValue::Double(18.0)),
                    Some(DataPointValue::Boolean(false)),
                ],
            )
            .unwrap();
        tablet
            .add_row(
                3,
                vec![
                    Some(DataPointValue::Text(Binary::from_str("south"))),
                    Some(DataPointValue::Double(19.0)),
                    Some(DataPointValue::Boolean(true)),
                ],
            )
            .unwrap();
        assert_eq!(writer.write_tablet_and_reset(&mut tablet).unwrap(), 2);
        assert_eq!(tablet.row_size(), 0);

        writer.flush().unwrap();
        writer.close().unwrap();
    }

    {
        let mut table_reader = DeviceTableModelReader::new(&path).unwrap();
        let schema = table_reader.get_table_schema("metrics").unwrap().unwrap();
        assert_eq!(schema.table_name, "metrics");
        assert_eq!(schema.tag_column_count(), 1);
        assert_eq!(
            schema
                .measurement_schemas
                .iter()
                .map(|measurement| measurement.measurement_id.clone())
                .collect::<Vec<_>>(),
            vec![
                "region".to_string(),
                "temperature".to_string(),
                "status".to_string()
            ]
        );

        assert_eq!(table_reader.get_all_table_schemas().unwrap().len(), 1);
        assert_eq!(
            table_reader.get_table_devices("metrics").unwrap(),
            vec!["metrics.d1".to_string(), "metrics.d2".to_string()]
        );

        let query_result = table_reader.query_all_columns("metrics", 1, 3).unwrap();
        assert_eq!(
            query_result.columns(),
            &[
                "device_id".to_string(),
                "region".to_string(),
                "temperature".to_string(),
                "status".to_string()
            ]
        );
        assert_eq!(query_result.rows().len(), 3);

        let first_row = &query_result.rows()[0];
        assert_eq!(first_row.timestamp, 1);
        assert_eq!(field_text(first_row, 0), "metrics.d1");
        assert_eq!(field_text(first_row, 1), "north");
        assert_eq!(field_f64(first_row, 2), 21.5);
        assert!(field_bool(first_row, 3));

        let third_row = &query_result.rows()[2];
        assert_eq!(third_row.timestamp, 3);
        assert_eq!(field_text(third_row, 0), "metrics.d2");
        assert_eq!(field_text(third_row, 1), "south");
        assert_eq!(field_f64(third_row, 2), 19.0);
        assert!(field_bool(third_row, 3));

        table_reader.close().unwrap();
    }

    {
        let mut reader = TsFileReader::new(&path).unwrap();
        let table_schema_map = reader.get_table_schema_map().unwrap();
        assert!(table_schema_map.contains_key("metrics"));
        assert_eq!(reader.get_table_devices("metrics").unwrap().len(), 2);
    }
}

#[test]
fn test_sequence_reader_raw_chunk_page_apis_and_query_stack() {
    let path = temp_path();

    {
        let mut writer = TsFileWriter::new(&path).unwrap();
        writer
            .register_timeseries(
                "root.sg.raw".to_string(),
                measurement_schema("s1", TSDataType::Int32),
            )
            .unwrap();
        writer
            .register_timeseries(
                "root.sg.raw".to_string(),
                measurement_schema("s2", TSDataType::Double),
            )
            .unwrap();

        let mut record = TSRecord::new(1, "root.sg.raw".to_string());
        record.add_tuple(DataPoint::new_i32("s1".to_string(), 10));
        writer.write(record).unwrap();

        let mut record = TSRecord::new(2, "root.sg.raw".to_string());
        record.add_tuple(DataPoint::new_i32("s1".to_string(), 20));
        record.add_tuple(DataPoint::new_f64("s2".to_string(), 2.5));
        writer.write(record).unwrap();

        writer.close().unwrap();
    }

    {
        let mut sequence_reader = TsFileSequenceReader::new(&path).unwrap();
        assert_eq!(
            sequence_reader.read_head_magic().unwrap(),
            TsFileConstant::MAGIC_STRING
        );
        assert_eq!(
            sequence_reader.read_tail_magic().unwrap(),
            TsFileConstant::MAGIC_STRING
        );
        assert_eq!(sequence_reader.read_version_number().unwrap(), 4);
        assert!(sequence_reader.self_check().unwrap() > 0);

        sequence_reader
            .seek((TsFileConstant::MAGIC_STRING.len() + 1) as u64)
            .unwrap();
        assert_eq!(sequence_reader.read_marker().unwrap(), 0);

        let paths = sequence_reader
            .get_all_paths()
            .unwrap()
            .into_iter()
            .map(|path| format!("{}.{}", path.device, path.measurement))
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            vec!["root.sg.raw.s1".to_string(), "root.sg.raw.s2".to_string()]
        );

        let chunk_metadata = sequence_reader
            .read_chunk_metadata_list("root.sg.raw", "s1")
            .unwrap();
        assert_eq!(chunk_metadata.len(), 1);

        let chunk_header = sequence_reader
            .read_chunk_header_at(chunk_metadata[0].offset_of_chunk_header as u64)
            .unwrap();
        assert_eq!(chunk_header.measurement_id, "s1");
        assert_eq!(chunk_header.data_type, TSDataType::Int32);

        let mem_chunk = sequence_reader.read_mem_chunk(&chunk_metadata[0]).unwrap();
        let mem_chunk_at = sequence_reader
            .read_mem_chunk_at(chunk_metadata[0].offset_of_chunk_header as u64)
            .unwrap();
        assert_eq!(mem_chunk.data, mem_chunk_at.data);
        let raw_chunk = sequence_reader
            .read_chunk(
                chunk_metadata[0].offset_of_chunk_header as u64
                    + chunk_header.serialized_size() as u64,
                chunk_header.data_size as usize,
            )
            .unwrap();
        assert_eq!(raw_chunk, mem_chunk.data);

        let page_headers = sequence_reader
            .read_chunk_page_headers(&chunk_metadata[0])
            .unwrap();
        assert_eq!(page_headers.len(), 1);

        sequence_reader
            .seek(
                chunk_metadata[0].offset_of_chunk_header as u64
                    + chunk_header.serialized_size() as u64,
            )
            .unwrap();
        let page_header = sequence_reader
            .read_page_header(chunk_header.data_type, false)
            .unwrap();
        let page_data_pos = sequence_reader.position().unwrap();
        let compressed_page = sequence_reader.read_compressed_page(&page_header).unwrap();
        assert_eq!(compressed_page.len(), page_header.compressed_size as usize);
        sequence_reader.seek(page_data_pos).unwrap();
        let page_data = sequence_reader
            .read_page(&page_header, chunk_header.compression_type)
            .unwrap();
        assert_eq!(page_data.len(), page_header.uncompressed_size as usize);
        sequence_reader.seek(page_data_pos).unwrap();
        sequence_reader.skip_page_data(&page_header).unwrap();

        let pages = sequence_reader
            .read_chunk_pages(
                &chunk_metadata[0],
                Some(&Filter::TimeBetween { min: 2, max: 2 }),
            )
            .unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].len(), 1);
        assert_eq!(pages[0][0].timestamp, 2);
        assert_eq!(pages[0][0].value.as_i32(), Some(20));

        let tsblock = sequence_reader
            .read_tsblock(
                "root.sg.raw",
                &["s1".to_string(), "s2".to_string()],
                None,
            )
            .unwrap();
        assert_eq!(tsblock.position_count(), 2);
        assert_eq!(tsblock.time_by_index(0), Some(1));
        assert_eq!(tsblock.time_by_index(1), Some(2));
        assert_eq!(tsblock.column(0).unwrap().get_i32(0), Some(10));
        assert!(matches!(
            tsblock.column(1).unwrap().get(0),
            Some(ColumnValue::Null)
        ));
        assert_eq!(tsblock.column(1).unwrap().get_f64(1), Some(2.5));
    }

    {
        let mut metadata_querier = MetadataQuerierByFileImpl::new(&path).unwrap();
        let path_s1 = SeriesPath::new("root.sg.raw".to_string(), "s1".to_string());
        let metadata_map = metadata_querier
            .get_chunk_metadata_map(std::slice::from_ref(&path_s1))
            .unwrap();
        assert_eq!(metadata_map.get(&path_s1).unwrap().len(), 1);
        assert_eq!(
            metadata_querier.get_data_type(&path_s1).unwrap(),
            Some(TSDataType::Int32)
        );
        let ranges = metadata_querier
            .convert_space_to_time_partition(
                std::slice::from_ref(&path_s1),
                metadata_map.get(&path_s1).unwrap()[0].offset_of_chunk_header as u64,
                metadata_map.get(&path_s1).unwrap()[0].offset_of_chunk_header as u64 + 1,
            )
            .unwrap();
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].min, 1);
        assert_eq!(ranges[0].max, 2);

        let root_node = metadata_querier
            .get_whole_file_metadata()
            .unwrap()
            .metadata_index_node("")
            .unwrap()
            .clone();
        let devices = metadata_querier
            .device_iterator(root_node, None)
            .unwrap()
            .map(|(device_id, _)| device_id.to_string())
            .collect::<Vec<_>>();
        assert_eq!(devices, vec!["root.sg.raw".to_string()]);

        let loader = SimpleChunkMetadataLoader;
        let ts_metadata = metadata_querier
            .reader_mut()
            .read_timeseries_metadata("root.sg.raw", "s1", true)
            .unwrap()
            .unwrap();
        assert_eq!(loader.load_chunk_metadata_list(&ts_metadata).unwrap().len(), 1);

        let mut chunk_loader = CachedChunkLoaderImpl::new(&path).unwrap();
        let points = chunk_loader
            .load_points(&metadata_map.get(&path_s1).unwrap()[0], None)
            .unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].value.as_i32(), Some(10));
        chunk_loader.close().unwrap();
        metadata_querier.close().unwrap();
    }

    {
        let mut executor = TsFileExecutor::new(&path).unwrap();
        let dataset = executor
            .execute(QueryExpression::new(
                "root.sg.raw".to_string(),
                vec!["s1".to_string(), "s2".to_string()],
            ))
            .unwrap();
        assert_eq!(dataset.rows().len(), 2);
        assert_eq!(dataset.rows()[0].timestamp, 1);
        assert!(field_is_null(&dataset.rows()[0], 1));
        assert_eq!(field_f64(&dataset.rows()[1], 1), 2.5);

        let mut paged = dataset.clone();
        paged.set_without_any_null(true);
        assert!(paged.has_next());
        let row = paged.next().unwrap();
        assert_eq!(row.timestamp, 2);
        assert!(!paged.has_next());

        let tsblock =
            query_data_set_to_tsblock(&dataset, &[TSDataType::Int32, TSDataType::Double]).unwrap();
        assert_eq!(tsblock.position_count(), 2);
        assert_eq!(tsblock.column(0).unwrap().get_i32(1), Some(20));
        assert_eq!(tsblock.column(1).unwrap().get_f64(1), Some(2.5));

        let no_time_generator = executor
            .execute_without_time_generator(QueryExpression::new(
                "root.sg.raw".to_string(),
                vec!["s1".to_string(), "s2".to_string()],
            ))
            .unwrap();
        assert_eq!(no_time_generator.query_data_set().rows().len(), 2);

        executor.close().unwrap();
    }
}

#[test]
fn test_table_tsblock_writer_roundtrip() {
    let path = temp_path();
    let table_schema = TableSchema::from_columns(
        "metrics".to_string(),
        vec![
            ColumnSchema::new(
                "region".to_string(),
                TSDataType::Text,
                ColumnCategory::Tag,
            ),
            ColumnSchema::new(
                "temperature".to_string(),
                TSDataType::Double,
                ColumnCategory::Field,
            ),
            ColumnSchema::new(
                "status".to_string(),
                TSDataType::Boolean,
                ColumnCategory::Field,
            ),
        ],
    )
    .unwrap();

    let mut builder = TsBlockBuilder::new(vec![
        TSDataType::Text,
        TSDataType::Double,
        TSDataType::Boolean,
    ]);
    builder.declare_position(
        10,
        vec![
            ColumnValue::Binary(Binary::from_str("north")),
            ColumnValue::Double(21.5),
            ColumnValue::Boolean(true),
        ],
    );
    builder.declare_position(
        11,
        vec![
            ColumnValue::Binary(Binary::from_str("north")),
            ColumnValue::Double(22.0),
            ColumnValue::Boolean(false),
        ],
    );
    builder.declare_position(
        20,
        vec![
            ColumnValue::Binary(Binary::from_str("south")),
            ColumnValue::Double(18.5),
            ColumnValue::Boolean(true),
        ],
    );
    let tsblock = builder.build();

    {
        let inner = DeviceTableModelWriter::new(&path, table_schema.clone()).unwrap();
        let mut writer = TableTsBlock2TsFileWriter::new(inner);
        assert_eq!(writer.write_tsblock(&tsblock).unwrap(), 3);
        assert_eq!(writer.row_count(), 3);
        assert_eq!(writer.device_count(), 2);
        writer.flush().unwrap();
        writer.close().unwrap();
    }

    {
        let mut reader = DeviceTableModelReader::new(&path).unwrap();
        assert_eq!(
            reader.get_table_devices("metrics").unwrap(),
            vec!["metrics.north".to_string(), "metrics.south".to_string()]
        );

        let result = reader.query_all_columns("metrics", 10, 20).unwrap();
        assert_eq!(
            result.columns(),
            &[
                "device_id".to_string(),
                "region".to_string(),
                "temperature".to_string(),
                "status".to_string()
            ]
        );
        assert_eq!(result.rows().len(), 3);

        let first_row = &result.rows()[0];
        assert_eq!(first_row.timestamp, 10);
        assert_eq!(field_text(first_row, 0), "metrics.north");
        assert_eq!(field_text(first_row, 1), "north");
        assert_eq!(field_f64(first_row, 2), 21.5);
        assert!(field_bool(first_row, 3));

        let third_row = &result.rows()[2];
        assert_eq!(third_row.timestamp, 20);
        assert_eq!(field_text(third_row, 0), "metrics.south");
        assert_eq!(field_text(third_row, 1), "south");
        assert_eq!(field_f64(third_row, 2), 18.5);
        assert!(field_bool(third_row, 3));

        reader.close().unwrap();
    }
}

#[test]
fn test_time_generator_and_streaming_result_sets() {
    let tree_path = temp_path();

    {
        let mut writer = TsFileTreeWriter::new(&tree_path).unwrap();
        let schemas = vec![
            measurement_schema("s1", TSDataType::Int32),
            measurement_schema("s2", TSDataType::Double),
        ];
        writer
            .register_timeseries_batch("root.sg.expr".to_string(), schemas)
            .unwrap();

        let mut record = TSRecord::new(1, "root.sg.expr".to_string());
        record.add_tuple(DataPoint::new_i32("s1".to_string(), 10));
        writer.write_record(record).unwrap();

        let mut record = TSRecord::new(2, "root.sg.expr".to_string());
        record.add_tuple(DataPoint::new_i32("s1".to_string(), 20));
        record.add_tuple(DataPoint::new_f64("s2".to_string(), 2.5));
        writer.write_record(record).unwrap();

        let mut record = TSRecord::new(3, "root.sg.expr".to_string());
        record.add_tuple(DataPoint::new_i32("s1".to_string(), 30));
        record.add_tuple(DataPoint::new_f64("s2".to_string(), 3.5));
        writer.write_record(record).unwrap();

        writer.close().unwrap();
    }

    {
        let mut executor = QueryExecutorBuilder::new(&tree_path)
            .build_time_generator_executor()
            .unwrap();
        let expression = Expression::And(
            Box::new(Expression::GlobalTime(Filter::TimeBetween { min: 2, max: 3 })),
            Box::new(Expression::SingleSeries {
                path: "root.sg.expr.s2".to_string(),
                filter: Filter::ValueIsNotNull,
            }),
        );
        let data_set = executor
            .execute(
                "root.sg.expr",
                &["s1".to_string(), "s2".to_string()],
                &expression,
            )
            .unwrap();
        assert_eq!(data_set.generated_timestamps(), &[2, 3]);
        assert_eq!(data_set.query_data_set().rows().len(), 2);
        assert_eq!(field_i32(&data_set.query_data_set().rows()[0], 0), 20);
        assert_eq!(field_f64(&data_set.query_data_set().rows()[1], 1), 3.5);
        executor.close().unwrap();
    }

    {
        let mut reader = TsFileTreeReader::new(&tree_path).unwrap();
        let mut result_set = reader
            .query_tree_result_set(
                &["root.sg.expr".to_string()],
                &["s1".to_string(), "s2".to_string()],
                1,
                3,
            )
            .unwrap();
        let records = result_set.by_ref().collect::<Vec<_>>();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].device_id, "root.sg.expr");
        assert_eq!(records[0].timestamp, 1);
        assert_eq!(records[0].data_points[0].measurement_id, "s1");
        assert!(matches!(
            records[1].data_points[1].value,
            DataPointValue::Double(value) if value == 2.5
        ));
        reader.close().unwrap();
    }

    let table_path = temp_path();
    let table_schema = TableSchema::from_columns(
        "metrics".to_string(),
        vec![
            ColumnSchema::new(
                "region".to_string(),
                TSDataType::Text,
                ColumnCategory::Tag,
            ),
            ColumnSchema::new(
                "temperature".to_string(),
                TSDataType::Double,
                ColumnCategory::Field,
            ),
            ColumnSchema::new(
                "status".to_string(),
                TSDataType::Boolean,
                ColumnCategory::Field,
            ),
        ],
    )
    .unwrap();

    {
        let mut writer = DeviceTableModelWriter::new(&table_path, table_schema.clone()).unwrap();

        let mut record = TSRecord::new(1, "metrics.d1".to_string());
        record.add_tuple(DataPoint::new_string("region".to_string(), "north".to_string()));
        record.add_tuple(DataPoint::new_f64("temperature".to_string(), 21.5));
        record.add_tuple(DataPoint::new_bool("status".to_string(), true));
        writer.write_record(record).unwrap();

        let mut record = TSRecord::new(2, "metrics.d2".to_string());
        record.add_tuple(DataPoint::new_string("region".to_string(), "south".to_string()));
        record.add_tuple(DataPoint::new_f64("temperature".to_string(), 18.0));
        record.add_tuple(DataPoint::new_bool("status".to_string(), false));
        writer.write_record(record).unwrap();

        writer.close().unwrap();
    }

    {
        let mut table_reader = DeviceTableModelReader::new(&table_path).unwrap();
        let mut result_set = table_reader
            .query_all_columns_result_set("metrics", 1, 2, 1)
            .unwrap();
        let records = result_set.by_ref().collect::<Vec<_>>();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].device_id, "metrics.d1");
        assert_eq!(records[0].timestamp, 1);
        assert_eq!(records[0].data_points[0].measurement_id, "region");
        assert!(matches!(
            records[0].data_points[0].value,
            DataPointValue::Text(ref value) if value.to_string() == "north"
        ));
        assert!(matches!(
            records[1].data_points[2].value,
            DataPointValue::Boolean(false)
        ));
        table_reader.close().unwrap();
    }
}
