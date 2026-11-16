# AI Transparency Log — Stage API, Internal Contract, and Schema

- Date: 2026-11-17 (UTC); work item recorded 2026-11-16
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #850, `850-stage-api-schema`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)
- ADR: [`docs/adr/2026-11-16/01-stage-execution-and-evidence-boundary.md`](../../adr/2026-11-16/01-stage-execution-and-evidence-boundary.md)

## Intent and boundary

Add the stage resource contract and durable stage storage: the
`quantrule.api/v1` stage types, the `quantrule.internal/v1` stage record and
`quantrule.stage-manifest/v1`, the evidence `stage` schema on Postgres and
SQLite, and a profile-scoped content-addressed stage store. The work is
diagnostic only: a stage has no canonical run digest and is never evidence.

The AI implemented the contract, migrations, store, and tests. It did not add
HTTP routes, queue routes, worker dispatch, the gateway proxy, CLI, or GUI;
those belong to #851. It did not change the existing run lifecycle, the model,
strategy, execution, or portfolio contracts, or any dependency. The stage store
is not wired into the evidence service binary in this change.

## Verification and evidence

- `cargo fmt --all -- --check` — pass.
- `cargo clippy -p quantrule-api -p quantrule-service-api -p quantrule-evidence-service --all-targets -- -D warnings` — pass.
- `cargo test -p quantrule-api -p quantrule-service-api -p quantrule-evidence-service` — pass. The API crate ran 62 tests, the internal-contract crate 38, and the evidence crate ran its unit and integration suites, including the SQLite stage conformance and the stage/run separation test.
- `cargo test --workspace --no-fail-fast` — pass; every test target reported `0 failed`.
- `cargo test -p quantrule-evidence-service --test sqlite_parity` — 6 passed; this is the SQLite migration/parity suite the issue calls the migration test.
- `QR_TEST_DATABASE_URL` against a disposable `postgres:17` container:
  `cargo test -p quantrule-evidence-service --test sqlite_conformance -- --ignored`
  and `cargo test -p quantrule-evidence-service --test stage_conformance -- --ignored`
  both passed. The `runs.schema_migrations` ledger recorded the applied `stage`
  migration as version 14, and `stage.runs`, `stage.submissions`, and
  `stage.artifacts` existed.
- `python3 tools/check_schema_prefixes.py` — pass: 7 ledger rows, 16 tables,
  5 baselines.
- `python3 tools/check_links.py` and `python3 tools/check_changelog.py` — pass.
- `bash tools/checkers.sh` — all 24 checkers and their tests passed.

The Postgres store is exercised by the ignored conformance test against a live
database; it is not exercised by the fast gate. The stage store is not yet
reachable from a route, so no end-to-end HTTP evidence exists in this change.

## Learning and corrections

- SQLx 0.9 refuses a dynamically composed SQL string through `SqlSafeStr`. The
  first Postgres `SELECT` built its column list with `format!`; it was replaced
  with two literal profile-scoped and unscoped queries. The store never accepts
  a client-supplied table or column.
- The schema-parity suite reads a table body line by line and treats a line that
  does not start with `PRIMARY KEY`, `UNIQUE`, `CHECK`, `CONSTRAINT`,
  `FOREIGN KEY`, or `REFERENCES` as a column definition. A multi-line table
  `CHECK` therefore looked like a column. The stage `CHECK` is written on one
  line, and the parity suite now includes the stage tables.
- The schema-prefix checker's own fixture wrote every table of a crate into
  every store. That was harmless while evidence had one store; it broke when the
  stage store became a second evidence store. The fixture now writes each store
  its own tables and each baseline its own table set. The real checker and its
  post-change inventory pass.
- The first stage-identity test expected a change to `data_version` alone to
  change the stage id. That was rejected: the identity inputs are the kind, the
  input reference, the config hash, the model version, and the image digest. A
  model stage's data version is the input digest, and a later stage's data
  version is resolved from its frozen input. The test now changes each actual
  identity input.
- The stage retry budget reuses the control plane's `MAX_ATTEMPTS` rather than
  adding a fourth constant. A stage and a run fail over the same three
  deliveries.
- The SQLite stage tables are an additive version-2 migration. The applied
  version-1 baseline is not rewritten, so an existing desktop database keeps its
  run and research rows.
- `Schema::Stage` was added to the runtime mapping so the SQLite store spells the
  stage tables through the same `Schema` enum as every other service prefix.

## Residual risk

- Postgres coverage is conditional on `QR_TEST_DATABASE_URL`; the fast gate
  covers SQLite and memory only.
- The stage contract is not yet reachable from a client. #851 must reuse
  `derive_stage_run_id`, `NewStageRun::new`, and `StageManifest::new` rather than
  re-deriving the identity, or the store will reject the row.
