# 为什么你的 AI 助手记不住上下文？Agent 记忆设计指南

你一定遇到过这种情况：

> "帮我修改一下之前那个函数"
> — 请问是哪个函数？

> "用我们上次讨论的方式来实现"
> — 上次讨论的什么方式？

**不是 AI 笨，是记忆系统没设计好。**

## 记忆的本质问题

人脑的记忆是关联的、层级的、有选择性的。但 AI 的"记忆"只是上下文窗口里的 token——用完了就被覆盖。

```
上下文窗口：32K tokens
你的代码库：100万+ tokens
差距：31倍+
```

这就是 Agent 记忆要解决的核心问题：**如何在有限的上下文中，让 AI 记住真正重要的事情。**

## Agent 记忆的四种类型

| 记忆类型 | 存什么 | 存多久 | 怎么用 |
|----------|--------|--------|--------|
| **工作记忆** | 当前任务的中间状态 | 任务期间 | 直接塞进 prompt |
| **会话记忆** | 本次对话的历史 | 会话期间 | 按需检索 |
| **持久记忆** | 跨会话的重要信息 | 长期 | 向量检索 |
| **世界知识** | 常识和领域知识 | 永远 | 预训练+微调 |

### 1. 工作记忆（Working Memory）

这是 AI "正在思考"的内容：

```python
# 典型的实现
working_memory = {
    "current_goal": "修复登录bug",
    "completed_steps": [
        "定位到 auth.py 的 login 函数",
        "发现 token 过期时间计算错误"
    ],
    "pending_steps": [
        "修复时间计算",
        "运行测试验证"
    ]
}
```

**关键点**：工作记忆应该**精简**，只保留当前任务的关键状态。

### 2. 会话记忆（Session Memory）

记录一次对话中的关键信息：

```python
# 典型的检索逻辑
def retrieve_session_memory(query: str, session_id: str) -> list[Message]:
    # 1. 获取本次会话的所有消息
    messages = db.get_session_messages(session_id)

    # 2. 按时间排序，保留最近的 N 条
    recent = messages[-20:]

    # 3. 过滤：只保留与 query 相关的
    relevant = filter_by_similarity(recent, query)

    return relevant
```

**关键点**：不是记录所有对话，而是**选择性记住**用户明确要求"记住"的内容。

### 3. 持久记忆（Persistent Memory）

跨会话记住重要信息：

```python
# 典型的向量检索
def retrieve_persistent_memory(query: str, user_id: str) -> list[Memory]:
    # 1. 将 query 转为向量
    query_embedding = embed(query)

    # 2. 从向量数据库检索相似记忆
    memories = vector_db.search(
        user_id=user_id,
        query_embedding=query_embedding,
        top_k=5
    )

    # 3. 按时间+相关性加权
    return rank_by_recency_and_relevance(memories, query_embedding)
```

**关键点**：不是存储越多越好，而是**存储真正有价值**的信息。

### 4. 世界知识（World Knowledge）

预训练时学会的常识，不需要额外存储。

## 记忆系统的工程实现

### 1. 分层存储

```
┌─────────────────────────────────────┐
│           Prompt Layer              │  ← 直接塞进 LLM
│  [工作记忆] + [检索到的记忆片段]    │
└─────────────────────────────────────┘
              ▲
              │ 检索
┌─────────────────────────────────────┐
│         Fast Retrieval             │  ← Redis / 内存
│    [会话记忆] + [热点持久记忆]      │
└─────────────────────────────────────┘
              ▲
              │ 写入
┌─────────────────────────────────────┐
│          Slow Storage               │  ← SQLite / PostgreSQL
│         [持久记忆向量库]            │
└─────────────────────────────────────┘
```

### 2. 记忆的写入策略

**被动写入**：记录所有对话（太多了，检索困难）

**主动写入**：只有满足条件才记录：

```python
# 典型的主动写入逻辑
def should_save_memory(user_message: str, ai_response: str) -> bool:
    # 1. 用户明确要求记住
    if "记住" in user_message or "记得" in user_message:
        return True

    # 2. 涉及重要信息（项目配置、个人偏好）
    if contains_important_info(user_message):
        return True

    # 3. 长时间会话的总结
    if session_length > threshold:
        return True

    return False
```

### 3. 记忆的检索策略

```python
def retrieve_context(query: str, max_tokens: int = 8000) -> str:
    context = []

    # 1. 先检索最相关的持久记忆
    persistent = retrieve_persistent_memory(query)
    context.extend(persistent)

    # 2. 补充会话记忆
    session = retrieve_session_memory(query)
    context.extend(session)

    # 3. 裁剪到 token 限制
    return truncate_to_token_limit(context, max_tokens)
```

## 完整实现示例

