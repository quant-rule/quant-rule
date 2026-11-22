# AI Transparency Log — #1104 HA-006 Evidence Leader Lease, Fencing and Idempotent Finalisation

- Date: 2026-11-22 (UTC)
- Issue: [#1104](https://github.com/quant-rule/quant-rule/issues/1104)
- Tool: OpenCode implementation subagent
- Boundary: one evidence-service slice, its migration, tests, and documentation
  for HA-006. No quorum-queue change, active/active data, v3 renderer, Swarm,
  backup, or deployment. No commit, push, pull request, merge, tag, board
  change, secret, or deployment.

## Intent and boundary

The agent read `AGENTS.md`, the v1.6.0 execution plan, ADR-HA, the HA, control
plane, cross-database consistency, migration, deployment, and operations
contracts, the evidence service main/state/routes/finalizer/store/migrations,
the HA-004 lease and outbox code, and the HA-005 quorum consumer. The work makes
two evidence replicas safe: one database lease holder consumes `run.computed`,
finalisation is fenced by the leader term as well as the run-attempt term, and
both replicas keep serving reads.

The agent deliberately did not add a quorum queue, active/active data or
simulation, the v3 renderer, Swarm, or backup/DR. Those remain other leaves.

## Implemented

- `runs.leader_lease` (migration 21, pinned in
  `deploy/postgres/migration-ledger.json`): one row with holder, monotonic term,
  database-time expiry, and update time.
- `src/leader.rs`: the `LeaderLease` trait, the memory lease, the
  acquire-or-renew election loop, and the leader-only `run.computed` consumer.
- `src/postgres_leader.rs`: acquisition is a compare-and-swap on expiry,
  renewal requires holder and term, release expires the row immediately, and
  every read uses the database clock.
- Finalisation writes (`complete`, `fail_finalization`) lock the leader row and
  require its live term. The Postgres row lock serialises acquisition with a
  finalisation; the memory store takes the same lock order.
- A stale leader is refused and its delivery is not acknowledged. The consumer
  waits for the election loop to publish the loss so the in-flight delivery is
  not counted against the quorum delivery limit.
- `main` starts the election before the server, consumes only while leader, and
  keeps the outbox publisher and reconcilers active so a leader restart rebuilds
  queued work from run rows. A leadership acquisition also runs the outbox
  startup pass, so a long-running standby re-arms after a takeover without
  restarting.
- `QR_LEADER_ID` names the holder; the default is hostname plus process id.
- Tests: memory leader acquisition, renewal, expiry, takeover, release, stale
  finalisation refusal, duplicate delivery, one finalizer with a standby, a
  leader-acquisition re-arm, and a read while standby; ignored Postgres cases for
  the same lease and fence.
- The Postgres finalisation writes live in `src/postgres_finalize.rs`; the
  memory finalisation fence lives in `src/runs/memory/finalize.rs`, keeping both
  store modules below the 1,000-line limit.
- Docs: HA, control plane, cross-database consistency, service catalog, module
  map, deployment, operations, the multi-host manual, the operations manual,
  the changelog, and the migration ledger.

## Design decisions and boundaries

- The architecture draft named `evidence.leader_lease`; the evidence database
  owns `runs`, `research`, and `stage` schemas, so the implemented table is
  `runs.leader_lease` and the HA document was corrected. No new schema was
  introduced for one row.
- The evidence leader window is 30 s with a 10 s renewal. This is shorter than
  the 60 s/20 s run lease so a standby takes over promptly; the finalisation
  row lock, not the window, is the correctness fence.
- `fail_finalization` is separate from the worker-reported `fail`. A worker may
  report a failure through any replica; only the leader records the finalizer's
  verification failure.
- The outbox publisher and reconcilers stay active on both replicas because
  they are idempotent. Only the `run.computed` consumer is leader-gated.
- A graceful shutdown does not release the lease explicitly; expiry is the
  takeover path. A release is available for a future shutdown hook.
- Retention sweeps remain active/active. They delete only their own stage rows
  and objects and are idempotent; this slice does not gate them on leadership.
- The leadership loop keeps its term during a transient database error. The
  finalisation row lock still refuses a write when the lease is not live, so
  this cannot create a second finalizer.

## Verification and evidence

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy -p quantrule-evidence-service --all-targets -- -D warnings` | pass |
| `cargo test -p quantrule-evidence-service --lib` | 79 passed |
| `cargo test -p quantrule-evidence-service --tests --no-fail-fast` | 11 evidence, 3 leader (+3 ignored), 1 outbox conformance (+3 ignored), 19 research routes, 21 runs, 4 retention, and the remaining targets pass |
| Disposable Postgres, `leader --ignored` | 3 passed: acquisition/renewal/takeover, stale-leader refusal, standby read |
| Disposable Postgres, `outbox_conformance --ignored` | 3 passed: shared conformance, periodic intent, stale worker fence |
| `cargo test -p quantrule-runtime --test migration_contract` | 5 passed; migration 21 checksum matches |
| `cargo test -p quantrule-evidence-service --lib migrations::` | 12 passed |
| `qr-governance check --all` | `governance check ok` (rerun after the final doc edits) |
| `tools/gate.sh` | `gate passed: full with CARGO_TARGET_DIR=…/target/gate`; exit 0 |

The disposable Postgres cases ran in a throwaway container with the pinned
`postgres@sha256:846a…` image and one database per target. The full
`tools/postgres_composed.sh` wrapper was not run; its leader target was added to
the list. The full gate ran on the implementation; the final documentation edits
were checked by a rerun of `qr-governance check --all` and `cargo fmt --check`.

## Learning and corrections

- The first memory store required a leader term but had no shared row; the
  leader lease is now embedded in `MemoryRuns`, so a test acquires a term from
  the same state the store checks.
- The first research-route helper acquired a new term on every seeded run and
  failed on the second call. The helper now reuses the live row and acquires
  only when no term is live.
- The first stale-delivery path would have returned an error immediately, which
  could count a rejection while the old consumer was still attached. It now
  waits for the election loop to publish the loss before returning.
- The architecture documents were already at the 170-line cap. The leader
  sections were added by replacing and compressing existing prose, not by
  exceeding the cap.
- The Postgres `complete` transaction originally read the leader row without a
  lock. It now uses `FOR UPDATE`, so an acquisition and a finalisation cannot
  interleave and a stale write is refused deterministically.
- The first implementation pushed `postgres_runs.rs` and `runs/memory.rs` over
  the 1,000-line cap. The finalisation bodies moved to `postgres_finalize.rs`
  and `runs/memory/finalize.rs`; the trait methods delegate and the SQL/state
  logic stayed together.
- `crates/quantrule-runtime/tests/migration_contract.rs` is generic: it parses
  each service ledger and compares it with the pinned manifest. Adding migration
  21 to the manifest was sufficient; the contract test needed no code change and
  was run green.
