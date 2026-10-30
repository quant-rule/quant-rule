# Execution Plan — Architecture Hardening

- Date: 2026-10-30 (UTC)
- Status: **Approved** — approved by the human on 2026-10-30; implementation authorized. Push, pull request, and merge remain gated by AGENTS §9.
- Work item: `architecture-hardening` (no issue number assigned)
- Baseline: `main` at `9f0d0d5`
- Toolchain: Rust 1.99.1, edition 2024 (verified 2026-10-30).
- Author: Orchestrator / Software Architect (AI), audit and plan pass
- Related:
  - `docs/audits/2026-10-30/mvp-pre-release-audit.md`, especially AUD-2026-10-30-02
  - `docs/adr/2026-10-27/analysis-and-backtest-boundaries.md`
  - `docs/adr/2026-10-28/config-driven-engines-and-reports.md`
  - `docs/adr/2026-10-30/abstraction-levels-and-engine-boundaries.md`

> The human approved this plan and authorized implementation on 2026-10-30. Phase 3c still changes strategy behavior and needs its own option decision plus Quant Validator evidence; implementation must stop at that gate unless the human records the choice. Push, pull request, and merge are not authorized by plan approval.

## 1. Intent

### Problem

The crate graph is healthy, but two module-level seams have started to rot:

1. `quantrule-model` has a real `run ↔ state` module cycle. `run.rs` imports input/config types from `state.rs`, and `state.rs` imports `ModelRun` from `run.rs`. The two modules cannot be understood or tested independently.
2. `quantrule-shell` is a 30,013-line mini-monolith. `command` is 9,437 lines and imports thirteen shell modules; `command ↔ lab_summary` and `command ↔ render_state` are real module cycles. The shell also runs the strategy itself, in two places, while `quantrule-backtest::runner` runs it a third time. This is the architectural root of AUD-2026-10-30-02: the reported strategy and the executed strategy can diverge.

A smaller but foundational inversion sits in `quantrule-backtest`: `execution` and `portfolio` import `strategy` for `Side` and `ExecutionEngine::submit` accepts `Decision`. The decision stack says strategy is above execution, so this edge points the wrong way.

### Intended outcome

- Remove the `run ↔ state` cycle; `state` becomes a one-way view.
- Remove the upward `execution -> strategy` and `portfolio -> strategy` edges.
- Make `runner` the only place that walks the strategy, and the shell pipeline the only place that loads a timeframe set and builds model inputs.
- Reduce the shell command hub to parse → call pipeline → render, with parsed arguments split by command family.
- Keep the five-crate workspace. Do not add a crate to move files around.
- Preserve behavior in Phases 0–2. Any behavior change is isolated to Phase 3a (the `strategy-run` config fix) and Phase 3c (AUD-02), and is explicitly validated.

### Affected users

- Maintainers and AI agents working in the repository: smaller units, fewer edges, clearer ownership.
- Shell users: fewer divergent outputs between `strategy-run`, `report strategy`, and `backtest`; AUD-02 fixed.
- No external library users: the Rust API is internal to the workspace; the product interface is the command set.

## 2. Current state

### 2.1 Workspace shape

| Crate | Source lines | Role | Dependencies |
|---|---:|---|---|
| `quantrule-data` | 3,214 | Candles, timeframes, windows, gaps | none |
| `quantrule-model` | 22,873 | Readings, four axes, events, aggregates, views | `data` |
| `quantrule-analytics` | 6,772 | Research dataset, layers, summaries | `data`, `model` |
| `quantrule-backtest` | 8,760 | Strategy, execution, portfolio, runner | `data`, `model` |
| `quantrule-shell` | 30,013 | Command shell, config, store, charts, reports | `data`, `model`, `analytics`, `backtest`; external `png`, `rusqlite`, `serde_json`, `ureq` |

The crate DAG is `data ← model ← {analytics, backtest} ← shell`. It is acyclic and one-way. Engine crates have no external dependencies.

### 2.2 Findings

