# AI Transparency Log — Architecture Hardening

- Date: 2026-10-30 (UTC)
- Work item: `architecture-hardening`
- Plan: [`docs/execution-plans/2026-10-30/architecture-hardening.md`](../../execution-plans/2026-10-30/architecture-hardening.md)
- Tool: OpenCode agent, DeepSeek V4.1 Flash 1017 (model ID `deepseek-flash`), with shell and file tools
- Human decision owner: repository owner

## Entry 1 — Read-only audit and plan

- Date: 2026-10-30 (UTC)
- Intent and boundary: inspect the repository read-only and produce an architectural audit plus a phased execution plan. No code, tests, issues, or PRs were changed in this entry.
- What the AI did:
  - Inspected the workspace layout, crate manifests, module trees, line counts, public surface, tests, ADRs, and the existing MVP audit.
  - Traced module imports to find cycles and upward dependencies.
  - Wrote the audit findings (ARCH-2026-10-30-01 through -08) and the execution plan.
  - Proposed the target module graph: `model::input`/`consensus`/`timeframe`/`run`/`state`; `backtest::side` plus an execution-owned intent; a shell `pipeline`; split command arguments and renderers.
- Verification and evidence: the evidence is file/line references at `9f0d0d5`. The test suite was not run for the audit; no passing-test claim is made.
- Learning and corrections:
  - The initial plan counted `snapshot_at` as 13 parameters; the signature has 12. Corrected before approval.
  - The initial plan said “seven of eight” unused config re-exports; the count is seven of nine. Corrected before approval.
  - The crate graph is not the problem; the module graph is. The plan therefore rejects new crates.
  - Accepted ADRs constrain the solution: config parsing stays in the shell; descriptive views stay in the model; backtest stays one crate.

## Entry 2 — Phase 0 records

- Date: 2026-10-30 (UTC)
- Intent and boundary: create the repository audit, approved plan, and this log; establish the baseline. No production source changes.
- What the AI did:
  - Created the task worktree `../quant-rule--architecture-hardening` on branch `refactor/architecture-hardening` from `main` at `9f0d0d5`.
  - Copied the approved plan to `docs/execution-plans/2026-10-30/architecture-hardening.md` and marked it Approved.
  - Wrote `docs/audits/2026-10-30/architecture-audit.md`.
  - Recorded the toolchain: Rust 1.99.1, Cargo 1.99.1, edition 2024.
- Verification and evidence: `git worktree list` shows the worktree; the files exist. Baseline test/clippy results are recorded in the phase evidence section below when run.
- Learning: the human approved the plan and authorized implementation; push, PR, and merge remain gated. The plan’s Phase 3c still requires an explicit A/B decision and quant-role evidence.

## Entry 3 — Phase 1 model de-cycling

- Date: 2026-10-30 (UTC)
- Intent and boundary: remove the `run ↔ state` cycle without changing numeric output. The model input contract, consensus diagnostic, and per-timeframe series were split into `input`, `consensus`, and `timeframe`; `state` is now a view over `run`.
- What the AI did:
  - Added `input.rs`, `consensus.rs`, and `timeframe.rs`; reduced `run.rs` to `ModelRun`; reduced `state.rs` to the view.
  - Folded the 12-parameter free `snapshot_at` into a `SnapshotInputs` request struct, removing the `too_many_arguments` allow.
  - Updated every workspace import from `run::TimeframeModel`/`run::ModelBar`/`state::MarketStateConfig`/`state::TimeframeInput`/`state::TimeframeConsensus` to the new modules.
  - Added `crates/quantrule-model/tests/boundaries.rs` with source-level guards.
- Verification and evidence: `cargo test -p quantrule-model` passed (442 unit tests plus the model integration tests and 2 boundary tests); `cargo clippy -p quantrule-model --all-targets` passed. No golden fixture was regenerated. `cargo check --workspace` passed after the import migration.
- Learning: the private `TimeframeModel` field access in `ModelRun::timeframe` had to become the public accessor; the move made the previous same-module privacy visible. The split is behavior-preserving so far.

## Entry 4 — Phase 2 backtest dependency inversion

- Date: 2026-10-30 (UTC)
- Intent and boundary: remove the upward `execution -> strategy` and `portfolio -> strategy` edges without changing fills, rejections, or accounting.
- What the AI did:
  - Moved `Side` to `quantrule-backtest::side`; `strategy` re-exports it for the existing public path.
  - Added `execution::OrderIntent` (`Hold`, `Enter`, `Exit`) carrying only side and bar identity.
  - Changed `ExecutionEngine::submit` to take `OrderIntent`; the runner maps `Decision` to the intent.
  - Updated execution/portfolio docs and tests to the new vocabulary.
  - Added `crates/quantrule-backtest/tests/boundaries.rs` with source-level guards.
- Verification and evidence: `cargo check --workspace --all-targets` passed; `cargo test -p quantrule-backtest` passed (152 unit tests, integration suites, and 2 boundary tests); `cargo clippy -p quantrule-backtest --all-targets` passed; `cargo test -p quantrule-backtest --doc` passed 38 doctests. The execution golden fixtures were not regenerated.
- Learning: the engine still validates the same bar index and open time, so the rejection behavior is preserved. The `Decision` name remains in the existing `ExecutionError` variants for compatibility; the type itself is no longer imported.

## Entry 5 — Phase 3a shell pipeline

