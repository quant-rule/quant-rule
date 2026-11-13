# Execution Plan — Queue topology, retries, DLQ, and cancellation

- Status: Proposed
- Claim type: Behaviour Claim
- Owner: Human
- Workflow: [Execution Plan](../../workflows/execution-plan.md)
- Records: DIST-041 (#653); epic #630
- Branch: `docs/653-queue-topology-plan` (plan), `feat/653-queue-topology` (implementation)

## 1. Intent

Make the control plane durable and observable. A submitted run is delivered to a
named work queue; a transient failure is retried with backoff; a run that fails
three times reaches a dead-letter queue and a named `failed` state; and a
cancelled run never publishes a bundle.

The operator sees the attempt count and the terminal state on the run row. The
worker stays idempotent: a redelivered message never executes a computed run
twice.

## 2. Current state

| Piece | Today |
|---|---|
| Exchange | `quantrule.runs`, direct, durable (`runtime/adapters/rabbitmq.rs`) |
| Queues | one per routing key: `run.requested`, `run.computed` |
| Consumer | `QueueConsumer::consume(queue, handler)`; handler receives bytes only |
| Ack | `Ok` acks, `Err` nacks with `requeue = true` |
| Retry | none; a nack requeues immediately |
| Dead letter | none |
| Attempts | no column and no counter |
| Claim | `queued → running` only; a running run is skipped |
| Redelivery | a killed worker leaves `running`; the redelivery is skipped, not re-executed |
| Cancel | gateway sets `cancelled` in the store; no `run.cancelled` message |
| Worker cancel check | before and after the coupled backtest, before CAS |

The gap: a killed worker's message can be redelivered forever or skipped, and a
failure has no bounded retry path.

## 3. Scope and non-goals

In scope:

- Direct exchange, `runs.backtest` and `runs.evidence` work queues, `runs.dead-letter`.
- Three attempts with backoff; the third failure marks the run `failed` and publishes `run.dead`.
- Attempt count on the run row, incremented on claim.
- Reclaim a redelivered, uncomputed `running` run.
- Gateway `run.cancelled` publication and worker checks before execution and before CAS.
- Queue binding and message metadata through one `Message` type.
- Unit and integration tests over the fake queue; topology constants for RabbitMQ.

Non-goals:

- No live RabbitMQ or Compose test; that is composed tier and depends on #643.
- No per-stage cancellation inside the coupled backtest; #649 keeps that follow-up.
- No `run.failed` consumer; the worker records the failure on the run row.
- No attempt count in the HTTP API; it stays on the run row.
- No new dependency.

## 4. Assumptions and uncertainty

- RabbitMQ sets `redelivered = true` on a message redelivered after a consumer
  disconnect. The adapter passes it through; the fake sets it explicitly in tests.
- The worker owns the retry policy because the attempt count lives in the run
  store, not in the broker.
- Sleeping in the handler blocks one consumer for at most 25 seconds. With
  prefetch 1 and one operator, this is acceptable; a TTL retry queue is a later
  improvement if it is not.
- A process kill during the backoff loses the remaining delay but not the
  message: the delivery is still unacked and is redelivered.

## 5. Decisions needing approval

| ID | Decision | Recommendation | Alternative |
|---|---|---|---|
| D1 | Attempt accounting | `runs.attempts` column, incremented atomically on claim; `claim(run_id, redelivered)` reclaims a running, uncomputed run; a fourth claim is `Exhausted` | Broker `x-death` headers; rejected because a killed worker's redelivery must count and the run row must be observable |
| D2 | Retry mechanism | Handler sleeps the backoff and republishes `run.requested` while the delivery is unacked; on exhaustion publishes `run.dead` | Per-attempt TTL retry queues; deferred because it needs a live broker to prove and #643 is not ready |
| D3 | Retry budget | 3 attempts; wait 5 s after attempt 1 and 25 s after attempt 2; the third failure is terminal | Fixed 30 s delay; simpler, but the issue asks for backoff |
| D4 | Topology | `runs.backtest` ← `run.requested`; `runs.evidence` ← `run.computed`; `runs.dead-letter` ← `run.dead`; `run.failed` is not consumed | Keep queue name = routing key; rejected because the two named work queues are the issue |
| D5 | Cancellation | Gateway publishes `run.cancelled`; worker checks the run row before execution and before CAS/finalization; internal backtest stages stay #649 | Inject a cancellation probe into the app backtest now; larger change, not needed to prove "cancel mid-run leaves cancelled and no bundle" |
| D6 | Consumer contract | `Message { routing_key, payload, redelivered }`; `consume(queue, keys, handler)`; payload remains `{run_id}` | Pass a tuple; rejected because three fields will grow and a struct names them |

## 6. Acceptance criteria

- A first handler failure republishes the same run id after 5 s; a second after 25 s.
- A third failure marks the run `failed`, records the error and attempts, publishes
  `run.dead` with `{run_id}`, and leaves no CAS artifact.
- A redelivered `run.requested` for an uncomputed `running` run executes; for a
  computed run it republishes `run.computed` without re-executing.
- `POST /runs/{id}/cancel` publishes `run.cancelled` with `{run_id}`; a cancel
  after claim leaves the run `cancelled` and writes no CAS object.
- The fake proves the queue binding: a `run.requested` message reaches the
  backtest consumer and a `run.computed` message reaches evidence.
- Existing run, worker, evidence, and gateway tests pass.

## 7. Work breakdown

| Step | Work | Paths |
|---|---|---|
| 1 | Add `attempts` to `runs.runs`, `RunRecord`, and the row loader | `migrations/0013_*.sql`, `runtime/adapters/postgres/`, `runs/store.rs` |
| 2 | `claim` takes `redelivered`, returns `Exhausted`; increment attempts; Postgres and memory; tests | `runs/store.rs`, `runtime/adapters/postgres/runs.rs` |
| 3 | `Message`, `consume(queue, keys, handler)`, fake bindings and redelivery | `runtime/adapters/mod.rs`, `runtime/adapters/rabbitmq.rs` |
| 4 | Retry policy module and worker handler; retries, dead-letter; tests | `backtest/retry.rs`, `backtest/worker.rs`, `bin/qr_backtest.rs` |
| 5 | Cancellation publish and worker checks; gateway test | `runtime/server.rs`, `backtest/worker.rs`, `tests/runs.rs`, `tests/worker.rs` |
| 6 | Evidence binary binds `runs.evidence` to `run.computed` | `bin/qr_evidence.rs` |
| 7 | Architecture doc, changelog, AI log, gate | `docs/architecture/control-plane.md`, `CHANGELOG.md`, `docs/ai-transparency-log/` |

Responsible roles: Orchestrator, Implementation, Test Engineer, Code Reviewer.
No model, strategy, execution, or portfolio behavior changes; no Quant Validator
record is required.

## 8. Verification

The plan itself changes no behavior, so the behavioral rows below are `Pending`
by design. The composed broker row is `N/A` until #643 exists.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Retry policy | The policy maps attempts to backoff and a terminal action | unit test | 1→5 s, 2→25 s, 3→dead letter | — | Pending | — | — | — |
| V2 | Attempts and claim | Claim increments attempts, reclaims a redelivered run, and exhausts at three | store tests | increments, reclaims, exhausts | — | Pending | — | — | — |
| V3 | First retry | A handler failure republishes the same run id after the delay | worker test | republish after 5 s | — | Pending | — | — | — |
| V4 | Third failure | The run is failed, `run.dead` is published, and no artifact exists | worker test | failed, `run.dead`, no artifacts | — | Pending | — | — | — |
| V5 | Redelivery after kill | A redelivered uncomputed running run executes | worker test | re-executes | — | Pending | — | — | — |
| V6 | Computed redelivery | A computed run republishes and never re-executes | worker test | republish only | — | Pending | — | — | — |
| V7 | Cancel mid-run | Cancelled run writes no CAS object and the gateway publishes `{run_id}` | worker and gateway tests | cancelled, no object | — | Pending | — | — | — |
| V8 | Topology | Backtest and evidence bindings and the DLQ are distinct | fake test and constants | routing proven | — | Pending | — | — | — |
| V9 | Gates | Tests, clippy, fmt, rustdoc, checkers | `tools/gate.sh` | pass | — | Pending | — | — | — |
| V10 | Live broker | The composed broker path is required before release | composed tier | deferred to #643 | — | Pending | — | — | — |


## 9. Risks and recovery

- The 25 s sleep blocks one consumer. Prefetch is 1; scaling adds workers, not
  shards. If it becomes a problem, move the delay into a TTL retry queue.
- A redelivery that the broker does not mark (`redelivered = false`) is treated as
  a duplicate and skipped. The stale-run sweep in #649 remains the safety net.
- The migration adds a column with `DEFAULT 0`; old rows claim to attempt 1.
- `QueueConsumer` changes shape; both workers are updated in the same change.
- No live-broker evidence in this slice; the composed test is required before a
  release.

## 10. Stop conditions

- A fix requires changing run identity, artifact bytes, or the canonical digest.
- Per-stage cancellation inside the coupled backtest turns out to be required to
  satisfy the cancel test; that needs an app-service API change and a revised plan.
- A live broker is needed to prove a behavior that the fake cannot model.
- A new dependency is required.