| ID | Severity | Finding | Evidence | Impact |
|---|---|---|---|---|
| F1 | Critical | The shell runs the strategy itself, in two places, and the runner runs it a third time | `command/report_run.rs:319` (`run_strategy`); `command/strategy.rs:263` (`collect_records`); `backtest/runner/mod.rs:138` (`strategy.on_bar`); AUD-2026-10-30-02 reports 166 strategy entries vs 93 executed | Shell holds domain logic; reported strategy ≠ executed strategy |
| F2 | Major | `execution` and `portfolio` depend upward on `strategy` | `execution/engine.rs:80`; `execution/position.rs:10`; `portfolio/state.rs:10`, `portfolio/sizing.rs:13`, `portfolio/ledger.rs:22`; `ExecutionEngine::submit(decision: Decision, …)` at `execution/engine.rs:290` | Decision stack inverted; execution cannot be tested or reused without strategy |
| F3 | Major | `model::run` ↔ `model::state` module cycle | `run.rs:32` imports `state::{MarketStateConfig, TimeframeConsensus, TimeframeInput}`; `state.rs:19` imports `run::ModelRun`; `state.rs:181` `from_run`; `run.rs:463` free `snapshot_at` | No independent testing; `state` mixes input/config types with a view |
| F4 | Major | The shell `command` subsystem is a 9,437-line hub with two module cycles | `command/types.rs` imports `chart`, `config`, `csv`, `export`, `fetch`, `lab_summary`, `render_report`, `source`, `store`, `quantrule_analytics`, `quantrule_backtest`; `command/lab.rs:204` + `lab_summary.rs:17`; `command/model.rs:16` + `render_state.rs:19`; 63 `Command` variants, 17 parsers, 84 repeated parser error strings, 32 help entries | Any subsystem change touches the hub; cycles block independent units; help/dispatch/parser drift |
| F5 | Major | Ladder and model-input construction are duplicated; `strategy-run` ignores the run's model config | Four `TimeframeInput` sites: `command/layers.rs:43`, `command/model.rs:337`, `command/report_run.rs:571`, `command/strategy.rs:222`; `LADDER` at `command/report.rs:25`, `command/report_run.rs:33`, `config/data.rs:20`; `command/strategy.rs:233` uses `MarketStateConfig::default()`; `Timeframe::ALL` already exists at `data/timeframe.rs:21` | A ladder or config change silently diverges between commands |
| F6 | Minor | Oversized modules and functions | `analytics/research/columns.rs` 1,706 lines; `command/report_run/bundles.rs` 1,274; `shell/tests/cli.rs` 1,991; `research/metadata.rs::codes_csv` 408-line function; `axes/progression/mod.rs::compute_with_config` 200; `config/run.rs::validate_cross` 163; the free `run.rs::snapshot_at` function has 12 parameters | Review/test friction; violates the 1,000-line guideline |
| F7 | Minor | Blanket lint suppression hides rot | `config/mod.rs:14` `#![allow(dead_code, unused_imports)]`; seven of nine `pub(crate) use` re-exports are unused outside `config`; module doc still says “until #172–#176” | Dead code passes clippy; stale documentation survives |
| F8 | Minor | Normative doc omits a pipeline crate | `AGENTS.md` §2 lists `data`, `model`, `shell`, `analytics`; `Cargo.toml` members include `quantrule-backtest` | Agents and readers miss a pipeline stage |

### 2.3 What is healthy

- Clean crate DAG; no cycles at crate level.
- Pure engine crates with zero external dependencies.
- `#![forbid(unsafe_code)]` in every crate.
- No production `HashMap`/`HashSet` iteration, clock read, or randomness.
- Strong golden, causality, determinism, and edge-case tests.
- Command families are already split; the problem is the shared hub, not the family modules.
- Accepted ADRs are sound: config parsing stays in the shell; descriptive views stay in the model; backtest stays one crate.

### 2.4 Audit limitations

- Static audit at `9f0d0d5`; the test suite was not run.
- Line numbers are from that baseline.
- No profiling, no runtime reproduction of AUD-02 in this plan; the existing audit records the observed counts.
- The audit does not cover the other open MVP findings (AUD-2026-10-30-03 through -10); they remain separate remediation work.

## 3. Scope

### 3.1 In scope

- Phase 0: audit/plan/AI-log documents and baseline verification.
- Phase 1: de-cycle `quantrule-model`.
- Phase 2: invert `quantrule-backtest`'s strategy/execution/portfolio dependency.
- Phase 3a: one shell pipeline for source, timeframe set, and model inputs; one ladder source; `strategy-run` honors the run model config.
- Phase 3b: one strategy walk, owned by `quantrule-backtest`; delete both shell strategy loops.
- Phase 3c: AUD-2026-10-30-02 strategy/execution state reconciliation, with Quant Validator evidence.
- Phase 4: split the shell command arguments, break the two shell cycles, split the report bundle module and the CLI integration test.
- Phase 5: split oversized analytics tables, remove the blanket lint allow, split `lab_summary`, update `AGENTS.md`.

### 3.2 Out of scope

- Adding a crate (`quantrule-command`, `quantrule-view`, `quantrule-stats`, etc.).
- Moving config parsing into the engine crates. The accepted ADR keeps config schemas in the shell.
- Moving `dashboard`/`report` descriptive views out of `quantrule-model`. The accepted ADR keeps them there.
- Splitting `quantrule-backtest` into separate strategy/execution/portfolio crates.
- Changing command names, arguments, output contracts, or error text.
- Changing policy/strategy configuration schemas.
- Fixing the other open MVP audit findings (AUD-03 through -10).
- Rewriting the SQLite store, the chart renderer, or the research column contract.
- Performance optimization; this is a structural refactor. If an allocation changes, inspect it but do not optimize in this plan.
- New dependencies, new `unsafe`, new background work, or new I/O surfaces.

## 4. Assumptions and uncertainty

| ID | Assumption | If wrong |
|---|---|---|
| A1 | The current golden, causality, and CLI tests are sufficient to prove byte-identical behavior for Phases 0–2 | Stop and strengthen the fixture coverage before refactoring |
| A2 | `TimeframeModel` can move to its own module without changing numeric output | Stop; compare outputs bar by bar and report the mismatch |
| A3 | `Side` can move to a neutral module and `ExecutionEngine::submit` can take an execution-owned intent without changing fills/rejections | Stop; keep the current API and re-plan Phase 2 |
| A4 | `strategy-run` is intended to honor the run's model config; today it uses the default | If not intended, keep the default and document the divergence; do not silently change it |
| A5 | The only consumers of the internal Rust API are the workspace crates | If an external consumer exists, add a compatibility shim and re-plan |
| A6 | The CLI output is the observable contract; tests cover it byte for byte | If not, add missing command tests before Phase 4 |
| A7 | AUD-02 option B (stateless strategy) is the better long-term shape, but it changes results | If the human chooses A, the reconciliation path is smaller and does not remove strategy state |
| A8 | No accepted ADR conflicts with these module moves | Stop and request a new ADR or a plan revision |
| A9 | The workspace builds on the installed toolchain (Rust 1.99, edition 2024) | Record the actual toolchain and adjust only version-specific syntax |

