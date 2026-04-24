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

import org.apache.tsfile.read.TsFileReader;
import org.apache.tsfile.read.query.dataset.QueryDataSet;
import org.apache.tsfile.read.query.expression.QueryExpression;
import org.apache.tsfile.read.common.Path;
import org.apache.tsfile.read.common.RowRecord;
import java.io.File;
import java.util.ArrayList;
import java.util.List;

public class TestRustFile {
    public static void main(String[] args) throws Exception {
        String filePath = "/tmp/multi_device_test.tsfile";
        System.out.println("Reading: " + filePath);
        
        try {
            TsFileReader reader = new TsFileReader(new org.apache.tsfile.read.TsFileSequenceReader(filePath));
            
            // Query all data
            List<Path> paths = new ArrayList<>();
            paths.add(new Path("device_a", "x", true));
            paths.add(new Path("device_b", "y", true));
            
            QueryExpression expr = QueryExpression.create(paths, null);
            QueryDataSet dataSet = reader.query(expr);
            
            int rowCount = 0;
            while (dataSet.hasNext()) {
                RowRecord record = dataSet.next();
                System.out.println(record);
                rowCount++;
            }
            
            System.out.println("\nTotal rows: " + rowCount);
            reader.close();
            System.out.println("SUCCESS!");
        } catch (Exception e) {
            System.err.println("FAILED: " + e.getMessage());
            e.printStackTrace();
        }
    }
}
