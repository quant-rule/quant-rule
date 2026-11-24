# Control Plane — Run Lifecycle

- Status: Current; released as v0.9.0 on 2026-11-15 (prepared as v1.0.0); runtime drain, retries, reconnect, and the HA-013 stage lease/outbox in v1.6.0; approved 2026-11-12
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
| `queued` | Accepted, not started | evidence |
| `running` | A worker holds the message | backtest |
| `succeeded` | Artifact set complete and digest recorded | evidence |
| `failed` | A stage failed or retries exhausted | backtest or evidence |
| `cancelled` | Operator cancelled before completion | evidence |

There is no `rejected` state: invalid input is a `400` at submission and creates
no run row.

## 2. Messages — `quantrule.queue/v1`

```json
{ "v": 1, "type": "run.requested", "run_id": "sha256:…", "payload": {} }
```

| Type | Producer | Consumer | Payload |
|---|---|---|---|
| `run.requested` | evidence (outbox) | backtest | run id only; the run row holds the inputs |
| `run.computed` | evidence (outbox), backtest | evidence | run id only; the run row holds the manifest |
| `run.cancelled` | evidence | backtest | run id only; the run row is authoritative |
| `run.failed` | backtest | none required | run id only; the run row holds the error |
| `run.dead` | backtest | operator | run id only; the run row holds the error and attempts |
| `stage.requested` | evidence (outbox) | backtest | stage run id only; the stage row holds the inputs |
| `stage.computed` | backtest | none required | stage run id only; the stage row holds the artifact |
| `stage.cancelled` | evidence | backtest | stage run id only; the stage row is authoritative |
| `stage.failed` | backtest | none required | stage run id only; the stage row holds the error |

A diagnostic stage is not a run. It has its own `stage` schema, its own
`stage_run_id`, a content-addressed artifact, and `evidence: false`; it never
produces a canonical digest or a run record.

Rules:

- A message names a run. It never contains a reading, a price, or a decision.
- The run row is the source of truth for status; messages are notifications of
  intent.
- Consumers are idempotent: the same message twice produces the same state.
- `run.requested` and `run.computed` are published from the evidence outbox;
  `stage.requested` from the stage outbox. Each intent commits with its row. See
  [cross-database consistency](cross-database-consistency.md).
## 3. Topology

```text
exchange  quantrule.runs          (direct, durable)
exchange  quantrule.runs.dlx      (direct, durable, dead-letter)
queue     runs.backtest.g1        run.requested, run.cancelled
queue     runs.evidence.g1        run.computed
queue     runs.dead-letter.g1     run.dead and dead-lettered work
queue     stages.worker.g1        stage.requested, stage.cancelled
```

Every queue is a durable quorum queue. The three work queues declare
`x-delivery-limit=3` and `x-dead-letter-exchange=quantrule.runs.dlx`; the
dead-letter queue is quorum and has no dead-letter target, so an exhausted
message is kept once and cannot loop. A failed handler rejects with requeue,
which RabbitMQ 4.3 and later count as a failed delivery; the next failed delivery
after the limit is dead-lettered with its original routing key and an `x-death`
header. The run row remains the record of the run.

The `.g1` suffix is the queue generation, read from `QR_QUEUE_GENERATION`
(default 1). Every service in one deployment must use the same generation; a
mismatch leaves publishers and consumers on different queues. A changed
declaration is deployed as a new generation: the services declare new names and
never redeclare a live queue with different arguments. The exchange names stay
stable. Stop new submissions, let the old generation's consumers drain its
queues to zero, stop them, then unbind or delete the empty queues before the new
generation publishes; the service never deletes a queue. A definitions export
records the topology but not the queued messages.

The stage queue is separate so a long diagnostic stage cannot delay an evidence
run. Each consumer owns its channel and one prefetch slot, so the run and stage
consumers cannot hold each other's delivery; the worker starts
`QR_BACKTEST_CONCURRENCY` run consumers. The stage consumer registers after the
run consumer and declares RabbitMQ consumer priority `x-priority = -1`; the run
consumer keeps the default `0`.

The run and stage outboxes are the durable producers; a repeated delivery is a lookup.
Only the leader consumes `runs.evidence.g<n>` and finalises; a standby serves
reads and acquires the lease after expiry. The finalisation write carries the
run-attempt term and the leader term, and the lease-row lock refuses a stale
leader. Retry budget: three attempts. The handler holds the unacked delivery for
5 s after the first failure and 25 s after the second; the third failure marks the
run `failed` and publishes `run.dead`. A dead-lettered run is never silently
lost; the run row records the attempt count. A run or stage claim records a monotonic term
and a 60 s lease, and the worker renews it every 20 s. A queued row is
claimable; a running row only when its lease is absent or expired. A computed
run is only republished; a terminal stage is never claimed. The quorum delivery limit is the second guard for a
consumer that cannot reach that path. A consumer error reconnects in-process
with 1–30 s backoff before the container restarts.

A SIGTERM stops new claims and gives the in-flight run up to
`QR_DRAIN_TIMEOUT_WORK` (25 s) to finish. An aborted run keeps its lease and is
recovered by lease expiry and the outbox; shutdown never marks it failed.
## 4. Idempotency

```text
run_id = sha256(data_version ‖ config_hash ‖ model_version ‖ image_digest)
```

A promoted run appends the tagged frozen model component:

```text
run_id = sha256(.. full components .. ‖ "model-artifact" ‖ model_artifact_id)
```

The full-run id is unchanged. A promoted run is distinct from the same full run,
records `model_artifact_id` on the run row, and can still produce the same
canonical digest.

- The first submission returns `202`; an existing `run_id` returns `200`.
- The run row and `run.requested`, and the stage row and `stage.requested`,
  commit in one transaction, so a retry finds the same record and never creates
  a second run or stage.
- A restart or a leader acquisition re-arms every unfinished run and stage: a running run without a digest wants
  `run.requested`, a running run with a digest wants `run.computed`, and an unfinished stage wants `stage.requested`.
- The same promoted submission returns the same promoted run; a full and a
  promoted submission never collide.
- A claim takes a new lease term; a running run is reclaimable only after its
  lease expires. `computed`, `complete`, and `failed` carry the term, so a stale
  worker cannot overwrite the new attempt.
- Cancellation followed by completion is impossible: the worker checks the
  cancellation flag after the claim, before the model call, before the backtest,
  and before the first CAS write.
## 5. Concurrency

| Setting | Meaning | Default |
|---|---|---|
| `QR_BACKTEST_CONCURRENCY` | Runs this worker executes at once | 1 |
| Prefetch | Messages held per worker | 1 |

A run is never split across backtest workers. Scaling adds workers, not shards.
## 6. Cancellation

1. Client calls `POST /runs/{id}/cancel`.
2. Evidence writes `cancelled` on the run row, increments the lease term, clears
   the lease, and publishes `run.cancelled` best-effort; the row is authoritative.
3. The worker checks the flag after the claim, before the model call, before the
   backtest, and before the first CAS write.
4. If it stops, the run stays `cancelled`; no artifact is finalized. A check
   inside the backtest is made after it returns, at the before-CAS boundary.

Cancellation is best-effort by design: a run that has already reached the
evidence service has completed, and the response says so.
## 7. What the control plane must never do

- Carry a model reading.
- Change a run's inputs after submission.
- Decide a trade.
- Depend on delivery order for a result.