## 5. Decisions

| ID | Decision | Status | Rationale |
|---|---|---|---|
| D1 | Keep the five-crate workspace; do not add crates | Approved in principle | The crate DAG is clean; new crates would move files without removing edges |
| D2 | Phases 0–2 are behavior-preserving; golden fixtures must not change | Approved in principle | Refactor risk is controlled by byte-identical evidence |
| D3 | `Side` moves to a neutral `backtest::side` module; `execution` owns `OrderIntent`; `runner` maps `Decision` → intent | Proposed | Removes the upward strategy dependency without changing behavior |
| D4 | `model::input` owns `MarketStateConfig` and `TimeframeInput`; `model::consensus` owns `TimeframeConsensus`; `model::timeframe` owns `TimeframeModel`; `model::state` is a view only | Proposed | Breaks the cycle and gives each type one home |
| D5 | The shell pipeline owns `ModelInputs` (the four owned candle vectors and `[TimeframeInput; 4]`); the model stays source-agnostic | Proposed | The shell owns source access; the model consumes explicit inputs |
| D6 | `command/args/*` owns parsed arguments; `command/types.rs` keeps `Command`, `ParseError`, `RunError`, and shared vocabulary | Proposed | Shrinks the hub without changing the command contract |
| D7 | `Axis` moves to `render_state`; `lab_summary::Request` replaces `LabSummaryArgs` in the writer API | Proposed | Breaks both shell cycles in the correct direction |
| D8 | Phase 3c uses option A (reconcile fills/rejections) as the interim fix and option B (stateless strategy) as the target, or the human chooses one | **Deferred — human decision required** | Both change strategy behavior; B is cleaner but larger; quant roles required |
| D9 | No new dependencies, no `unsafe`, no command-contract changes | Approved in principle | Out of scope by default |
| D10 | One worktree for the program; one branch and PR per phase; branch from latest `main` after the prior merge | Proposed | Keeps reviews focused and avoids parallel write conflicts |
| D11 | Phase 1 and Phase 2 may run in parallel only with explicit human authorization; otherwise sequential | Proposed | They touch disjoint crates, but the repository’s default is one work item in flight |
| D12 | The formal artifacts use the slug `architecture-hardening`; no issue number is invented | Proposed | AGENTS forbids inventing issue numbers |

## 6. Acceptance criteria

### 6.1 Overall

- `quantrule-model::run` does not import `quantrule_model::state`; `state` is a one-way view over `run`.
- `quantrule-backtest::execution` and `quantrule-backtest::portfolio` do not import `quantrule_backtest::strategy`.
- The only production `on_bar` walk lives in `quantrule-backtest`; the shell has none.
- The only production timeframe-set load and model-input construction lives in the shell `pipeline` module.
- No `LADDER` constant remains in the shell; the ladder comes from `Timeframe::ALL` / `Timeframe::code()`.
- `strategy-run --config` uses the run's model config, not `MarketStateConfig::default()`.
- No `command ↔ lab_summary` or `command ↔ render_state` module cycle.
- The blanket `#![allow(dead_code, unused_imports)]` is gone; no unused config re-exports remain.
- `cargo fmt --all --check`, `cargo clippy --all-targets --all-features`, `cargo test --workspace`, and `cargo doc --workspace --no-deps` pass.
- Golden fixtures are unchanged for Phases 0–2. Phase 3a changes only the documented `strategy-run` config behavior. Phase 3c changes strategy results only with recorded Quant Validator evidence.
- No new dependencies; no new `unsafe`; no command-contract changes.

### 6.2 Phase 0

- `docs/audits/2026-10-30/architecture-audit.md` exists with the F1–F8 table, evidence, and status.
- The plan of record exists at `docs/execution-plans/2026-10-30/architecture-hardening.md`.
- The AI transparency log exists at `docs/ai-transparency-log/2026-10-30/architecture-hardening.md`.
- Baseline command results are recorded with observed output.
- No source file changes.

### 6.3 Phase 1

- `run.rs` contains no `use crate::state` and no `MarketStateConfig`/`TimeframeInput`/`TimeframeConsensus` definitions.
- `state.rs` contains only the `MarketState` view types and their accessors.
- `input.rs`, `consensus.rs`, and `timeframe.rs` exist with module docs and rustdoc.
- `snapshot_at` is a `TimeframeModel` method (or a request struct with at most three parameters); the `clippy::too_many_arguments` allow on it is removed.
- All model golden fixtures are byte-identical.
- `cargo test -p quantrule-model`, `cargo clippy -p quantrule-model --all-targets`, and `cargo doc -p quantrule-model` pass.

### 6.4 Phase 2

- `grep -rn 'use crate::strategy' crates/quantrule-backtest/src/execution crates/quantrule-backtest/src/portfolio` returns no matches.
- `ExecutionEngine::submit` does not accept `Decision`.
- The runner maps `Decision` to `OrderIntent` and preserves the existing bar/time mismatch errors.
- The full backtest golden run is byte-identical; fills, rejections, and metrics are unchanged.
- `cargo test -p quantrule-backtest` and the new boundary test pass.

### 6.5 Phase 3a

