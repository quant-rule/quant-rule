# Architecture Audit

- Date: 2026-10-30 (UTC)
- Audit ID: AUDIT-2026-10-30-architecture
- Status: Published
- Auditor: Audit Agent (AI), sequential audit pass; not an independent model judgment
- Baseline: `main` at `9f0d0d5`, clean working tree
- Toolchain: Rust 1.99.1, edition 2024
- Scope: module boundaries, atomicity, coupling, cohesion, cycle freedom, and shell/engine separation
- Limitations: static inspection only; the test suite was not run for this audit; the findings are reproducible from the named files and commands. The other open findings in `docs/audits/2026-10-30/mvp-pre-release-audit.md` are out of scope.

## Summary

The crate graph is clean and acyclic: `data ← model ← {analytics, backtest} ← shell`. The engine crates are pure and have no external dependencies. The architectural risk is inside the two largest crates: `quantrule-model` has a real `run ↔ state` module cycle, and `quantrule-shell` is a 30k-line mini-monolith that runs strategy logic and maintains two module cycles. `quantrule-backtest` also has a dependency inversion: execution and portfolio import the higher-level strategy module.

The workspace is not a ball of mud at the crate level. It is at risk of becoming one at the module level if these seams are left in place.

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| ARCH-2026-10-30-01 | Critical | shell | The shell runs the strategy itself in two places; the runner runs it a third time | `crates/quantrule-shell/src/command/report_run.rs:319`; `crates/quantrule-shell/src/command/strategy.rs:263`; `crates/quantrule-backtest/src/runner/mod.rs:138`; AUD-2026-10-30-02 reports 166 strategy entries vs 93 executed | Shell holds domain logic; the reported strategy can diverge from the executed strategy | A. Move the walk into `BacktestRun` and delete both shell loops; B. Keep the shell loops and reconcile their state; C. Report both states explicitly | A | No production `on_bar` in the shell; `report strategy` and `strategy-run` agree; AUD-02 regression | Planned | [`plan`](../../execution-plans/2026-10-30/architecture-hardening.md) |
| ARCH-2026-10-30-02 | Major | backtest | `execution` and `portfolio` depend upward on `strategy` | `crates/quantrule-backtest/src/execution/engine.rs:80`; `execution/position.rs:10`; `portfolio/state.rs:10`; `portfolio/sizing.rs:13`; `portfolio/ledger.rs:22`; `ExecutionEngine::submit` accepts `Decision` at `execution/engine.rs:290` | The decision stack is inverted; execution cannot be tested or reused without strategy | A. Neutral `side` module plus execution-owned `OrderIntent`; B. Move `Decision` into a shared vocabulary module; C. Leave it and document the coupling | A | No `use crate::strategy` in execution/portfolio; backtest golden run unchanged | Planned | [`plan`](../../execution-plans/2026-10-30/architecture-hardening.md) |
| ARCH-2026-10-30-03 | Major | model | `model::run` and `model::state` form a module cycle | `crates/quantrule-model/src/run.rs:32` imports `state::{MarketStateConfig, TimeframeConsensus, TimeframeInput}`; `state.rs:19` imports `run::ModelRun`; `state.rs:181` `from_run` | The modules cannot be understood or tested independently; `state` mixes input/config types with a view | A. Extract `input` and `consensus`, split `timeframe`, keep `state` as a view; B. Move `ModelRun` into `state` and make one larger module; C. Add a facade and leave the cycle | A | No `run -> state` import; model golden fixtures unchanged | Planned | [`plan`](../../execution-plans/2026-10-30/architecture-hardening.md) |
| ARCH-2026-10-30-04 | Major | shell | `command` is a 9,437-line hub with two module cycles | `crates/quantrule-shell/src/command/types.rs` imports ~12 subsystems; `command/lab.rs:204` + `lab_summary.rs:17`; `command/model.rs:16` + `render_state.rs:19`; 63 `Command` variants, 17 parsers, 84 repeated parser error strings | Any subsystem change touches the hub; cycles block independent units; help/dispatch/parser drift | A. Split args per family, move shared vocabulary, break cycles; B. Extract a command crate; C. Keep the hub and add tests only | A | `types.rs` under 500 lines; no `lab_summary`/`render_state` → `command`; CLI output byte-identical | Planned | [`plan`](../../execution-plans/2026-10-30/architecture-hardening.md) |
| ARCH-2026-10-30-05 | Major | shell | Ladder and model-input construction are duplicated; `strategy-run` ignores the run model config | Four `TimeframeInput` sites at `command/layers.rs:43`, `command/model.rs:337`, `command/report_run.rs:571`, `command/strategy.rs:222`; `LADDER` at `command/report.rs:25`, `command/report_run.rs:33`, `config/data.rs:20`; `command/strategy.rs:233` uses `MarketStateConfig::default()`; `Timeframe::ALL` exists at `data/timeframe.rs:21` | A ladder or config change silently diverges between commands | A. One shell `pipeline::ModelInputs`, one ladder source, use `run.model`; B. Duplicate a helper per command; C. Move the ladder into the shell config only | A | No `TimeframeInput` literal under `command/`; no `LADDER` const; `strategy-run` test with non-default config | Planned | [`plan`](../../execution-plans/2026-10-30/architecture-hardening.md) |
| ARCH-2026-10-30-06 | Minor | analytics/shell | Oversized modules and functions | `research/columns.rs` 1,706 lines; `command/report_run/bundles.rs` 1,274; `shell/tests/cli.rs` 1,991; `codes_csv` 408-line function; `compute_with_config` 200; `validate_cross` 163; the free `snapshot_at` has 12 parameters | Review and test friction; violates the 1,000-line guideline | A. Split by bundle/layer/family; B. Leave and justify; C. Split only the largest | A | No source file over 1,000 lines; parameter objects where needed | Planned | [`plan`](../../execution-plans/2026-10-30/architecture-hardening.md) |
| ARCH-2026-10-30-07 | Minor | shell | Blanket lint suppression hides dead code | `config/mod.rs:14` `#![allow(dead_code, unused_imports)]`; seven of nine re-exports unused outside `config`; module doc mentions stale issue numbers | Dead code passes clippy; stale documentation survives | A. Remove the allow and delete unused re-exports; B. Keep it and add a comment; C. Replace with item-level allows | A | No blanket allow; no unused re-exports; clippy clean | Planned | [`plan`](../../execution-plans/2026-10-30/architecture-hardening.md) |
| ARCH-2026-10-30-08 | Minor | docs | `AGENTS.md` omits `quantrule-backtest` | `AGENTS.md` §2 crate list; `Cargo.toml` workspace members | Agents and readers miss a pipeline stage | A. Add the crate and its role; B. Leave it to the ADRs | A | `AGENTS.md` lists the crate | Planned | [`plan`](../../execution-plans/2026-10-30/architecture-hardening.md) |

