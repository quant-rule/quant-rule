# Cross-Database Consistency

- Status: Current for the outbox, reference and leader slice (HA-004, HA-006); artifact references and run lineage added by RA-006
- Issue: [#1102](https://github.com/quant-rule/quant-rule/issues/1102)
- Claim type: Behaviour Claim
- Related: [HA architecture](ha.md), [control plane](control-plane.md), [history retention](history-retention.md), [four databases](../config/2026-11-22/four-databases.md), [ADR-HA](../adr/2026-11-22/1082-ha-deployment-profile.md)

Four logical databases share one cluster and no transaction. A step that spans
them commits its own record first and carries an intent, a tombstone, or an
idempotency key that a reconciler can retry. No service reads another service's
schema, and no query joins across databases.

## 1. Invariants

| Invariant | Meaning |
|---|---|
| One owner per record | The service that owns a row is its only writer. |
| Commit, then notify | A queue message is an observation; the row is authoritative. |
| At-least-once delivery | A publish may repeat; a consumer treats a repeat as a lookup. |
| One identity | A run is content-addressed; a submission and a publish cannot create a second run. |
| Reference, not payload | A message carries ids, never a reading, a candle, or a path. |
| Safe direction | A leaked object is safe; a missing shared object, a lost run, and an unprotected dataset are not. |
| Tombstone survives | An erase commits its tombstone with the delete; the tombstone outlives the row. |

## 2. Run notification outbox

`runs.outbox` holds `(run_id, routing_key, attempts, last_error, published_at)`.
The unique key is `(run_id, routing_key)` and the row references `runs.runs`
with `ON DELETE CASCADE`.

| Step | Transaction |
|---|---|
| Submit | The run, the submission, the stage rows, and a `run.requested` intent commit together. |
| Compute | The digest, the artifacts, the stages, and a `run.computed` intent commit together. |
| Complete | The terminal state and the deletion of the run's intents commit together. |
| Fail, cancel | The terminal state commits first; the intent delete follows and a terminal run is never re-armed. |
| Erase | The run delete removes any surviving intent through the cascade. |

The publisher runs every 5 s. Each pass reconciles and drains:

- **Startup** re-arms every unfinished run, even if its last publish was
  confirmed. This rebuilds the queue from run rows after a broker loss.
- **Periodic** inserts an intent a pre-outbox run is missing and repeats
  nothing that was already confirmed.
- **Drain** publishes oldest first, marks a confirmed row, and stops on the
  first failure so a broker outage is one warning, not a storm.

A live run wants exactly one intent: `queued`, and `running` without a digest
whose lease is absent or expired, want `run.requested`; `running` with a digest
wants `run.computed`. A terminal run wants none and its rows are deleted.

### Run lease and fencing

`runs.runs` carries `lease_holder`, `lease_term`, and `lease_expires_at`. The
lease uses database time; the term is monotonic per run.

The holder is `QR_WORKER_ID`, falling back to the process id. The term, not
the holder, is the fence.

| Rule | Behaviour |
|---|---|
| Claim | A queued run is claimable. A running run is claimable only when its lease is absent or expired, or when the same holder continues its own live term. A live lease is never stolen. |
| Renew | The worker renews every 20 s from a control-plane task; the lease window is 60 s. A heartbeat with a stale holder or term is refused. |
| Fence | `computed`, `complete`, and `fail` carry the term. Evidence refuses a stale term, so an old worker cannot publish a succeeded run or a partial artifact set. |
| Cancel | Cancellation increments the term and clears the lease, so a worker holding the old term cannot finish the run. |
| Release | A succeeded, failed, or cancelled run clears the holder and expiry. |
| Reconcile | A live lease is not re-armed. An absent or expired lease is wanted; startup re-arms it, and a periodic pass re-arms it after the lease window since its last publish. |

A queue rebuild therefore republishes a running run whose lease has expired; a
new worker claims a new term and a stale worker is fenced out. A live worker
keeps its lease and is not disturbed.

A submission is accepted when the row and its intent commit. A broker that is
down delays the message; it does not fail the submission and it cannot lose the
run. A repeated submission finds the same run row and republishes the queued
run, so a lost publish is recovered; the worker treats the repeat as a lookup.

The backtest worker still publishes `run.computed` after its lifecycle call.
That publish is an optimisation on the same broker: the outbox row is the
durable copy, and finalisation skips a second delivery of a finished run.

Leader finalisation uses `runs.leader_lease`; the live leader term plus the run
term fences the write. Only the leader consumes `run.computed`; reads stay active.

## 3. Dataset references

A consumer registers a reference before it uses a dataset and releases it before
it erases its own row. Registration is idempotent on
`(profile_id, data_version, reference_id)`.

| Event | Record | Retry |
|---|---|---|
| Register | `catalog.dataset_references` in the data service | Startup sweep registers a reference a pre-registry row is missing. |
| Release | `runs.dataset_reference_releases` in evidence, written with the consumer delete | The 30 s sweep retries until the data service confirms. |

A release tombstone carries the profile, the data version, the reference id, the
consumer kind, the attempt count, and the last error. It is retained after the
release, so a repeated release is a no-op and the erase keeps its evidence. A
`404` means the dataset row is gone, so nothing can be blocked; the tombstone is
confirmed instead of retried forever.

### Artifact references and lineage

`catalog.artifact_references` follows the same shape for computed signal
artifacts: a consumer registers before it cites the artifact, and the retention
pass refuses to collect a referenced row. The row and its
`catalog.retention_events` tombstone commit before an unshared object is deleted.

A run's research references live in `runs.run_lineage_references` in evidence.
That is an evidence write, not a cross-database one: evidence does not read the
catalog schema and does not join to it. The run-v5 integration that resolves a
pack records the set; a repeated set is idempotent and a different set is
refused. Registering the signal-artifact reference is the signal writer's call
when that writer lands.

## 4. Erase boundaries

| Record | Preconditions | Tombstone |
|---|---|---|
| Run | Archived; `failed` or `cancelled` | `runs.retention_events` + a release tombstone for `run:<id>` |
| Dataset | Archived; zero registered references | `catalog.retention_events` |
| Research record | Archived; not decided | `research.retention_events`; a failed release records a tombstone |

A dataset with any registered reference refuses erase with `history.protected`.
A reference left by a failed run submission is either released in the request or
left for the release sweep; it never blocks erase without a retry path.

## 5. Failure matrix

| Failure | Behaviour |
|---|---|
| Crash after the run commit, before the publish | The intent is pending; the next drain publishes it. |
| Submit retried after a commit | The same run row is returned; one intent, one run. |
| Broker down during submit | The run is accepted; the publisher retries; no run is lost. |
| Worker publishes `run.computed` twice | Evidence finalises once; a finished run is skipped. |
| Worker or delivery lost while the run is running | Its lease expires; the startup or periodic pass re-arms `run.requested`; a new worker claims a new term. |
| A live worker is still executing | Its lease is live; the outbox does not re-arm it and no other worker may steal it. |
| A stale worker returns after a reclaim | Its term no longer matches; `computed`, `complete`, and `fail` are refused. |
| Evidence restarts with queued runs | The startup pass re-arms and republishes every unfinished run. |
| Crash after the run delete, before the release | The tombstone is committed with the delete; the sweep releases it. |
| Data service down during erase | The erase stands; the release remains pending. |
| Release of an already-erased dataset | `404` confirms the tombstone. |

## 6. Not in this slice

| Item | State |
|---|---|
| Quorum queues, publisher confirms, delivery limit | Quorum queues are current (HA-005); publisher confirms and the delivery limit remain the queue guard. |
| Diagnostic stage notifications | They keep their existing publish path; this outbox covers runs only. |
| Simulation dataset references | Not registered; the research plane has no data client. |
| Mixed worker/evidence protocol versions | The claim and finalisation payloads carry the lease fields; an in-flight run is not supported across a mixed control plane. Drain it or let the lease expire before a rollback. |
| Crash between a reference registration and the run commit | The reference is registered first (the safe direction); this orphan is not released automatically. |
| Research erase tombstone in the store transaction | The route records a tombstone after erase; a crash in that window is not retried. |
| Profile erase | No command exists; a profile erase would be a new capability and a human decision. |
| Orphan-object sweep | A leaked object from a crash is accepted. The signal-artifact retention pass removes an unreferenced, expired artifact and its unshared object; it does not sweep an arbitrary orphan. |

## 7. Verification

- `cargo test -p quantrule-evidence-service --lib` covers the memory outbox,
  restart re-arm, terminal cleanup, the release tombstone, the run lease, and
  the leader lease.
- `--test outbox_conformance` runs the shared body against memory; with
  `QR_TEST_DATABASE_URL=<disposable>` and `--ignored`, against Postgres.
- `--test leader` covers acquisition, renewal, expiry, takeover, the single
  finalizer, stale-leader refusal, duplicate delivery, and standby reads.
- `--test lineage`, `--test pack_conformance` and `--test artifact_conformance`
  run their store bodies against memory and Postgres.
- `cargo test -p quantrule-runtime --test migration_contract` pins migrations 18
  through 21, catalog migrations 7 and 8, and every checksum.
