# AI transparency log — DIST-201 P1c SQLite evidence runs and research

- Date: 2026-11-15 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Requirements & Design + Execution Plan; issue #693 (DIST-201,
  P1c of #692); claim type Behaviour Claim; plan
  `docs/execution-plans/2026-11-15/692-desktop-v1.1.md` (DIST-200), section 7.
- Intent: implement the evidence service's embedded SQLite profile behind the
  existing `RunStore` and `HypothesisStore` traits: the runs/research baseline,
  the run state machine with `BEGIN IMMEDIATE` and a re-read instead of
  `FOR UPDATE`, the sealed-hypothesis immutability rule, the `qr-evidence` profile
  branch, and parity/conformance tests. Postgres remains the default.
- Boundary: `crates/quantrule-evidence-service/src/{lib.rs,main.rs,migrations.rs}`,
  new `src/sqlite/{mod.rs,runs.rs,research.rs}`, new
  `migrations/sqlite/0001_runs_research.sql`, new tests and fixture,
  `CHANGELOG.md`, and this log. No route, contract, digest, engine, gateway,
  data, model, simulation, or desktop-shell change.
- Environment: worktree `/home/zantgo/Proyectos/quant-rule--693-evidence`, branch
  `feat/693-evidence`, base `3772ecdf` (P1a #812 and P2a #811 merged).

## What was read

- `AGENTS.md` sections 2, 3, 4, 7, 8, 9, 11, 12, 13, and 14.
- Issue #693 and the v1.1 plan's P1c row, section 5.1–5.4 (backend selection,
  table prefixes, TEXT/JSON/time mapping, migration ownership), and verification
  rows V13, V15, V17, V18, and V19.
- `crates/quantrule-runtime/src/adapters/sqlite.rs`, `adapters/mod.rs`, and
  `migrations.rs`; the P1a gateway `SqliteProfileStore`, `qr_gateway.rs`, SQLite
  baseline, and its parity/conformance tests.
- `crates/quantrule-evidence-service/src/postgres.rs`, `runs/store.rs`,
  `research/store.rs`, `evidence/finalize.rs`, `migrations.rs`, `main.rs`,
  `state.rs`, and the run/hypothesis routes and tests.
- `crates/quantrule-service-api/src/run.rs` for `RunRecord`, `NewRun`, the claim,
  cancel, and submit outcomes, `MAX_ATTEMPTS`, and `derive_run_id`.
- The Postgres migration history `0002`, `0004`, `0005`, `0008`, `0009`, and
  `0013`, which the SQLite baseline is compared against.

## Implemented

- `migrations/sqlite/0001_runs_research.sql` is the final logical shape at
  version 1 under the `runs` ledger: `runs__runs`, `runs__stages`,
  `runs__artifacts`, `runs__submissions`, and `research__hypotheses`, all
  `STRICT`. JSON documents and instants are `TEXT`; booleans, if a later column
  needs one, would be `INTEGER`; `runs__artifacts.bytes` stays an `INTEGER` with
  a non-negative check. The baseline keeps the run status index, the three
  cascade foreign keys, and the sealed-row rule.
- `src/sqlite/runs.rs` implements every `RunStore` method over the SQLite
  database: submit, list, get, cancel, get_any, claim, computed, complete, and
  fail. The row decode mirrors the Postgres store's fallbacks (`unknown-config`,
  `unknown`, `unknown-image`) and its rejections of a malformed id, status,
  stage, digest, data version, artifact hash, or negative count.
- `src/sqlite/research.rs` implements `HypothesisStore` insert and load. The
  whole sealed record is stored as JSON `TEXT`; `costs` and `split` are separate
  JSON `TEXT` columns for inspection. No update or delete statement exists in
  the repository.
- `src/migrations.rs` gains `sqlite()`, one baseline at version 1 in the `runs`
  ledger, and keeps the Postgres list unchanged.
- `src/main.rs` branches once on `QR_PROFILE=desktop`: it opens
  `<QR_DATA_DIR>/quantrule.sqlite`, applies the SQLite baseline, uses the SQLite
  run and hypothesis stores, and registers the `sqlite` health probe. The server
  branch is the previous Postgres path with the import renamed. The object store
  and broker still use their configured adapters until the storage and queue
  phases (P2b/P2c) wire the desktop profile end to end.
- `tests/sqlite_conformance.rs` is one store-contract body run against SQLite and,
  opt-in, Postgres. `tests/sqlite_parity.rs` derives the released Postgres table
  shape from the migration files and compares every SQLite column in position
  and type, then writes one representative run row and reads it through the raw
  table and `SqliteRunStore`. `tests/sqlite_transactions.rs` opens two
  connections to one file and proves the claim and reclaim behaviour.

## Transaction and state-machine evidence