## Detail

### ARCH-2026-10-30-01 — Shell strategy walks

- Expected: one strategy walk in the workspace; the shell renders published decisions.
- Observed: `report_run::run_strategy` and `strategy::collect_records` both instantiate `AxisAgreementStrategy` and call `on_bar`; `run_backtest` does the same walk for execution. The report and the execution can disagree (AUD-2026-10-30-02).
- Evidence: the three call sites named in the table; the existing MVP audit's `allow_short=false` reproduction.
- Options: A is the structural fix; B preserves the divergence and adds reconciliation; C reports two states but leaves the shell loop in place.
- Recommended: A. Expose one `StrategyWalk` from `BacktestRun`; use it for the strategy bundle and the diagnostic command.
- Verification: no production `on_bar` in the shell; a test asserts `report strategy` and `strategy-run` produce the same decisions for the same config/data; the AUD-02 regression test is added in the separate behavior-change phase.

### ARCH-2026-10-30-02 — Backtest dependency inversion

- Expected: execution and portfolio depend only on their own vocabulary and the model's published values.
- Observed: both import `crate::strategy::Side`; `ExecutionEngine::submit` takes `Decision`.
- Evidence: the five import sites and the `submit` signature.
- Options: A removes the edge with a neutral `Side` and an execution-owned `OrderIntent`; B moves `Decision` into a shared module but still lets execution know strategy decisions; C documents the coupling.
- Recommended: A. The runner remains the composition point that maps decisions to intents.
- Verification: boundary test; the full backtest golden run is byte-identical.

