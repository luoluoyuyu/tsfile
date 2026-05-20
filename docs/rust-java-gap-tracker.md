# Rust 与 Java 差异跟踪表

用途：

- 记录 Java 基准能力与 Rust 当前状态
- 记录缺失方法、行为差异、实现优先级
- 便于后续每次补齐后持续更新

状态建议：

- `未开始`
- `部分实现`
- `名义支持但不等价`
- `基本对齐`
- `已验证`

优先级建议：

- `P0` 正确性/兼容性
- `P1` 核心公共 API
- `P2` 查询与高级能力
- `P3` 性能与工程化

## 跟踪表

| ID | 模块/类 | Java 基准 | Rust 对应 | 当前状态 | 缺失方法/行为 | 优先级 | 是否有测试 | 负责人 | 目标版本 | 最后更新 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 压缩 | `CompressionType` + `ICompressor/IUnCompressor` | `rust/src/compress/mod.rs` | 名义支持但不等价 | `ZSTD`/`LZMA2` 未真实实现；`LZO/SDT/PAA/PLA` 运行期不支持 | P0 | 否 |  |  | 2026-05-09 | 先处理 correctness |
| 2 | 编码器矩阵 | `TSEncoding` + encoder/decoder | `rust/src/common/enums.rs`, `rust/src/encoding/*` | 名义支持但不等价 | `CHIMP/SPRINTZ/RLBE/CAMEL` 映射为 Gorilla；`DIFF` 映射为 Zigzag | P0 | 否 |  |  | 2026-05-09 | 需要 API 和实现同时修 |
| 3 | `TsFileSequenceReader` 文件校验 | `TsFileSequenceReader` | `rust/src/read/tsfile_sequence_reader.rs` | 部分实现 | version 校验不严格；unknown marker silent break | P0 | 否 |  |  | 2026-05-09 | 先补错误处理 |
| 4 | `TsFileSequenceReader` metadata API | `TsFileSequenceReader` | `rust/src/read/tsfile_sequence_reader.rs` | 部分实现 | 已补 `getAllDevices`、`getAllPaths`、`readDeviceMetadata`、`readChunkMetadataList`；仍缺 `iterAllTimeseriesMetadata`、device iterator、aligned metadata API | P1 | 集成测试 |  |  | 2026-05-11 | 查询主路径已可用，深层 API 还没齐 |
| 5 | `TsFileSequenceReader` raw chunk/page API | `TsFileSequenceReader` | `rust/src/read/tsfile_sequence_reader.rs` | 部分实现 | 已补 `readChunkHeader`、`readPageHeader`、`readPageData`、`readChunkPages`；仍缺 `readCompressedPage`、`skipPageData`、`readMarker`、`selfCheck`、raw range read | P1 | 集成测试 |  |  | 2026-05-11 | 查询栈前置依赖已补一部分 |
| 6 | `TsFileReader` 生命周期与 schema 查询 | `TsFileReader` | `rust/src/read/tsfile_reader.rs` | 基本对齐 | `close`、`getMeasurement` 已有；返回类型和 Java 仍不完全一致 | P1 | 集成测试 |  |  | 2026-05-11 | 高层 reader 主接口已可用 |
| 7 | `TsFileReader` 查询语义 | `TsFileReader` + executor/query dataset | `rust/src/read/tsfile_reader.rs`, `rust/src/read/expression/mod.rs` | 部分实现 | 已有基础 executor/query dataset，但没有 Java 的 expression optimizer、time generator、streaming result set | P2 | 集成测试 |  |  | 2026-05-11 | 现在是“可查”，不是“Java 等价查询引擎” |
| 8 | `read/filter` DSL | `read/filter/basic/factory/operator` | `rust/src/read/filter/mod.rs` | 部分实现 | 缺 `IN/NOT IN/LIKE/REGEXP/GROUP BY`、serialize/deserialize、TsBlock filter | P2 | 否 |  |  | 2026-05-09 | 建议拆目录而不是继续堆一个文件 |
| 9 | `read/query` 栈 | `QueryDataSet` / `TsFileExecutor` / timegenerator | `rust/src/read/query/*` | 部分实现 | 已补 `QueryDataSet`、`TsFileExecutor`、`TableQueryExecutor`；仍缺 `ExecutorWithTimeGenerator`、`DataSetWith/WithoutTimeGenerator`、`TreeResultSet`、`TableResultSet`、task、timegenerator 节点树 | P2 | 集成测试 |  |  | 2026-05-11 | 目录级缺口仍然很大 |
| 10 | `read/controller` 栈 | `MetadataQuerierByFileImpl` / `CachedChunkLoaderImpl` | `rust/src/read/controller/*` | 部分实现 | 已有 metadata querier、chunk loader、builder；仍缺 `DeviceMetaIterator`、`IChunkMetadataLoader`、space partition 转换、Java 式 LRU/定位逻辑 | P2 | 集成测试 |  |  | 2026-05-11 | 不是空白，但还远没到齐 |
| 11 | `Tablet` 数据结构 | `write/record/Tablet.java` | `rust/src/write/tablet.rs` | 部分实现 | 缺 bitmap、column category、serialize/deserialize、typed addValue；目前是行式结构 | P1 | 否 |  |  | 2026-05-09 | 同时影响性能 |
| 12 | `Tablet` 性能路径 | `write(Tablet)` columnar batch | `rust/src/write/tablet.rs`, `rust/src/write/tsfile_writer.rs` | 部分实现 | 目前 `Tablet -> TSRecord -> write`，不是直接批写 | P3 | 否 |  |  | 2026-05-09 | 优化前先补功能 |
| 13 | `TsFileWriter` schema/table/aligned 写入 | `TsFileWriter.java` | `rust/src/write/tsfile_writer.rs` | 部分实现 | `registerAlignedTimeseries`、`registerTableSchema`、`writeAligned`、`writeTable` 已有；仍缺 Java 内部 group writer 语义、table column category 检查、device split 批写路径 | P1 | 集成测试 |  |  | 2026-05-11 | 高层 API 已补，内部执行仍简化 |
| 14 | `TsFileWriter` flush/memory control | `TsFileWriter.java` | `rust/src/write/tsfile_writer.rs` | 部分实现 | `flush` 已有；阈值仍基于 statistics size，非真实缓冲占用；aligned/non-aligned last time 管理仍不等价 | P0 | 集成测试 |  |  | 2026-05-11 | 影响稳定性 |
| 15 | `MeasurementSchema` | `MeasurementSchema.java` | `rust/src/write/schema.rs` | 部分实现 | 缺 getter/setter、partial serialize/deserialize、encoder/compressor accessor、compare/hash | P1 | 否 |  |  | 2026-05-09 | 可分批补 |
| 16 | `MeasurementSchemaBuilder` | `MeasurementSchemaBuilder.java` | `rust/src/write/schema.rs` | 部分实现 | 默认压缩与 `TsFileConfig` 不一致；缺 Java 风格 parity 行为 | P0 | 否 |  |  | 2026-05-09 | 先修默认值一致性 |
| 17 | `TSFileConfig` | `TSFileConfig.java` | `rust/src/common/config.rs` | 部分实现 | 仅少量字段；缺大量 getter/setter、存储系统参数、加密参数 | P1 | 否 |  |  | 2026-05-09 | 建议先按最常用项补 |
| 18 | `TSFileDescriptor` | `TSFileDescriptor.java` | `rust/src/common/config.rs` | 未开始 | properties 加载、descriptor 单例、custom settings 覆盖全部缺失 | P2 | 否 |  |  | 2026-05-09 | 依赖配置体系设计 |
| 19 | `ReadWriteIOUtils` | `ReadWriteIOUtils.java` | `rust/src/utils/read_write_io_utils.rs` | 部分实现 | map/set/list/object/skip/checkMagic 等大批工具方法缺失 | P1 | 否 |  |  | 2026-05-09 | 工具方法多，建议自动化对照补 |
| 20 | `TsBlock` | `read/common/block/TsBlock.java` | `rust/src/read/block/tsblock.rs` | 部分实现 | 缺 region view/copy 区分、iterator、reverse、append/insert columns、memory size APIs | P2 | 否 |  |  | 2026-05-09 | 与执行引擎绑定较深 |
| 21 | `Column` 列模型 | `common/block/column/*` | `rust/src/read/block/column.rs` | 未开始 | 缺专用列实现、dictionary/RLE/null column、column encoding | P3 | 否 |  |  | 2026-05-09 | 性能优化基础设施 |
| 22 | `BatchData` | `read/common/BatchData.java` | `rust/src/read/common.rs` | 部分实现 | 缺 typed API、flip、serialize、按时间索引、memory stats | P2 | 否 |  |  | 2026-05-09 | 查询栈依赖 |
| 23 | `read/common/type` | Java type system | 无 | 未开始 | `Type`、`TypeFactory`、`RowType`、`BooleanType` 等全缺 | P2 | 否 |  |  | 2026-05-09 | table/query 能力需要 |
| 24 | `read/common/parser` | path parser/visitor | 无 | 未开始 | 路径解析器、parse error、visitor 全缺 | P2 | 否 |  |  | 2026-05-09 | 可以晚于基础读写 |
| 25 | `encrypt` | Java encrypt package | 无 | 未开始 | encrypt utils、encrypt parameter、encryptor/decryptor 缺失 | P1 | 否 |  |  | 2026-05-09 | 若要兼容加密文件必须补 |
| 26 | `fileSystem` | FS factory / HDFS / object storage | 无 | 未开始 | 本地以外存储抽象缺失 | P2 | 否 |  |  | 2026-05-09 | 先明确范围 |
| 27 | `common/regexp` | LIKE/regexp runtime | 无 | 未开始 | pattern/NFA/DFA/matcher 缺失 | P3 | 否 |  |  | 2026-05-09 | 与 filter 结合后再补 |
| 28 | `common/cache` | cache/LRU | 无 | 未开始 | cache 基础设施缺失 | P3 | 否 |  |  | 2026-05-09 | 可以随 query/cache 补 |
| 29 | `read/v4` | `ITsFileReader` + table schema query | `rust/src/read/v4/mod.rs` | 部分实现 | table/tree facade 与 schema 查询已可用；仍缺 Java 的 `TreeResultSet` / `TableResultSet` / `TsBlockReader` 流式查询形态和 tag filter 语义 | P1 | 集成测试 |  |  | 2026-05-11 | facade 可用，但执行模型仍浅 |
| 30 | `write/v4` | `ITsFileWriter` | `rust/src/write/v4/mod.rs` | 部分实现 | `write_record`/`write_tablet` 已有；`TableTsBlock2TsFileWriter.write_tsblock()` 已补；仍缺 Java 的 direct chunk-group writer 路线与 synthetic time 选项 | P1 | 集成测试 |  |  | 2026-05-11 | 主写路径已经打通 |

