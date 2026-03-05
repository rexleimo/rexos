# Windows 说明

LoopForge 在 Windows 上运行不依赖 WSL。

## Harness workspace 的 init scripts

Harness 初始化 workspace 时会创建：

- `init.sh`
- `init.ps1`

在 Windows 上 LoopForge 会优先执行 `init.ps1`，避免误调用 `bash.exe`（WSL 启动器）导致 “未安装发行版” 的错误。

## 工具差异

- Windows：`shell` 工具使用 PowerShell
- Unix：`shell` 工具使用 bash

