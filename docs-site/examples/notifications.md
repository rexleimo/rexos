# Notifications (`channel_send`)

`channel_send` enqueues an outbound message into the outbox. Delivery happens out-of-band via the dispatcher:

```bash
loopforge channel drain
```

Or run a long-lived worker:

```bash
loopforge channel worker --interval-secs 5
```

## Example: send a console notification

```bash
loopforge agent run --workspace . --prompt "Use channel_send to enqueue: channel=console recipient=me subject=Hello message=Done"
loopforge channel drain
```

## Example: send to a webhook

```bash
export REXOS_WEBHOOK_URL="https://example.com/my-webhook"
loopforge agent run --workspace . --prompt "Use channel_send to enqueue: channel=webhook recipient=user1 message=hello"
loopforge channel drain
```
