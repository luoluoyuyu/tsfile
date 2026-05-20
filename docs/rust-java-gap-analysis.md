# Rust 与 Java 实现差异扫描报告

生成时间：2026-05-11  
扫描范围：

- `java/tsfile/src/main/java/org/apache/tsfile`
- `rust/src`

校验动作：

- 结构扫描：目录、模块、公开 API、核心类对照
- 人工抽查：`TsFileReader`、`TsFileSequenceReader`、`TsFileWriter`、`Tablet`、`MeasurementSchema`、`Filter`、`QueryExpression`、`TsBlock`、`TSFileConfig`、`ReadWriteIOUtils`
- 基础验证：`cargo test` 已通过

说明：

- 文中的“方法数量”是基于源码文本扫描得到的近似值，用于看规模差距，不是 AST 级精确统计。
- Java 侧为了避免把 `external/commons` 这种内嵌三方代码混进核心结论，下面优先使用“排除 `external/commons` 后”的统计口径。

## 1. 总结结论

结论非常明确：当前 Rust 版本还不是 Java 版本的 feature parity 实现，更接近“可读写基础 TsFile 的 MVP + 一部分兼容 API 壳”。从模块覆盖、公开 API、测试规模、执行路径复杂度四个角度看，都明显落后于 Java。

这不是简单的“少几个方法”，而是三层差距同时存在：

1. 模块层缺失：Java 有完整的查询执行、过滤器工厂、metadata controller、filesystem、encryption、config descriptor、regexp、cache、parser/type 等体系，Rust 侧很多根本没有。
2. API 层缺失：即使 Rust 有同名 struct 或 trait，很多 Java 公共方法也没有暴露出来。
3. 语义层不等价：有些 Rust 枚举和 API 名字看上去对齐，但内部实际做的是降级实现、别名实现，甚至直接 fallback。

补充判断：

- 如果只看“能不能写入一份文件再读出来”，Rust 现在已经不是空壳，主路径是能跑的。
- 但如果按“代码量 + 目录级类族 + 执行链完整性”判断，当前依然明显没有到 Java 对齐阶段。
- 现在更准确的表述应该是：
  - `write/v4` 主写路径已经从 `UnsupportedOperation` 提升到“可用”
  - `read/controller`、`read/query` 已经从“完全没有”提升到“有可运行骨架”
  - 但距离 Java 的完整查询栈、流式 block reader、time generator、aligned metadata/raw API 仍然差很多

## 2. 规模对比

按核心功能代码统计：

| 维度 | Java（排除 `external/commons`） | Rust |
| --- | ---: | ---: |
| 源码文件数 | 455 | 144 |
| 公开方法匹配数 | 3890 | 611 |
| 测试文件数 | 174 | 1 |
| 测试方法/标记数 | 855 | 16 |

如果把 `external/commons` 也算进去，Java 文件数约 580、公开方法匹配数约 4481，体量差距会更大。

这说明 Rust 目前还处于“功能骨架已搭起来，但大部分深层能力没补齐”的阶段。

## 3. 当前 Rust 实现里比较明显的问题

### 3.0 用代码量直接看当前完成度

下面这组数字不是最终真理，但对“是不是已经差不多补完”很有参考价值：

| 对比对象 | Java LOC | Rust LOC | Rust / Java |
| --- | ---: | ---: | ---: |
| `TsFileSequenceReader` | 3400 | 861 | 25.3% |
| `read/query` 目录 | 2592 | 433 | 16.7% |
| `read/controller` 目录 | 755 | 230 | 30.5% |
| `TsFileWriter` | 830 | 495 | 59.6% |
| `TableTsBlock2TsFileWriter` | 270 | 197 | 73.0% |

解释：

- `TableTsBlock2TsFileWriter` 和 `TsFileWriter` 的比例已经不算极低，说明写路径主干确实补上了一大截。
- 但 `TsFileSequenceReader` 只有约四分之一，`read/query` 只有约六分之一，这直接说明查询执行和底层 reader 仍然不是 Java 等价实现。
- 更关键的是，Java `read/query` 是 22 个文件，Rust 现在只有 3 个文件；这不是语法差异能解释掉的量级。

### 3.1 名义支持和真实支持不一致

#### 3.1.1 压缩算法存在“声明支持，但实际不等价”

文件：