## 2026-05-11 代码量校验

这一节专门回答“从代码量上看是不是已经差不多补完”。

| 模块 | Java LOC | Rust LOC | Rust / Java | 判断 |
| --- | ---: | ---: | ---: | --- |
| `TsFileSequenceReader` | 3400 | 861 | 25.3% | 明显未对齐 |
| `read/query` 目录 | 2592 | 433 | 16.7% | 明显未对齐 |
| `read/controller` 目录 | 755 | 230 | 30.5% | 明显未对齐 |
| `TsFileWriter` | 830 | 495 | 59.6% | 主写路径已补大半，但内部语义仍简化 |
| `TableTsBlock2TsFileWriter` | 270 | 197 | 73.0% | 主功能已补，仍非 Java 原生 chunk writer 路径 |

当前更准确的结论：

- `write/v4` 主路径已经不是空白，不能再说“完全没做”
- 但 `read/query` 和 `TsFileSequenceReader` 从体量上看仍然差得很远
- 所以“可运行主路径”不等于“Java 对齐完成”

## 方法级记录模板

当你开始补某个类时，建议在对应模块下面继续加子表，不要只改大表状态。

### 模板

| 类 | Java 方法 | Rust 状态 | 是否已实现 | 是否语义对齐 | 测试情况 | 备注 |
| --- | --- | --- | --- | --- | --- | --- |
| 示例：TsFileReader | `getMeasurement` | 未开始 | 否 | 否 | 无 | 需要先补 `MeasurementSchema` 返回类型 |
| 示例：TsFileReader | `close` | 未开始 | 否 | 否 | 无 | 建议实现 `Drop` 之外的显式 close |

### 建议记录规则

- 一个方法一行
- “是否已实现”和“是否语义对齐”分开记录
- 如果只是同名 API 但实现是降级版，状态写 `名义支持但不等价`
- 测试情况至少记录：
  - `无`
  - `单测`
  - `集成测试`
  - `Java 互操作测试`
