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

print(f'File size: {len(data)}')
print(f'\nBytes 124-200:')
for i in range(124, min(200, len(data))):
    print(f'[{i:03d}] 0x{data[i]:02x}', end='  ')
    if (i - 124 + 1) % 8 == 0:
        print()

# Parse
offset = 124
type_byte = data[offset]; offset += 1
print(f'\n\ntype_byte: {type_byte}')

# measurementID
len_byte = data[offset]; offset += 1
str_len = len_byte >> 1  # zigzag decode for positive
print(f'measurementID length: {str_len}')
measurement_id = data[offset:offset+str_len].decode(); offset += str_len
print(f'measurementID: {measurement_id}')

# dataType
dt = data[offset]; offset += 1
print(f'dataType: {dt}')

# chunk size
cs = data[offset]; offset += 1
print(f'chunkMetaDataListDataSize: {cs}')

# Statistics
print(f'\nStatistics at {offset}:')
cnt = data[offset]; offset += 1
print(f'count: {cnt}')

st = struct.unpack('<q', data[offset:offset+8])[0]; offset += 8
print(f'startTime: {st} (0x{st:016x})')

et = struct.unpack('<q', data[offset:offset+8])[0]; offset += 8
print(f'endTime: {et} (0x{et:016x})')

sdt = data[offset]; offset += 1
print(f'stats.dataType: {sdt}')

# If INT64
if sdt == 2:
    min_val = struct.unpack('<q', data[offset:offset+8])[0]; offset += 8
    print(f'min: {min_val}')
    max_val = struct.unpack('<q', data[offset:offset+8])[0]; offset += 8
    print(f'max: {max_val}')
    first = struct.unpack('<q', data[offset:offset+8])[0]; offset += 8
    print(f'first: {first}')
    last = struct.unpack('<q', data[offset:offset+8])[0]; offset += 8
    print(f'last: {last}')
    sum_val = struct.unpack('<q', data[offset:offset+8])[0]; offset += 8
    print(f'sum: {sum_val}')

print(f'\nTotal Statistics size: {offset - (124 + 4)} bytes')
print(f'Total TimeseriesMetadata size: {offset - 124} bytes')