```python
# memory_system.py
from dataclasses import dataclass
from enum import Enum
from typing import Optional
import sqlite3
import time
import pickle

class MemoryType(Enum):
    WORKING = "working"
    SESSION = "session"
    PERSISTENT = "persistent"

@dataclass
class Memory:
    id: Optional[int]
    memory_type: MemoryType
    content: str
    embedding: list[float]
    created_at: float
    workspace_id: str

class MemorySystem:
    def __init__(self, db_path: str):
        self.conn = sqlite3.connect(db_path)
        self._init_db()

    def _init_db(self):
        self.conn.execute("""
            CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY,
                memory_type TEXT NOT NULL,
                content TEXT NOT NULL,
                embedding BLOB,
                created_at REAL NOT NULL,
                workspace_id TEXT NOT NULL
            )
        """)
        self.conn.commit()

    def save_memory(
        self,
        memory_type: MemoryType,
        content: str,
        embedding: list[float],
        workspace_id: str
    ) -> int:
        """保存记忆"""
        cursor = self.conn.execute("""
            INSERT INTO memories (memory_type, content, embedding, created_at, workspace_id)
            VALUES (?, ?, ?, ?, ?)
        """, (
            memory_type.value,
            content,
            pickle.dumps(embedding),
            time.time(),
            workspace_id
        ))
        self.conn.commit()
        return cursor.lastrowid

    def retrieve(
        self,
        query_embedding: list[float],
        workspace_id: str,
        memory_types: list[MemoryType] = None,
        top_k: int = 5
    ) -> list[Memory]:
        """检索记忆"""
        if memory_types is None:
            memory_types = [MemoryType.PERSISTENT]

        type_values = [t.value for t in memory_types]
        placeholders = ",".join("?" * len(type_values))

        cursor = self.conn.execute(f"""
            SELECT id, memory_type, content, embedding, created_at
            FROM memories
            WHERE workspace_id = ? AND memory_type IN ({placeholders})
            ORDER BY created_at DESC
            LIMIT ?
        """, [workspace_id] + type_values + [top_k])

        return [
            Memory(
                id=row[0],
                memory_type=MemoryType(row[1]),
                content=row[2],
                embedding=pickle.loads(row[3]) if row[3] else [],
                created_at=row[4],
                workspace_id=workspace_id
            )
            for row in cursor
        ]

    def build_context(self, query: str, workspace_id: str) -> str:
        """构建完整上下文"""
        query_embedding = self._simple_embed(query)

        # 检索持久记忆 + 会话记忆
        persistent = self.retrieve(query_embedding, workspace_id, [MemoryType.PERSISTENT], top_k=3)
        session = self.retrieve(query_embedding, workspace_id, [MemoryType.SESSION], top_k=5)

        parts = ["## 相关记忆"]
        for m in persistent + session:
            parts.append(f"- [{m.memory_type.value}] {m.content[:200]}")

        return "\n".join(parts)

    def _simple_embed(self, text: str) -> list[float]:
        """简化版 embedding"""
        import hashlib
        h = hashlib.sha256(text.encode()).digest()
        return [b / 255.0 for b in h[:32]]
```

## LoopForge 的记忆设计

作为本地优先的 Agent OS，LoopForge 实现了完整的多层记忆系统：

### 1. Workspace 级记忆

```bash
# LoopForge 会记住每个 workspace 的关键信息
.workspace/
├── features.json      # 项目特性需求
├── rexos-progress.md  # 任务进度日志
├── .memory/           # 项目级记忆
│   ├── summary.md     # 项目摘要
│   └── context/       # 检索向量库
```

### 2. Session 级记忆

```python
# 每次会话自动保存
session_memory = {
    "id": "sess_xxx",
    "workspace": "myproject",
    "messages": [...],
    "tool_calls": [...],
    "created_at": "2026-03-06T10:00:00Z"
}
```

### 3. 跨 Workspace 记忆

```bash
# 用户偏好、常用模式等跨项目记忆
~/.rexos/
├── user_memory/       # 个人偏好
│   ├── coding_style.md
│   └── preferred_providers.md
```

## 实践建议

### 给开发者的记忆设计 Checklist

- [ ] 区分工作记忆 / 会话记忆 / 持久记忆
- [ ] 设定合理的 token 预算（建议 prompt 的 50-70%）
- [ ] 实现主动写入，而非被动记录所有
- [ ] 定期压缩/总结长期记忆（防止向量库膨胀）
- [ ] 提供用户手动管理记忆的能力

### 不要做的事情

- ❌ 把所有对话都存进向量库（噪音太多）
- ❌ 只依赖向量检索（相似度不等于有用）
- ❌ 忘记清理过期记忆（GDPR 也有关系）
- ❌ 不给用户控制权（用户想删记忆怎么办？）

## 总结

Agent 的记忆不是"记住所有事情"，而是**在对的时刻想起对的事情**。

好的记忆系统 = 合适的存储分层 + 精准的检索策略 + 用户可控的记忆管理

当你的 AI 助手能够：
- 记住项目的技术栈和架构
- 记住你的编码偏好
- 记住之前任务的状态和上下文

它就不再是每次都要从头开始的"新助手"，而是真正懂你的**个人工程师**。

---

**相关链接**

- [LoopForge 记忆模块源码（rexos-memory）](https://github.com/rexleimo/LoopForge/tree/main/meos/crates/rexos-memory)
- [LoopForge 概念与记忆模型](../explanation/concepts.md)
- [Harness 教程：长任务执行](../tutorials/harness-long-task.md)
