/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.
 *
 * Interoperability test: generates a TsFile that Rust code can read.
 * Usage: mvn exec:java -Dexec.mainClass="org.apache.tsfile.InteropTestWrite" -pl examples
 */

package org.apache.tsfile;

import org.apache.tsfile.enums.TSDataType;
import org.apache.tsfile.exception.write.WriteProcessException;
import org.apache.tsfile.file.metadata.enums.CompressionType;
import org.apache.tsfile.file.metadata.enums.TSEncoding;
import org.apache.tsfile.write.TsFileWriter;
import org.apache.tsfile.write.record.TSRecord;
import org.apache.tsfile.write.record.datapoint.DataPoint;
import org.apache.tsfile.write.schema.MeasurementSchema;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;

/**
 * Generates test TsFiles for cross-language interoperability testing.
 *
 * <p>Generates two files:
 *
 * <ul>
 *   <li>interop_int32.tsfile - device1 with sensor1 (INT32, PLAIN, UNCOMPRESSED)
 *   <li>interop_multi.tsfile - device_a with x (INT64, PLAIN, UNCOMPRESSED) and device_b with y
 *       (DOUBLE, PLAIN, SNAPPY)
 * </ul>
 */
public class InteropTestWrite {

  public static void main(String[] args) throws Exception {
    String outputDir = args.length > 0 ? args[0] : ".";
    writeInt32File(outputDir + "/interop_int32.tsfile");
    writeMultiDeviceFile(outputDir + "/interop_multi.tsfile");
    System.out.println("Interop test files written successfully to: " + outputDir);
  }

  /** Write a simple INT32 file with UNCOMPRESSED + PLAIN encoding. */
  public static void writeInt32File(String path) throws IOException, WriteProcessException {
    File f = new File(path);
    if (f.exists()) {
      Files.delete(f.toPath());
    }

    try (TsFileWriter writer = new TsFileWriter(f)) {
      writer.registerTimeseries(
          new org.apache.tsfile.read.common.Path("device1"),
          new MeasurementSchema(
              "sensor1", TSDataType.INT32, TSEncoding.PLAIN, CompressionType.UNCOMPRESSED));

      for (int i = 0; i < 5; i++) {
        TSRecord record = new TSRecord("device1", i * 1000L);
        record.addTuple(
            DataPoint.getDataPoint(TSDataType.INT32, "sensor1", String.valueOf(i * 10)));
        writer.writeRecord(record);
      }
    }
    System.out.println("Written: " + path);
  }

  /** Write a multi-device file with INT64+UNCOMPRESSED and DOUBLE+SNAPPY. */
  public static void writeMultiDeviceFile(String path) throws IOException, WriteProcessException {
    File f = new File(path);
    if (f.exists()) {
      Files.delete(f.toPath());
    }

    try (TsFileWriter writer = new TsFileWriter(f)) {
      writer.registerTimeseries(
          new org.apache.tsfile.read.common.Path("device_a"),
          new MeasurementSchema(
              "x", TSDataType.INT64, TSEncoding.PLAIN, CompressionType.UNCOMPRESSED));
      writer.registerTimeseries(
          new org.apache.tsfile.read.common.Path("device_b"),
          new MeasurementSchema("y", TSDataType.DOUBLE, TSEncoding.PLAIN, CompressionType.SNAPPY));

      for (int i = 0; i < 3; i++) {
        TSRecord r1 = new TSRecord("device_a", i * 1000L);
        r1.addTuple(DataPoint.getDataPoint(TSDataType.INT64, "x", String.valueOf(i * 100L)));
        writer.writeRecord(r1);

        TSRecord r2 = new TSRecord("device_b", i * 1000L);
        r2.addTuple(DataPoint.getDataPoint(TSDataType.DOUBLE, "y", String.valueOf(i * 3.14)));
        writer.writeRecord(r2);
      }
    }
    System.out.println("Written: " + path);
  }
}
