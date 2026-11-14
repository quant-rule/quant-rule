# AI transparency log — B-P8 gateway finalization

- Date: 2026-11-15 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Execution Plan; issue #788 (P8 of #776), claim type ADR
  (Architecture Decision).
- Intent: retire `quantrule-server`, move the backtest worker and `qr-backtest`
  to `quantrule-backtest-service`, rename the remaining gateway crate to
  `quantrule-gateway`, and make the #651 service boundary guard real. The six
  binary names, the worker HTTP path, the retry and cancellation boundaries,
  the canonical digest, and the shell's 27-command contract must not change.
- Boundary: `crates/quantrule-gateway/**` (renamed from `quantrule-server`),
  `crates/quantrule-backtest-service/**`, workspace `Cargo.toml`/`Cargo.lock`,
  the #651 guard in `crates/quantrule-gateway/tests/service_boundary.rs`, the
  runtime binary inventory, `docs/architecture/module-map.md`,
  `docs/architecture/services.md`, the architecture glossary line,
  `README.md`/`AGENTS.md` crate lists, `CHANGELOG.md`, the plan evidence rows,
  the salvage P8 note, and this log. No shell, browser, data/model/evidence/
  simulation service, Dockerfile, compose, Makefile, CI, port-parity, or model
  change.

## Implemented

- Created `quantrule-backtest-service` with one binary, `qr-backtest`. The
  worker, retry policy, and workspace materializer moved unchanged from
  `quantrule-server::backtest`; the only source changes are the crate path, the
  binary path (`src/main.rs`), the `#![forbid(unsafe_code)]` root, and the source
  test that reads `src/main.rs` instead of `src/bin/qr_backtest.rs`. The worker
  still consumes `run.requested` and `run.cancelled`, reads config and dataset
  through the data client, obtains the model bundle through the model client,
  calls `AppService::run_backtest_from_model`, writes the same artifacts, and
  records the same canonical digest through evidence.
- Renamed the remaining crate to `quantrule-gateway`, package and library. It
  keeps `qr-gateway`, the profile store in `gateway.profiles`, the static client,
  the proxy table, and health. Removed `quantrule-app` and `quantrule-data` from
  its dependency set; it now links only `quantrule-api`,
  `quantrule-service-api`, `quantrule-runtime`, and transport crates. Deleted the
  last worker module, binary, and worker tests from the gateway.
- Moved the worker config fixtures to the backtest service and split the test
  support: the gateway keeps only the profile/proxy helpers; the worker gets its
  in-memory object store and CSV canonicalizer.
