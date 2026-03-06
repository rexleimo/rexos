# 5分钟学会 MCP：让 AI Agent 真正掌控你的开发环境

如果你的 AI 助手还停留在"聊天"阶段，是时候升级了。

MCP（Model Context Protocol）正在重新定义 AI 与开发工具的关系——它不是又一个 API，而是一套让 AI 直接操作系统资源的通用协议。

## 传统方式的尴尬

过去我们怎么让 AI 帮我们干活？

```python
# 方式1: 把代码贴进对话
"帮我看看这个函数有什么bug" + 粘贴100行代码

# 方式2: 调用 REST API
curl -X POST https://api.example.com/analyze \
  -d '{"code": "..."}' \
  -H "Authorization: Bearer xxx"
```

**问题很明显**：
- 上下文有限（塞不下一整个代码库）
- 每次都要手动复制粘贴
- 工具之间各自为政，无法协同

## MCP 来了

MCP 是 Anthropic 提出的开放协议，它的核心理念是：

> **让 AI 拥有"手"和"眼睛"，直接操作文件系统、调用工具、访问资源。**

### 三个核心概念

| 概念 | 作用 |
|------|------|
| **Resources** | AI 可以读取的数据（文件、API 响应、数据库） |
| **Tools** | AI 可以调用的函数（搜索、执行命令、发送请求） |
| **Prompts** | 预定义的提示模板 |

### 工作原理

```
┌─────────────┐      MCP       ┌─────────────┐
│   AI Model  │ ◄──────────►  │  Your App   │
│ (Claude等)  │   JSON-RPC    │ (VSCode等)  │
└─────────────┘               └─────────────┘
```

AI 发送 JSON-RPC 请求，应用执行后返回结果。整个过程 AI 知道自己能用什么工具、什么时候该用。

## 实战：5分钟集成 MCP

以 LoopForge 为例，看看 MCP 如何工作。

### 1. 定义 MCP Server

```json
// mcp-servers.json
{
  "servers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/your/project"]
    },
    "git": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-git"]
    },
    "search": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-brave-search", "YOUR_API_KEY"]
    }
  }
}
```

### 2. 启动 LoopForge

```bash
loopforge agent run --workspace myproject \
  --mcp-config mcp-servers.json \
  --prompt "搜索最近关于 AI Agent 的论文，总结3个核心观点"
```

### 3. AI 自动操作

AI 收到任务后，会：
1. 调用 brave-search 搜索论文
2. 获取结果后，调用 filesystem 写入笔记
3. 整个过程你只需等待结果

## LoopForge 的 MCP 能力

作为本地优先的 Agent OS，LoopForge 原生支持 MCP：

### 已集成的能力

| 工具 | 功能 |
|------|------|
| `file_read` / `file_write` | 读写任意文件 |
| `shell_exec` | 执行系统命令 |
| `web_fetch` | 获取网页内容 |
| `web_search` | 搜索互联网 |
| `browser_*` | 浏览器自动化 |

### MCP 扩展

通过配置可以接入更多 MCP Server：

```bash
# 接入更多工具生态
loopforge config add-mcp-server docker
loopforge config add-mcp-server postgres
loopforge config add-mcp-server linear
```

## 为什么这很重要

### 1. 上下文爆炸

传统 RAG 只解决"找文档"问题。MCP 让 AI 直接读取源码、运行测试、分析日志——**真正的全栈理解**。

### 2. 工具即能力

不需要等模型本身学会什么新技能。只要有 MCP Server，AI 立刻获得新能力：
- 接入数据库 → AI 变成数据分析专家
- 接入 GitHub → AI 变成代码审查专家
- 接入 Linear/Jira → AI 变成项目管理专家

### 3. 本地优先

LoopForge + MCP 可以在本地运行，不依赖云端——**数据不出本地，能力不打折**。

## 快速开始

```bash
# 1. 安装 LoopForge
curl -L https://loopforge.dev/install | bash

# 2. 初始化项目
loopforge init my-workspace

# 3. 配置 MCP（可选）
cat > mcp-servers.json << 'EOF'
{
  "servers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "./"]
    }
  }
}
EOF

# 4. 启动 Agent
loopforge agent run \
  --workspace my-workspace \
  --mcp-config mcp-servers.json \
  --prompt "阅读 src/main.rs，分析代码结构，写一份500字的架构说明"
```

## 进阶：自定义 MCP Server

如果官方 Server 不满足需求，可以自己写：

### 1. Python 实现

```python
# my_mcp_server.py
from mcp.server import Server
from mcp.server.stdio import stdio_server
from pydantic import AnyUrl
import json

app = Server("my-custom-server")

@app.list_resources()
async def list_resources():
    return [
        Resource(
            uri=AnyUrl("myapp://config"),
            name="app_config",
            description="Application configuration"
        )
    ]

@app.read_resource()
async def read_resource(uri: AnyUrl):
    if uri == "myapp://config":
        with open("config.json") as f:
            return json.load(f)
    raise ValueError(f"Unknown resource: {uri}")

@app.list_tools()
async def list_tools():
    return [
        Tool(
            name="analyze_code",
            description="Analyze Python code for issues",
            inputSchema={
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"}
                },
                "required": ["file_path"]
            }
        )
    ]

@app.call_tool()
async def call_tool(name: str, arguments: dict):
    if name == "analyze_code":
        # 实现代码分析逻辑
        return analyze_python(arguments["file_path"])
    raise ValueError(f"Unknown tool: {name}")

if __name__ == "__main__":
    stdio_server.run(app)
```

### 2. 注册到 LoopForge

```json
{
  "servers": {
    "my-custom": {
      "command": "python",
      "args": ["/path/to/my_mcp_server.py"]
    }
  }
}
```

### 3. 更多官方 MCP Server

| Server | 功能 |
|--------|------|
| `@modelcontextprotocol/server-filesystem` | 文件系统操作 |
| `@modelcontextprotocol/server-git` | Git 操作 |
| `@modelcontextprotocol/server-github` | GitHub API |
| `@modelcontextprotocol/server-brave-search` | 网页搜索 |
| `@modelcontextprotocol/server-sqlite` | SQLite 数据库 |
| `@modelcontextprotocol/server-postgres` | PostgreSQL |
| `@modelcontextprotocol/server-memory` | 知识图谱记忆 |

## 常见问题

### Q: MCP 和 API 有什么区别？

API 是"你问我答"，MCP 是"我知我能"。MCP 让 AI 知道自己有什么工具可用，不需要每次都通过 prompt 告知。

### Q: 安全吗？

MCP 本身是协议，安全取决于 Server 实现。建议：
- 本地开发用 filesystem server
- 生产环境用细粒度权限控制
- 不推荐直接暴露 MCP Server 到公网

### Q: 支持流式输出吗？

支持。MCP 基于 JSON-RPC 2.0，支持 `server->client` 通知，可以实现实时进度更新。

## 总结

MCP 不仅仅是一个协议——它是 AI 从"问答助手"进化到"执行引擎"的关键一步。

当 AI 能够：
- 读取你的代码库
- 运行你的测试
- 搜索外部知识
- 操作你的工具

它就不再是玩具，而是真正的**个人工程师**。

---

**相关链接**

- [MCP 官方文档](https://modelcontextprotocol.io)
- [LoopForge 快速开始](../tutorials/quickstart-ollama.md)
- [MCP Server 生态列表](https://github.com/modelcontextprotocol/servers)
