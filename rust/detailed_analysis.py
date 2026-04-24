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

def read_unsigned_varint(data, pos):
    """Read unsigned varint from data at pos"""
    value = 0
    shift = 0
    while True:
        b = data[pos]
        pos += 1
        value |= (b & 0x7F) << shift
        if (b & 0x80) == 0:
            break
        shift += 7
    return value, pos

def read_i64(data, pos):
    """Read i64 big-endian"""
    value = struct.unpack('>q', data[pos:pos+8])[0]
    return value, pos + 8

def read_string(data, pos):
    """Read varint-string"""
    length, pos = read_unsigned_varint(data, pos)
    string = data[pos:pos+length].decode('ascii', errors='replace')
    return string, pos + length

with open('/tmp/rust_generated.tsfile', 'rb') as f:
    data = f.read()

print(f"Total file size: {len(data)} bytes")

# Find SEPARATOR
separator_pos = None
for i in range(len(data)-10, 0, -1):
    if data[i] == 0x02:
        separator_pos = i
        break

print(f"\nSEPARATOR at position: {separator_pos}")

# Read metaSize
meta_size_pos = len(data) - 10
meta_size = struct.unpack('>I', data[meta_size_pos:meta_size_pos+4])[0]
print(f"Metadata size: {meta_size} bytes")

# Metadata area
meta_start = separator_pos + 1
meta_end = meta_size_pos
print(f"Metadata area: {meta_start} to {meta_end} ({meta_end - meta_start} bytes)")

# Try to parse as TsFileMetadata first (at the end of metadata area)
print(f"\n=== Parsing TsFileMetadata (last {meta_size} bytes) ===")
pos = meta_end - meta_size
print(f"TsFileMetadata starts at: {pos}")

# TsFileMetadata format:
# - metaOffset (i64)
# - bloomFilter (optional)
# - tableMetadataIndexNodeMap

# Actually, let's work backwards
# The last thing before metaSize should be TsFileMetadata
# Let's try to find the MetadataIndexNode first

print(f"\n=== Trying to parse metadata structure ===")
pos = meta_start

# According to Java's TsFileSequenceReader.readFileMetadata:
# 1. Read TimeseriesMetadata for each series
# 2. Read MetadataIndexNode (device level)
# 3. Read TsFileMetadata

# But the actual format depends on the version
# Let's just dump all bytes
print(f"\nRaw bytes from {meta_start} to {meta_end}:")
for i in range(meta_start, meta_end):
    if (i - meta_start) % 16 == 0:
        print(f"\n  [{i:3d}] ", end='')
    print(f"{data[i]:02X} ", end='')
print()

# Try to find the pattern
# We expect: nodeType (1 byte) at the END of each MetadataIndexNode
# nodeType should be 0, 1, 2, or 3

print(f"\n\n=== Searching for MetadataIndexNode patterns ===")
for pos in range(meta_start, meta_end - 10):
    # Try to parse as MetadataIndexNode
    if data[pos] <= 10:  # Reasonable children count
        # Try parsing
        try:
            child_count, p1 = read_unsigned_varint(data, pos)
            if child_count <= 5:  # Reasonable
                # Read first child
                name1, p2 = read_string(data, p1)
                offset1, p3 = read_i64(data, p2)
                end_offset, p4 = read_i64(data, p3)
                node_type = data[p4]
                
                if node_type <= 3:
                    print(f"\nPossible MetadataIndexNode at {pos}:")
                    print(f"  children: {child_count}")
                    print(f"  first child: {name1} @ {offset1}")
                    print(f"  end_offset: {end_offset}")
                    print(f"  node_type: {node_type}")
                    print(f"  total size: {p4 + 1 - pos}")
        except:
            pass
