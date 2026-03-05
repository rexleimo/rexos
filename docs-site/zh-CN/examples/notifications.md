# 通知（`channel_send`）

`channel_send` 只负责把消息写入 outbox。真正的投递会在你运行 dispatcher 时发生：

```bash
loopforge channel drain
```

或者跑一个常驻 worker：

```bash
loopforge channel worker --interval-secs 5
```

## 示例：发送 console 通知

```bash
loopforge agent run --workspace . --prompt "使用 channel_send 入队：channel=console recipient=me subject=Hello message=Done"
loopforge channel drain
```

## 示例：发送到 webhook

```bash
export REXOS_WEBHOOK_URL="https://example.com/my-webhook"
loopforge agent run --workspace . --prompt "使用 channel_send 入队：channel=webhook recipient=user1 message=hello"
loopforge channel drain
```