- `rust/src/compress/mod.rs:29`

问题：

- `Zstd` 和 `Lzma2` 没有真实实现，而是直接退化成 `Uncompressed`
- `Lzo`、`Sdt`、`Paa`、`Pla` 只保留了枚举兼容，但运行时直接 `UnsupportedOperation`

影响：

- 如果读到真实使用 `ZSTD` / `LZMA2` 压缩的 TsFile，行为很可能不正确
- 从 API 看像是支持了，但实际上是“能编译、不能等价跑”

### 3.2 编码器支持矩阵与真实实现不一致

文件：

- `rust/src/common/enums.rs:265`
- `rust/src/encoding/encoder.rs:108`
- `rust/src/encoding/decoder.rs:101`

问题：

- `is_supported_for()` 把 `CHIMP`、`SPRINTZ`、`RLBE`、`CAMEL` 视为支持
- 但 `create_encoder()` / `create_decoder()` 实际上把这些编码统一映射到 `Gorilla`
- `DIFF` 直接映射到了 `Zigzag`

这类问题比“方法没写”更危险，因为它会制造一种“兼容已经完成”的假象。

### 3.3 Reader 过度依赖全量加载，查询执行模型过薄

文件：

- `rust/src/read/tsfile_reader.rs:49`
- `rust/src/read/tsfile_reader.rs:105`
- `rust/src/read/expression/mod.rs:20`

问题：

- `TsFileReader` 的很多能力是先 `read_all_data()` 再本地拼装
- `query()` 只支持“单设备 + measurement 列表 + 可选时间范围”
- `QueryExecutor` 的 filter 逻辑是“只要任意字段满足就保留整行”，与 Java 的表达式树 / 查询执行器模型差得很远

影响：

- 大文件会有明显内存放大
- 复杂查询语义无法和 Java 对齐
- filter pushdown 仅停留在非常浅的层面

### 3.4 TsFileSequenceReader 对文件校验过于宽松

文件：

- `rust/src/read/tsfile_sequence_reader.rs:57`
- `rust/src/read/tsfile_sequence_reader.rs:169`

问题：

- `verify_magic()` 读了 version byte，但没有真正校验版本合法性
- 扫描 chunk 时遇到未知 marker 直接 `break`，不是报错

影响：

- 不利于发现损坏文件或不兼容版本
- 可能出现“悄悄读少了，但调用方不知道”

### 3.5 Writer 的内存估算和顺序检查都明显简化了

文件：

- `rust/src/write/tsfile_writer.rs:197`
- `rust/src/write/tsfile_writer.rs:139`

问题：

- flush 阈值判断使用的是 `statistics().serialized_size()` 近似，不是 chunk/page 真正的内存占用
- 只按 `device -> last_timestamp` 做顺序检查，而 Java 侧区分 aligned device 和 non-aligned timeseries

影响：

- flush 时机可能明显失真
- 顺序写校验语义和 Java 不一致

### 3.6 Tablet 数据结构是行式的，和 Java 的列式批写模型差距很大

文件：

- `rust/src/write/tablet.rs:10`
- `rust/src/write/tsfile_writer.rs:218`

问题：

- Rust `Tablet` 用 `Vec<Vec<Option<DataPointValue>>>` 存每一行
- 写入时先 `to_records()`，再逐行转成 `TSRecord`
- Java `Tablet` 是面向批写的列式结构，配套 bitmap、primitive array、column category、序列化/反序列化接口

影响：

- 性能和内存利用率都比 Java 低很多
- 根本没有发挥 batch write 的优势

### 3.7 配置体系非常薄，而且内部默认值不一致

文件：

- `rust/src/common/config.rs:8`
- `rust/src/write/schema.rs:132`

问题：

- Rust 只有一个很薄的 `TsFileConfig` 和 `get_config()`
- 没有 Java `TSFileDescriptor` 的 properties 加载、环境注入、descriptor 单例、细粒度 getter/setter
- `MeasurementSchemaBuilder` 默认 compression 写死成 `Lz4`
- 但 `TsFileConfig::default()` 的默认 compressor 是 `Snappy`

这属于内部设计不一致，后面会变成维护负担。

## 4. 模块覆盖率差异

### 4.1 Java 已完整、Rust 基本缺失或只有很薄壳的区域

