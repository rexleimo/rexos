# Daemon Health Check

The daemon currently exposes a simple health endpoint:

```bash
loopforge daemon start --addr 127.0.0.1:8787
curl http://127.0.0.1:8787/healthz
```

Use it for container readiness / supervision, and keep the rest of LoopForge logic in the CLI for now.
