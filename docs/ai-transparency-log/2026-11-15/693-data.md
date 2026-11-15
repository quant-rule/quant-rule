# AI transparency log — DIST-201 P1b data catalog and config store on SQLite

- Date: 2026-11-15 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Requirements & Design + Execution Plan; issue #693, claim type
  Behaviour Claim; plan `docs/execution-plans/2026-11-15/692-desktop-v1.1.md`
  section 7, P1b.
- Intent: implement the desktop profile's data-service persistence: SQLite
  implementations of the existing `DatasetCatalog` and `ConfigCatalog` traits,
  the catalog SQLite baseline under the shared ledger, and the `qr-data` binary
  branch that opens `quantrule.sqlite` when `QR_PROFILE=desktop`.
- Boundary: `crates/quantrule-data-service/src/{sqlite.rs,rows.rs,postgres.rs,
  main.rs,lib.rs,migrations.rs}`, `migrations/sqlite/0001_catalog.sql`,
  `crates/quantrule-data-service/{Cargo.toml,tests/**}`, `CHANGELOG.md`, and
  this log. No gateway, model, evidence, simulation, engine, route-contract,
  canonical-digest, or Postgres behavior change. No new crate version.
- Environment: worktree `/home/zantgo/Proyectos/quant-rule--693-data`, branch
  `feat/693-data`, base `3772ecdf` (P1a SQLite runtime and gateway, #694 P2a
  filesystem CAS, and the v1.1 plan merged), rebased onto `aafe6b0e` after PR
  #813 (P3 packaging) merged. The data-service suite and the gate were rerun
  after the rebase.

## What was read

- `AGENTS.md` sections 2, 3, 4, 7, 8, 9, 11, 12, 13, and 14.
- Issue #693 and the v1.1 plan P1b row, decisions D1-D4, and section 5.2-5.4.
- `crates/quantrule-runtime/src/adapters/sqlite.rs`, `adapters/mod.rs`,
  `migrations.rs`, and `config.rs` (the P1a adapter, ledger, dialect, and
  profile selector).
- The P1a gateway pattern:
  `crates/quantrule-gateway/src/adapters/sqlite/mod.rs`,
  `src/bin/qr_gateway.rs`, and `tests/{profile_conformance,sqlite_parity,sqlite_profile}.rs`.
- `crates/quantrule-data-service/src/{postgres.rs,lib.rs,catalog.rs,configs/**,
  routes.rs,state.rs,migrations.rs,main.rs}`, the five Postgres catalog
  migrations, and the existing data-service tests including `d1.rs` and the
  route fixtures.

## Implemented (P1b)

- `crates/quantrule-data-service/src/sqlite.rs` adds `SqliteDatasetCatalog` and
  `SqliteConfigStore` behind the existing traits. Every operation runs on the
  runtime's blocking pool through `SqliteDatabase::call`; the route shape is
  unchanged. A config upsert writes the shared bundle and the profile-scoped row
  in one `BEGIN IMMEDIATE` transaction.
- `crates/quantrule-data-service/src/rows.rs` holds the select-order tuple and
  its decoder once, shared by the Postgres and SQLite repositories. This removes
  the copy of `dataset_record`, `dataset_count`, `config_origin`, and
  `config_record` that a second backend would otherwise duplicate.
- `migrations/sqlite/0001_catalog.sql` is the final logical baseline:
  `catalog__datasets`, `catalog__configs`, and `catalog__config_bundles`, with
  `profile_id` on both profile-scoped relations, JSON as `TEXT` guarded by
  `json_valid`, booleans as `INTEGER` guarded by `IN (0, 1)`, RFC 3339 UTC
  `TEXT` instants, and the Postgres checks (`candles >= 0`, `bytes >= 0`,
  `first_bar <= last_bar`, the origin set, and a valid row has a hash).
- `migrations.rs` adds the `catalog` SQLite ledger at version 1 and keeps the
  five Postgres versions untouched.
- `main.rs` branches once: the desktop profile opens
  `<QR_DATA_DIR>/quantrule.sqlite`, applies the catalog SQLite baseline, builds
  both SQLite stores, and registers `sqlite` health; the server profile keeps
  the Postgres branch and the five Postgres migrations unchanged.