| Java 模块 | Rust 情况 | 说明 |
| --- | --- | --- |
| `encrypt` | 缺失 | 没有等价加密/解密与参数体系 |
| `fileSystem` | 缺失 | 没有 FSFactory / HDFS / object storage 抽象 |
| `read/controller` | 部分实现 | 已有 metadata querier / chunk loader 骨架，但缺 iterator、space partition、chunk metadata loader |
| `read/query` | 部分实现 | 已有 `QueryDataSet`、executor、table query facade，但缺 time generator、task、流式 block reader/result set 体系 |
| `read/filter/basic/factory/operator` | 极简 | Rust 只有一个扁平 `Filter` 枚举 |
| `read/common/parser` | 缺失 | 没有 path parser、visitor、错误模型 |
| `read/common/type` | 缺失 | 没有完整 Type 系统 |
| `common/regexp` | 缺失 | LIKE / regexp 相关能力没有 |
| `common/cache` | 缺失 | LRU cache 等基础组件没有 |
| `common/conf` | 极简 | 只有简化版 config，没有 descriptor 体系 |
| `compatibility` | 缺失 | 老版本兼容与反序列化配置缺失 |
| `encoding/fire` | 缺失 | 没有 FIRE 相关逻辑 |

### 4.2 Rust 已经有骨架，但深度明显不足的区域

| 模块 | 现状 |
| --- | --- |
| `compress` | 基本骨架有，但压缩格式支持不完整 |
| `encoding` | 基本编码器/解码器有，但多种算法只是别名或退化实现 |
| `file/metadata` | 结构体很多，但和 Java 的 metadata API 深度仍有差距 |
| `read` | 有基本 reader，且已补一部分 raw metadata/chunk/page API，但查询栈仍然很浅 |
| `write` | 能完成基础写入，且已补 template/table/aligned 高层入口，但内部 writer 分支仍简化 |
| `write/v4` | facade 已经可用，`TableTsBlock2TsFileWriter.write_tsblock()` 已补，但还是行式桥接实现 |
| `read/block` | 有 `TsBlock` / `Column` 骨架，但远不如 Java 列式实现完整 |
| `utils` | 只有常用 IO/encoding helpers，和 Java 的工具集差距极大 |

## 5. 关键类与方法差异

下面按“用户最容易直接调用的 API”来列。

### 5.1 `TsFileReader`

对照：

- Java: `java/tsfile/src/main/java/org/apache/tsfile/read/TsFileReader.java`
- Rust: `rust/src/read/tsfile_reader.rs`

现状判断：

- Rust 不是缺一个两个方法，而是整个查询语义与 Java 不同

Rust 已有：

- `new`
- `read_timeseries`
- `read_timeseries_by_time_range`
- `get_point_reader`
- `query`
- `read_rows`
- `get_all_devices`
- `get_measurements_for_device`
- `contains_device`
- `contains_timeseries`
- `read_all`

Java 有但 Rust 没有同名入口：

- `getAllDeviceIds`
- `getMeasurement`
- `close`

说明：

- `close` 缺失意味着 Rust 高层 reader 没有明确生命周期 API
- `getMeasurement` 缺失导致不能像 Java 那样直接读取 `MeasurementSchema`
- Rust 自己新增的 `read_all()` / `contains_*()` 更像便捷 API，不是 Java parity

### 5.2 `TsFileSequenceReader`

对照：

- Java: `java/tsfile/src/main/java/org/apache/tsfile/read/TsFileSequenceReader.java`
- Rust: `rust/src/read/tsfile_sequence_reader.rs`

Java 规模：

- 公开方法约 75 个

Rust 规模：

- 公开方法约 15 个

Rust 已有的核心入口：

- `read_file_metadata`
- `read_all_chunks`
- `read_chunk_infos`
- `read_all_data`
- `read_chunk_by_metadata`
- `read_by_chunk_metadata`
- `read_by_chunk_metadata_with_filter`
- `read_timeseries_metadata_at`
- `read_metadata_index_node_at`
- `find_timeseries_metadata_offset`
- `read_timeseries_metadata`
- `read_timeseries_by_index`
- `read_timeseries_by_index_with_filter`
- `read_timeseries_by_metadata_offset`
- `read_chunk_data`

Java 有而 Rust 明显缺失的方法，按类别列出：

文件与完整性相关：

