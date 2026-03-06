# LoopForge v1.0.0 发布说明

LoopForge `v1.0.0` 是一次正式的品牌与运行面硬切换发布。

## 这次更新了什么

- 对外品牌统一为 `LoopForge`。
- 默认配置/数据目录切到 `~/.loopforge`。
- workspace 运行产物目录切到 `.loopforge/`。
- 对外环境变量前缀统一为 `LOOPFORGE_*`。
- Harness 进度文件改为 `loopforge-progress.md`。
- 文档、示例和仓库链接统一指向 `https://github.com/rexleimo/LoopForge`。

## 为什么这次更新重要

这次发布的目标很直接：不再让新用户碰到旧的 `RexOS` 命令、路径或仓库地址，避免安装、搜索和复制命令时走错入口。

## 升级提示

如果你本地还有旧脚本或笔记，请统一改成：

- CLI：`loopforge`
- 配置路径：`~/.loopforge/config.toml`
- workspace 产物目录：`.loopforge/`
- 环境变量前缀：`LOOPFORGE_*`

## 相关链接

- [更新日志](https://github.com/rexleimo/LoopForge/blob/main/CHANGELOG.md#100---2026-03-06)
- [什么是 LoopForge？](what-is-loopforge.md)
- [快速开始（Ollama）](../tutorials/quickstart-ollama.md)