- `crates/quantrule-shell/src/pipeline.rs` is the only place that loads a `TimeframeSet` and constructs `TimeframeInput`s.
- No `TimeframeInput {` literal remains under `command/` outside tests.
- No `const LADDER` remains in `quantrule-shell`; `config/data.rs` validates against `Timeframe::ALL` codes.
- `strategy-run --config` uses `run.model`; a test with a non-default `structure_scale` proves the decisions match `report strategy`.
- CLI output is byte-identical except the documented `strategy-run` config fix.
- `cargo test -p quantrule-shell` passes.

### 6.6 Phase 3b

- No production `on_bar` call remains in `quantrule-shell`.
- `BacktestRun` exposes the one `StrategyWalk` (inputs, decisions, and the derived counts) computed by the runner.
- `report strategy` and `strategy-run` agree on the same config and data; a test proves it.
- The strategy report golden is unchanged (unless Phase 3a’s config fix changes the fixture, which is documented).
- AUD-2026-10-30-02 remains open after 3b; 3c closes it.

### 6.7 Phase 3c

- A regression test fails before the fix and passes after it: an `allow_short=false` run cannot report a short that was not executed, and an exit side that does not match the position is rejected.
- The chosen option (A or B) is recorded in the plan and the PR.
- Quant Researcher, Quant Developer, and Quant Validator records are present; the validator reproduces the run from the recorded data/config.
- Golden changes are deliberate, reviewed, and limited to the intended strategy behavior change.
- AUD-2026-10-30-02 moves to `Fixed` in the PR; the Improvement Verifier moves it to `Verified` after merge.

### 6.8 Phase 4

- `command/types.rs` is under 500 lines and contains no per-family args structs.
- `command/args/*` owns the parsed args, one module per family.
- `lab_summary.rs` and `render_state.rs` do not import `command`.
- `report_run` is split into per-bundle modules; no shell source file exceeds 1,000 lines.
- `tests/cli.rs` is split by command family; the test count is unchanged.
- CLI output, including error text, is byte-identical.
- The help/dispatch consistency test exists and passes (or a documented reason it is impractical).

### 6.9 Phase 5

- `research/columns.rs` is split per layer; no file exceeds 1,000 lines.
- `codes_csv` is a static table in its own module; no 400-line function remains.
- `config/mod.rs` has no blanket allow and no unused re-exports; the stale module doc is corrected.
- `AGENTS.md` lists `quantrule-backtest` and its role.
- `lab_summary.rs` is split into read/text/write units.
- All checks pass.

## 7. Work breakdown

### Phase 0 — Records and baseline

| Step | Work | Paths | Role | Depends on |
|---|---|---|---|---|
| 0.1 | Verify repository state: `git status --short --branch`, `git worktree list`, `git log -1`; confirm `main` at the baseline or record the new baseline | repository | Orchestrator | — |
| 0.2 | Verify toolchain: `rustc --version`, `cargo --version` | repository | Release/Ops | 0.1 |
| 0.3 | Write the architecture audit with F1–F8, evidence, impact, options, and status | `docs/audits/2026-10-30/architecture-audit.md` | Audit/Planner | 0.1 |
| 0.4 | Write this plan of record and the AI transparency log | `docs/execution-plans/2026-10-30/architecture-hardening.md`, `docs/ai-transparency-log/2026-10-30/architecture-hardening.md` | Planner/Orchestrator | 0.3 |
| 0.5 | Run the baseline checks and record observed output | repository | Test/Release | 0.1 |
| 0.6 | Create the worktree and branch only after plan approval | `../quant-rule--architecture-hardening`, `refactor/architecture-hardening` | Orchestrator | human approval |

**Phase 0 deliverable:** audit, plan, AI log, baseline evidence. No source changes.

### Phase 1 — De-cycle the model

| Step | Work | Paths | Role | Depends on |
|---|---|---|---|---|
| 1.1 | Create `input.rs`; move `MarketStateConfig` and `TimeframeInput` from `state.rs`; add module docs | `crates/quantrule-model/src/input.rs`, `state.rs` | Implementation | Phase 0 |
| 1.2 | Create `consensus.rs`; move `TimeframeConsensus` and its `compute`/`from_bars` | `crates/quantrule-model/src/consensus.rs`, `state.rs` | Implementation | 1.1 |
| 1.3 | Create `timeframe.rs`; move `TimeframeModel`, its `compute`, accessors, `tail`, and the snapshot logic; fold `snapshot_at` into a private method | `crates/quantrule-model/src/timeframe.rs`, `run.rs` | Implementation | 1.1 |
| 1.4 | Reduce `run.rs` to `ModelRun` orchestration; remove the `state` import | `crates/quantrule-model/src/run.rs` | Implementation | 1.2, 1.3 |
| 1.5 | Reduce `state.rs` to `MarketState`/`TimeframeMarketState`; import `run` and `consensus`; remove input/config definitions | `crates/quantrule-model/src/state.rs` | Implementation | 1.2, 1.4 |
| 1.6 | Update `lib.rs` module list and all consumers (`dashboard`, `report`, analytics, backtest, shell) | `crates/quantrule-model/src/lib.rs`, workspace | Implementation | 1.4, 1.5 |
| 1.7 | Add a boundary test: `run.rs` has no `use crate::state` | `crates/quantrule-model/tests/boundaries.rs` | Test | 1.4 |
| 1.8 | Run model tests, clippy, doc; compare golden fixtures | repository | Test/Code Reviewer | 1.6 |

**Phase 1 acceptance:** as §6.3. **Rollback:** revert the Phase 1 commits; no data or schema changes.

### Phase 2 — Invert the backtest dependency

