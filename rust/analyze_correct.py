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
"""正确分析多设备TsFile结构"""
import struct

with open('/tmp/multi_device_test.tsfile', 'rb') as f:
    data = f.read()

print(f"File size: {len(data)} bytes\n")

# 找到SEPARATOR
sep_pos = data.find(b'\x02', 50)
print(f"SEPARATOR at: {sep_pos}")

offset = sep_pos + 1
print(f"\n=== Metadata starts at {offset} ===\n")

# 读取所有TimeseriesMetadata
ts_count = 0
while offset < len(data) - 50:  # 保留尾部空间
    start_pos = offset
    type_byte = data[offset]
    
    # 如果type看起来不合理，可能是MetadataIndexNode
    if type_byte > 3:  # TimeseriesMetadata type应该是0或1
        print(f"\nExpected TimeseriesMetadata but got type={type_byte}")
        print(f"Likely MetadataIndexNode or TsFileMetadata starts here")
        break
    
    offset += 1
    len_byte = data[offset]; offset += 1
    str_len = len_byte >> 1
    
    if str_len <= 0 or str_len > 100:
        print(f"\nInvalid string length: {str_len}")
        break
    
    measurement_id = data[offset:offset+str_len].decode('utf-8', errors='replace')
    offset += str_len
    
    dt = data[offset]; offset += 1
    cs = data[offset]; offset += 1
    
    print(f"TimeseriesMetadata[{ts_count}] at {start_pos}:")
    print(f"  measurementID: '{measurement_id}'")
    print(f"  dataType: {dt}")
    print(f"  chunkMetaDataListDataSize: {cs}")
    
    # Statistics
    stats_start = offset
    cnt = data[offset]; offset += 1
    st = struct.unpack('>q', data[offset:offset+8])[0]; offset += 8
    et = struct.unpack('>q', data[offset:offset+8])[0]; offset += 8
    
    print(f"  Statistics:")
    print(f"    count: {cnt}")
    print(f"    startTime: {st}")
    print(f"    endTime: {et}")
    
    # Typed stats based on dataType
    if dt == 1:  # INT32
        stats_size = 1 + 8 + 8 + 4*4 + 8  # count + start + end + 4*i32 + sum(i64)
        offset += 28  # skip remaining stats
    elif dt == 2:  # INT64
        stats_size = 1 + 8 + 8 + 5*8  # count + start + end + 5*i64
        offset += 49  # skip remaining stats
    elif dt == 4:  # DOUBLE
        stats_size = 1 + 8 + 8 + 5*8  # count + start + end + 5*f64
        offset += 49  # skip remaining stats
    
    print(f"  Statistics size: {offset - stats_start}")
    
    # Chunk metadata
    if cs > 0:
        print(f"  Chunk metadata ({cs} bytes):")
        if (type_byte & 0x3F) != 0:
            # Has chunk statistics
            chunk_off = struct.unpack('>q', data[offset:offset+8])[0]; offset += 8
            print(f"    chunk offset: {chunk_off}")
            # Skip chunk statistics (same as TimeseriesMetadata stats)
            cnt2 = data[offset]; offset += 1
            st2 = struct.unpack('>q', data[offset:offset+8])[0]; offset += 8
            et2 = struct.unpack('>q', data[offset:offset+8])[0]; offset += 8
            print(f"    chunk stats: count={cnt2}, start={st2}, end={et2}")
            # Skip typed stats
            if dt == 2 or dt == 4:
                offset += 40
            elif dt == 1:
                offset += 28
        else:
            # No chunk statistics, just offset
            chunk_off = struct.unpack('>q', data[offset:offset+8])[0]; offset += 8
            print(f"    chunk offset: {chunk_off}")
    
    ts_size = offset - start_pos
    print(f"  Total size: {ts_size}\n")
    ts_count += 1

print(f"\n=== Read {ts_count} TimeseriesMetadata, now at offset {offset} ===")
print(f"Remaining bytes: {len(data) - offset}")

# 显示剩余字节
if offset < len(data):
    print(f"\nRemaining data (hex):")
    remaining = data[offset:]
    for i in range(0, min(len(remaining), 100), 16):
        hex_str = ' '.join(f'{b:02x}' for b in remaining[i:i+16])
        print(f"  [{offset+i:03d}] {hex_str}")
