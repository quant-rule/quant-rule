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
| `run.requested` | gateway | backtest worker | none; the run row holds the inputs |
| `run.computed` | backtest | evidence | artifact manifest |
| `run.completed` | evidence | none required | run id and digest |
| `run.cancelled` | gateway | backtest | none |
| `run.failed` | backtest | evidence | stage and error code |

Rules:

- A message names a run. It never contains a reading, a price, or a decision.
- The run row is the source of truth for status; messages are notifications of
  intent.
- Consumers are idempotent: the same message twice produces the same state.

## 3. Topology

```text
exchange  quantrule.runs        (direct, durable)
queue     runs.backtest         routing key run.requested
queue     runs.evidence         routing keys run.computed, run.failed
queue     runs.dead-letter      after the retry budget
```

Retry budget: three attempts with exponential backoff, then dead-letter and
`failed`. A dead-lettered run is never silently lost; the run row records the
attempt count.

## 4. Idempotency

```text
run_id = sha256(data_version ‖ config_hash ‖ model_version ‖ image_digest)
```

- A submission whose `run_id` exists returns the existing run with `200`.
- A first submission returns `202`.
- A redelivered message re-executes the same run; content addressing makes the
  result identical.
- Cancellation followed by completion is impossible: the worker checks the
  cancellation flag before each stage boundary.

## 5. Concurrency

| Setting | Meaning | Default |
|---|---|---|
| `BACKTEST_CONCURRENCY` | Runs this worker executes at once | 1 |
| `MODEL_SHARDS` | Timeframe shards this worker executes at once | 1 |
| Prefetch | Messages held per worker | 1 |

A run is never split across backtest workers. Scaling adds workers, not shards.

## 6. Cancellation

1. Client calls `POST /runs/{id}/cancel`.
2. Gateway writes `cancel_requested` on the run row and publishes `run.cancelled`.
3. The worker checks the flag between stages and before finalization.
4. If it stops, the run becomes `cancelled`; no artifact is finalized.

Cancellation is best-effort by design: a run that has already reached the
evidence service has completed, and the response says so.

## 7. What the control plane must never do

- Carry a model reading.
- Change a run's inputs after submission.
- Decide a trade.
- Depend on delivery order for a result.
