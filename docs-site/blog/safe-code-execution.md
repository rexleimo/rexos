# 让 AI 放心执行代码：隔离执行环境的最佳实践

你敢让 AI 帮你运行代码吗？

大多数 AI 编程助手能做到**写代码**，但不敢让你**运行它生成的代码**。

**原因很简单**：AI 生成的代码可能：
- 删除你的重要文件
- 无限循环耗尽资源
- 发起网络攻击
- 窃取敏感数据

这就是为什么需要一个**安全的代码执行环境**。

## 风险分析

### AI 代码的三大风险

| 风险类型 | 示例 | 后果 |
|----------|------|------|
| **文件破坏** | `rm -rf /` | 数据丢失 |
| **资源耗尽** | `while(true){}` | 系统崩溃 |
| **数据泄露** | 读取 `.env` 发送出去 | 安全事件 |

### 传统的两难

```bash
# 选项1: 不让 AI 运行代码
AI: "我建议把这段代码加到 main.py"
你: "我自己加"
→ AI 无法验证自己的代码是否正确

# 选项2: 让 AI 随意运行
AI: "让我帮你运行这个脚本"
→ rm -rf /  ← 灾难
```

**我们需要的是：让 AI 能运行代码，但只能在安全的边界内。**

## 隔离执行的核心原则

### 1. 最小权限原则

```
AI 能访问的范围 = 完成任务的最小必要范围
```

```python
# ❌ 危险：AI 可以访问整个系统
def execute_code(code: str):
    exec(code)  # 完全不受控

# ✅ 安全：限定工作目录
def execute_code(code: str, workspace: str):
    # 只允许在 workspace 目录内操作
    os.chdir(workspace)
    # 禁用危险函数
    restricted_globals = {
        '__builtins__': {
            'open': restricted_open,
            'eval': None,  # 禁止
            'exec': None,  # 禁止
        }
    }
    exec(code, restricted_globals)
```

### 2. 资源限制

```python
import signal
import resource

def run_with_limits(code: str, timeout: int = 30, memory_mb: int = 512):
    # 超时限制
    def timeout_handler(signum, frame):
        raise TimeoutError("Code execution timeout")

    signal.signal(signal.SIGALRM, timeout_handler)
    signal.alarm(timeout)

    # 内存限制 (Linux)
    resource.setrlimit(resource.RLIMIT_AS, (memory_mb * 1024 * 1024, memory_mb * 1024 * 1024))

    # 执行代码...
```

### 3. 文件系统隔离

```
允许:  /workspace/project/src/
禁止:  /workspace/project/../  (escape to parent)
禁止:  ~/.ssh/
禁止:  /etc/
```

```python
def safe_file_read(path: str, allowed_root: str) -> str:
    # 解析真实路径
    real_path = os.path.realpath(path)
    real_root = os.path.realpath(allowed_root)

    # 检查是否在允许范围内
    if not real_path.startswith(real_root + os.sep):
        raise PermissionError(f"Path {path} outside allowed directory")

    # 检查 symlink escape
    if os.path.islink(path):
        raise PermissionError("Symlinks not allowed")

    return open(real_path).read()
```

### 4. 网络隔离

```python
# 默认禁止网络访问
ALLOW_NETWORK = False

# 只允许特定场景
if enable_network:
    # 白名单域名
    ALLOWED_DOMAINS = [
        "api.github.com",
        "pypi.org",
    ]
```

## 实现方案对比

### 方案 1: 容器隔离（Docker）

```dockerfile
# 轻量级执行环境
FROM python:3.11-slim

# 非 root 用户
RUN useradd -m -s /bin/bash appuser

# 只读文件系统（大部分）
VOLUME ["/workspace"]

USER appuser

# 无网络（根据需要开启）
# RUN echo '127.0.0.1 pypi.org' >> /etc/hosts
```

**优点**：隔离彻底
**缺点**：启动慢、资源消耗大

### 方案 2: gVisor（轻量级沙盒）

```bash
# 使用 gVisor 运行不受信任的代码
runsc --unsafe --net-raw run --container httpserver \
  python3 /workspace/script.py
```

**优点**：比 Docker 轻量
**缺点**：兼容性可能有问题

### 方案 3: WebAssembly 沙盒

```rust
// 用 wasmtime 运行代码
let engine = Engine::default();
let module = Module::from_file(&engine, "script.wasm")?;
let instance = Instance::new(&module, &imports)?;
let result = instance.call(&mut store, "run", &[])?;
```

