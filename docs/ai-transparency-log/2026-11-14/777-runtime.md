# AI transparency log — B-P1 runtime and service-api extraction

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Execution Plan; issue #777 (P1 of #776), claim type ADR
  (Architecture Decision).
- Intent: extract the shared runtime and the internal contract from
  `quantrule-server` without changing a run, model, route, or artifact result,
  and fix the migration advisory lock that #772 observed releasing through the
  pool.
- Boundary: new `crates/quantrule-runtime/**` and
  `crates/quantrule-service-api/**`; workspace `Cargo.toml` and `Cargo.lock`;
  `crates/quantrule-server/Cargo.toml` and the moved `src/runtime/**` paths; the
  crate-boundary ADR; the parked #646 salvage map; `CHANGELOG.md`; this log. No
  shell, simulation client, app, Docker, Compose, Makefile, workflow, engine, or
  parked-worktree change. No later phase was implemented.

## Implemented

- `quantrule-runtime` owns `Config`, `HealthRegistry`, logging, `clock`, queue
  routing names, shutdown, the generic HTTP `serve`, the adapter traits,
  `PgDatabase`, the Postgres/S3/RabbitMQ adapters, the migration runner and plan,
  and a bytes-only `ServiceClient` contract with an in-memory fake. The crate
  depends on `quantrule-api` and its backend client libraries; it has no engine
  and no `quantrule-app` edge.
- `quantrule-service-api` owns the `quantrule.internal/v1` types: `NewRun` and
  `RunRecord`, claim/cancel outcomes, claim/computed/failed payloads,
  `RunManifest` and `canonical_digest`, `ConfigBundle`, `DatasetObject`, and
  `ModelBundleManifest`, with round-trip and strict-field tests.
- `quantrule-server` keeps its three binaries, routes, worker, evidence, and
  tests. The old `runtime` module is deleted. `server.rs` and `static_files.rs`
  stay in the server because they are the gateway's surface; the Postgres domain
  repositories move to `quantrule-server::adapters` until P3/P5.
- The migration runner takes one `PoolConnection` for the advisory lock, the
  ledger read, every migration transaction, and the unlock. If the unlock fails
  while the session is alive, the session is closed instead of returning a locked
  connection to the pool. `PgDatabase` exposes the pool; the service keeps its
  own migration list in `src/migrations.rs`.
- `crates/quantrule-runtime/tests/migration_lock.rs` starts two migrators
  against one database under a 10 s budget, checks one ledger row per migration,
  and proves a third session can take the same lock immediately afterwards. It
  is `#[ignore]` in the fast gate and needs `QR_TEST_DATABASE_URL`.
- The adapter boundary test moved to the runtime crate; the server's remaining
  boundary test allows `sqlx` only under `src/adapters/postgres`.

## Verification

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `cargo test --workspace` | passed; the composed lock test is the new ignored case |
| `cargo tree -p quantrule-runtime --depth 1` | `quantrule-api` only from the workspace; no `quantrule-data`, `-model`, `-backtest`, `-app` |
| `cargo test -p quantrule-runtime --test boundary` | passed |
| `cargo test -p quantrule-service-api` | 17 passed |
| Migration lock, fixed runner, `postgres:17` on `127.0.0.1:55432` | `1 passed`, 0.61 s |
| Migration lock, previous pool-based runner (temporary local revert, restored) | failed at the 10 s budget: `two migrators deadlocked: the advisory lock outlived its session` |
| `docker stop qr-777-locktest` | container removed; no volume remained |
| `python3 tools/check_file_size.py` | passed |
| `python3 tools/check_structure.py` | passed |
| `python3 tools/check_links.py` | passed; 935 Markdown files |
| `python3 tools/check_doc_length.py` | passed; 34 files within cap |
| `python3 tools/check_execution_plans.py` | passed |
| `python3 tools/check_changelog.py` | passed |
| `tools/gate.sh` | passed; see the PR verification row |

The fail-before run used a temporary edit of `adapters/postgres.rs` that
restored the pool-based lock and unlock; the fixed file was restored from a copy
before the passing run. No test was weakened.

## Learning and rejected alternatives

- The old unlock could return `false` on another pooled connection while the
  first session kept the lock idle; the next migrator then waited forever. The
  new test's postcondition (`pg_try_advisory_lock` from a third session) catches
  a leaked lock even when the two-migrator race happens to avoid the timeout.
- `RunRecord` moved with its `summary`/`detail` projections. Its `queued`
  constructor had to become public for the server's in-memory store; it is the
  documented "new record starts queued" constructor, not a second schema.
- The domain Postgres repositories could not move to the runtime: they name
  `crate::data`, `crate::runs`, `crate::profile`, `crate::configs`, and
  `crate::research`. Moving them would have made the runtime depend on the
  server. They stay in `quantrule-server::adapters` and travel with their
  schemas in P3/P5.
- Rejected: a concrete hyper client in P1. The plan places the transport with
  the first proxied route (P3); P1 freezes the bytes-only seam and the fake, so
  no route or timeout policy is invented early.
- Rejected: a concurrent pool-pressure task in the composed test. The restored
  old runner already fails the test without it, and a busy loop would add a
  timing-dependent flake.
- Recorded, not fixed: D1, `PgDatasetCatalog` decodes `quality` and
  `created_at` in the wrong order (P3); D2, `0013_run_attempts.sql` is not
  registered in the migration list (P5). Both are in the salvage map with the
  #671 evidence. P1 keeps the current behavior.
- Plan §5.8 maps the parked `auth.unauthorized` error code to P1. The P1 issue
  excludes the bearer seam, so the code is mapped with its middleware to P8 and
  nothing from the parked tree was copied.

## Not verified here

The composed tier for the six-service stack, the later service crates, and any
route that P3-P8 own. This log does not claim them. The parked #646 worktree was
read read-only; it was not changed, merged, or cited as landed.
