# AI Transparency Log — #1102 HA-004 Cross-DB Outbox, Idempotency and Reconciliation

- Date: 2026-11-22 (UTC)
- Issue: [#1102](https://github.com/quant-rule/quant-rule/issues/1102)
- Tool: OpenCode implementation subagent
- Boundary: one focused evidence-service slice, its contract documents, tests,
  and verification for HA-004. No quorum queues, evidence leader, active/active
  data or simulation, v3 renderer, Swarm, backup, new database, cross-database
  transaction or join. No commit, push, pull request, merge, tag, board change,
  secret, or deployment.

## Intent and boundary

The agent read `AGENTS.md`, the v1.6.0 execution plan, ADR-HA, the HA and
four-database contracts, the migration contract, the evidence and data service
sources, the runtime queue and backend adapters, the dataset-reference registry,
and the run/research erase paths.

The work makes the HA-002 database split safe for the run lifecycle and dataset
references: a run and its notification commit together, and an erase commits a
release tombstone with the delete. The remaining HA-004 items are recorded as
boundaries rather than silently widened.

## Implemented

- `runs.outbox` (migration 18): `(run_id, routing_key)` unique, one row per
  `run.requested` or `run.computed`, cascade with the run, pending index.
- A per-run lease on `runs.runs` (migration 19): `lease_holder`, `lease_term`,
  `lease_expires_at`. A claim records a monotonic term and a 60 s expiry; the
  worker renews every 20 s from a control-plane task; `computed`, `complete`,
  and `fail` carry the term; cancellation increments the term and clears the
  lease; a terminal run releases it.
- `runs.dataset_reference_releases` (migration 18): release tombstone written in
  the consumer delete transaction, retained after the release.
- `src/outbox.rs`: the `RunOutbox` trait, reconcile (startup re-arm, periodic
  insert), drain (oldest first, stop on the first failure), the run-id payload,
  and the 5 s publisher loop.
- `src/releases.rs` and `reconcile::release_all`: the release log trait, the
  retry loop, and `404` confirmation of a dataset that is already gone.
- `src/postgres_outbox.rs`: the Postgres outbox and release implementations.
- `runs/store.rs` split into the trait (`store.rs`) and the memory store
  (`memory.rs` + `memory/tests.rs`) at the file-size seam; no behaviour change.
- Submission, computed, complete, fail, cancel, and erase SQL changed so the
  intent, the tombstone, and the state transition are one transaction.
- `EvidenceState`/`EvidenceServices` carry optional `outbox` and `releases`
  handles; the service main wires the same `PgRunStore` to both.
- Route changes: submission drains the outbox best-effort and is accepted on the
  commit; the internal computed route drains after the digest write; run erase
  retries from the tombstone; a failed research create or erase records a
  release tombstone when the handle is present.
- Tests: memory outbox and release tests, an in-process outbox conformance
  suite, an opt-in Postgres conformance case registered in
  `tools/postgres_composed.sh`, and updated route/harness tests.
- Docs: `docs/architecture/cross-database-consistency.md`, the architecture
  index, control-plane, history retention, four-database verification, module
  map, `CHANGELOG.md`, and the migration ledger pin.

## Verification and evidence

| Check | Observed |
|---|---|
| `cargo test -p quantrule-evidence-service --lib` | 70 passed |
| `cargo test -p quantrule-evidence-service --tests --no-fail-fast` | all targets passed |
| `cargo test -p quantrule-evidence-service --test outbox_conformance` | 1 passed, 3 ignored (Postgres) |
| Disposable Postgres, `outbox_conformance --ignored` | 3 passed |
| `cargo test -p quantrule-runtime --test migration_contract` | 5 passed; migrations 18 and 19 checksums match |
| `cargo clippy -p quantrule-evidence-service --all-targets -- -D warnings` | pass |
| `cargo fmt --all -- --check` | pass |
| `tools/gate.sh` | full gate passed: fmt, workspace clippy, workspace tests, rustdoc, `governance check ok` |

## Review correction

The first submission omitted `running` runs without a digest from
`wanted_notifications`, so a startup re-arm could not name a run whose worker
or delivery was lost. The query and the memory store now yield `run.requested`
for `queued` and `running` without a digest, and `run.computed` for `running`
with a digest. A memory unit test and the shared outbox conformance case prove
the startup re-arm and the periodic no-repeat against both stores. The
worker-claim boundary — only a broker redelivery proves that a running holder
is gone — was recorded in the contract. The human then approved lease recovery;
it is implemented below and the boundary is removed.

## Lease recovery

The claim protocol now uses a lease instead of the broker's redelivery flag. A
queued run is claimable. A running run is claimable only when its lease is
absent or expired, or when the same holder continues its own live term; a live
lease is never stolen. The claim records a new term, and the worker starts a
control-plane heartbeat task that renews the lease every 20 s against a 60 s
window. `computed`, `complete`, and `fail` carry the term; evidence refuses a
stale term, so an old worker cannot publish a succeeded run or a partial
artifact set. Cancellation increments the term and clears the lease, and a
terminal state releases it.

The outbox selection now includes `running` without a digest only when the
lease is absent or expired. A startup pass re-arms it; a periodic pass re-arms
it after the lease window since its last publish, so a live lease does not spin
messages. The `PgRunStore` and `MemoryRuns` accept an explicit lease TTL so a
composed or memory test can observe an expired lease without sleeping.

The worker tests thread the continuation term through a retry; the
`MemoryEvidence` fake mirrors the live, continuation, expired, stale, and
terminal rules. The composed conformance covers the expired selection and the
Postgres fence: a live lease is not stolen, a stale term cannot compute, fail,
or complete, the same holder can continue, and the terminal state releases the
lease.

## Learning and corrections

- The first submission draft kept the direct publish and returned `503` when
  the broker failed. That defeats the outbox: the row already committed, so a
  `503` tells the client a run was refused when it exists and will run. The
  route now accepts on the commit and logs the retry. This changes the
  documented failure behaviour and is reported as a decision needing approval,
  not as a silent contract change.
- The first re-arm draft ran on every publisher tick, which would republish
  every queued run every 5 s. Startup and periodic modes are now separate; only
  a restart re-arms a confirmed publish.
- The original `RunStore` split would have pushed `store.rs` past the file-size
  limit. Moving the memory store and its tests into `runs/memory.rs` and
  `runs/memory/tests.rs` kept both files cohesive and under the limit; the move
  is mechanical.
- Releasing the reference before writing the tombstone leaves the same crash
  window the issue exists to close. The tombstone is written first (in the
  erase transaction where possible), then the data-service call is attempted.
- The `pending` name collided between the run outbox and the release log once
  both were implemented by `MemoryRuns`; the release method was renamed
  `pending_releases` rather than disambiguated at each call site.
- The research erase tombstone is written by the route after the store delete,
  not by the research store transaction. A crash exactly between the two is not
  retried; this is recorded as a boundary rather than concealed.
- The registration still happens before the run commit. A crash exactly between
  the data-service registration and the evidence commit leaves a blocking
  reference, which is the safe direction but not automatically released. Closing
  that window needs a pre-registration claim row, which is a larger protocol and
  is recorded as a boundary.
- Simulation does not register dataset references today and the research plane
  has no data client. Adding that would widen the plane; it is recorded as a
  boundary.
- A lease alone does not distinguish "the same worker is retrying" from "a
  duplicate delivery arrived". The worker threads the previous term through
  `ProcessOutcome::Retry` and the claim accepts only that explicit continuation;
  a new message with no continuation cannot take a live lease.
- The first memory `complete`/`fail` cleared the lease map after cloning the
  record, so the returned record still showed the old holder and expiry. The
  fields are now cleared on the record before the clone.
- The composed conformance cannot wait 60 s for an expired lease. The stores
  accept an explicit TTL for a test, and the production constructor keeps 60 s;
  the shared body uses zero and the Postgres-only test uses the real default
  plus direct term checks.
- A mixed worker/evidence control plane is not supported for in-flight runs:
  the new claim and finalisation payloads carry lease fields. The schema stays
  additive and the previous image reads it; the operator drains in-flight runs
  or lets the lease expire before a rollback. This is recorded in the contract.
- The Postgres `wanted_notifications` missed the lease condition while the
  memory store had it; every test expired the lease before asserting, so the
  mismatch was invisible. The query now requires an absent or expired lease,
  and the memory and Postgres tests assert that a live lease is not wanted and
  an expired one is. The contract verification note now names migrations 18 and
  19.