**优点**：隔离极强、无特权
**缺点**：需要编译为 WASM

### 方案 4: 进程级限制（简单场景）

```python
# LoopForge 采用的方案
import subprocess
import os

def safe_shell_exec(cmd: str, cwd: str) -> str:
    # 环境变量清理
    env = os.environ.copy()
    env.pop('PATH', None)  # 限制 PATH

    # 执行
    result = subprocess.run(
        cmd,
        shell=True,
        cwd=cwd,
        env=env,
        capture_output=True,
        timeout=30,
        text=True
    )
    return result.stdout + result.stderr
```

## LoopForge 的安全执行设计

### 多层防护

```
┌────────────────────────────────────────────┐
│           用户指令层                        │
│  "修复 test_api.py 中的失败测试"           │
└──────────────────┬─────────────────────────┘
                   │
┌──────────────────▼─────────────────────────┐
│          工具权限检查                       │
│  - 允许 file_read: ./src/**                │
│  - 允许 file_write: ./src/**               │
│  - 允许 shell_exec: ./test.sh             │
│  - 禁止: rm -rf /                          │
└──────────────────┬─────────────────────────┘
                   │
┌──────────────────▼─────────────────────────┐
│          执行环境层                         │
│  - workspace 目录隔离                       │
│  - timeout: 60s                             │
│  - 禁用危险 shell 命令                      │
│  - 环境变量清理                             │
└──────────────────┬─────────────────────────┘
                   │
┌──────────────────▼─────────────────────────┐
│          结果验证层                         │
│  - 运行测试脚本                             │
│  - 检查退出码                               │
│  - 捕获输出                                 │
└────────────────────────────────────────────┘
```

### 具体实现

```rust
// RexOS 工具沙盒实现
pub fn execute_shell(cmd: &str, workspace: &Path) -> Result<ExecutionResult> {
    // 1. 权限检查
    if is_dangerous_command(cmd) {
        return Err(Error::CommandBlocked(cmd.to_string()));
    }

    // 2. 工作目录检查
    if !cmd.contains(workspace) && !cmd.starts_with("./") {
        return Err(Error::PathOutsideWorkspace(cmd.to_string()));
    }

    // 3. 超时限制
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(workspace)
        .env_clear()
        .output()
        .timeout(Duration::from_secs(60))?;

    // 4. 返回结果
    Ok(ExecutionResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code(),
    })
}
```

### 危险命令黑名单

```rust
fn is_dangerous_command(cmd: &str) -> bool {
    let dangerous = [
        "rm -rf /",
        "dd if=",
        "mkfs",
        ":(){:|:&};:",  // fork bomb
        "curl | sh",
        "wget | sh",
        // ... 更多
    ];
    dangerous.iter().any(|d| cmd.contains(d))
}
```

## 完整实现：Python 沙盒执行器

