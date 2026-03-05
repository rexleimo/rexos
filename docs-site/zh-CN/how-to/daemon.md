# 运行 Daemon

LoopForge 内置一个 HTTP daemon（目前功能最小化）。

## 启动

```bash
loopforge daemon start --addr 127.0.0.1:8787
```

## 健康检查

```bash
curl http://127.0.0.1:8787/healthz
```

预期返回：

```json
{ "status": "ok" }
```

