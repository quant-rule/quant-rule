# AI Transparency Log — #1106 HA-008 Simulation Decision and Mark Concurrency

- Date: 2026-11-22 (UTC)
- Issue: [#1106](https://github.com/quant-rule/quant-rule/issues/1106)
- Tool: OpenCode implementation subagent
- Boundary: read-only repository review, implementation, tests, and documentation
  for HA-008 only. No commit, push, pull request, merge, tag, board change,
  secret, or deployment.

## Intent and boundary

The work makes two simulation replicas safe against concurrent marks and the
terminal decision. The agent read the v1.6.0 plan, ADR-HA, the HA architecture,
the four-database contract, the migration contract, the simulation service and
domain crates, and the existing memory and Postgres test seams before changing
anything.

The agent did not implement the evidence leader (HA-006), active/active data,
quorum queues, the v3 renderer, Swarm, MinIO, backup/DR, or a simulation queue.
The two-replica claim is limited to the database concurrency contract; the
deployment profile itself remains a later release step.

## Implemented

- The simulation repository locks one `simulation.runs` row with
  `SELECT ... FOR UPDATE` for mark, oracle, window, check, decision, and erase
  transactions. Different runs still proceed independently.
- The decision read now happens under that lock instead of through a second
  pooled connection. The `UPDATE` repeats `decision IS NULL AND status <>
  'decided'` and requires one affected row, so exactly one decision commits.
- A later decision, including an identical retry, returns the stable
  `simulation.locked` conflict. It is not overwritten and is not treated as a
  new success.
- A mark identity is an upsert: the stored mark id and creation time survive an
  exact retry, while quality and note may be corrected. A mark that loses to a
  decision is refused before the insert.
- Erase takes the same run lock as a decision, so a decided run cannot be erased
  concurrently.
- Migration 16 adds a decision-completeness check constraint and a
  `BEFORE UPDATE` trigger that refuse a changed or cleared decision. The
  migration is pinned in `deploy/postgres/migration-ledger.json`.
- The memory repository mirrors the same terminal-decision and idempotent-mark
  semantics.
- Documentation records the contract in
  `docs/architecture/simulation-concurrency.md`, links it from the HA,
  service, deployment, multi-host, API, and manual pages, and states that the
  two-replica simulation is supported only with this contract.
- Tests cover two concurrent decisions (one winner), concurrent marks (full
  set), a mixed mark/decision race (one consistent outcome), duplicate decisions
  (conflict), duplicate marks (same identity), a schema-level decision overwrite
  refusal, an injected failed mark transaction (no partial mark or status), and
  a reader during an uncommitted mark and decision (committed state only).

## Verification and evidence

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy -p quantrule-simulation-service -p quantrule-runtime --all-targets -- -D warnings` | pass |
| `cargo test -p quantrule-simulation-service` | 16 unit, 5 memory concurrency, 2 health, 17 simulation; pass |
| `cargo test -p quantrule-runtime --test migration_contract` | 5 passed |
| `QR_TEST_DATABASE_URL=$QR_TEST_DATABASE_URL cargo test -p quantrule-simulation-service --test concurrency -- --ignored --test-threads=1` | 7 passed against a disposable PostgreSQL 17 container |
| `tools/postgres_composed.sh` | all 8 targets passed, including 7 simulation concurrency cases; the bootstrap, owner/app roles, and cross-database refusal also passed |
| `cargo run --quiet -p quantrule-governance -- check --all` | `governance check ok` |
| `tools/gate.sh` | `gate passed: full with CARGO_TARGET_DIR=…/target/gate`; fmt, workspace clippy, workspace tests, workspace docs, and governance check all passed |

The Postgres cases used a disposable local container, and the composed script
ran the same target with its own disposable database; the real multiple-host HA
lab was not run for this log. The two-replica claim is therefore a contract and
a single-database concurrency result, not a real multi-host failure-matrix
result.

## Learning and corrections

- The existing code already locked the run row for marks, but the decision read
  used a different pooled connection and treated an identical retry as success.
  Moving the decision read under the existing lock was narrower than adding a
  client-supplied etag or a new version column.
- The mark upsert previously replaced `created_at` on conflict. Exact retries
  now preserve the stored creation time, so the retry is actually idempotent
  rather than merely returning the same identity.
- A decision trigger is a second guard, not the primary concurrency mechanism.
  The row lock prevents the race; the trigger keeps the decision terminal if a
  later code path or an operator bypasses the repository.
- The injected-failure test deliberately fails the status update after the mark
  insert. That proves the repository transaction rolls the mark back instead of
  leaving a mark on a draft run.
