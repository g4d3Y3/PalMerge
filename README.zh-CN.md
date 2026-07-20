# PalMerge

[English](README.md) | [简体中文](README.zh-CN.md)

PalMerge 是一个安全优先、本地运行且开源的工具集，用于理解 Palworld 存档，并在未来安全地迁移或合并存档。仓库目前处于基础建设早期阶段，**尚不支持**存档合并、迁移、修复或写入。

项目方向和验收标准记录在 [项目 Goal #1](https://github.com/g4d3Y3/PalMerge/issues/1) 中。

## 当前能力

- 只读发现 `Level.sav`、`LevelMeta.sav`、`LocalData.sav`、`WorldOption.sav`，以及 `Players/` 目录直属的 `.sav` 文件。
- 流式计算 SHA-256 指纹，不需要额外运行时依赖。
- 有边界地解析 GVAS 元数据，包括存档/包版本、引擎构建与分支、自定义版本 GUID 和 SaveGame 类。
- 以失败关闭方式清点旧版 Unreal 顶层属性标签，包括类型专用标签元数据、大小、数组索引和可选 GUID。
- 严格解析 `PlZ` 和 `CNK` 容器头，并验证单层及双层 zlib 数据。
- 以流式方式检查解压结果，验证声明长度并设置 2 GiB 安全上限；不会在内存中保留完整解压存档。
- 识别新版 `PlM`/Oodle 容器并明确标记为暂不支持，不会误当成 zlib。
- 提供英文和简体中文的人类可读输出。
- 提供稳定且不受界面语言影响的 JSON 检查输出。
- 发现结果顺序确定，并具有明确的资源数量限制。

## 尚未实现

- `PlM`/Oodle 解压、嵌套属性值解码和 Palworld 专用结构定义。
- 领域模型、实体索引、依赖图和引用完整性校验。
- 迁移、合并、修复、GUID 重写、事务写入、备份、回滚和 GUI 工作流。

请勿将当前版本当作存档合并器使用。检查功能有意保持只读。

## 使用预编译程序

普通用户应从 [Releases](https://github.com/g4d3Y3/PalMerge/releases) 下载对应操作系统的软件包，解压后运行 `palmerge`。运行时不需要 Rust、Cargo、Python、编译器、包管理器或网络连接。

带标签的正式版本将提供 Windows x86-64、Linux x86-64、macOS Apple Silicon 和 macOS Intel 软件包。在第一个正式版本发布前，贡献者可按下文从源码构建。

每个版本会提供各平台的自包含压缩包和 `SHA256SUMS.txt`。下载后请先核对压缩包的 SHA-256 校验和，再进行解压。Alpha 版本只是只读检查预览版，不能当作存档合并器使用。

## 检查存档

```console
palmerge inspect /path/to/Level.sav
palmerge inspect /path/to/world-directory --lang zh-CN
palmerge inspect /path/to/world-directory --format json
palmerge --version
```

命令只读取文件、计算哈希、验证受支持的 zlib 容器，并报告有边界的 GVAS 头及顶层属性标签元数据，不会修改存档。当前阶段会跳过属性值，不会把它们描述为已解码。JSON 使用 `schema_version: 1`、稳定错误码和不翻译的机器字段。

## 从源码构建

源码构建仅面向贡献者和高级用户。安装 Rust 1.77 或更高版本，然后运行：

```console
cargo build --release --locked
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

可执行文件输出到 `target/release/palmerge`（Windows 为 `palmerge.exe`）。

## 安全与隐私

- 检查功能只读。
- 未知格式和暂不支持的 `PlM` 容器会被报告，不会被猜测。
- 存档数据保留在本机；核心功能不上传数据，也不包含遥测。
- 不得将真实的私人存档提交为测试样本。
- 未来的写入功能必须具备试运行、显式输出、备份、隔离写入、重新解析、校验和恢复说明，才能标记为可用于生产。

## 工作区结构

- `palmerge-core`：稳定错误、本地化、文件指纹和共享基础类型。
- `palmerge-parser`：有边界的世界存档发现、保守格式探测、受资源限制的 zlib 校验、有边界的 GVAS 元数据解析和旧版顶层属性标签清点。
- `palmerge-cli`：可脚本化的文本和 JSON 检查界面。

核心和 CLI crate 只使用 Rust 标准库。解析器使用 `flate2` 的纯 Rust 后端处理 zlib；普通用户获得的程序仍然是自包含的，不需要额外运行时或共享压缩库。

## 参与贡献

改动应保持小而易于审查。请添加测试、运行格式化和 Clippy、保持机器字段稳定、同步更新两份 README，并为面向用户的行为同时提供英文和简体中文。不得把路线图功能描述成已完成能力。

## 许可证

MIT
