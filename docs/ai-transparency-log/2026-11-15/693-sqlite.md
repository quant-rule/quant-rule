# AI transparency log — DIST-201 P1a SQLite runtime and gateway

- Date: 2026-11-15 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Requirements & Design + Execution Plan; issue #693, claim type
  Behaviour Claim; plan PR #810 (`DIST-200`) section 7, P1a.
- Intent: land the P1a spike for the embedded desktop profile: a SQLite adapter
  over the workspace `rusqlite` 0.42, a shared forward-only migration ledger,
  an explicit `server`/`desktop` config selector, and a SQLite implementation of
  the existing gateway `ProfileStore`. The gateway must start on SQLite, report
  `sqlite` at `/health`, and answer the same profile operations as the other
  stores.
- Boundary: `crates/quantrule-runtime/src/adapters/{mod.rs,sqlite.rs}`,
  `config.rs`, `lib.rs`; `crates/quantrule-gateway/src/{adapters,profile}`,
  `src/bin/qr_gateway.rs`; the gateway SQLite baseline and tests; `CHANGELOG.md`;
  this log. No data, evidence, or simulation repository rewrite; no queue table;
  no Postgres behavior change; no model, strategy, execution, or portfolio
  behavior change.

## Plan source and scope correction

The plan was read from PR #810's branch (`origin/arch/692-desktop-v1.1`) because
`main` was still at `6a714ba3` and did not contain it. The first draft of this
work had single-underscore prefixes (`catalog_datasets`), one ledger table per
schema, and all six SQLite baselines in the runtime. The plan requires
`service__table` prefixes, one shared `schema_migrations` table with a ledger
column, per-service baselines, and a P1a scope of runtime plus gateway. The
draft was corrected before the PR: the extra schema baselines and the queue table
were removed, and P1b-P1d will add them under their owning services.

The branch was then rebased onto `main` at `78acc6ae`, which contains #810 and
#811 (the P2a filesystem store). The plan text in `main` is identical to the
reviewed branch, and the full workspace gate was rerun after the rebase.

## Implemented

- `quantrule-runtime::adapters::sqlite` opens one file or an in-memory database
  with WAL, `foreign_keys`, a five-second busy timeout, full synchronous commits,
  and `STRICT` tables. Repository work runs on `tokio::task::spawn_blocking`
  behind one guarded connection through `SqliteDatabase::call`; `Database`
  itself stays the two-method liveness/backend trait.
- `apply_migrations_in` runs one service baseline under the shared
  `schema_migrations` table with a `ledger` column. The ledger, plan, SQL, and
  record share one `BEGIN IMMEDIATE` transaction. A second apply returns 0; a
  changed checksum, an unknown applied version, and a database ahead are refused
  by the existing pure `migrations::plan`; a failed plan rolls back the whole
  transaction.
- `Dialect::table` maps `gateway.profiles` to `gateway__profiles`; the SQLite
  type spellings are `TEXT` for `jsonb`/`timestamptz` and `INTEGER` for
  `boolean`.
- `Config` reads `QR_PROFILE=server|desktop` (default `server`) and
  `QR_DATA_DIR`; the desktop profile requires the data directory.
- The gateway's `SqliteProfileStore` implements the existing `ProfileStore`
  trait over `gateway__profiles`; the binary branches once on the profile,
  applies the gateway SQLite baseline, registers the `sqlite` health probe, and
  leaves the Postgres branch unchanged.
- The gateway SQLite baseline is
  `crates/quantrule-gateway/migrations/sqlite/0001_profiles.sql`, version 1 in
  the `gateway` ledger.

## Verification

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `cargo test -p quantrule-runtime` | 70 lib (including #694's filesystem tests) + 5 inventory + 1 boundary + 13 SQLite tests pass; the Postgres migration-lock test is ignored without `QR_TEST_DATABASE_URL` |
| `cargo test -p quantrule-gateway` | the full gateway suite passes in `cargo test --workspace`; the targeted P1a suites are 2 conformance passes (the Postgres case is ignored), 4 SQLite parity passes, and 1 SQLite profile pass |
| `cargo test --workspace` | passed |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | passed |
| `tools/gate.sh` | passed; the workspace gate and all 22 checkers passed with `CARGO_TARGET_DIR=.../target/gate` |
| Manual `QR_PROFILE=desktop` gateway run with `QR_DATA_DIR=/tmp/opencode/qr-693-desktop` and a dummy data URL | startup logged `gateway SQLite migrations applied applied=1`; `GET /health` returned `200` with `state:"healthy"` and `services:[{"name":"sqlite",...}]`; `quantrule.sqlite` was created (20,480 bytes). `GET /profiles` returned `503` because the route asks the deliberately unreachable data service for its counts; the profile store itself is covered by the conformance test. |

Not verified here: the opt-in Postgres profile conformance test, because no
`QR_TEST_DATABASE_URL` was available. It is the same conformance body and is
ignored in the fast tier. No full desktop fixture run exists yet; that needs
P1b-P1d and #694.

## Learning and rejected alternatives

- **Rejected — `sqlx-sqlite`.** The workspace already links
  `libsqlite3-sys` 0.40 through `rusqlite` 0.42; SQLx's SQLite driver requires a
  lower `libsqlite3-sys` and Cargo permits only one package to link the native
  library. The plan records the conflict; this change uses the workspace
  `rusqlite`.
- **Rejected — widening `Database` into a query abstraction.** The domain
  repository traits are the seam. `Database` remains a liveness/backend trait;
  the SQLite repository is a concrete adapter, and the shared dialect helper
  names a table and a type without building SQL.
- **Rejected — one ledger table per schema.** The plan replaces it with one
  `schema_migrations` table and a `ledger` column. The runner filters by ledger,
  so the physical table is shared while each service keeps its own version
  sequence and baseline.
- **Rejected — single-underscore prefixes.** The plan uses `service__table` so a
  table name may contain one underscore without ambiguity.
- **Deferred — data, evidence, simulation, conformance inventory, and prefix
  guard.** They are P1b-P1d and P1e. The queue tables are P2b (#694). This PR
  does not claim the full canonical-digest done-when.

## Remaining work for #693

- P1b: SQLite `DatasetCatalog` and `ConfigCatalog` plus the catalog baseline.
- P1c: SQLite `RunStore` and `HypothesisStore` plus the runs/research baseline;
  the sealed-row trigger must keep refusing update and delete.
- P1d: SQLite `SimulationStore` plus the simulation baseline; `FOR UPDATE`
  becomes a transaction that re-reads the row.
- P1e: the shared conformance/inventory suite, the schema inventory fixtures,
  `tools/check_schema_prefixes.py`, and the deployment documentation.
- The full canonical-digest parity closes in P3 with #694 (storage and queue) and
  #695 (packaging).