| Step | Work | Paths | Role | Depends on |
|---|---|---|---|---|
| 2.1 | Add `side.rs` with `Side`; move the enum from `strategy/decision.rs`; update re-exports | `crates/quantrule-backtest/src/side.rs`, `strategy/decision.rs`, `strategy/mod.rs`, `lib.rs` | Implementation | Phase 1 |
| 2.2 | Update `strategy`, `execution`, and `portfolio` to use `crate::side::Side` | `strategy/*`, `execution/*`, `portfolio/*` | Implementation | 2.1 |
| 2.3 | Add execution-owned `OrderIntent`/`SubmitRequest` carrying side, bar index, open time, and enter/exit kind | `crates/quantrule-backtest/src/execution/intent.rs`, `execution/mod.rs` | Implementation | 2.2 |
| 2.4 | Change `ExecutionEngine::submit` to take the intent; preserve the bar/time mismatch checks and rejection behavior | `execution/engine.rs` | Implementation | 2.3 |
| 2.5 | Map `Decision` → `OrderIntent` in the runner; keep the decision validation order | `runner/mod.rs` | Implementation | 2.4 |
| 2.6 | Add a boundary test: no `use crate::strategy` in `execution` or `portfolio` | `crates/quantrule-backtest/tests/boundaries.rs` | Test | 2.5 |
| 2.7 | Run backtest tests and the golden run; compare fills, rejections, metrics | repository | Test/Code Reviewer | 2.6 |

**Phase 2 acceptance:** as §6.4. **Rollback:** revert the Phase 2 commits; behavior is unchanged, so no fixture migration.

### Phase 3a — One shell pipeline and one ladder source

| Step | Work | Paths | Role | Depends on |
|---|---|---|---|---|
| 3a.1 | Add `pipeline.rs` with `LoadedSet`, `load_set`, `ModelInputs` (four owned candle vectors + `[TimeframeInput; 4]`), `run_model`, `run_backtest` | `crates/quantrule-shell/src/pipeline.rs`, `main.rs` | Implementation | Phases 1–2 |
| 3a.2 | Replace `command/model.rs::model_run_from_set*` with `pipeline` | `command/model.rs` | Implementation | 3a.1 |
| 3a.3 | Replace the `layers.rs` four-input construction with `pipeline::ModelInputs` | `command/layers.rs` | Implementation | 3a.1 |
| 3a.4 | Replace `report_run.rs::run_pipeline`/`load_set` with `pipeline` | `command/report_run.rs` | Implementation | 3a.1 |
| 3a.5 | Replace `strategy.rs` source/model construction with `pipeline`; use `run.model` | `command/strategy.rs` | Implementation | 3a.1 |
| 3a.6 | Replace `LADDER` consts with `Timeframe::ALL`; validate `config/data.rs` codes against `Timeframe::code()` | `command/report.rs`, `command/report_run.rs`, `config/data.rs` | Implementation | 3a.1 |
| 3a.7 | Add a test with a non-default `structure_scale`: `strategy-run` decisions match `report strategy` | `crates/quantrule-shell/tests/strategy_run.rs`, config fixture | Test | 3a.5 |
| 3a.8 | Run the shell suite; record the intentional `strategy-run` behavior change | repository | Test/Code Reviewer | 3a.7 |

**Phase 3a acceptance:** as §6.5. **Rollback:** revert Phase 3a; the config fix is the only behavior change.

### Phase 3b — One strategy walk

| Step | Work | Paths | Role | Depends on |
|---|---|---|---|---|
| 3b.1 | Add `StrategyWalk` (inputs, decisions, derived counts) in `strategy` or `runner`; document it | `crates/quantrule-backtest/src/strategy/walk.rs` or `runner/walk.rs` | Implementation | Phase 3a |
| 3b.2 | Have `run_backtest` build the walk once; expose it from `BacktestRun` | `runner/mod.rs`, `strategy/mod.rs` | Implementation | 3b.1 |
| 3b.3 | Delete `report_run::run_strategy`; build the strategy bundle from `BacktestRun`’s walk | `command/report_run.rs`, `command/report_run/bundles.rs` | Implementation | 3b.2 |
| 3b.4 | Delete `command/strategy.rs::collect_records`; use `runner::walk_strategy` for the diagnostic command | `command/strategy.rs`, `runner/mod.rs` | Implementation | 3b.2 |
| 3b.5 | Add the agreement test: `report strategy` decisions == `strategy-run` decisions for the same config/data | `crates/quantrule-shell/tests/` | Test | 3b.3, 3b.4 |
| 3b.6 | Run the full suite; confirm AUD-02 remains open | repository | Test/Code Reviewer | 3b.5 |

**Phase 3b acceptance:** as §6.6. **Rollback:** revert Phase 3b; the report returns to the second walk.

### Phase 3c — AUD-2026-10-30-02

This phase requires a human decision and the quant roles. Do not start it as part of a behavior-preserving refactor.

**Option A — reconcile fills and rejections**

- Add `Strategy::on_fill(&Fill)` and `Strategy::on_reject(&Rejection)`.
- Feed each execution batch back to the strategy after `advance`; the strategy position reflects actual fills.
- Validate `Exit.side` against the open position side; reject a mismatch.
- Tests: `allow_short=false` run; reported entries == fills + rejected entries; no virtual short; exit side checked.

**Option B — stateless strategy (recommended target)**

