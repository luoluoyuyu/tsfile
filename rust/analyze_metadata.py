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

with open('/tmp/rust_generated.tsfile', 'rb') as f:
    data = f.read()

print(f"Total file size: {len(data)} bytes")

# Find SEPARATOR
for i in range(len(data)-10, 0, -1):
    if data[i] == 0x02:
        separator_pos = i
        break

print(f"\nSEPARATOR at position: {separator_pos}")
print(f"Metadata starts at: {separator_pos + 1}")

# Read metaSize
meta_size_pos = len(data) - 10
meta_size = struct.unpack('>I', data[meta_size_pos:meta_size_pos+4])[0]
print(f"Metadata size: {meta_size} bytes")
print(f"Metadata should be at: {len(data) - 10 - meta_size}")

# Analyze metadata area
pos = separator_pos + 1
print(f"\n=== Metadata area from position {pos} ===")

# We expect: TimeseriesMetadata + chunk metadata + MetadataIndexNodes
# Let's see what's at each position
count = 0
while pos < meta_size_pos and count < 100:
    byte_val = data[pos]
    print(f"  [{pos}] = 0x{byte_val:02X} ({byte_val})")
    pos += 1
    count += 1

# -34 as signed byte = 222 as unsigned byte = 0xDE
print(f"\n=== Searching for 0xDE (which would be -34 as signed byte) ===")
for i, b in enumerate(data):
    if b == 0xDE:
        print(f"  Found 0xDE at position {i}")

# Search for valid node types in metadata area
print(f"\n=== Searching for valid MetadataIndexNodeType values (0-3) ===")
meta_start = separator_pos + 1
for i in range(meta_start, meta_size_pos):
    if data[i] <= 3:
        print(f"  Position {i}: 0x{data[i]:02X} ({data[i]})")
