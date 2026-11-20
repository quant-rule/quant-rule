# AI transparency log — Queue confirms and consumer isolation (AUD-40, AUD-41)

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) execution for
  `AUD-2026-11-17-40` and `-41`.
- Claim type: Behaviour Claim.
- Intent: make a control-plane publish wait for the broker, and stop the run
  and stage consumers from sharing one channel and one prefetch slot.
- Boundary: branch `fix/audit-queue-concurrency`, based on `main` `17893f6f`.

## What changed

- The RabbitMQ publisher channel enters confirm mode; `publish` awaits
  `Confirmation::Ack` with a 5 s deadline and maps a nack, a missing confirm
  mode, a channel error, or a timeout to a failed publish.
- Each `consume` call opens its own channel, declares the queue there, and sets
  `basic_qos(1)` on that channel.
- The backtest worker starts `QR_BACKTEST_CONCURRENCY` run consumers plus one
  stage consumer, each on its own channel. The compose backtest service exposes
  the variable.
- A failed publish returns `503 queue.unavailable`; a duplicate submission
  republishes the queued run, so a client retry recovers it.
- `tools/check_queue_isolation.sh` asserts the channel, prefetch, and confirm
  topology in the composed tier.

## Verification observed

| Command | Result |
|---|---|
| `tools/gate.sh` | pass; 25 checker suites and the workspace tests |
| `tools/check_queue_isolation.sh` before the rebuild | failed: both consumers shared one channel |
| `tools/check_queue_isolation.sh` after the rebuild | pass: 1 run + 1 stage on 2 channels, 2 publisher channels confirm |
| `QR_BACKTEST_CONCURRENCY=2 docker compose up -d --force-recreate backtest` | log reports `runs=2`; 2 run + 1 stage on 3 channels |
| `tools/test_tiers.sh composed` | pass; queue isolation, 13 canonical artifacts per run, digest `sha256:a0372f02…` |

## Quant validation

Not required: control-plane delivery and worker concurrency only.

## Learning

- A consumer's QoS belongs to its channel, not to the queue. Sharing a channel
  also shares the prefetch slot.
- A confirm-mode channel is a publisher concern; keeping consumers on their own
  channels preserves both properties without a second connection.
