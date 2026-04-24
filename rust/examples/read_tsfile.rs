// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Example: read a TsFile and print its contents.

use tsfile::read::TsFileReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let input_path = "/tmp/example_write.tsfile";
    println!("Reading TsFile from: {}", input_path);

    let mut reader = TsFileReader::new(input_path)?;

    // Get all devices
    let devices = reader.get_all_devices()?;
    println!("Devices in file: {:?}", devices);

    for device_id in &devices {
        println!("\n=== Device: {} ===", device_id);
        let measurements = reader.get_measurements_for_device(device_id)?;
        println!("  Measurements: {:?}", measurements);

        for measurement in &measurements {
            let pairs = reader.read_timeseries(device_id, measurement)?;
            println!(
                "  Timeseries {}.{}: {} points",
                device_id,
                measurement,
                pairs.len()
            );
            for pair in &pairs {
                println!("    ts={}, value={:?}", pair.timestamp, pair.value);
            }
        }
    }

    println!("\nDone reading TsFile.");
    Ok(())
}
