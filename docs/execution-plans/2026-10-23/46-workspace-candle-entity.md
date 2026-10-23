# Execution Plan: Bootstrap the Rust Workspace and the Candle Entity

- Issue: [#46](https://github.com/quant-rule/quant-rule/issues/46) — feat: bootstrap the Rust workspace and the candle entity
- Branch: `feat/46-workspace-candle-entity`
- Worktree: `/home/zantgo/Proyectos/quant-rule--46-workspace-candle-entity`
- Status: Verified — acceptance criteria evaluated against observed evidence; Git handoff authorized on 2026-10-23
- Date: 2026-10-23 (UTC)
- Transparency log: [`docs/ai-transparency-log/2026-10-23/46-workspace-candle-entity.md`](../../ai-transparency-log/2026-10-23/46-workspace-candle-entity.md)

## Intent

Create the Rust workspace and the data engine's candle entity. Every later data-engine and model-engine issue builds on these types.

Affected users: the human developer, who will use the shell command built in #48, and the model-engine implementers, who will consume the candle type.

## Current state

- `origin/main` is at `811f703`; the repository holds `AGENTS.md`, `README.md`, and `docs/` only. There is no `Cargo.toml`, Rust source, test directory, or CI configuration.
- The workspace layout is not recorded in `AGENTS.md` section 2, which requires it to be recorded in the same change that creates the workspace.
- The local `data/` directory contains 17 CSVs, 25,698,960 rows, and 1.6 GB. It is gitignored by the user's uncommitted `.gitignore` change.
- Rustup is installed at `~/.cargo/bin`. The stable toolchain was repaired on 2026-10-23; `cargo 1.99.1`, `rustc 1.99.1`, `rustfmt 1.10.0-stable`, and `clippy 0.1.99` are available when `~/.cargo/bin` is on `PATH`.
- Issue #46 is `In Progress` on the project board with start date 2026-10-23.

## Scope

Included:

- Workspace with `quantrule-data`, `quantrule-model`, and `quantrule-shell`.
- Dependency direction: shell to data and model; model to data.
- `AGENTS.md` section 2 crate layout.
- `Candle` with `series_index`, UTC timestamp, OHLC, and optional volume.
- `RawCandle` for the shell-to-engine boundary.
- `Timeframe::M1`.
- UTC timestamp parsing and formatting for the source format.
- Unit tests for validation, timestamps, and timeframe.
- The `/data/` gitignore line.
- This plan and its transparency log.

Explicit non-goals:

- No CSV parsing, no `CandleWindow`, and no shell command. Those are #47 and #48.
- No indicator, model behavior, strategy, or backtest.
- No higher timeframe construction or gap handling. Those are #50.
- No third-party dependency.
- No I/O, clock, or randomness in the data engine.

## Assumptions and uncertainty

- The source timestamp format is exactly `YYYY-MM-DD HH:MM:SS+00:00`; all 17 local CSVs use it.
- A candle's volume is optional. `None` means absent; `Some(0.0)` is a real zero.
- Negative prices are valid; zero-range candles are valid.
- The first source timeframe is M1.
- The workspace root is virtual and holds no package, so integration tests belong in each crate's `tests/` directory.

## Decisions

| # | Decision | Choice | Rationale |
|---|---|---|---|
| P1 | Crate location | `crates/quantrule-data`, `crates/quantrule-model`, `crates/quantrule-shell` | One place for packages; names cannot collide with dependency names |
| P2 | Toolchain | Pin stable `1.99.1` in `rust-toolchain.toml` | Reproducible builds; the installed toolchain is verified |
| P3 | Dependencies | Standard library only | No dependency approval is needed, and the first slice does not justify one |
| P4 | Candle field set | `series_index`, timestamp, OHLC, optional volume | The minimum the data engine and model contract need |
| P5 | Index semantics | `series_index` is stored, stable, and assigned on ingestion | Identity survives window eviction |
| P6 | Timestamp representation | UTC Unix seconds in the engine; RFC 3339 in the shell | One explicit numeric format; presentation stays in the shell |
| P7 | Validation | Reject non-finite prices, OHLC inconsistencies, negative volume | Invalid state cannot be represented |
| P8 | Model crate | Created empty and documented | The declared layout is real from the first change; behavior is #14 |

## Acceptance criteria

- The workspace holds the three crates and builds.
- `AGENTS.md` section 2 records the crate names and paths.
- `Candle` exposes every field through read accessors and rejects invalid data.
- `RawCandle` is the shell-to-engine boundary.
- `Timeframe::M1` reports 60 seconds.
- UTC timestamp parsing and formatting round-trip for valid values and reject malformed ones.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo doc --no-deps` are clean.
- No third-party dependency is present.
- The `.gitignore` keeps `/data/` out of the repository.

## Work breakdown

| Step | Action | Role | Paths |
|---|---|---|---|
| 1 | Write this plan and log | Orchestrator | `docs/` |
| 2 | Create workspace, crate manifests, toolchain pin | Implementation | `Cargo.toml`, `rust-toolchain.toml`, `crates/` |
| 3 | Implement candle, timestamp, timeframe | Implementation | `crates/quantrule-data/src/` |
| 4 | Create empty model crate and shell stub | Implementation | `crates/quantrule-model/`, `crates/quantrule-shell/` |
| 5 | Update `AGENTS.md` section 2 and `.gitignore` | Implementation | `AGENTS.md`, `.gitignore` |
| 6 | Write and run tests; verify formatting, clippy, docs | Test Engineer | `crates/` |
| 7 | Review the complete diff | Code Reviewer | all |
| 8 | Commit, push, open PR after authorization | Orchestrator | git state, PR |

## Verification

Commands run on 2026-10-23 from the worktree, with `~/.cargo/bin` on `PATH`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

Observed evidence:

- `cargo fmt --all -- --check` exited 0.
- `cargo clippy --all-targets -- -D warnings` checked the three crates and exited 0 with no warnings.
- `cargo test` exited 0: 34 unit tests passed in `quantrule-data`, 0 in `quantrule-model`, 0 in `quantrule-shell`, and 2 doctests passed. The tests cover valid and invalid OHLC, non-finite values, zero and absent volume, leap years, bad timestamps, offsets, and the M1 timeframe.
- `cargo doc --no-deps` generated documentation for all three crates and exited 0.
- `Cargo.lock` lists only `quantrule-data`, `quantrule-model`, and `quantrule-shell`; there is no third-party dependency.
- None of these verification commands read `data/`; the tests use in-code fixtures only.

The acceptance criteria are met. The commit, push, and pull request remain subject to the human authorization gate.

## Risks and recovery

- **The candle type becomes a contract the model engine reads.** Mitigation: keep the field set minimal and documented; a field change is a contract change.
- **The toolchain pin can force a download on another machine.** Mitigation: the pin is the verified installed stable version and rustup can install it.
- **The empty model crate could be seen as speculative.** Mitigation: the workspace layout is already declared in `AGENTS.md` section 2; the crate is the smallest thing that makes the layout real.
- **Recovery.** The change is one branch and one PR; it can be reverted with a new commit on `main`. The worktree is preserved until the PR is accepted.

## Stop conditions

Stop and request human direction if:

- A third-party dependency is needed.
- The timestamp format cannot be represented without a decision outside this plan.
- The crate layout cannot be recorded without changing an unrelated `AGENTS.md` rule.
- Verification cannot be completed with the installed toolchain.
