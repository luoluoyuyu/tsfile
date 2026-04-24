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

import org.apache.tsfile.write.TsFileWriter;
import org.apache.tsfile.write.schema.TsFileSchema;
import org.apache.tsfile.write.record.Tablet;
import org.apache.tsfile.write.schema.MeasurementSchema;
import org.apache.tsfile.enums.TSDataType;
import org.apache.tsfile.write.record.datapoint.LongDataPoint;
import java.io.File;
import java.util.ArrayList;
import java.util.List;

public class GenerateJavaTsFile {
    public static void main(String[] args) throws Exception {
        String filePath = "/tmp/java_generated.tsfile";
        File file = new File(filePath);
        if (file.exists()) file.delete();

        TsFileSchema schema = new TsFileSchema();
        schema.registerTimeseries("device1", "sensor1", 
            new MeasurementSchema("sensor1", TSDataType.INT64));

        try (TsFileWriter writer = new TsFileWriter(file, schema)) {
            Tablet tablet = new Tablet("device1", schema.getRegisteredTimeseries("device1"), 5);
            long[] timestamps = {100, 200, 300, 400, 500};
            long[] values = {10, 20, 30, 40, 50};
            
            for (int i = 0; i < 5; i++) {
                tablet.addTimestamp(i, timestamps[i]);
                tablet.addValue("sensor1", i, values[i]);
            }
            
            writer.write(tablet);
        }

        System.out.println("Java TsFile generated: " + filePath);
        System.out.println("File size: " + file.length() + " bytes");
    }
}