- Change the strategy contract to `decide(&StrategyInput, position: Option<Side>) -> Decision`.
- Remove the strategy’s position field.
- The runner passes the current portfolio/execution position at the close.
- `strategy-run` uses a clearly labeled virtual-fill tracker, or reports decisions with the position supplied by a diagnostic adapter.
- Tests: the same `allow_short=false` scenario; no divergence; exit side checked.

| Step | Work | Paths | Role | Depends on |
|---|---|---|---|---|
| 3c.1 | Record the chosen option and the quant hypothesis/protocol | plan + AI log + PR | Quant Researcher | human decision |
| 3c.2 | Implement the chosen reconciliation; keep the model/strategy contract docs in sync | `crates/quantrule-backtest/src/strategy/*`, `runner/*`, shell reports | Quant Developer | 3c.1 |
| 3c.3 | Add the fail-before/pass-after regression test | `crates/quantrule-backtest/tests/`, shell tests | Test/Quant Developer | 3c.2 |
| 3c.4 | Reproduce from the recorded data/config; check look-ahead, costs, and state | repository | Quant Validator | 3c.3 |
| 3c.5 | Update AUD-02 to `Fixed` in the PR; the Improvement Verifier marks `Verified` after merge | audit doc + PR | Improvement Verifier | merge |

**Phase 3c acceptance:** as §6.7. **Rollback:** revert Phase 3c; AUD-02 returns to `Open`.

### Phase 4 — Slim the shell command subsystem

| Step | Work | Paths | Role | Depends on |
|---|---|---|---|---|
| 4.1 | Move the 19 args structs from `types.rs` into `command/args/<family>.rs`; keep `Command`, `ParseError`, `RunError`, shared enums | `command/types.rs`, `command/args/*` | Implementation | Phase 3b |
| 4.2 | Move `Axis` to `render_state` (or a shell `vocabulary` module); update `command/types.rs` and `command/model.rs` | `render_state.rs`, `command/types.rs`, `command/model.rs` | Implementation | 4.1 |
| 4.3 | Define `LabSummaryRequest` in `lab_summary`; construct it in `command/lab.rs`; remove the `command` import | `lab_summary.rs`, `command/lab.rs` | Implementation | 4.1 |
| 4.4 | Add a `FlagReader`/`Args` helper for repeated flag handling; preserve the exact error strings; add parser tests | `command/parse.rs`, `command/*.rs` | Implementation | 4.1 |
| 4.5 | Split `report_run.rs` and `bundles.rs` into `report/bundles/{model,strategy,execution,portfolio,analytical}.rs` + `common.rs` | `command/report_run*` | Implementation | 4.2 |
| 4.6 | Split `tests/cli.rs` by family; share helpers through `tests/common/mod.rs` | `crates/quantrule-shell/tests/` | Test | 4.5 |
| 4.7 | Add a help/dispatch/Command consistency test, or document why it is impractical | shell tests | Test | 4.6 |
| 4.8 | Run the full shell suite; compare CLI output byte for byte | repository | Test/Code Reviewer | 4.7 |

**Phase 4 acceptance:** as §6.8. **Rollback:** revert Phase 4; the command contract is unchanged.

### Phase 5 — Hygiene and documentation

| Step | Work | Paths | Role | Depends on |
|---|---|---|---|---|
| 5.1 | Split `research/columns.rs` per layer; keep the registry order | `crates/quantrule-analytics/src/research/columns/*` | Implementation | Phase 4 |
| 5.2 | Move `codes_csv` to a static code table module | `crates/quantrule-analytics/src/research/codes.rs`, `metadata.rs` | Implementation | 5.1 |
| 5.3 | Remove `#![allow(dead_code, unused_imports)]`; delete unused re-exports; correct the stale module doc | `crates/quantrule-shell/src/config/mod.rs` | Implementation | Phase 4 |
| 5.4 | Add `quantrule-backtest` and its role to `AGENTS.md` §2 | `AGENTS.md` | Docs | Phase 0 |
| 5.5 | Split `lab_summary.rs` into read/text/write units | `crates/quantrule-shell/src/lab_summary/*` | Implementation | 5.3 |
| 5.6 | Optional: move pure `store/ingest/validate.rs` validation into `quantrule-analytics`; keep SQL in the shell | `store/ingest/validate.rs`, `analytics` | Architect/Implementation | 5.5 |
| 5.7 | Run fmt, clippy, tests, doc; update the AI log | repository | Test/Release | all |

**Phase 5 acceptance:** as §6.9. **Rollback:** revert Phase 5; no behavior change.

## 8. Verification

### 8.1 Commands

Run after every phase, from the phase worktree:

| Check | Command | Expected |
|---|---|---|
| Format | `cargo fmt --all --check` | no diff |
| Lints | `cargo clippy --all-targets --all-features` | no warnings |
| Tests | `cargo test --workspace` | all pass |
| Docs | `cargo doc --workspace --no-deps` | no warnings |
| Model goldens | `cargo test -p quantrule-model` | golden fixtures unchanged |
| Backtest goldens | `cargo test -p quantrule-backtest` | golden run unchanged |
| Shell CLI | `cargo test -p quantrule-shell --test cli` | byte-identical output |
| Boundaries | `cargo test -p quantrule-model --test boundaries`, `cargo test -p quantrule-backtest --test boundaries`, `cargo test -p quantrule-shell --test architecture` | pass |
| Fixture diff | `git diff --exit-code -- crates/*/tests` | no unexpected fixture change |
| Smoke | `cargo run -p quantrule-shell -- market-state <fixture>`; `cargo run -p quantrule-shell -- backtest --config <fixture> --out <tmp>` | succeeds; output matches the expected shape |

