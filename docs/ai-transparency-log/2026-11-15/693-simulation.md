# AI transparency log — DIST-201 P1d SQLite simulation service

- Date: 2026-11-15 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Requirements & Design + Execution Plan; issue #693 (DIST-201),
  claim type Behaviour Claim; the v1.1 plan (PR #810) section 7, P1d.
- Intent: land the simulation tier of the embedded desktop profile: a SQLite
  implementation of the existing `SimulationStore` trait and the simulation
  SQLite baseline, so the desktop profile can run the research plane on the
  embedded database without a route, contract, or engine change.
- Boundary: `crates/quantrule-simulation-service/src/sqlite/**`,
  `migrations/sqlite/0001_simulation.sql`, `src/migrations.rs`, `src/lib.rs`,
  `src/main.rs`, `Cargo.toml`, `tests/sqlite_{conformance,migrations,profile}.rs`,
  `CHANGELOG.md`, and this log. No gateway, data, model, evidence, runtime,
  simulation-domain, API, or run-contract change; no Postgres behavior change.
  The object-store and queue wiring for the desktop profile remains P2c.
- Environment: worktree `/home/zantgo/Proyectos/quant-rule--693-simulation`,
  branch `feat/693-simulation`, base `3772ecdf` (P1a #693 and P2a #694 merged).

## What was read

- `AGENTS.md` sections 2, 3, 4, 7, 8, 9, 11, 12, 13, and 14.
- Issue #693 and the v1.1 plan's P1d/P1e rows and verification rows V14–V19.
- The runtime SQLite adapter, its shared `schema_migrations` runner, the
  `Schema`/`Dialect` mapping, and the migration `plan` refusal rules.
- The gateway P1a pattern: `SqliteProfileStore`, `migrations/sqlite/0001_profiles.sql`,
  the SQLite conformance and parity tests, and the `qr_gateway` profile branch.
- The simulation service: `store.rs`, `postgres.rs`, `records.rs`, the eleven
  routes, `research.rs`'s CSV mirror, `migrations/0014_simulation.sql`,
  `main.rs`, and the in-memory conformance store in `tests/common/mod.rs`.

## Implemented

- `migrations/sqlite/0001_simulation.sql` is the final logical shape at version
  1 under the shared ledger's `simulation` name. It writes `simulation__runs`
  and `simulation__marks` with `TEXT` JSON, RFC 3339 UTC `TEXT` instants, and
  `INTEGER` counts. The mark identity `(sim_id, origin, chart, bar, action,
  side)` is unique, the mark cascades with its run, and a decision, its note,
  and its time exist exactly when the run is decided. Triggers close marking
  after a decision and freeze a decided run's status and decision while still
  allowing its check result to be refreshed.
- `src/sqlite/` implements the existing `SimulationStore` over
  `SqliteDatabase`. `src/sqlite/rows.rs` decodes the run and mark projections;
  `src/sqlite/mod.rs` holds the store. Every statement names only
  `simulation__runs` or `simulation__marks` and filters by the injected
  profile. No route or use case changed.
- SQLite has no `FOR UPDATE`. Every mark and decision mutation opens an immediate
  transaction, which takes the write lock before the first read, and re-reads
  the run row inside it. The public outcomes are the Postgres ones:
  `StoreError::Locked` after a decision, `StoreError::NotFound` for an unknown
  run, and the same status transitions (`draft` to `marking`, `reviewed` unless
  `checked`, `checked` unless `decided`, `decided` final).
- `qr-simulation` branches once on `QR_PROFILE=desktop`: it opens
  `QR_DATA_DIR/quantrule.sqlite`, applies the simulation baseline to the
  `simulation` ledger, uses the SQLite store, and registers `sqlite` health. The
  server profile keeps the Postgres store and `postgres` health. The binary is
  `src/main.rs` in this crate, so the branch lives there; no `src/bin/` move.
- Tests: a shared store conformance suite (SQLite always, Postgres opt-in and
  ignored), a router suite over SQLite (health, the research loop, profile
  isolation, a file restart, and the CSV mirror), and a migration suite (apply
  twice, the schema against the Postgres columns, changed-body/unknown/ahead
  refusals, a failed-baseline rollback, and the trigger and identity rules).

## Transaction and mark evidence

- The conformance suite proves the mark upsert keeps one row per identity, the
  oracle replacement is idempotent, the check is idempotent, and a second
  different decision is `Locked` while the same decision returns its original
  time. It then proves every mark mutation (`put_mark`, `delete_mark`,
  `replace_oracle`, `save_windows`) is `Locked` after the decision and a check
  refresh still succeeds.
- `tests/sqlite_migrations.rs` proves the same rules at the schema level: an
  insert, update, or delete on `simulation__marks` after a decision is refused
  by a trigger, a rewritten decision is refused, a check refresh is allowed,
  a duplicate mark identity is refused by the unique constraint, and a marking
  run that carries a decision is refused by the check.
- `tests/sqlite_profile.rs` proves the router sees one mark after a restart over
  the same file and that the second open applies zero migrations, and that a
  profile reading another profile's run gets `404`/an empty list.

## Verification

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | passed after `cargo fmt --all` |
| `cargo clippy -p quantrule-simulation-service --all-targets -- -D warnings` | passed |
| `cargo clippy --workspace --all-targets -j 2 -- -D warnings` | passed |
| `cargo test -p quantrule-simulation-service` | passed: 20 lib, 2 health, 13 simulation, 1 SQLite conformance (the Postgres case is ignored), 9 SQLite migrations, 5 SQLite profile; 0 failed |
| `cargo test --workspace --no-fail-fast` (inside `tools/gate.sh`) | passed; the workspace suite ran with `CARGO_TARGET_DIR=target/gate` |
| `tools/gate.sh` | passed: workspace fmt, clippy, tests, rustdoc, and all 22 checkers; printed `workspace gate passed with CARGO_TARGET_DIR=.../target/gate` |
| `QR_PROFILE=desktop QR_DATA_DIR=... qr-simulation` smoke | startup logged `simulation SQLite migrations applied applied=1`; `/health` reported `sqlite` healthy; `quantrule.sqlite` carried `schema_migrations`, `simulation__runs`, `simulation__marks`, and the ledger row `simulation|1|simulation`. `object-store` reported down because this branch is database-only and the route still builds the S3 store; P2c wires the filesystem store. |

## Learning and rejected alternatives

- **Kept the binary at `src/main.rs`.** The brief names `qr_simulation.rs`, but
  this crate's binary is `qr-simulation` at `src/main.rs`; moving it to
  `src/bin/` would be an unrelated structural change. The profile branch is in
  the binary that ships.
- **Rejected a string-encoded store error through `SqliteDatabase::call`.** The
  runtime collapses a closure error into one `DatabaseError`, which would lose
  `Locked` and `NotFound`. The transaction body returns
  `Result<Result<T, StoreError>, rusqlite::Error>`, so the outer call reports a
  backend failure and the inner result keeps its variant.
- **Rejected a read after commit for mutations.** The Postgres repository reads
  the row through the pool after committing. The SQLite repository reads it
  inside the immediate transaction, which sees its own writes and avoids a
  window where another writer could change the row between commit and read.
- **Rejected store-only enforcement.** The triggers are defense in depth: the
  repository returns `Locked` before a direct write would ever reach them, and a
  writer that bypasses the repository cannot reopen a decided run.
- **Index names use the double prefix.** `simulation__runs_profile_idx` and
  `simulation__marks_run_idx` keep one prefix spelling in the file, so the P1e
  prefix guard has nothing to exempt.
- **Deferred the desktop object store and queue.** The routes still build their
  S3 object store in this change, so a desktop smoke reports `sqlite` healthy
  and `object-store` down. P2c owns the filesystem object store and the
  in-process queue wiring; this change owns the database repository.

## Remaining work for #693

- P1e: the shared cross-service conformance suite, the SQLite and Postgres
  schema inventory fixtures, `tools/check_schema_prefixes.py`, and the
  deployment documentation. The simulation conformance suite here covers the
  store; P1e extends it across the six services.
- P2c: desktop object-store and queue wiring and the end-to-end fixture run.
- The opt-in Postgres conformance test was not run in the fast tier; it needs a
  disposable `QR_TEST_DATABASE_URL`.