- `loadMetadataSize`
- `getFileMetadataPos`
- `getTsFileMetadataSize`
- `getFileMetadataSize`
- `getAllMetadataSize`
- `readTailMagic`
- `isComplete`
- `readHeadMagic`
- `readVersionNumber`
- `position`
- `readRaw`
- `selfCheck`
- `selfCheckWithInfo`
- `close`
- `getFileName`
- `fileSize`

table schema / bloom / encryption：

- `setEnableCacheTableSchemaMap`
- `getTableSchemaMap`
- `readBloomFilter`
- `getEncryptParam`
- `getFirstEncryptParam`
- `getDeserializeContext`

device / measurement metadata：

- `readDeviceMetadata`
- `clearCachedDeviceMetadata`
- `readTimeseriesMetadata`
- `readITimeseriesMetadata`
- `getAllDevices`
- `getAllDevicesIteratorWithIsAligned`
- `getLazyDeviceIterator`
- `getTableDevicesIteratorWithIsAligned`
- `readChunkMetadataInDevice`
- `getAllPaths`
- `getPathsIterator`
- `hasNext`
- `next`
- `isAlignedDevice`
- `getTimeColumnMetadata`
- `getTimeseriesMetadataOffsetByDevice`
- `getChunkMetadataListByTimeseriesMetadataOffset`
- `getDeviceTimeseriesMetadata`
- `getAllTimeseriesMetadata`
- `iterAllTimeseriesMetadata`
- `getDeviceTimeseriesMetadataWithoutChunkMetadata`
- `getAllMeasurements`
- `getMeasurement`
- `getFullPathDataTypeMap`
- `getDeviceMeasurementsMap`
- `getDeviceNameInRange`

chunk / page 级低层读取：

- `readChunkGroupHeader`
- `readPlanIndex`
- `readChunkHeader`
- `readChunk`
- `readMemChunk`
- `readTimeseriesCompressionTypeAndEncoding`
- `getMeasurementSchema`
- `readPageHeader`
- `skipPageData`
- `readCompressedPage`
- `readPage`
- `readMarker`
- `checkChunkAndPagesStatistics`
- `getChunkMetadataList`
- `getIChunkMetadataList`
- `getAlignedChunkMetadata`
- `getAlignedChunkMetadataByMetadataIndexNode`
- `readChunkMetaDataList`
- `readIChunkMetaDataList`

plan index / iterator：

- `countChunksPerChunkGroup`
- `readFileMetadata`
- `getMinPlanIndex`
- `getMaxPlanIndex`
- `getMeasurementChunkMetadataListMapIterator`

这块是 Rust 与 Java 差距最大的核心类之一。

### 5.3 `TsFileWriter`

对照：

- Java: `java/tsfile/src/main/java/org/apache/tsfile/write/TsFileWriter.java`
- Rust: `rust/src/write/tsfile_writer.rs`

Rust 已有：

- `new`
- `new_with_schema`
- `set_chunk_group_size_threshold`
- `set_unseq`
- `record_count`
- `schema`
- `register_timeseries`
- `register_timeseries_batch`
- `write`
- `write_tablet`
- `write_tablet_and_reset`
- `flush_all_chunk_groups`
- `close`

Java 有而 Rust 缺失：

配置与注册：

- `setMemoryThreshold`
- `registerSchemaTemplate`
- `registerDevice`
- `registerTimeseries`
- `registerAlignedTimeseries`
- `registerTableSchema`

多写入模式：

- `writeRecord`
- `writeTree`
- `writeAligned`
- `writeTable`

状态/控制入口：

- `flush`
- `getIOWriter`
- `getSchema`
- `isTableWriteAligned`
- `setTableWriteAligned`
- `isGenerateTableSchemaForTree`
- `setGenerateTableSchema`

不仅缺方法，行为差异也很明显：

- Java 有 aligned / non-aligned / table model 多条写入路径
- Rust 目前本质上还是围绕“device + measurement + record/tablet”这条简化路径

### 5.4 `Tablet`

对照：

- Java: `java/tsfile/src/main/java/org/apache/tsfile/write/record/Tablet.java`
- Rust: `rust/src/write/tablet.rs`

Rust 已有：

- `new`
- `row_size`
- `max_row_number`
- `is_full`
- `reset`
- `add_row`
- `to_records`