- `tests/catalog_conformance.rs` runs one store body against the memory,
  SQLite, and (opt-in) Postgres stores: dataset insert/duplicate/get/list/count,
  profile scope, shared object key, quality JSON, config upsert/get/list/order,
  shared bundle, and profile independence.
- `tests/sqlite_migrations.rs` proves the first apply records one `catalog`
  ledger row, the second apply returns 0, a changed body is refused with
  `ChecksumMismatch`, an unknown applied version is refused, a database ahead of
  the binary is refused, and two ledgers stay independent in one file.
- `tests/sqlite_catalog.rs` asserts the relation names, column order and mapped
  types, the stored wire digest and bare hash, TEXT JSON, `INTEGER` booleans,
  the first-seen config instant, a malformed stored hash as a decode error, and
  that `json_valid` and the boolean check refuse a bad row.
- `tests/sqlite_routes.rs` builds the desktop state and exercises the real
  router: `/health` names `sqlite`, a dataset upload is `201` and a duplicate
  lookup `200`, profile scoping holds, the internal dataset and count routes
  answer, a config upload is `201`, the internal config bundle returns its five
  files, the fixture scan still lists, and a tolerant upload round-trips its
  quality report through SQLite.

## Verification

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed; no warning, exit 0 |
| `cargo test -p quantrule-data-service` | 123 passed, 2 ignored (the opt-in Postgres `d1` and catalog conformance cases); the new SQLite suites pass 6 catalog-shape, 5 migration, 3 route, and 2 conformance tests, with the catalog Postgres case ignored |
| `cargo test --workspace` | passed; 2063 passed, 0 failed, 12 ignored, exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | passed; exit 0 |
| `CARGO_BUILD_JOBS=2 tools/gate.sh` | passed on the rebased tree; the workspace gate and every installed checker reported ok, the script printed `workspace gate passed with CARGO_TARGET_DIR=.../target/gate`, exit 0 |
| `QR_PROFILE=desktop qr-data` smoke | first start logged `catalog SQLite migrations applied applied=1`; two restarts logged `applied=0`; `/health` reported `sqlite` healthy and `object-store` down (the S3-compatible branch is unchanged until P2c); `/catalog/configs` scanned three fixture configs into `catalog__configs` with `INTEGER` booleans and valid JSON bundles; the internal dataset count and dataset list were empty; a restart kept the three config rows and the one `catalog` ledger row; `sqlite3` showed `catalog__datasets`, `catalog__configs`, `catalog__config_bundles`, and `schema_migrations` |

## Learning and rejected alternatives

- **Extracting the row decoder was kept.** The tuple order is the contract that
  caused the D1 regression; copying it into a second repository would have made
  two places that must agree. The Postgres repository only loses its private
  copies; its SQL, types, bindings, and transaction stay identical.
- **A shared query builder was rejected.** The plan keeps `Dialect` to a table
  name, a placeholder, and the type/now spellings; the SQLite queries differ
  where SQLite forces it (no `::jsonb`, no `to_char`, `?` placeholders). A
  query abstraction would be a larger seam than the two statements need.
- **A separate SQLite schema inventory was deferred.** P1e owns the checked-in
  inventory and the Postgres composed comparison; P1b asserts the SQLite shape
  directly and runs the same store body on Postgres when a disposable database
  is available.
- **Wiring the filesystem object store into `qr-data` was not done.** P1b owns
  the database branch only; the desktop object store and queue are #694 and the
  service wiring is P2c. The desktop smoke therefore reports the SQLite probe
  healthy and leaves the S3-compatible probe as the unchanged branch until that
  wiring lands.

## Remaining work for #693

- P1c: SQLite `RunStore` and `HypothesisStore` plus the runs/research baseline.
- P1d: SQLite `SimulationStore` plus the simulation baseline.
- P1e: the shared inventory and Postgres composed comparison,
  `tools/check_schema_prefixes.py`, and the deployment documentation.
- The canonical-digest parity closes in P3 with #694 and #695.