The evidence Postgres store does not use `SELECT ... FOR UPDATE`; its claim uses
conditional `UPDATE ... WHERE status = 'queued'` and its decision paths use a
`status = 'running'` predicate. The SQLite implementation nonetheless uses the
dialect's lock-then-decide shape the plan requires: every write opens
`BEGIN IMMEDIATE`, loads the run inside that transaction, decides from that
row, writes, reloads, and commits. The state machine and attempt semantics are
the memory/Postgres ones:

- A queued run becomes `running` with `attempts = 1`.
- A live second delivery is `Running` and does not change the count.
- A redelivery of an uncomputed running run reclaims while
  `attempts < MAX_ATTEMPTS`; at the budget it returns `Exhausted`.
- A computed run is `Running` for the same delivery and is not executed twice.
- Only a running run can succeed or fail; a late complete or fail returns the
  current record unchanged.
- Cancel checks `is_cancellable` and writes inside one immediate transaction, so
  a worker that completed first yields `Conflict` rather than overwriting a
  `succeeded` row.
- A consumer killed after a claim leaves a `running` row with no digest; the next
  redelivery reclaims it, and the attempt count and terminal state survive
  closing and reopening the file.

The two-connection test `two_live_deliveries_claim_a_queued_run_once` asserts
exactly one `Claimed` and one `Running` with stored `attempts = 1`, which is the
observable difference between an atomic read-decide-write and two independent
reads. `a_killed_consumer_is_reclaimed_and_the_state_survives_reopen` asserts the
reclaim reaches attempt 2, the computed digest blocks a second execution, and a
reopened file still shows `succeeded`, attempts 2, the digest, and four done
stages.

## Verification

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo test -p quantrule-evidence-service` | passed: 75 passed, 0 failed, 1 ignored (the opt-in Postgres conformance); 27 lib, 11 existing run tests, 7 hypothesis tests, 18 evidence tests, 3 SQLite conformance, 6 SQLite parity, 2 SQLite transactions, 2 SQLite finalize, 0 doc tests |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed; `Finished` with no warning or error |
| `cargo test --workspace` | passed: 2061 tests, 0 failed, 12 ignored across 111 test-result groups, including the 2 SQLite finalize tests |
| `tools/gate.sh` | passed: fmt, clippy, the workspace tests above, `cargo doc --workspace --no-deps` with `RUSTDOCFLAGS="-D warnings"`, and all 22 checkers; `workspace gate passed with CARGO_TARGET_DIR=.../target/gate` |
| `QR_PROFILE=desktop` `qr-evidence` smoke | applied 1 migration and created `quantrule.sqlite` under `QR_DATA_DIR`; the five tables and the `runs` ledger row (`1`, `runs-research`) are present, and a direct `DELETE` of a sealed hypothesis is refused with `a sealed hypothesis is immutable`; startup then stops at `Queue("IO error: Connection refused")` because no broker is configured, which is the P2b/P2c gap and not a database failure |

## Learning and rejected alternatives

- **Corrected — one trigger per event.** The plan and the Postgres migration
  spell the sealed-row rule as `BEFORE UPDATE OR DELETE`. SQLite accepts only one
  event per trigger, so the first baseline failed with `near "OR": syntax error`.
  The SQLite baseline creates two triggers, `..._immutable_update` and
  `..._immutable_delete`, each raising the same abort. Both refusals are tested.
- **Rejected — a shared SQL builder or a widened `Database` trait.** The domain
  traits are the seam, and the plan's option B (per-dialect repositories) keeps
  each dialect idiomatic. The stores share no statement assembly beyond the
  table constants; decoding is written against the same contract but not hidden
  behind a dynamic query layer.
- **Rejected — upsert for a hypothesis insert.** A plain `INSERT` preserves the
  Postgres duplicate-fingerprint error and the immutability rule; an upsert would
  turn a second seal into an edit.
- **Recorded residual — timestamp normalization.** SQLite stores the RFC 3339
  string the caller supplies. The product route supplies `clock::now_utc()` at
  second precision, which is the exact shape the Postgres `to_char` projection
  returns, so the route path is parity. A direct store call with fractional
  seconds or a non-UTC offset would be normalized by Postgres and stored
  verbatim by SQLite. P1e's time-mapping row (V15) owns making that explicit.
- **Recorded residual — the sealed JSON columns.** `costs` and `split` are
  `TEXT` guarded by `json_valid`. The store reads the whole sealed record back
  from `body`, never echoing those columns, so the API record is identical; the
  split columns remain an inspection aid as in Postgres.

## Remaining work for #693

- P1d: the SQLite `SimulationStore` and its baseline.
- P1e: the shared conformance/inventory suite, schema inventory fixtures,
  `tools/check_schema_prefixes.py`, and the deployment documentation.
- The opt-in Postgres conformance case was not run because no
  `QR_TEST_DATABASE_URL` was available. It is the same body and is ignored in the
  fast tier.
- A full desktop fixture run and canonical-digest parity need P2b/P2c and P3;
  this change does not claim the issue's digest done-when.