- Date: 2026-10-30 (UTC)
- Intent and boundary: make one shell pipeline own the source-to-model path, remove the duplicate ladder/input construction, and make `strategy-run --config` honor the run model config. The strategy decision rule is unchanged.
- What the AI did:
  - Added `shell/src/pipeline.rs` with `WindowMode`, `ModelInputs`, `load_set`, `load_extended_set`, `load_set_with_limit`, `model_run_from_set`, `run_model`, `run_backtest`, and `ladder_secs`.
  - Replaced the four `TimeframeInput` construction sites in `command/model.rs`, `command/layers.rs`, `command/report_run.rs`, and `command/strategy.rs`.
  - Removed the duplicate `LADDER` constants from `command/report.rs`, `command/report_run.rs`, and `config/data.rs`; the ladder now comes from `Timeframe::ALL`.
  - `strategy-run` now carries `run.model` into the model run instead of `MarketStateConfig::default()`.
  - Added an end-to-end test that runs `strategy-run --config` against a minor-structure-scale config and compares its decisions with `report strategy`.
- Verification and evidence: `cargo test -p quantrule-shell` passed (including the 99 CLI tests, all report/backtest/store suites, and the new 12th strategy-run test); `cargo clippy -p quantrule-shell --all-targets` passed. The new test was also checked manually: the minor config produced 17 records while the default major config produced 13, so the test is sensitive to the fix.
- Learning: computing the full four-timeframe model for `strategy-run` adds work compared with the old single-timeframe path, but it uses the same model contract as the report/backtest path and removes the last shell `TimeframeInput` literal. The command is a diagnostic, so the extra computation is acceptable in this refactor.

## Entry 6 — Phase 3b one strategy walk

- Date: 2026-10-30 (UTC)
- Intent and boundary: make the backtest runner the only place that walks the strategy; the shell reports and the diagnostic command read the recorded walk. The strategy rule is unchanged.
- What the AI did:
  - Added `strategy::StrategyWalk` and `AgreementCounts`, carrying inputs, decisions, agreement counts, and the final strategy position.
  - Added `runner::walk_strategy`; `run_backtest` computes the walk once and feeds execution from it.
  - `BacktestRun` stores the walk; `decisions()` and `strategy_walk()` read it.
  - Deleted `report_run::run_strategy`; the strategy bundle now reads `BacktestRun::strategy_walk()`.
  - Deleted `command/strategy.rs::collect_records`'s `on_bar` loop; it reads the walk instead.
- Verification and evidence: `cargo test -p quantrule-backtest` passed (152 unit tests and all integration suites); `cargo test -p quantrule-shell` passed (all suites, including 99 CLI tests and the new config test); `cargo clippy --workspace --all-targets` initially found one needless borrow, which was fixed. No `on_bar` call remains in `crates/quantrule-shell/src`.
- Learning: `report strategy` now runs the full backtest to obtain the executed walk. The output is unchanged for the current strategy; the benefit is that the report cannot re-walk the rule and drift from execution. AUD-02 remains open because the strategy still carries its own virtual position.

## Phase evidence

| Phase | Command | Observed | Date |
|---|---|---|---|
| 0 | `rustc --version` | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` | 2026-10-30 |
| 0 | `cargo --version` | `cargo 1.99.1 (5f94df478 2026-08-27)` | 2026-10-30 |
| 0 | `git worktree list` | worktree created at `/home/zantgo/Proyectos/quant-rule--architecture-hardening` | 2026-10-30 |
| 0 | `cargo fmt --all --check` | no output | 2026-10-30 |
| 0 | `cargo test --workspace --quiet` | all reported suites passed | 2026-10-30 |
| 0 | `cargo clippy --all-targets` | finished with no warnings | 2026-10-30 |
| 1 | `cargo test -p quantrule-model` | 442 unit tests, model integration suites, and 2 boundary tests passed | 2026-10-30 |
| 1 | `cargo clippy -p quantrule-model --all-targets` | no warnings | 2026-10-30 |
| 1 | `cargo check --workspace` | passed | 2026-10-30 |
| 2 | `cargo check --workspace --all-targets` | passed | 2026-10-30 |
| 2 | `cargo test -p quantrule-backtest` | 152 unit tests, integration suites, and 2 boundary tests passed | 2026-10-30 |
| 2 | `cargo clippy -p quantrule-backtest --all-targets` | no warnings | 2026-10-30 |
| 2 | `cargo test -p quantrule-backtest --doc` | 38 doctests passed | 2026-10-30 |
| 3a | `cargo test -p quantrule-shell` | all shell suites passed; 12 strategy-run tests including the new config test | 2026-10-30 |
| 3a | `cargo clippy -p quantrule-shell --all-targets` | no warnings | 2026-10-30 |
| 3b | `cargo test -p quantrule-backtest` | all suites passed | 2026-10-30 |
| 3b | `cargo test -p quantrule-shell` | all suites passed | 2026-10-30 |
| 3b | `cargo clippy --workspace --all-targets` | one needless-borrow warning fixed; final run clean | 2026-10-30 |

## Open items

- Phase 3c option (A reconcile or B stateless) is not chosen yet. Implementation must stop at that gate unless the human records the choice.
- The other open findings in `docs/audits/2026-10-30/mvp-pre-release-audit.md` are out of scope.
- Push, pull request, and merge are not authorized by plan approval.