### 8.2 Evidence policy

- Record the actual command output in the phase PR and the AI transparency log. Do not claim a check that was not run.
- Phases 0–2: any golden fixture diff is a stop condition. Do not regenerate fixtures to make the refactor pass.
- Phase 3a: the only intended change is `strategy-run` honoring the run model config. Add a new fixture or a focused test; do not rewrite unrelated goldens.
- Phase 3c: golden changes are deliberate. Show the before/after values, the reason, and the Quant Validator’s reproduction record.
- Phase 4–5: CLI output is byte-identical; test the error paths, not just the happy path.
- No network, no live market data, no wall clock.

### 8.3 Boundary tests

Add source-scan tests inside the repository (std only, no external I/O):

- `quantrule-model/tests/boundaries.rs`: `run.rs` contains no `use crate::state`.
- `quantrule-backtest/tests/boundaries.rs`: `src/execution/*.rs` and `src/portfolio/*.rs` contain no `use crate::strategy`.
- `quantrule-shell/tests/architecture.rs`: `lab_summary.rs` and `render_state.rs` contain no `use crate::command`.

If a source-scan test is considered too brittle, replace it with a compile-fail or visibility design that makes the forbidden edge impossible. Do not add a dependency for this.

## 9. Risks and recovery

| Risk | Likelihood | Impact | Mitigation | Recovery |
|---|---|---|---|---|
| Model numeric drift during the module split | Medium | High | Golden fixtures; move code without edits; compare byte for byte | Revert the Phase 1 commit |
| Backtest fill/rejection drift during the intent mapping | Medium | High | Golden run; engine tests; preserve validation order | Revert the Phase 2 commit |
| CLI error-text drift during parser consolidation | Medium | Medium | Parser table tests; exact strings; CLI suite | Revert the Phase 4 commit |
| Missed internal API consumer | Medium | Medium | `cargo check --workspace`; grep all crates; update in the same PR | Fix forward in the same phase |
| Merge conflicts across phases | Medium | Medium | Sequential phases; branch from latest `main`; one worktree | Re-plan the order; do not rebase published branches |
| Scope creep into other audit findings | High | Medium | Explicit out-of-scope list; stop conditions | Split into a new plan |
| AUD-02 option B changes results unexpectedly | Medium | High | Quant Researcher protocol; Quant Validator reproduction; deliberate fixture update | Revert Phase 3c; keep AUD-02 Open |
| Performance regression from extra copies | Low | Low | No hot-path change expected; inspect allocations if benchmarks exist | Optimize only in a separate plan |
| Docs drift | Medium | Low | Update docs in the same PR; `cargo doc` | Fix in the same phase |
| Toolchain mismatch | Low | Medium | Record `rustc --version`; avoid version-specific features | Adjust syntax; no feature changes |

## 10. Stop conditions

Stop and request human direction if any of these occurs:

1. A golden fixture changes unexpectedly in Phase 0–2.
2. A phase needs a new dependency, a new crate, or a new `unsafe` block.
3. A phase needs to change a command name, argument, output, or error contract.
4. A phase needs to change a policy/strategy/config schema.
5. A phase needs to change model or strategy behavior outside Phase 3a/3c.
6. A phase conflicts with an accepted ADR.
7. A test or clippy failure cannot be fixed within the phase scope.
8. A file outside the listed paths must change for a reason not in the plan.
9. The human has not approved commits, push, or the PR.
10. AUD-02’s option is not chosen before Phase 3c.
11. The `strategy-run` config fix is disputed.
12. The Quant Validator record is missing for Phase 3c.

## 11. Traceability

| Finding | Phase | Acceptance | Verification |
|---|---|---|---|
| F1 shell strategy loops | 3b, 3c | No production `on_bar` in shell; one walk; AUD-02 regression | Shell tests; backtest regression; Quant Validator |
| F2 execution/portfolio → strategy | 2 | No `use crate::strategy` in execution/portfolio; `submit` takes no `Decision` | Boundary test; backtest goldens |
| F3 model run ↔ state | 1 | No `run -> state`; `state` is a view | Boundary test; model goldens |
| F4 command hub and cycles | 4 | Args split; no `lab_summary`/`render_state` → `command` | Architecture test; CLI suite |
| F5 ladder/input duplication | 3a | One pipeline; no `LADDER`; `strategy-run` uses `run.model` | Shell tests; config fixture |
| F6 oversized files/functions | 4, 5 | No file > 1,000 lines; parameter objects where needed | Review; `cargo clippy` |
| F7 blanket lint allow | 5 | Allow removed; no unused re-exports | `cargo clippy`; review |
| F8 AGENTS crate list | 5 | Backtest crate documented | Doc review |
| AUD-2026-10-30-02 | 3c | Fail-before/pass-after; no virtual short | Quant Validator; regression test |

## 12. Roles and approvals

| Role | Responsibility |
|---|---|
| Orchestrator | Sequence phases, collect evidence, report contradictions, stop at gates |
| Software Architect | Review the target module boundaries and the Phase 1/2 dependency changes |
| Implementation | Make the code changes within the approved phase |
| Test Engineer | Boundary, golden, CLI, and regression tests; report what each test actually proves |
| Code Reviewer | Review the full diff; correctness, scope, and conventions |
| Quant Researcher | Phase 3c hypothesis, option, and evaluation protocol |
| Quant Developer | Phase 3c strategy/execution implementation |
| Quant Validator | Phase 3c independent reproduction and validation record |
| Release/Ops | Run the verification commands; no deployment |
| Human | Approve the plan, Phase 3c option, commits, push, PR, merge, and acceptance |

