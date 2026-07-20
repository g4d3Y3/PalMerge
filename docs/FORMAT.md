# Palworld container notes / Palworld 容器说明

PalMerge implements only structures supported by public format evidence and tests. Unknown bytes are never guessed. / PalMerge 只实现具有公开格式依据且经过测试的结构，绝不猜测未知字节。

## Recognized layouts / 已识别布局

The common header is 12 bytes: little-endian uncompressed length, little-endian compressed length, three-byte magic, one-byte save type, then the payload. A `CNK` wrapper moves the effective header to byte 12 and the payload to byte 24. / 常见头部为 12 字节：小端未压缩长度、小端压缩长度、3 字节魔数、1 字节存档类型，随后是负载。`CNK` 包装会把有效头移动到第 12 字节，并让负载从第 24 字节开始。

| Magic | Type | Current behavior / 当前行为 |
|---|---:|---|
| `PlZ` | `0x31` | Stream and validate one zlib layer / 流式解压并验证单层 zlib |
| `PlZ` | `0x32` | Stream and validate two nested zlib layers / 流式解压并验证双层 zlib |
| `PlM` | `0x31` | Detect as Oodle and report unsupported / 识别为 Oodle 并报告暂不支持 |

Validation checks declared sizes, decompression errors, the configured output limit, and the embedded `GVAS` prefix. It never writes decoded data back to disk. / 校验会检查声明长度、解压错误、配置的输出上限和内嵌 `GVAS` 前缀，且不会把解压数据写回磁盘。

## GVAS metadata / GVAS 元数据

For raw GVAS and validated `PlZ` payloads, PalMerge parses only the bounded metadata prefix: save-game version, UE4/UE5 package versions, engine version and branch, up to 4,096 custom-version entries, and the SaveGame class string. Strings are limited to 65,536 code units and must be valid, null-terminated FString data. Truncated, oversized, or invalid metadata fails closed. Property-body parsing is not implemented yet. / 对原始 GVAS 和已验证的 `PlZ` 负载，PalMerge 只解析有边界的元数据前缀：存档版本、UE4/UE5 包版本、引擎版本与分支、最多 4,096 个自定义版本条目，以及 SaveGame 类字符串。字符串最多允许 65,536 个代码单元，且必须是有效、以空字符结尾的 FString 数据。截断、超限或无效元数据会被拒绝。属性正文解析尚未实现。

## Format references / 格式依据

- [`cheahjs/palworld-save-tools` `palsav.py`](https://github.com/cheahjs/palworld-save-tools/blob/main/palworld_save_tools/palsav.py) documents `PlZ`, `CNK`, and save types `0x31`/`0x32`.
- [`deafdudecomputers/PalworldSaveTools` repository instructions](https://github.com/deafdudecomputers/PalworldSaveTools/blob/main/AGENTS.md) record the newer `PlM`/Oodle distinction.
- [`trumank/uesave` GVAS implementation](https://github.com/trumank/uesave/blob/1917a22ab69e9a4045e613d9d7f39a49fe00bd46/uesave/src/lib.rs) provides the version-aware Unreal SaveGame header layout used as an interoperability reference.

These projects are implementation references, not runtime dependencies. / 这些项目仅作为格式实现依据，不是 PalMerge 的运行时依赖。
