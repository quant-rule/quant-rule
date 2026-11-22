# AI Transparency Log — #1103 HA-005 RabbitMQ Quorum Queues

- Date: 2026-11-22 (UTC)
- Issue: [#1103](https://github.com/quant-rule/quant-rule/issues/1103)
- Tool: OpenCode implementation subagent
- Boundary: read-only repository review, implementation, tests, and documentation
  for HA-005 only. No commit, push, pull request, merge, tag, board change,
  secret, or deployment.

## Intent and boundary

The work replaces the classic durable control-plane queues with a quorum
topology that survives one broker loss: quorum queues, a delivery limit, a
dead-letter exchange and queue, publisher confirms, manual ack, and generated
queue names. The agent read the v1.6.0 plan, ADR-HA, HA architecture, control
plane, RabbitMQ adapter, queue contract, backtest and evidence consumers,
Compose, and the multi-host deployment before changing anything.

The agent deliberately did not implement the evidence leader (HA-006),
active/active data or simulation, the v3 renderer, Swarm, MinIO, or the HA
backup/DR profile. The three-node RabbitMQ cluster used for the broker-loss test
was a disposable Docker test topology, not a deployed HA environment.

## Implemented

- `QueueNames` and `QR_QUEUE_GENERATION` (default 1) append `.g<generation>` to
  every queue name. Consumers use the generated names; the adapter refuses a
  base name or another generation's name.
- Exchange `quantrule.runs` and dead-letter exchange `quantrule.runs.dlx` are
  durable direct exchanges. The run, evidence, and stage queues are durable
  quorum queues with `x-delivery-limit=3` and the dead-letter exchange. The
  dead-letter queue is quorum with no dead-letter target.
- The dead-letter queue is bound for explicit `run.dead` publishes and for
  broker-dead-lettered work under the original routing keys. Exhausted messages
  keep their body and `x-death` header.
- A failed handler uses `basic.reject` with requeue, not `basic.nack`, because
  RabbitMQ 4.3 and later count only a rejected delivery toward the delivery
  limit. Each rejection logs the queue, routing key, and delivery count.
- The topology is declared on connect by both producers and consumers; the
  consumer re-declares only its own queue with the same arguments.
- The `QueueConsumer` trait no longer takes routing keys. The topology owns the
  bindings, so a consumer cannot declare a second, narrower binding set.
- Compose and the v2 multi-host renderer set `QR_QUEUE_GENERATION=1`; the
  composed queue check asserts quorum type, durability, delivery limit,
  dead-letter bindings, and the generated names.
- The operations, manual, deployment, HA architecture, control-plane, and
  troubleshooting documents record queue inspection, the generation cutover,
  and the separate definitions export/import. A definitions file records
  topology, not messages.
- Unit tests cover declaration arguments, DLQ bindings, generation names, and
  the confirm result. An ignored integration test covers publish/confirm,
  manual ack, delivery-limit exhaustion to the DLQ, and an optional broker-loss
  pause.

## Verification and evidence

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace --no-fail-fast` | pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | pass |
| `cargo run --quiet -p quantrule-governance -- check --all` | `governance check ok` |
| `tools/gate.sh` | `gate passed: full with CARGO_TARGET_DIR=…/target/gate` |
| `cargo test -p quantrule-deploy` | 28 tests passed |
| `make up` against the worktree | stack healthy; `QuantRule v1.5.2 ready`; generated `.g1` queues declared by the running services |
| `tools/check_queue_isolation.sh` against the live stack | `generation 1, 1 run consumer(s) and one stage consumer on 2 channels; quorum work queues and DLQ verified; 2 publisher channel(s) confirm` |
| `tools/verify.sh` against the live stack | two runs completed; canonical digest `sha256:a0372f02db7bc65460906eba3242943da8c56f26c46fde470726cee856bc4cea` |
| Real RabbitMQ 4.4.0, single disposable broker: `QR_RABBITMQ_TEST_URL=… cargo test -p quantrule-runtime --test rabbitmq_quorum -- --ignored --nocapture` | 1 passed; published message acknowledged, poison message reached the DLQ with `x-death` reason `delivery_limit` |
| Real RabbitMQ 4.4.0, three-node Docker cluster: publish on `rabbit1`, stop `rabbit1`, reconnect to `rabbit2`, run the same ignored test with `QR_RABBITMQ_TEST_RECONNECT_URL` and `QR_RABBITMQ_TEST_PAUSE_MS=8000` | 1 passed; `rabbit2` became quorum leader in term 2, the acknowledged message was consumed after the loss, and the poison message reached the DLQ |
| Definitions action on a fresh RabbitMQ 4.4.0 container | `rabbitmqctl export_definitions`, `docker cp`, and `rabbitmqctl import_definitions` succeeded; the export was 714 bytes and listed zero queues |

The Compose stack and its volumes were removed after the test. The standalone
`tools/test_tiers.sh composed` wrapper was not run as one command; its queue
check and run-parity script were run directly against the live stack, while
`tools/postgres_composed.sh` was not rerun in this worktree. The real HA failure
matrix and the three-host deployment remain HA-014 work. No evidence in this log
is a real-host or AZ failure result.

## Learning and corrections

- The first delivery-limit test failed because the handler used `basic.nack`
  with requeue. RabbitMQ 4.3 changed the delivery-limit counter to increment
  only for genuine failures such as `basic.reject`; the adapter now rejects.
  This is a behaviour requirement, not a test workaround.
- `x-delivery-limit=3` allows three failed redeliveries and dead-letters the
  next failed delivery. The integration test therefore exercises the limit plus
  one; the constant is documented with that meaning.
- The first integration test published the poison message from the connection
  to the broker stopped in the failover step. The test now publishes from the
  reconnected consumer connection, which is what a real client does after a
  broker loss.
- The consumer's `keys` argument was removed after the first test exposed that a
  central topology and a per-consumer key list were two competing binding
  definitions. One declaration surface is simpler and prevents drift.
- The definitions export was kept out of the backup archive: it contains broker
  users and password hashes and does not contain messages. The docs require mode
  600 and state that queued work is rebuilt from run rows.
- The exchange names stay stable while queue names carry the generation. An old
  queue therefore remains bound until the operator retires it; the upgrade
  runbook says to drain it and unbind or delete it before the new generation
  publishes. This is a deliberate operational step, not automatic deletion.