Java 有而 Rust 缺失的核心能力：

结构与元数据：

- `setInsertTargetName`
- `setSchemas`
- `getSchemas`
- `getMaxRowNumber`
- `getDeviceID`
- `getDeviceId`
- `setDeviceId`
- `getTableName`
- `setTableName`
- `setColumnCategories`
- `getColumnTypes`

逐列填充与空值管理：

- `initBitMaps`
- `addTimestamp`
- `addValue`
- `addObjectPathValue`
- `isNull`
- `getValue`
- `getBitMaps`
- `setBitMaps`

直接暴露底层批量数组：

- `getTimestamp`
- `getTimestamps`
- `setTimestamps`
- `getValues`
- `setValues`
- `getRowSize`
- `setRowSize`

序列化与可比较性：

- `serialize`
- `deserialize`
- `readBitMapsFromBuffer`
- `readvaluesFromBuffer`
- `equals`
- `append`
- `ramBytesUsed`
- `isSorted`

结论：

- Rust `Tablet` 目前更像“为了提供 batch 写接口而临时拼出来的行缓冲”
- Java `Tablet` 则是生产级别的批写数据结构

### 5.5 `MeasurementSchema` / `MeasurementSchemaBuilder`

对照：

- Java: `java/tsfile/src/main/java/org/apache/tsfile/write/schema/MeasurementSchema.java`
- Rust: `rust/src/write/schema.rs`

Rust `MeasurementSchema` 已有：

- `new`
- `with_props`
- `validate`
- `serialize`
- `deserialize`

Java 有而 Rust 缺失：

- `deserializeFrom`
- `partialDeserializeFrom`
- `getSchemaType`
- `getMeasurementName`
- `setMeasurementName`
- `getProps`
- `getEncodingType`
- `getType`
- `getTypeInByte`
- `getTimeTSEncoding`
- `setProps`
- `getTimeEncoder`
- `getSubMeasurementsList`
- `getSubMeasurementsTSDataTypeList`
- `getSubMeasurementsTSEncodingList`
- `getSubMeasurementsEncoderList`
- `getValueEncoder`
- `getCompressor`
- `serializeTo`
- `serializedSize`
- `partialSerializeTo`
- `isLogicalView`
- `equals`
- `hashCode`
- `compareTo`
- `toString`
- `setDataType`
- `getSubMeasurementIndex`
- `getSubMeasurementsCount`
- `containsSubMeasurement`
- `setEncoding`
- `setCompressionType`
- `ramBytesUsed`

Rust `MeasurementSchemaBuilder` 已有：

- `new`
- `with_encoding`
- `with_compression`
- `with_property`
- `with_properties`
- `build`

Java 有而 Rust 没有同名 camelCase 入口：

- `withEncoding`
- `withCompression`
- `withProperty`
- `withProperties`

语义问题：

- Rust builder 默认压缩写死为 `Lz4`
- Java builder 默认取 `TSFileDescriptor.getInstance().getConfig().getCompressor(dataType)`

### 5.6 `BatchData`

对照：

- Java: `java/tsfile/src/main/java/org/apache/tsfile/read/common/BatchData.java`
- Rust: `rust/src/read/common.rs`

现状：

- Rust `BatchData` 只是 `Vec<(i64, TimeValue)> + cursor`
- Java `BatchData` 是专门优化过的类型化批量数据结构

Java 有而 Rust 缺失的主要方法：

生命周期与元数据：

- `init`
- `getDataType`
- `setDataType`
- `getBatchDataType`
- `resetBatchData`
- `close`

类型化写入：

- `putBoolean`
- `putInt`
- `putLong`
- `putFloat`
- `putDouble`
- `putBinary`
- `putVector`
- `putAnObject`
- `setTime`

类型化读取：

- `getBoolean`
- `getInt`
- `getLong`
- `getFloat`
- `getDouble`
- `getBinary`
- `getVector`
- `currentTsPrimitiveType`

按索引和按时间检索：

- `getTimeByIndex`
- `getLongByIndex`
- `getDoubleByIndex`
- `getIntByIndex`
- `getFloatByIndex`
- `getBinaryByIndex`
- `getBooleanByIndex`
- `getVectorByIndex`
- `getLastPairBeforeOrEqualTimestamp`
- `getValueInTimestamp`
- `getMaxTimestamp`
- `getMinTimestamp`

