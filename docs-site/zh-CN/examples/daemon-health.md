# Daemon 健康检查

目前 daemon 仅提供一个简单的健康检查接口：

```bash
rexos daemon start --addr 127.0.0.1:8787
curl http://127.0.0.1:8787/healthz
```

可以用于容器 readiness / supervisor；更复杂的能力建议先用 CLI。
