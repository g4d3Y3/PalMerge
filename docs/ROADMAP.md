# Delivery roadmap / 交付路线图

This roadmap turns [Project Goal #1](https://github.com/g4d3Y3/PalMerge/issues/1) into reviewable milestones. A checked item means code, tests, documentation, and CI are complete. / 本路线图将 [项目 Goal #1](https://github.com/g4d3Y3/PalMerge/issues/1) 拆分为可审查的里程碑。勾选表示代码、测试、文档和 CI 均已完成。

## M1 — Safe inspection / 安全检查

- [x] Bounded world-file discovery / 有边界的世界存档发现
- [x] Streaming SHA-256 fingerprints / 流式 SHA-256 指纹
- [x] Bounded GVAS header parsing / 有边界的 GVAS 头解析
- [x] Legacy top-level property-tag inventory / 旧版顶层属性标签清点
- [x] English, Simplified Chinese, and stable JSON output / 英文、简体中文和稳定 JSON 输出
- [x] `PlZ`/`CNK` header parsing and resource-limited zlib validation / `PlZ`/`CNK` 容器头解析与受资源限制的 zlib 校验
- [ ] `PlM`/Oodle decompression / `PlM`/Oodle 解压
- [ ] Complete GVAS property parser with malformed-input limits / 带异常输入限制的完整 GVAS 属性解析器

## M2 — Models and validation / 模型与校验

- [ ] Typed domain model and entity index / 强类型领域模型与实体索引
- [ ] Dependency graph with cycle handling / 支持环处理的依赖图
- [ ] Structural, referential, and semantic validators / 结构、引用和语义校验器
- [ ] Save diff and explainable reports / 存档差异和可解释报告

## M3 — Safe planning and writing / 安全规划与写入

- [ ] Deterministic migration and merge planner / 确定性迁移与合并规划器
- [ ] Dry-run conflict and GUID rewrite report / 试运行冲突与 GUID 重写报告
- [ ] Backup, isolated transactional output, and rollback / 备份、隔离事务输出和回滚
- [ ] Re-parse and semantic post-write verification / 重新解析与写后语义验证

No write feature is production-ready until every M3 safety item is complete. / 在 M3 的全部安全项目完成前，任何写入功能都不能标记为生产可用。

## M4 — Distribution and GUI / 发行与图形界面

- [ ] Versioned portable release archives and checksums / 带版本的便携发行包与校验和
- [ ] Desktop GUI using the shared core / 使用共享核心的桌面 GUI
- [ ] Visual dependencies, conflicts, progress, and recovery / 可视化依赖、冲突、进度与恢复
