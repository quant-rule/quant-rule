# AI transparency log — B-P2 six-binary milestone M1

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Execution Plan; issue #780 (P2 of #776), claim type ADR
  (Architecture Decision).
- Intent: add `qr-data`, `qr-model`, and `qr-simulation` so the six-binary
  inventory is true, while every existing gateway route and run behavior stays
  unchanged.
- Boundary: new `crates/quantrule-data-service/**`,
  `crates/quantrule-model-service/**`, and
  `crates/quantrule-simulation-service/**` (manifest, health/version router,
  entry point, tests); `crates/quantrule-runtime/tests/binary_inventory.rs`;
  workspace `Cargo.toml` and `Cargo.lock`; the P2 rows of the execution plan;
  `CHANGELOG.md`; this log. No route, handler, worker, shell, app, engine,
  Docker, Compose, Makefile, CI, or port-parity change. No later phase was
  implemented.

## Implemented

- Three service crates exist as M1 skeletons. Each exposes only `/health` and
  `/version`, uses `quantrule_runtime::Config`, `HealthRegistry`, `logging::init`,
  and `serve`, and has no engine or `quantrule-app` dependency.
  `qr-data` claims no catalog or dataset route (`P3`), `qr-model` claims no model
  route (`P4`), and `qr-simulation` claims no research endpoint (`P7`).
- The binary targets are explicit: `qr-data` in `quantrule-data-service`,
  `qr-model` in `quantrule-model-service`, and `qr-simulation` in
  `quantrule-simulation-service`. The three existing `qr-*` binaries stay in
  `quantrule-server`.
- `crates/quantrule-runtime/tests/binary_inventory.rs` reads the workspace
  member list, each service package manifest, and the `src/main.rs` and
  `src/bin/*.rs` entry points. It asserts the observed service-binary set equals
  the six names and has unit cases for a removed binary and an extra binary.
- The guard scopes to `quantrule-server` and the `*-service` packages. The
  workspace also builds the `quantrule` shell client binary from
  `quantrule-shell`; that client is outside the service inventory, and B may not
  edit the shell. The first guard run failed on `quantrule` for this reason; the
  package filter and the test documentation now state the boundary instead of
  renaming or hiding the client.
- Each new crate has a two-test `tests/health.rs` that exercises the router with
  `tower::ServiceExt::oneshot`, so no socket is bound and no dependency is
  started. The tests decode the real `HealthReport` and `VersionInfo` contracts.
- No dependency version was added. The crates use `axum`, `quantrule-api`,
  `quantrule-runtime`, and `tokio`; tests use `serde_json` and `tower`. All are
  already in `Cargo.lock`.

## Verification

| Command | Observed |
|---|---|
| `cargo build --workspace --bins` | passed; `target/debug/qr-{gateway,data,model,backtest,evidence,simulation}` exist; the client `quantrule` also builds |
| `cargo test -p quantrule-runtime --test binary_inventory` | 3 passed |
| Temporary removal of the `qr-simulation` `[[bin]]` target | guard failed: `missing ["qr-simulation"], unexpected ["quantrule-simulation-service"]`; manifest restored and guard passed |
| `cargo test -p quantrule-data-service -p quantrule-model-service -p quantrule-simulation-service` | 2 health tests passed per crate |
| `cargo tree -p quantrule-{data,model,simulation}-service --depth 1` | `axum`, `quantrule-api`, `quantrule-runtime`, `tokio`; no engine, app, database, or object-store edge |
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `cargo test --workspace` | passed |
| `tools/gate.sh` | passed |
| `git diff --check` | passed |

## Learning and rejected alternatives

- A bucket-wide Python inventory checker would duplicate the Rust guard and add a
  product script the scope excludes. The manifest-reading Rust test is the guard.
- Rejected: invoking `cargo metadata` from inside `cargo test`. It is the
  authoritative target list, but a nested Cargo call adds a build-lock and
  toolchain dependency to a fast test; reading the real manifests and source
  entry points covers explicit and auto-discovered targets without it.
- Rejected: a shared health/version router in `quantrule-runtime` for this
  phase. The three skeletons are temporary M1 surfaces and P3/P4/P7 replace
  them; adding a runtime abstraction now would claim a stable shared shape
  before the services exist. The runtime change is limited to the guard test.
- The shell-client binary means the literal phrase "the workspace build produces
  exactly six binaries" is false if the client is counted. The guard and this log
  count six service binaries; the existing client stays untouched, as the plan
  requires.
- No model, strategy, execution, portfolio, run, artifact, or route behavior was
  changed. The new `/health` and `/version` surfaces are on new processes and new
  ports; the gateway's routes and defaults are unchanged.

## Not verified here

The data, model, and simulation domain routes, schema ownership, the gateway
proxy, the worker HTTP path, and the simulation parity gate belong to P3-P7 and
are not claimed. The image and Compose packaging of the three new binaries remains
#643; this log does not claim it.
