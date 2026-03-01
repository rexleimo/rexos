# Tools Reference

RexOS exposes a small set of tools to the agent runtime.

## `fs_read`

Read a UTF-8 text file **relative to the workspace root**.

- rejects absolute paths
- rejects `..` traversal
- rejects symlink escapes

## `fs_write`

Write a UTF-8 text file **relative to the workspace root** (creates parent directories).

Same sandboxing rules as `fs_read`.

## `shell`

Run a shell command inside the workspace:

- Unix: runs via `bash -c`
- Windows: runs via PowerShell

RexOS enforces a timeout and runs with a minimal environment.

## `web_fetch`

Fetch an HTTP(S) URL and return a small response body.

By default it rejects loopback/private IPs (basic SSRF protection). For local testing you can set `allow_private=true`.