**Approval gates:**

- Plan of record approval.
- Phase 3c option and quant protocol.
- Commits, push, PR creation, and merge (AGENTS §9).
- Any new dependency or command-contract change (not planned).

## 13. Sequence and estimate

| Phase | Branch (proposed) | Depends on | Estimate |
|---|---|---|---:|
| 0 Records | `docs/architecture-hardening` | — | 0.5 day |
| 1 Model de-cycling | `refactor/model-decycle` | 0 | 1–2 days |
| 2 Backtest inversion | `refactor/backtest-inversion` | 0 | 1–2 days |
| 3a Pipeline/inputs | `refactor/pipeline-inputs` | 1, 2 | 1 day |
| 3b Strategy walk | `refactor/strategy-walk` | 3a | 1 day |
| 3c AUD-02 | `fix/aud-2026-10-30-02-strategy-state` | 3b + human decision + quant roles | 1–2 days |
| 4 Shell slimming | `refactor/shell-command-slimming` | 3b | 2–3 days |
| 5 Hygiene/docs | `chore/architecture-hygiene` | 4 | 1 day |

Total: **8–12 working days**, sequential. Phase 1 and Phase 2 may run in parallel only with explicit authorization; they touch disjoint crates.

**Worktree:** `../quant-rule--architecture-hardening`. Reuse it sequentially; do not switch a branch another session is using. Merge each phase before creating the next branch.

## 14. Artifacts

- `docs/audits/2026-10-30/architecture-audit.md`
- `docs/execution-plans/2026-10-30/architecture-hardening.md`
- `docs/ai-transparency-log/2026-10-30/architecture-hardening.md`
- Phase PRs with the verification table and observed results.
- Phase 3c Quant Validator record (in the PR and the plan evidence; optionally a standalone `docs/audits/2026-10-30/aud-2026-10-30-02-validation.md`).
- Updated `AGENTS.md` in Phase 5.

## 15. Open questions

1. **Issue/board:** create a tracking issue and sub-issues for the phases? If yes, create them after plan approval and move each to `In Progress`/`Done` per AGENTS §9. If no, the PRs reference this plan and the board is not used.
2. **AUD-02 option:** A (reconcile) or B (stateless strategy)? Recommendation: B as the target, A as the interim. This is a human decision and needs the quant roles.
3. **Parallel Phase 1/2:** default sequential; confirm if parallel work is authorized.
4. **Branch slugs:** confirm the proposed slugs (no issue number exists). If an issue is created, rename to `<issue>-<slug>`.
5. **`strategy-run` config fix:** confirm that using `run.model` is intended. If not, keep the default and document the divergence instead of changing behavior.

## 16. Implementation evidence (2026-10-30)

| Phase | Status | Evidence |
|---|---|---|
| 0 Records | Complete | `6874e28`; baseline `cargo test --workspace` and `cargo clippy --all-targets` passed before source changes |
| 1 Model de-cycling | Complete | `cc6afe6`; `run` no longer imports `state`; 442 model unit tests, model integration suites, and the boundary tests passed |
| 2 Backtest inversion | Complete | `cf5df38`; no `use crate::strategy` in execution/portfolio; 152 backtest unit tests, integration suites, and boundary tests passed |
| 3a Shell pipeline | Complete | `ec11683`; one `pipeline` input builder; no `LADDER` const; `strategy-run` config test passed |
| 3b One strategy walk | Complete | `7276c6d`; no production `.on_bar(` in the shell; `report strategy` and `strategy-run` agree |
| 3c AUD-2026-10-30-02 | Complete | Stateless strategy (option B) implemented; validation record at `docs/audits/2026-10-30/aud-2026-10-30-02-validation.md`; the executed walk has no virtual short |
| 4 Shell slimming | **Partial** | `f8d743f`; renderer cycles broken and architecture guards added. Args split, bundle split, and CLI-test split are deferred |
| 5 Hygiene and docs | **Partial** | `6640066`, `3db20e5`; config allow removed, AGENTS crate list updated, columns split. `lab_summary` split and the optional store-validation move are deferred |

### AUD-02 quant validation

The human selected the recommended option B (stateless strategy). The Quant
Researcher, Quant Developer, and Quant Validator passes were performed
sequentially by one agent; this is not an independent-agent review. The
validation record is `docs/audits/2026-10-30/aud-2026-10-30-02-validation.md`.

Observed on a full-year `allow_short=false` run:

- strategy decisions: 7,565; entry decisions: 7,528 (3,509 long, 4,019 short);
- execution: 43 entries, 43 exits, 86 fills, 7,485 `missing_quantity` rejections;
- portfolio: 43 long trades, 0 short trades, open position null;
- strategy bundle open position: null.

The executed walk never carries a short position and never emits a short exit.
The decision-only `report strategy`/`strategy-run` views still list short entry
decisions, which the execution bundle reports as rejected. That residual is a
reporting distinction, not a virtual-position divergence.

### Verification run

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets`: no warnings.
- `cargo test --workspace`: all suites passed, including 442 model unit tests, 152 backtest unit tests, 272 shell unit tests, 99 CLI tests, and the new boundary/architecture tests.
- `cargo doc --workspace --no-deps`: passed.
- No dependency was added; `Cargo.toml` is unchanged.
- No golden fixture was regenerated.