### ARCH-2026-10-30-03 — Model run/state cycle

- Expected: `state` is a view over `run`; input/config types have their own module.
- Observed: `run` imports three types from `state`; `state` imports `ModelRun`.
- Evidence: the imports and `MarketState::from_run`.
- Options: A extracts `input`/`consensus` and splits `timeframe`; B merges the modules; C adds a facade without removing the cycle.
- Recommended: A. The model contract keeps `MarketState` as a view; the numeric code is moved without edits.
- Verification: no `run -> state`; model golden fixtures byte-identical.

### ARCH-2026-10-30-04 — Command hub

- Expected: command families own their arguments; renderers do not depend on the parser.
- Observed: `types.rs` is the shared hub; `lab_summary` and `render_state` import `command`.
- Evidence: the imports and the counts in the table.
- Options: A splits args and moves shared vocabulary; B extracts a command crate; C keeps the hub.
- Recommended: A. A new crate would add a dependency edge without removing the conceptual coupling.
- Verification: CLI output byte-identical; no `lab_summary`/`render_state` → `command`.

### ARCH-2026-10-30-05 — Ladder and input duplication

- Expected: one source of truth for the timeframe ladder and one model-input builder.
- Observed: four `TimeframeInput` construction sites, three ladder declarations, and a default model config in `strategy-run` despite loading a run config.
- Evidence: the sites in the table.
- Options: A creates `pipeline::ModelInputs`, uses `Timeframe::ALL`, and honors `run.model`; B duplicates a helper per command; C leaves the config divergence.
- Recommended: A.
- Verification: a non-default `structure_scale` test proves `strategy-run` and `report strategy` agree.

### ARCH-2026-10-30-06 — Oversized units

- Expected: a cohesive module under 1,000 lines where practical.
- Observed: the named files and functions exceed the guideline.
- Options: A splits along existing seams; B justifies the size; C splits only the largest.
- Recommended: A, in the hygiene phase, after the dependency fixes.
- Verification: no source file over 1,000 lines; behavior unchanged.

### ARCH-2026-10-30-07 — Blanket allow

- Expected: no blanket suppression; unused items are removed.
- Observed: `config/mod.rs:14`; seven unused re-exports.
- Options: A removes the allow and the re-exports; B keeps it; C uses item-level allows.
- Recommended: A.
- Verification: clippy clean; no unused re-exports.

### ARCH-2026-10-30-08 — Crate list

- Expected: `AGENTS.md` names every workspace crate and its role.
- Observed: `quantrule-backtest` is missing.
- Options: A adds it; B relies on the ADRs.
- Recommended: A.
- Verification: doc review.

## Traceability

| Finding | Plan phase | Regression guard |
|---|---|---|
| ARCH-01 | Phase 3b, 3c | Shell/strategy agreement test; AUD-02 fail-before/pass-after |
| ARCH-02 | Phase 2 | Backtest boundary test; golden run |
| ARCH-03 | Phase 1 | Model boundary test; model goldens |
| ARCH-04 | Phase 4 | Shell architecture test; CLI suite |
| ARCH-05 | Phase 3a | Non-default config test; input-construction grep |
| ARCH-06 | Phase 4, 5 | File-size review; clippy |
| ARCH-07 | Phase 5 | Clippy; review |
| ARCH-08 | Phase 5 | Doc review |
