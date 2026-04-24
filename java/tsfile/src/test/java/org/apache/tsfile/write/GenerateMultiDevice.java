/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

package org.apache.tsfile.write;

import org.apache.tsfile.common.conf.TSFileConfig;
import org.apache.tsfile.common.conf.TSFileDescriptor;
import org.apache.tsfile.enums.TSDataType;
import org.apache.tsfile.file.metadata.enums.CompressionType;
import org.apache.tsfile.file.metadata.enums.TSEncoding;
import org.apache.tsfile.write.record.TSRecord;
import org.apache.tsfile.write.record.datapoint.LongDataPoint;
import org.apache.tsfile.write.record.datapoint.DoubleDataPoint;
import org.apache.tsfile.write.schema.MeasurementSchema;

import java.io.File;
import java.io.IOException;

public class GenerateMultiDevice {
  public static void main(String[] args) throws IOException {
    TSFileConfig conf = TSFileDescriptor.getInstance().getConfig();

    String path = "/tmp/java_multi_device.tsfile";
    File file = new File(path);
    if (file.exists()) file.delete();

    TsFileWriter writer = new TsFileWriter(file);

    // Register device_a.x
    writer.registerTimeseries(
        new MeasurementSchema(
            "x", TSDataType.INT64, TSEncoding.PLAIN, CompressionType.UNCOMPRESSED));

    // Register device_b.y
    writer.registerTimeseries(
        new MeasurementSchema(
            "y", TSDataType.DOUBLE, TSEncoding.PLAIN, CompressionType.UNCOMPRESSED));

    // Write data
    for (int i = 0; i < 3; i++) {
      long timestamp = i * 1000L;

      TSRecord r1 = new TSRecord("device_a", timestamp);
      r1.addTuple(new LongDataPoint("x", i * 100L));
      writer.write(r1);

      TSRecord r2 = new TSRecord("device_b", timestamp);
      r2.addTuple(new DoubleDataPoint("y", i * 3.14));
      writer.write(r2);
    }

    writer.close();
    System.out.println("Java multi-device file written to: " + path);
    System.out.println("File size: " + file.length() + " bytes");
  }
}
