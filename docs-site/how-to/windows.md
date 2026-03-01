# Windows Notes

RexOS runs on Windows runners without requiring WSL.

## Init scripts in harness workspaces

When the harness initializes a workspace, it creates:

- `init.sh` (Unix-like smoke checks)
- `init.ps1` (Windows smoke checks)

On Windows, RexOS **prefers** `init.ps1` to avoid accidentally invoking `bash.exe` (WSL launcher) when no WSL distro is installed.

## Tool runtime differences

- `shell` tool uses **PowerShell** on Windows.
- `shell` tool uses **bash** on Unix.

If you want cross-platform commands, prefer simple patterns:

- PowerShell: `(Get-Location).Path`
- Unix: `pwd`

## Paths

Tools are sandboxed to `--workspace`. Use relative paths inside tool calls (`hello.txt`, `src/main.rs`, etc.).