迭代与序列化：

- `serializeData`
- `deserialize`
- `flip`
- `getBatchDataIterator`
- `totalLength`
- `hasNextTimeValuePair`
- `nextTimeValuePair`
- `currentTimeValuePair`
- `getUsedMemorySize`

### 5.7 `QueryExpression` / `Filter` / 查询栈

对照：

- Java: `read/expression`, `read/filter/basic`, `read/filter/factory`, `read/filter/operator`, `read/query`
- Rust: `rust/src/read/expression/mod.rs`, `rust/src/read/filter/mod.rs`, `rust/src/read/result_set.rs`

Rust 现状：

- `QueryExpression` 只支持：
  - `device_id`
  - `measurements`
  - `time_range`
- `Filter` 只支持：
  - `Eq / NotEq / Gt / GtEq / Lt / LtEq`
  - `TimeBetween`
  - `ValueIsNull / ValueIsNotNull`
  - `And / Or / Not`

Java 有而 Rust 缺失的关键能力：

`QueryExpression` 侧：

- `create`
- `addSelectedPath`
- `setSelectSeries`
- `getExpression`
- `setExpression`
- `getSelectedSeries`
- `hasQueryFilter`
- `getDataTypes`
- `setDataTypes`

Filter DSL / operator 侧：

- `FilterFactory`
- `TimeIn`
- `TimeNotIn`
- `TimeBetweenAnd`
- `TimeNotBetweenAnd`
- `ValueIn`
- `ValueNotIn`
- `ValueRegexp`
- `ValueNotRegexp`
- `ValueLike`
- `ValueNotLike`
- `GroupByFilter`
- `GroupByMonthFilter`
- `ExtractTimeFilterOperators`
- `ExtractValueFilterOperators`
- `satisfyRow`
- `satisfyTsBlock`
- `canSkip`
- `allSatisfy`
- `satisfyStartEndTime`
- `containStartEndTime`
- `getTimeRanges`
- `reverse`
- `copy`
- `serialize`
- `deserialize`

查询执行侧：

- `TsFileExecutor`
- `QueryDataSet`
- `MetadataQuerierByFileImpl`
- `CachedChunkLoaderImpl`
- `IChunkLoader`
- `IMetadataQuerier`
- time generator 相关能力

这部分不补，Rust 只能算“能读”，不能算“具备 Java 查询能力”。

### 5.8 `TsBlock` / `Column`

对照：

- Java: `read/common/block/TsBlock.java`、`common/block/column/*`
- Rust: `rust/src/read/block/tsblock.rs`、`rust/src/read/block/column.rs`

Rust `TsBlock` 已有：

- `new`
- `position_count`
- `time_by_index`
- `column`
- `columns`
- `times`
- `value_column_count`
- `is_empty`
- `region`
- `row`
- `serialize`
- `deserialize`

Java `TsBlock` 有而 Rust 缺失：

- `wrapBlocksWithoutCopy`
- `setPositionCount`
- `getStartTime`
- `getEndTime`
- `getRetainedSizeInBytes`
- `getSizeInBytes`
- `getRegion` 的 view/copy 区分
- `appendValueColumns`
- `insertValueColumn`
- `subTsBlock`
- `skipFirst`
- `getTimeColumn`
- `getValueColumns`
- `getTimeAndValueColumn`
- `getColumns`
- `getAllColumns`
- `getTsBlockSingleColumnIterator`
- `getTsBlockRowIterator`
- `getTsBlockAlignedRowIterator`
- `reverse`
- `reset`
- `currentTimeValuePair`
- `buildTsBlock`
- `fillTrailingNulls`

`Column` 侧更明显：

- Java 是接口 + 多种专用实现（`IntColumn`、`LongColumn`、`BinaryColumn`、`DictionaryColumn`、`RunLengthEncodedColumn` 等）
- Rust 目前是一个统一的 `enum ColumnValue + Vec<ColumnValue>` 泛型容器

这意味着 Rust 在列式执行、内存布局、零拷贝视图、字典编码列、RLE 列这些方面都还没接近 Java。

### 5.9 `TSFileConfig` / `TSFileDescriptor`

对照：

- Java: `common/conf/TSFileConfig.java`, `TSFileDescriptor.java`
- Rust: `rust/src/common/config.rs`

