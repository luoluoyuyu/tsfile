#!/usr/bin/env python3
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
import struct

with open('/tmp/multi_device_test.tsfile', 'rb') as f:
    data = f.read()

print(f"File size: {len(data)}")

# Metadata starts after SEPARATOR at 76
offset = 77

print(f"\n=== TimeseriesMetadata at {offset} ===")
type_byte = data[offset]; offset += 1
print(f"type: {type_byte}")

len_byte = data[offset]; offset += 1
str_len = len_byte >> 1
measurement_id = data[offset:offset+str_len].decode()
offset += str_len
print(f"measurementID: '{measurement_id}'")

dt = data[offset]; offset += 1
print(f"dataType: {dt}")

cs = data[offset]; offset += 1
print(f"chunkMetaDataListDataSize: {cs}")

print(f"\n=== Statistics at {offset} ===")
cnt = data[offset]; offset += 1
print(f"count: {cnt}")

st = struct.unpack('>q', data[offset:offset+8])[0]
offset += 8
print(f"startTime: {st}")

et = struct.unpack('>q', data[offset:offset+8])[0]
offset += 8
print(f"endTime: {et}")

print(f"(dataType not in stream, from TimeseriesMetadata: {dt})")

# For INT64 (dataType=2), typed stats = 5 * 8 bytes = 40 bytes
if dt == 2:  # INT64
    print(f"Reading 5 i64 values for LongStats...")
    for name in ['min', 'max', 'first', 'last', 'sum']:
        val = struct.unpack('>q', data[offset:offset+8])[0]
        offset += 8
        print(f"  {name}: {val}")

stats_end = offset
print(f"\nStatistics ends at: {stats_end}")
print(f"Statistics size: {stats_end - (offset - cs - 3)}")

print(f"\n=== Chunk metadata at {offset}, size={cs} ===")
if cs > 0:
    if (type_byte & 0x3F) != 0:
        # Has stats in chunk
        chunk_offset = struct.unpack('>q', data[offset:offset+8])[0]
        offset += 8
        print(f"chunk offset: {chunk_offset}")
        # Chunk stats would be here too
    else:
        # No stats in chunk, just offset
        chunk_offset = struct.unpack('>q', data[offset:offset+8])[0]
        offset += 8
        print(f"chunk offset: {chunk_offset}")

print(f"\nNext should be at: {offset}")
print(f"Remaining bytes: {len(data) - offset}")

if offset < len(data):
    print(f"\n=== Next TimeseriesMetadata at {offset} ===")
    if offset < len(data):
        next_type = data[offset]
        print(f"type: {next_type}")
        if offset + 1 < len(data):
            next_len = data[offset + 1] >> 1
            print(f"measurementID length: {next_len}")
            if next_len > 0 and next_len < 50 and offset + 2 + next_len <= len(data):
                next_id = data[offset+2:offset+2+next_len].decode('utf-8', errors='replace')
                print(f"measurementID: '{next_id}'")