- Added `crates/quantrule-gateway/tests/service_boundary.rs` (#651). It checks:
  - the gateway dependency closure and direct manifest, including dev
    dependencies, reach no app, engine, or domain service package;
  - the gateway source names no app/engine/service crate;
  - each service package ships exactly its own binary;
  - no service package depends on another service package;
  - each service names only its own schema in source string literals and SQL
    migrations;
  - simulation reaches no app/engine/service crate and names no strategy, run
    store, run manifest, queue, or model call.
- Added a per-service import allowlist and one seeded-violation test per rule: a gateway → runtime → app edge, a
  foreign `runs.runs` read, a simulation → `quantrule_backtest::strategy` read, a
  second gateway binary plus a retired package, and a comment-only source to
  prove the guard reads production code rather than its own documentation.
- Updated the runtime six-binary inventory for P8: the package→binary mapping is
  now explicit, `quantrule-gateway` replaces `quantrule-server`, a package that
  ships two targets fails, and a test asserts the retired crate is not a
  workspace member.
- Updated the module map, service catalog dependency direction, the architecture
  glossary's service count, the README crate list, and the AGENTS.md crate list.

## Implementation decisions and rejected alternatives

- The worker got its own crate rather than staying as a second binary in the
  gateway. A gateway with a worker binary would link `quantrule-app` and the
  engines, making the compile boundary false. The crate name is the plan's
  `quantrule-backtest-service`.
- The backtest service links `quantrule-app` and `quantrule-data` because the
  staged use case and the workspace materializer need them. It links no schema
  and no `sqlx`; the worker reaches runs only through the evidence client.
- The gateway dependency proof is transitive, not just a direct-manifest check.
  A future `runtime → app` edge would otherwise quietly give the gateway the
  pipeline through its one shared dependency.
- The schema guard scans string literals in Rust and uncommented SQL, not raw
  source text. A local variable named `runs` or `catalog` is not a schema read,
  and a module doc that names another service is not a query. The guard strips
  `#[cfg(test)]`, so a test fixture cannot fail or satisfy the production rule.
- The simulation isolation guard strips comments before matching. The service's
  module docs say it never reads a strategy; treating that sentence as a read
  would make the guard unusable. The seeded test proves the production path is
  still matched.
- A second rejected alternative was to keep the six-binary inventory only in the
  runtime crate. The #651 guard now also asserts the package→binary mapping, so
  deletion, rename, and duplication all fail in the owning gateway test and in
  the runtime inventory.
- The gateway health surface was not extended. It still aggregates the probes
  it registers (Postgres today); `/health/ready` and service-to-service health
  probes are not part of #788's Done-when and remain recorded as V23 `Pending`.
  The backtest worker serves no HTTP health endpoint, so claiming five-service
  health aggregation would be false.
- `Dockerfile`, compose, the Makefile, and the image entrypoint still name
  `quantrule-server` as the image or build package. They are #643-owned and were
  not edited. The Dockerfile's `-p quantrule-server --bins` must become
  `-p quantrule-gateway -p quantrule-backtest-service --bins` (or a workspace
  build) before the image is built again; this is recorded as a coordination
  risk in the pull request, not claimed as done.

## Verification

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `cargo test --workspace` | 100 test targets passed, 0 failed |
| `cargo test -p quantrule-gateway -p quantrule-backtest-service` | gateway 14 lib + 1 boundary + 7 evidence + 9 gateway + 6 model + 7 proxy + 13 service boundary + 3 signal + 6 simulation + 6 static; backtest 2 lib + 21 worker; 0 failed |
| `cargo test -p quantrule-runtime --test binary_inventory` | 5 passed: six-binary inventory, package→binary mapping, retired-crate absence, seeded removal, seeded extra |
| `cargo test -p quantrule-shell` | 22 lib + 8 architecture + 2 client parity + 35 HTTP commands + 1 parity command passed |
| `python3 tools/check_command_surface.py` | `27 implemented, 0 planned`; README rows 27; tutorial files 17 |
| `cargo build --workspace --bins` | six binaries: `qr-backtest`, `qr-data`, `qr-evidence`, `qr-gateway`, `qr-model`, `qr-simulation` |
| `cargo build -p quantrule-gateway --bins` | passed |
| `cargo metadata --no-deps` | 16 packages; no `quantrule-server` |
| `cargo tree -p quantrule-gateway` | no `quantrule-app`, engine, or domain service package in the closure |
| `tools/gate.sh` | passed; fmt, clippy, workspace tests, rustdoc, and every checker reported ok |
| `cargo test -p quantrule-gateway --test service_boundary` | 13 passed, including five seeded violations |

The full composed stack was not started. #643 owns compose and the image, and
the current compose file does not set the `QR_*_URL` variables the P3-P7
services require. The coupled worker baseline test
`path::a_queued_run_matches_the_coupled_baseline` passed in the worker suite, so
the artifact names, bytes, and canonical digest are unchanged in-process.

## Limitations

- V23 (`/health` five-service aggregation and `/health/ready`) remains
  `Pending`; see the decision above.
- The image build is expected to fail until #643 changes the Dockerfile package
  name. No image or compose command was run.
- The guard is static and fast-tier. It proves the manifest and source edges; it
  does not compile a deliberately broken crate. The release-time `cargo tree -p
  quantrule-gateway` output is the second proof.
- The data and model service boundary tests still name `quantrule-server` as a
  forbidden string even though the crate is gone. That is harmless and those
  files are outside P8's allowed paths; the central #651 guard carries the new
  service package names.

## Learning

- A crate rename is not complete at the package manifest: workspace test paths,
  `CARGO_MANIFEST_DIR` fixtures, runtime inventory heuristics, and the image
  build all encode the old name. Each one needed a deliberate check.
- The strongest boundary proof was the dependency closure, not the import scan.
  The import scan is the readable rule; the closure is what makes a transitive
  edge impossible to miss.
- A guard needs a negative case that exercises its matcher, not only a comment
  saying it would fail. The seeded tests caught the difference between scanning
  code and scanning documentation.
