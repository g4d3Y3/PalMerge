# Release procedure / 发布流程

PalMerge releases are built by GitHub Actions from an immutable version tag. Maintainers do not upload locally built binaries. / PalMerge 由 GitHub Actions 根据不可变版本标签构建；维护者不上传本地编译的二进制文件。

## Before tagging / 创建标签前

1. Confirm CI and the cross-platform Release workflow pass on `main`. / 确认 `main` 的 CI 与跨平台 Release 工作流均通过。
2. Confirm `README.md` and `README.zh-CN.md` describe the implemented behavior only. / 确认中英文 README 只描述已经实现的能力。
3. Confirm `Cargo.lock` is committed and the version in the workspace is correct. / 确认已提交 `Cargo.lock`，且工作区版本号正确。
4. Confirm write, merge, and repair features remain clearly unavailable until their safety gates are complete. / 在写入、合并和修复功能满足安全门槛前，必须继续明确标记为不可用。

## Publish / 发布

Create a tag such as `v0.1.0-alpha.1` on the verified `main` commit and push it. The workflow builds four portable packages, adds both README files and the license, produces `SHA256SUMS.txt`, and creates a prerelease automatically. / 在通过验证的 `main` 提交上创建并推送类似 `v0.1.0-alpha.1` 的标签。工作流会构建四个平台便携包，加入中英文 README 与许可证，生成 `SHA256SUMS.txt`，并自动创建预发布版本。

Stable tags such as `v1.0.0` create a normal release. Tags containing a suffix such as `-alpha.1`, `-beta.1`, or `-rc.1` create a prerelease. / `v1.0.0` 等稳定标签会创建正式版本；包含 `-alpha.1`、`-beta.1` 或 `-rc.1` 等后缀的标签会创建预发布版本。

## Verify / 验证

Download every archive from the Release page, verify it against `SHA256SUMS.txt`, extract it, and run `palmerge --version` plus a read-only inspection smoke test. Never test a release against the only copy of a real save. / 从 Releases 页面下载所有压缩包，依据 `SHA256SUMS.txt` 校验，解压后运行 `palmerge --version` 和只读检查冒烟测试。不得使用真实存档的唯一副本测试版本。
