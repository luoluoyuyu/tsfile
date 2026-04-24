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
"""
详细的TsFile格式分析工具
用于验证Rust生成的TsFile是否与Java完全兼容
"""
import struct
import sys

def read_unsigned_varint(data, offset):
    """Read unsigned varint"""
    result = 0
    shift = 0
    while True:
        b = data[offset]
        result |= (b & 0x7F) << shift
        offset += 1
        if (b & 0x80) == 0:
            break
        shift += 7
    return result, offset

def read_varint_string(data, offset):
    """Read varint-string with zigzag encoding"""
    length, offset = read_unsigned_varint(data, offset)
    # Zigzag decode
    if length & 1:
        length = ~(length >> 1)
    else:
        length = length >> 1
    length = abs(length)
    
    if length == 0:
        return "", offset
    
    string_data = data[offset:offset+length].decode('utf-8', errors='replace')
    return string_data, offset + length

def analyze_tsfile(filepath):
    """Complete TsFile analysis"""
    with open(filepath, 'rb') as f:
        data = f.read()
    
    print("="*80)
    print(f"TsFile Analysis: {filepath}")
    print("="*80)
    print(f"File size: {len(data)} bytes\n")
    
    offset = 0
    
    # 1. Magic string (6 bytes)
    magic = data[0:6]
    print(f"[0-5] Magic: {magic}")
    offset = 6
    
    # 2. Version number (2 bytes, big-endian)
    version = struct.unpack('>H', data[6:8])[0]
    print(f"[6-7] Version: {version}")
    offset = 8
    
    # 3. Chunk data section
    print(f"\n[8-?] Chunk Data Section")
    print("-" * 80)
    
    # Find all chunk group headers
    chunk_groups = []
    pos = 8
    while pos < len(data) - 50:  # Reserve space for metadata
        marker = data[pos]
        
        # ChunkGroupHeader marker = 0
        if marker == 0:
            pos += 1
            device_id, pos = read_varint_string(data, pos)
            print(f"  ChunkGroupHeader at {pos-1-len(device_id)-1}: device='{device_id}'")
            chunk_groups.append({'device': device_id, 'start': pos})
        
        # Chunk markers (1 or 5)
        elif marker in [1, 5]:
            pos += 1
            # Chunk header parsing would go here
            # For now, just skip
            pos += 1  # measurementID length
            measurement_id, pos = read_varint_string(data, pos-1)
            print(f"  ChunkHeader at {pos}: measurement='{measurement_id}', marker={marker}")
            # Skip rest of chunk header and data
            # This is simplified - would need full parsing
            pos += 20  # Rough skip
        
        else:
            # Likely reached SEPARATOR or metadata
            break
    
    # 4. Find SEPARATOR (0x02)
    print(f"\nSearching for SEPARATOR...")
    sep_pos = -1
    for i in range(8, len(data) - 50):
        if data[i] == 0x02:
            # Check if next looks like TimeseriesMetadata
            if i + 1 < len(data) and data[i+1] in [0, 1]:
                sep_pos = i
                break
    
    if sep_pos == -1:
        print("  ERROR: SEPARATOR not found!")
        return
    
    print(f"  SEPARATOR at: [{sep_pos}]")
    print(f"  Metadata starts at: {sep_pos + 1}")
    
    # 5. Parse TimeseriesMetadata
    print(f"\nTimeseriesMetadata Section")
    print("-" * 80)
    
    offset = sep_pos + 1
    ts_list = []
    
    while offset < len(data) - 100:  # Reserve space for index nodes and footer
        type_byte = data[offset]
        
        # If type is not 0 or 1, we've reached MetadataIndexNode
        if type_byte > 3:
            print(f"\n  Reached MetadataIndexNode at {offset} (type={type_byte})")
            break
        
        ts_start = offset
        offset += 1
        
        # measurementID
        measurement_id, offset = read_varint_string(data, offset)
        
        # dataType
        data_type = data[offset]
        offset += 1
        
        # chunkMetaDataListDataSize
        chunk_data_size, offset = read_unsigned_varint(data, offset)
        
        type_names = {0: 'BOOLEAN', 1: 'INT32', 2: 'INT64', 3: 'FLOAT', 4: 'DOUBLE', 5: 'TEXT'}
        
        print(f"\n  TimeseriesMetadata at {ts_start}:")
        print(f"    type_byte: {type_byte}")
        print(f"    measurementID: '{measurement_id}'")
        print(f"    dataType: {data_type} ({type_names.get(data_type, 'UNKNOWN')})")
        print(f"    chunkMetaDataListDataSize: {chunk_data_size}")
        
        # Statistics
        stats_start = offset
        count, offset = read_unsigned_varint(data, offset)
        start_time = struct.unpack('>q', data[offset:offset+8])[0]
        offset += 8
        end_time = struct.unpack('>q', data[offset:offset+8])[0]
        offset += 8
        
        print(f"    Statistics:")
        print(f"      count: {count}")
        print(f"      startTime: {start_time}")
        print(f"      endTime: {end_time}")
        
        # Skip typed stats based on dataType
        stats_size = 0
        if data_type == 1:  # INT32
            stats_size = 4*4 + 8  # 4*i32 + sum(i64)
        elif data_type == 2:  # INT64
            stats_size = 5*8  # 5*i64
        elif data_type == 3:  # FLOAT
            stats_size = 4*4 + 8  # 4*f32 + sum(f64)
        elif data_type == 4:  # DOUBLE
            stats_size = 5*8  # 5*f64
        
        offset += stats_size
        print(f"      Statistics total size: {offset - stats_start}")
        
        # Chunk metadata
        if chunk_data_size > 0:
            print(f"    Chunk metadata ({chunk_data_size} bytes):")
            chunk_start = offset
            
            # Parse based on type_byte
            has_chunk_stats = (type_byte & 0x3F) != 0
            
            while offset < chunk_start + chunk_data_size:
                chunk_offset = struct.unpack('>q', data[offset:offset+8])[0]
                offset += 8
                print(f"      chunk offset: {chunk_offset}")
                
                if has_chunk_stats:
                    # Skip chunk statistics
                    cnt, offset = read_unsigned_varint(data, offset)
                    offset += 16  # start + end time
                    offset += stats_size  # typed stats
                    print(f"        chunk stats: count={cnt}")
            
            print(f"      Chunk metadata total: {offset - chunk_start} bytes")
        
        ts_size = offset - ts_start
        print(f"    Total TimeseriesMetadata size: {ts_size}")
        ts_list.append({
            'measurement': measurement_id,
            'type': data_type,
            'offset': ts_start,
            'size': ts_size
        })
    
    # 6. MetadataIndexNode section
    print(f"\nMetadataIndexNode Section")
    print("-" * 80)
    
    index_start = offset
    # Parse MetadataIndexNode structure
    # node_type (1 byte)
    if offset < len(data):
        node_type = data[offset]
        offset += 1
        
        type_names = {
            0: 'INTERNAL_DEVICE',
            1: 'LEAF_DEVICE', 
            2: 'INTERNAL_MEASUREMENT',
            3: 'LEAF_MEASUREMENT'
        }
        
        print(f"\n  MetadataIndexNode at {index_start}:")
        print(f"    node_type: {node_type} ({type_names.get(node_type, 'UNKNOWN')})")
        
        # end_offset (i64)
        if offset + 8 <= len(data):
            end_offset = struct.unpack('>q', data[offset:offset+8])[0]
            offset += 8
            print(f"    end_offset: {end_offset}")
            
            # children count
            if offset < len(data):
                children_count, offset = read_unsigned_varint(data, offset)
                print(f"    children_count: {children_count}")
                
                for i in range(children_count):
                    if offset + 8 <= len(data):
                        child_offset = struct.unpack('>q', data[offset:offset+8])[0]
                        offset += 8
                        child_name, offset = read_varint_string(data, offset)
                        print(f"      child[{i}]: '{child_name}' at {child_offset}")
    
    # 7. TsFileMetadata
    print(f"\nTsFileMetadata Section")
    print("-" * 80)
    
    meta_start = offset
    if offset < len(data):
        # Should contain another MetadataIndexNode (root)
        node_type = data[offset]
        offset += 1
        print(f"\n  Root MetadataIndexNode at {meta_start}:")
        print(f"    node_type: {node_type}")
        
        if offset + 8 <= len(data):
            end_offset = struct.unpack('>q', data[offset:offset+8])[0]
            offset += 8
            print(f"    end_offset: {end_offset}")
    
    # 8. Footer
    if offset + 4 <= len(data):
        meta_size = struct.unpack('>i', data[offset:offset+4])[0]
        offset += 4
        print(f"\n  Metadata size: {meta_size}")
    
    if offset < len(data):
        closing_magic = data[offset:offset+6]
        print(f"  Closing magic: {closing_magic}")
        offset += 6
    
    print(f"\n{'='*80}")
    print(f"Analysis Complete")
    print(f"{'='*80}")
    print(f"\nTotal TimeseriesMetadata: {len(ts_list)}")
    for ts in ts_list:
        print(f"  - {ts['measurement']} (type={ts['type']}) at {ts['offset']}, size={ts['size']}")

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: python3 analyze_tsfile_complete.py <tsfile_path>")
        sys.exit(1)
    
    filepath = sys.argv[1]
    analyze_tsfile(filepath)
