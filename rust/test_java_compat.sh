#!/bin/bash
# Licensed to the Apache Software Foundation (ASF) under one
# or more contributor license agreements.  See the NOTICE file
# distributed with this work for additional information
# regarding copyright ownership.  The ASF licenses this file
# to you under the Apache License, Version 2.0 (the
# "License"); you may not use this file except in compliance
# with the License.  You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing,
# software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
# KIND, either express or implied.  See the License for the
# specific language governing permissions and limitations
# under the License.
#
echo "Testing Java compatibility..."
echo ""

# Test if jar exists
JAR="../java/tsfile/target/tsfile-2.2.0-SNAPSHOT.jar"
if [ ! -f "$JAR" ]; then
    echo "Java jar not found, trying alternative location..."
    JAR="../java/tsfile/target/original-tsfile-2.2.0-SNAPSHOT.jar"
fi

if [ ! -f "$JAR" ]; then
    echo "ERROR: No Java jar found"
    exit 1
fi

echo "Using jar: $JAR"
echo ""

# Try to list available classes
echo "Checking for TsFileSequenceReader..."
jar tf "$JAR" | grep -i "TsFileSequenceReader" | head -5

echo ""
echo "Checking for TimeseriesMetadata..."  
jar tf "$JAR" | grep -i "TimeseriesMetadata" | head -5
