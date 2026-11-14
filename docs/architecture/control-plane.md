# Control Plane — Run Lifecycle

- Status: Target v1.0.0; approved 2026-11-12
- Part of: [`README.md`](README.md)
- Decision: [`ADR-2026-11-12-02`](../adr/2026-11-12/02-two-plane-control-and-data.md)
- Claim type: Behaviour Claim

The control plane decides which run happens when. It is asynchronous, retryable,
and carries no measurement.

## 1. States

```text
queued ──► running ──► succeeded
              │
              ├──► failed
              └──► cancelled
```

| State | Meaning | Set by |
|---|---|---|
| `queued` | Accepted, not started | gateway |
| `running` | A worker holds the message | backtest |
| `succeeded` | Artifact set complete and digest recorded | evidence |
| `failed` | A stage failed or retries exhausted | backtest or evidence |
| `cancelled` | Operator cancelled before completion | backtest |

There is no `rejected` state: invalid input is a `400` at submission and creates
no run row.

## 2. Messages — `quantrule.queue/v1`

```json
{ "v": 1, "type": "run.requested", "run_id": "sha256:…", "payload": {} }
```

| Type | Producer | Consumer | Payload |
|---|---|---|---|
| `run.requested` | gateway | backtest | run id only; the run row holds the inputs |
| `run.computed` | backtest | evidence | run id only; the run row holds the manifest |
| `run.cancelled` | gateway | backtest | run id only; the run row is authoritative |
| `run.failed` | backtest | none required | run id only; the run row holds the error |
| `run.dead` | backtest | operator | run id only; the run row holds the error and attempts |

Rules:

- A message names a run. It never contains a reading, a price, or a decision.
- The run row is the source of truth for status; messages are notifications of
  intent.
- Consumers are idempotent: the same message twice produces the same state.

## 3. Topology

```text
exchange  quantrule.runs        (direct, durable)
queue     runs.backtest         routing keys run.requested, run.cancelled
queue     runs.evidence         routing key run.computed
queue     runs.dead-letter      routing key run.dead
```

Retry budget: three deliveries. The handler holds the unacked delivery for 5 s
after the first failure and 25 s after the second; the third failure marks the
run `failed` and publishes `run.dead`. A dead-lettered run is never silently
lost; the run row records the attempt count. A redelivered message reclaims a
running, uncomputed run; a computed run is only republished.

## 4. Idempotency

```text
run_id = sha256(data_version ‖ config_hash ‖ model_version ‖ image_digest)
```

- A submission whose `run_id` exists returns the existing run with `200`.
- A first submission returns `202`.
- A redelivered message reclaims a running, uncomputed run; a running run with
  a recorded digest republishes `run.computed` so a lost publish still reaches
  evidence. Neither is executed twice.
- Cancellation followed by completion is impossible: the worker checks the
  cancellation flag before execution and before the first CAS write.

## 5. Concurrency

| Setting | Meaning | Default |
|---|---|---|
| `BACKTEST_CONCURRENCY` | Runs this worker executes at once | 1 |
| `MODEL_SHARDS` | Timeframe shards this worker executes at once | 1 |
| Prefetch | Messages held per worker | 1 |

A run is never split across backtest workers. Scaling adds workers, not shards.

## 6. Cancellation

1. Client calls `POST /runs/{id}/cancel`.
2. Gateway writes `cancelled` on the run row and publishes `run.cancelled`
   best-effort; the row is authoritative.
3. The worker checks the flag before execution and before the first CAS write.
4. If it stops, the run stays `cancelled`; no artifact is finalized. A check
   inside the coupled backtest is still made after it returns; per-stage probes
   remain #649.

Cancellation is best-effort by design: a run that has already reached the
evidence service has completed, and the response says so.

## 7. What is not in the control plane

Simulation has no queue and no lifecycle. It is synchronous and interactive, and
its oracle and check complete inside a request. If they ever exceed a few
seconds, they join this plane with a recorded decision. See
[`services.md`](services.md) §7.

## 8. What the control plane must never do

- Carry a model reading.
- Change a run's inputs after submission.
- Decide a trade.
- Depend on delivery order for a result.