Rust 目前只有：

- `TsFileConfig::new`
- `get_config`

Java `TSFileConfig` 有而 Rust 缺失的大类方法：

基础参数：

- `getGroupSizeInByte`
- `setGroupSizeInByte`
- `getPageSizeInByte`
- `setPageSizeInByte`
- `getMaxNumberOfPointsInPage`
- `setMaxNumberOfPointsInPage`
- `getMaxDegreeOfIndexNode`
- `setMaxDegreeOfIndexNode`
- `getMaxStringLength`
- `setMaxStringLength`
- `getFloatPrecision`
- `setFloatPrecision`

编码与压缩：

- `getTimeEncoder`
- `setTimeEncoder`
- `getValueEncoder`
- `setValueEncoder`
- `getCompressor`
- `setCompressor`
- 各类型单独的 `get/setBooleanEncoding`、`get/setInt32Encoding`、`get/setInt64Encoding`、`get/setFloatEncoding`、`get/setDoubleEncoding`、`get/setTextEncoding`
- 各类型单独的 `setBooleanCompression`、`setInt32Compression`、`setInt64Compression`、`setFloatCompression`、`setDoubleCompression`、`setTextCompression`

加密与运行环境：

- `getEncryptType`
- `setEncryptType`
- `getEncryptKey`
- `setEncryptKey`
- `setEncryptKeyFromToken`
- `setEncryptSalt`
- `getEncryptSalt`

存储系统：

- `getTSFileStorageFs`
- `setTSFileStorageFs`
- `getCoreSitePath`
- `setCoreSitePath`
- `getHdfsSitePath`
- `setHdfsSitePath`
- `getHdfsIp`
- `setHdfsIp`
- `getHdfsPort`
- `setHdfsPort`
- `getDfsNameServices`
- `setDfsNameServices`
- `getDfsHaNamenodes`
- `setDfsHaNamenodes`
- `isDfsHaAutomaticFailoverEnabled`
- `setDfsHaAutomaticFailoverEnabled`
- `getDfsClientFailoverProxyProvider`
- `setDfsClientFailoverProxyProvider`

其他运行参数：

- `getBatchSize`
- `setBatchSize`
- `getBloomFilterErrorRate`
- `setBloomFilterErrorRate`
- `getPageCheckSizeThreshold`
- `setPageCheckSizeThreshold`
- `getPatternMatchingThreshold`
- `setPatternMatchingThreshold`

`TSFileDescriptor` 侧，Rust 也缺：

- `getInstance`
- `getConfig`
- `overwriteConfigByCustomSettings`
- property loader 相关逻辑

### 5.10 `ReadWriteIOUtils`

对照：

- Java: `java/tsfile/src/main/java/org/apache/tsfile/utils/ReadWriteIOUtils.java`
- Rust: `rust/src/utils/read_write_io_utils.rs`

Rust 已覆盖的只是最常用一小部分：

- `write_bool`
- `write_byte`
- `write_i16`
- `write_i32`
- `write_i64`
- `write_f32`
- `write_f64`
- `write_bytes`
- `write_string`
- `write_var_int_string`
- `write_binary`
- `write_data_type`
- `write_compression_type`
- `write_encoding`
- `read_bool`
- `read_byte`
- `read_i16`
- `read_i32`
- `read_i64`
- `read_f32`
- `read_f64`
- `read_bytes`
- `read_string`
- `read_var_int_string`
- `read_binary`
- `read_data_type`
- `read_compression_type`
- `read_encoding`

Java 有而 Rust 缺失的大类方法：

- `readBoolObject`
- `readIsNull`
- `write(Map<String, String>)`
- `write(List<Map<String, String>>)`
- `writeWithoutSize`
- `sizeToWrite`
- `readShort`
- `readInt`
- `readLong`
- `readStringWithLength`
- `getByteBuffer`
- `readStringFromDirectByteBuffer`
- `readMap`
- `readLinkedHashMap`
- `readMaps`
- `readBytesWithSelfDescriptionLength`
- `readByteBufferWithSelfDescriptionLength`
- `readAsPossible`
- `readStringList`
- `writeStringList`
- `readIntegerSet`
- `writeIntegerSet`
- `readBooleanSet`
- `readLongSet`
- `readFloatSet`
- `readDoubleSet`
- `readBinarySet`
- `readStringSet`
- `readObjectSet`
- `writeObjectSet`
- `writeBooleanSet`
- `writeLongSet`
- `writeFloatSet`
- `writeDoubleSet`
- `writeBinarySet`
- `writeStringSet`
- `checkIfMagicString`
- `writeObject`
- `readObject`
- `writeInts`
- `readInts`
- `skip`

