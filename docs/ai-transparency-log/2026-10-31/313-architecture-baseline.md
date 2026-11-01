# AI Transparency Log — Architecture Baseline and Crate-Graph Guard

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #313, branch `arch/313-architecture-baseline`
- Plan: `docs/execution-plans/2026-10-31/full-system-audit.md` (W6)
- Audit: `docs/audits/2026-10-31/full-system-audit.md` (AUD-2026-10-31-47)

## Intent and boundary

The human approved the full-system audit plan on 2026-10-31. W6 aligns the living
architecture baseline with the workspace manifests and adds a guard that reads
the manifests, so the documented graph cannot drift from the real one again.
The change is documentation plus an integration test. It does not change model
values, strategy behavior, runtime architecture, or dependencies.

## Implemented

- `AGENTS.md` section 2 now records that `quantrule-analytics` depends on
  `quantrule-model` and `quantrule-data`.
- `docs/architecture/README.md` and `docs/architecture/characteristics.md` list
  both edges in the crate table; the characteristics dependency sketch says
  analytics reads data and model.
- `docs/adr/2026-10-31/diagnosis-artifact.md` no longer claims the diagnosis
  crate dependency is model-only.
- `crates/quantrule-shell/tests/architecture.rs` gains a crate-graph guard:
  - it reads the five `crates/*/Cargo.toml` manifests;
  - it extracts the `quantrule-*` keys of each `[dependencies]` table;
  - it compares the graph with one allowed-edge table and names the crate pair
    on a forbidden or missing edge;
  - it ignores external dependencies and `[dev-dependencies]`;
  - a synthetic `quantrule-model -> quantrule-backtest` edge fails and names the
    pair.

## Verification and evidence

Commands actually run, with observed results:

| Command | Observed |
|---|---|
| `cargo test -p quantrule-shell --test architecture` | 6 passed, 0 failed |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace` | 1,554 passed, 0 failed, 7 ignored; 47 suites |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The graph test passes against the current workspace. The synthetic test proves
the guard rejects an edge that does not exist today, so a future manifest change
that adds one fails with the pair named rather than silently.

## Learning and rejected suggestions

- The first version joined `crates` twice (`CARGO_MANIFEST_DIR` is already inside
  `crates/quantrule-shell`). The failing read named the wrong path; resolving the
  parent once fixed it.
- The guard parses only `[dependencies]`. A workspace edge in
  `[dev-dependencies]` is still a build-time edge for tests, but no crate uses
  one today; the parser test pins the boundary so a future addition is a visible
  decision rather than an accidental edge.
- I did not add a crate or a build script for the graph. The shell already owns
  integration tests and the parsing is a few dozen lines of `std::fs`.
- I considered moving the test to the workspace root, but a virtual workspace
  root holds no package and a root `tests/` directory is never compiled.

## Limitations

- The guard checks direct workspace dependencies, not module-level imports or
  transitive third-party crates.
- The check is same-agent; no Quant Validator record is required because no model
  or strategy behavior changes.
- The architecture documents remain prose; the test guards the graph they state,
  not every sentence in them.