```python
# sandbox_executor.py
import os
import subprocess
import signal
import resource
from pathlib import Path
from typing import Optional
import logging

logger = logging.getLogger(__name__)

class SandboxConfig:
    """沙盒配置"""
    def __init__(
        self,
        workspace_root: str,
        max_timeout: int = 60,
        max_memory_mb: int = 512,
        allowed_commands: Optional[list[str]] = None,
        allow_network: bool = False,
    ):
        self.workspace_root = Path(workspace_root).resolve()
        self.max_timeout = max_timeout
        self.max_memory_mb = max_memory_mb
        self.allowed_commands = allowed_commands or ["python", "node", "npm", "cargo"]
        self.allow_network = allow_network

class SandboxExecutor:
    """安全的代码执行器"""

    # 危险命令黑名单
    DANGEROUS_PATTERNS = [
        "rm -rf /",
        "rm -rf ~",
        "dd if=",
        "mkfs",
        ":(){:|:&};:",
        "fork()",
        "curl | sh",
        "wget | sh",
        "> /dev/sda",
        "chmod 777 /",
        "chown -R",
    ]

    def __init__(self, config: SandboxConfig):
        self.config = config

    def execute(self, command: str, cwd: Optional[str] = None) -> dict:
        """执行命令"""
        # 1. 安全检查
        self._check_command(command)

        # 2. 工作目录检查
        work_dir = self._resolve_workdir(cwd)

        # 3. 设置资源限制
        self._set_limits()

        # 4. 清理环境变量
        env = self._clean_env()

        # 5. 执行
        try:
            result = subprocess.run(
                command,
                shell=True,
                cwd=str(work_dir),
                env=env,
                capture_output=True,
                timeout=self.config.max_timeout,
                text=True,
            )
            return {
                "success": result.returncode == 0,
                "stdout": result.stdout,
                "stderr": result.stderr,
                "exit_code": result.returncode,
            }
        except subprocess.TimeoutExpired:
            return {
                "success": False,
                "stdout": "",
                "stderr": "Execution timeout",
                "exit_code": -1,
            }

    def _check_command(self, command: str):
        """检查命令是否安全"""
        # 黑名单检查
        for pattern in self.DANGEROUS_PATTERNS:
            if pattern in command:
                raise SecurityError(f"Dangerous command detected: {pattern}")

        # 白名单命令检查
        if self.config.allowed_commands:
            cmd_name = command.split()[0]
            if cmd_name not in self.config.allowed_commands:
                raise SecurityError(f"Command not allowed: {cmd_name}")

    def _resolve_workdir(self, cwd: Optional[str]) -> Path:
        """解析工作目录"""
        if cwd:
            work_dir = (self.config.workspace_root / cwd).resolve()
        else:
            work_dir = self.config.workspace_root

        # 检查是否在允许范围内
        try:
            work_dir.relative_to(self.config.workspace_root)
        except ValueError:
            raise SecurityError(f"Path outside workspace: {cwd}")

        return work_dir

    def _set_limits(self):
        """设置资源限制"""
        # 超时通过 subprocess.run 的 timeout 参数处理

        # 内存限制 (Linux)
        try:
            soft, hard = resource.getrlimit(resource.RLIMIT_AS)
            limit = self.config.max_memory_mb * 1024 * 1024
            resource.setrlimit(resource.RLIMIT_AS, (limit, hard))
        except (AttributeError, ValueError):
            pass  # macOS 不完全支持

    def _clean_env(self) -> dict:
        """清理环境变量"""
        env = os.environ.copy()

        # 移除危险环境变量
        dangerous_vars = [
            "PATH",  # 限制 PATH
            "LD_PRELOAD",
            "LD_LIBRARY_PATH",
            "PYTHONPATH",
        ]
        for var in dangerous_vars:
            env.pop(var, None)

        # 设置安全的 PATH
        env["PATH"] = "/usr/local/bin:/usr/bin:/bin"

        # 网络控制
        if not self.config.allow_network:
            env.pop("HTTP_PROXY", None)
            env.pop("HTTPS_PROXY", None)
            env.pop("http_proxy", None)
            env.pop("https_proxy", None)

        return env

# 使用示例
if __name__ == "__main__":
    config = SandboxConfig(
        workspace_root="/path/to/workspace",
        max_timeout=30,
        max_memory_mb=256,
        allow_network=False,
    )

    executor = SandboxExecutor(config)

    try:
        result = executor.execute("python test.py")
        print(f"Success: {result['success']}")
        print(f"Output: {result['stdout']}")
    except SecurityError as e:
        print(f"Blocked: {e}")
```

## 最佳实践清单

### 部署前检查

- [ ] 确认执行环境网络隔离
- [ ] 设置合理的超时时间
- [ ] 配置内存和 CPU 限制
- [ ] 启用审计日志
- [ ] 定期更新危险命令黑名单

### 运行时的监控

```bash
# 监控异常行为
journalctl -u loopforge | grep -i "blocked\|denied"

# 查看资源使用
ps aux | grep loopforge
```

### 事故响应

```python
# 快速隔离
def emergency_stop():
    # 1. 停止执行队列
    # 2. 暂停 workspace
    # 3. 记录现场
    # 4. 告警
```

## 总结

让 AI 执行代码不是"开或关"的选择题，而是**如何在安全边界内最大化 AI 能力**的艺术。

核心策略：
1. **最小权限**：只给 AI 完成任务需要的权限
2. **多层防护**：指令层 → 权限层 → 执行层 → 验证层
3. **快速失败**：危险操作直接拒绝，不犹豫
4. **可审计**：所有操作记录日志，可追溯

当执行环境足够安全，AI 就能从"建议者"变成"执行者"——你的个人工程师才真正开始工作。

---

**相关链接**

- [LoopForge 安全与沙盒](../explanation/security.md)
- [故障排查（网络与权限）](../how-to/troubleshooting.md)
- [Harness 教程：长任务执行](../tutorials/harness-long-task.md)