## 6. `v4` facade 对齐情况

### 6.1 `read/v4`

Java `ITsFileReader` 支持：

- `query(tableName, columnNames, startTime, endTime)`
- `query(..., tagFilter)`
- `getTableSchemas(tableName)`
- `getAllTableSchema`
- `close`

Rust `read/v4` 只有：

- `ITsFileReader::query(QueryExpression)`
- `TsFileTreeReader`
- `DeviceTableModelReader`

差异很大，尤其是 table model schema 查询能力基本没有。

### 6.2 `write/v4`

Java `ITsFileWriter` 支持：

- `write(Tablet)`
- `write(TSRecord)`
- `close()`

Rust `write/v4` 只有：

- trait `ITsFileWriter::write_tablet`
- `TsFileTreeWriter::register_timeseries`
- `close`
- `DeviceTableModelWriter::table_schema`

也就是说，Rust 的 `v4` 目前更像 facade 包装层，不是完整 API 对齐。

## 7. 为什么看起来“对比 Java 少了很多”

根因不是单一的，至少有五个：

1. Rust 现在主打的是“先把基础读写跑通”，不是“完整 Java API 平移”。
2. Java 的查询侧能力很重，包含 metadata controller、filter/operator DSL、query executor、dataset 体系；Rust 目前基本没有这套。
3. Java 的批写与列式块模型非常成熟，Rust 侧很多地方还是通用 `Vec` + enum 容器。
4. Java 的 config / filesystem / encrypt / compatibility 都是完整子系统，Rust 没跟上。
5. Java 的测试面非常大，Rust 目前只覆盖了少量 happy path。

## 8. 建议的改进优先级

### P0：先修正确性与“假支持”

1. 真正实现或显式禁用 `ZSTD` / `LZMA2`
2. 把 `CHIMP` / `SPRINTZ` / `RLBE` / `CAMEL` / `DIFF` 的“别名实现”改成：
   - 要么真实实现
   - 要么在 API 与文档中明确标记 unsupported
3. `TsFileSequenceReader` 严格校验 version / marker / footer
4. 把未知 marker 从 silent break 改成错误返回

### P1：补齐核心公共 API

1. `TsFileSequenceReader` 的 metadata / device / path / raw page/chunk API
2. `TsFileReader` 的 `close`、`getMeasurement`
3. `TsFileWriter` 的 aligned/table/schema registration 关键入口
4. `MeasurementSchema` / `Tablet` 的关键序列化与元数据接口

### P2：补查询执行栈

1. `read/filter` 按 Java 的 factory/operator/basic 分层
2. `QueryExpression` 改成 path + expression 树模型
3. 引入 `MetadataQuerier` / `ChunkLoader` / executor / dataset

### P3：补性能与工程化

1. `Tablet` 改回列式内存布局
2. `TsBlock` / `Column` 改成专用列实现
3. 完善 `TSFileConfig` / `TSFileDescriptor`
4. 增加 read/write/query 互操作测试与回归测试

## 9. 我对当前 Rust 版本的判断

如果目标是：

- “能写出简单 TsFile，能读回一些基础数据”，当前 Rust 可以继续迭代
- “作为 Java TsFile 的等价实现或替代 SDK”，当前 Rust 还远远不够

更准确地说，当前 Rust 版本最大的问题不是“没做完”，而是：

- 有些地方会让使用者误以为已经兼容
- 但实际上只是同名 API 或同名 enum，语义并没有对齐

这会比单纯缺方法更危险。

## 10. 建议后续工作方式

建议以后所有补齐动作都基于“类级对照表 + 状态跟踪表”推进，而不是零散补代码。  
我已经另外生成了一份可持续维护的跟踪表：

- `docs/rust-java-gap-tracker.md`

建议每次补完一个类或一个模块，就更新：

- 当前状态
- 补了哪些方法
- 还有哪些行为未对齐
- 是否已经有互操作测试
