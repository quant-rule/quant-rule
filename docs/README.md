# QuantRule Documentation

Work-item documents use `docs/<kind>/<YYYY-MM-DD>/<file>.md`: the kind comes first, then the UTC date of the work item, then the file. Living system documents sit directly under their kind, as `docs/architecture/<file>.md`. A kind directory appears when it is first used.

## Kinds

| Kind | Holds | Path |
|---|---|---|
| Execution plans | The plan for a substantial work item, per `AGENTS.md` section 11. | `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md` |
| AI transparency log | The record of material AI contributions for a work item, per `AGENTS.md` section 6. One file per work item, named to match its execution plan. | `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md` |
| Agent roles | The quant and engineering roles, their handoffs, and when each is required. | `docs/roles/<YYYY-MM-DD>/<file>.md` |
| Architecture | The living system documentation: overview, product kata, architecture characteristics, style/quantum, and cross-cutting views. | `docs/architecture/<file>.md` |
| Tutorials | End-to-end procedures that run the product through shell commands, including role labs. | `docs/tutorials/<YYYY-MM-DD>/<file>.md` |
| Model | The analytical model's contract: the indicator and event definitions, the numeric output fields, the progression rules, and the validation boundary. The implementation follows it. | `docs/model/<YYYY-MM-DD>/<file>.md` |
| Analytics and research | The research output contract and the dataset definition: layers, columns, null rules, alignment, and export metadata. | `docs/analytics/<YYYY-MM-DD>/<file>.md` |
| Configuration | The per-engine JSON configuration: files, fields, validation, and cross-config rules. | `docs/config/<YYYY-MM-DD>/<file>.md` |
| Reports | The per-engine report bundles: artifacts, JSON and CSV schemas, boundaries, and availability. | `docs/reports/<YYYY-MM-DD>/<file>.md` |
| Releases | The notes for a tagged release: commands, configuration, reports, accounting rules, and evidence. | `docs/releases/<YYYY-MM-DD>/<file>.md` |
| Architecture decision records | A decision, the alternatives considered, and the consequences. | `docs/adr/<YYYY-MM-DD>/<file>.md` |
| Audits | Read-only findings, evidence, and remediation options. | `docs/audits/<YYYY-MM-DD>/<scope>-audit.md` |
| Governance | The improvement charter, green criteria, autonomy, and stop conditions. | `docs/governance/<YYYY-MM-DD>/<file>.md` |
| Workflows | The living Audit Workflow, Improvement Workflow, and run contract. | `docs/workflows/<file>.md` |
| Improvement registry | The committed run index, baselines, invalidation tombstones, and run record template. | `docs/improvement/<file>.md` |

## Current architecture

| Document | Status | Path |
|---|---|---|
| System architecture entry point | Current; approved 2026-10-31 | `docs/architecture/README.md` |
| The Trustworthy Backtest Kata | Current; approved 2026-10-31 | `docs/architecture/kata.md` |
| Architecture characteristics | Current; approved 2026-10-31 | `docs/architecture/characteristics.md` |

The baseline decision is listed in the current architecture decisions below.

## Current model contracts

| Contract | Status | Path |
|---|---|---|
| Model v3 flat indicator/event model | Approved 2026-11-03; flat report, MACD, v3 config, ADX/DI, registry, and claim gate implemented; gate no edge | `docs/model/2026-11-03/model-v3.md` |
| Four-axis analytical model | Superseded for axes by model v3; retained as history | `docs/model/2026-10-21/four-axis-analytical-model.md` |
| Per-timeframe model | Superseded for axes by model v3; the per-timeframe boundary remains | `docs/model/2026-10-30/per-timeframe-model.md` |
| DashboardState four-axis CLI view | Historical; the axis view is retired with model v3 | `docs/model/2026-10-25/dashboard-state.md` |
| MarketReport full-period descriptive view | Implemented (#150); `market-report` command (#151); verified (#152); axis view retired with model v3 | `docs/model/2026-10-27/market-report.md` |
| Model evidence protocol | Superseded by the model v3 validation boundary; retained as history | `docs/model/2026-11-01/model-evidence-protocol.md` |
| Model v4 flat result and configurable events | Implemented; path gate no edge; compression-breakout 4 ATR baseline accepted | `docs/model/2026-11-04/model-v4.md` |
| Compression-range next-bar resolution | Implemented; dev and validation pass; accepted model baseline (human, 2026-11-04) | `docs/model/2026-11-04/compression-range.md` |
| Indicator coverage — 10 OHLCV areas | Implemented 2026-11-05: 46 indicators, 143 events, stable event codes, 37/37 checklist rows `verified`; spec v2 implemented; validation claims open | `docs/model/2026-11-05/indicator-coverage.md` |
| Model and strategy families | Approved 2026-11-05; `M1.0`, `S1.1–S1.5`, `M2.1–M2.5`, `S2.1+` | `docs/model/2026-11-05/model-families.md` |
| Model spec v1 | Implemented; condition-only; v2 adds readings, active events, prediction, panel/session | `docs/model/2026-11-05/model-spec-v1.md` |
| Model spec v2 | Implemented (P1); reading overrides, active events, causal vote, panel/session | `docs/model/2026-11-05/model-spec-v2.md` |
| Model prediction evaluation | Implemented (P2); forward/barrier outcomes, controls, per-instrument/per-year, grade | `docs/analytics/2026-11-05/model-prediction.md` |

## Current run contracts

| Contract | Status | Path |
|---|---|---|
| Per-engine configuration (v1) | Historical; superseded by the policy/strategy v3 contract | `docs/config/2026-10-28/per-engine-config.md` |
| Policy and strategy blueprints | Current; `run v2`, `policy v1`, `strategy v3`, `execution v2` | `docs/config/2026-10-30/policy-and-strategy.md` |
| Strategy v2 to v3 migration | Current | `docs/config/2026-10-30/migration-v2-to-v3.md` |
| Per-engine reports | Approved (#168); implemented by #173–#176; verified by #177 | `docs/reports/2026-10-28/per-engine-reports.md` |
| Cross-engine diagnosis v1 | Implemented; awaiting human acceptance | `docs/reports/2026-10-31/diagnosis-v1.md` |
| BTCUSD 2024 example run | Current example: five bundles, 29 artifacts, byte-identical rerun; captured on 2026-10-31 | `config/examples/btcusd-2024/` |
| Improvement run contract | Current; schema `quantrule.improvement-run/v1` implemented by `tools/improvement/` | `docs/workflows/run-contract.md` |
| Strategy candidates v3–v7 | Retired; historical `S1.1–S1.2`; none is a validated baseline | `docs/improvement/2026-11-01/STRATEGY-003-comparison.md` |
| Strategy v6 breakout (`S1.1`) | Historical; experimental, not validated | `docs/improvement/2026-11-01/STRATEGY-002-comparison.md` |
| Strategy v8 event composite (`S1.3`) | Historical; validation pending | `docs/execution-plans/2026-11-03/349-strategy-v8.md` |
| Strategy v9 compression breakout (`S1.4`) | Historical; `strategy_dev` no edge; validation not opened | `docs/improvement/2026-11-04/STRATEGY-V9-GATE.md` |
| Strategy v10 compression bracket (`S1.5`) | Historical; `strategy_dev` no edge; validation not opened | `docs/improvement/2026-11-04/STRATEGY-BRACKET-GATE.md` |

### Approved target contracts

The unified ladder contracts are approved under #228. The table records the
current implementation phase of each contract; the execution v3 loader schema is
the remaining pending phase.

| Contract | Status | Path |
|---|---|---|
| Available timeframes | `quantrule.data/v3`; implemented by #230 | `docs/config/2026-10-30/data-v3.md` |
| Model v3 flat indicator/event | `quantrule.model/v3`; schema approved 2026-11-03; parser in #346 | `docs/model/2026-11-03/model-v3.md` |
| Portfolio benchmarks and time metrics | Implemented 2026-11-03 (#345) | `docs/config/2026-11-03/portfolio-benchmarks.md` |
| MACD reading and events | Implemented 2026-11-03 (#347); opt-in on the legacy v2 path, active in the default config | `docs/config/2026-11-03/model-macd.md` |
| Indicator and parameter registry | Populated 2026-11-03 (#344); CI check | `docs/registry/README.md` |
| Current-architecture history index | Frozen 2026-11-05 at `32e6fb2`; P0 legacy inventory | `docs/registry/history.md` |
| Clean current architecture | P0.1 deletion manifest frozen 2026-11-05 at `8acf857`; P1–P8 gated by D1–D4 | `docs/execution-plans/2026-11-05/467-clean-current-architecture.md` |
| Per-timeframe model | `quantrule.model/v2`; superseded by v3; v2 parser retired in #346 | `docs/config/2026-10-30/model-v2.md` |
| Active ladder and context | Implemented (#233, #235); `ladder-context` is live | `docs/config/2026-10-30/strategy-v3.md` |
| Fill mechanics | `quantrule.execution/v3`; bracket resolution and risk-free config implemented (#234), v3 schema pending | `docs/config/2026-10-30/execution-v3.md` |
| Strategy/execution boundary | `TradeIntent`; implemented (#234) and wired into the unified rule (#235) | `docs/config/2026-10-30/trade-intent.md` |

## Current evidence

| Evidence | Status | Path |
|---|---|---|
| MVP v1 verification | Verified (#177); re-verified on current `main` after #189 and #196; release notes `v0.2.0` | `docs/reports/2026-10-28/mvp-v1-verification.md` |
| MODEL-2026-11-01-01 | Harness pilot; not green; no model baseline accepted | [`docs/improvement/2026-11-01/MODEL-2026-11-01-01-model-baseline.md`](improvement/2026-11-01/MODEL-2026-11-01-01-model-baseline.md) |
| MODEL-2026-11-01-02 | Multi-horizon study; not green; horizon rescue rejected | [`docs/improvement/2026-11-01/MODEL-2026-11-01-02-multi-horizon.md`](improvement/2026-11-01/MODEL-2026-11-01-02-multi-horizon.md) |
| MODEL-2026-11-01-03 | Context-conditioned study; not green; two isolated buckets only | [`docs/improvement/2026-11-01/MODEL-2026-11-01-03-context.md`](improvement/2026-11-01/MODEL-2026-11-01-03-context.md) |
| MODEL-2026-11-01-04 | Event study; not green; BTCUSD level cells only | [`docs/improvement/2026-11-01/MODEL-2026-11-01-04-events.md`](improvement/2026-11-01/MODEL-2026-11-01-04-events.md) |
| MODEL-2026-11-03-01 | v3 model gate, development; all 48 primary cells C; no edge | [`docs/improvement/2026-11-03/MODEL-2026-11-03-01-model-gate-dev.md`](improvement/2026-11-03/MODEL-2026-11-03-01-model-gate-dev.md) |
| MODEL-2026-11-03-02 | v3 model gate, validation; one B cell, not replicated; no edge | [`docs/improvement/2026-11-03/MODEL-2026-11-03-02-model-gate-validation.md`](improvement/2026-11-03/MODEL-2026-11-03-02-model-gate-validation.md) |
| PREDICTION-PILOT-RSI | EURUSD 1h RSI 30/70 reversion; primary failed development and validation; stop | [`docs/improvement/2026-11-03/PREDICTION-PILOT-RSI.md`](improvement/2026-11-03/PREDICTION-PILOT-RSI.md) |
| MACD-PILOT-XAG | XAGUSD 1h MACD line/signal cross; primary failed development and validation; stop | [`docs/improvement/2026-11-03/MACD-PILOT-XAG.md`](improvement/2026-11-03/MACD-PILOT-XAG.md) |
| STRUCTURE-PILOT-SPX | ESANDP500 1h confirmed pivot Level test; primary failed development and validation; stop | [`docs/improvement/2026-11-03/STRUCTURE-PILOT-SPX.md`](improvement/2026-11-03/STRUCTURE-PILOT-SPX.md) |
| PATH-PILOT-COPPER | COPPERCMD 1h confirmed pivot Level path; 1 ATR target before 1 ATR stop; primary failed development and validation; stop | [`docs/improvement/2026-11-03/PATH-PILOT-COPPER.md`](improvement/2026-11-03/PATH-PILOT-COPPER.md) |
| STRATEGY-001 | First strategy comparison; not green; no strategy baseline | [`docs/improvement/2026-11-01/STRATEGY-001-comparison.md`](improvement/2026-11-01/STRATEGY-001-comparison.md) |
| STRATEGY-002 | 5m/15m sample-size test; not green; v6 still too selective | [`docs/improvement/2026-11-01/STRATEGY-002-comparison.md`](improvement/2026-11-01/STRATEGY-002-comparison.md) |
| STRATEGY-003 | Level-fade v7 validation; not green; new rule family rejected | [`docs/improvement/2026-11-01/STRATEGY-003-comparison.md`](improvement/2026-11-01/STRATEGY-003-comparison.md) |
| PROGRAM-2026-11-05-03 | M2.x 1h panel; 10 exploratory + 5 primary cells; no primary passed; no edge; validation not opened | [`docs/improvement/2026-11-05/M2-DEV-SWARM.md`](improvement/2026-11-05/M2-DEV-SWARM.md) |
| MODEL-2026-11-04-04..05 | 1m 4 ATR compression baseline; both partitions pass; accepted | [`docs/improvement/2026-11-04/COMPRESSION-BASELINE-CANDIDATE.md`](improvement/2026-11-04/COMPRESSION-BASELINE-CANDIDATE.md) |
| MODEL-2026-11-04-06..07 | 1m compression-range next-bar resolution; both partitions pass; accepted |
| MODEL-2026-11-04-08 | 5m compression-range next-bar resolution; back-lag control fails; no edge |
| MODEL-2026-11-04-10..11 | 5m fresh-onset range; both partitions pass; accepted |
| MODEL-2026-11-04-12 | 5m onset barrier path; momentum and fade both no edge | [`docs/improvement/2026-11-04/COMPRESSION-RANGE-CANDIDATE.md`](improvement/2026-11-04/COMPRESSION-RANGE-CANDIDATE.md) |
| STRATEGY-2026-11-04-01 | Directional 1m compression strategy; `strategy_dev` no edge | [`docs/improvement/2026-11-04/STRATEGY-V9-GATE.md`](improvement/2026-11-04/STRATEGY-V9-GATE.md) |
| STRATEGY-2026-11-04-02 | 1m compression bracket; `strategy_dev` no edge | [`docs/improvement/2026-11-04/STRATEGY-BRACKET-GATE.md`](improvement/2026-11-04/STRATEGY-BRACKET-GATE.md) |
| STRATEGY-2026-11-04-03 | 5m fresh-onset bracket; `strategy_dev` no edge | [`docs/improvement/2026-11-04/STRATEGY-ONSET-5M-GATE.md`](improvement/2026-11-04/STRATEGY-ONSET-5M-GATE.md) |
| Workflow-reset audit | Program green; one Major fixed; Minor/Info routed | [`docs/audits/2026-11-01/workflow-reset-audit.md`](audits/2026-11-01/workflow-reset-audit.md) |

The experiment records above are pre-reset evidence. They are not improvement
runs under the run contract and are not imported as baselines.

## Current workflows

| Workflow | Status | Path |
|---|---|---|
| Audit Workflow | Current; approved 2026-11-01 | `docs/workflows/audit-workflow.md` |
| Improvement Workflow | Current; approved 2026-11-01 | `docs/workflows/improvement-workflow.md` |
| Improvement run contract | Current; schema `quantrule.improvement-run/v1` | `docs/workflows/run-contract.md` |
| Research Swarm Runbook | Current; exploratory and confirmatory model modes | `docs/workflows/research-swarm.md` |
| Improvement registry | Current; 1m 4 ATR compression, 1m compression-range, and 5m fresh-onset model baselines accepted; no strategy or portfolio baseline | `docs/improvement/README.md` |

## Current tutorials

| Tutorial | Status | Path |
|---|---|---|
| Run the unified ladder end to end | Current; verified 2026-10-30 | [`docs/tutorials/2026-10-30/unified-ladder.md`](tutorials/2026-10-30/unified-ladder.md) |
| The Improvement Workflow | Current; updated for the 2026-11-01 reset | [`docs/tutorials/2026-10-30/improvement-workflow.md`](tutorials/2026-10-30/improvement-workflow.md) |

## Current roles

| Role | Manual | Status |
|---|---|---|
| Quant Researcher | [Quant Researcher Manual](roles/2026-10-29/quant-researcher-manual.md) | Current; normative in `AGENTS.md` section 10 |
| Quant Developer | [Quant Developer Manual](roles/2026-10-29/quant-developer-manual.md) | Current; normative in `AGENTS.md` section 10 |
| Quant Validator | [Quant Validator Manual](roles/2026-10-29/quant-validator-manual.md) | Current; normative in `AGENTS.md` section 10 |
| All three | [Quant roles overview](roles/2026-10-29/quant-roles.md) | Current; role table and handoff chain |
| Improvement Workflow | [Improvement Workflow roles](roles/2026-10-30/improvement-workflow-roles.md) | Current; Loop Governor, Result Analyst, Triage Officer |
| Audit Workflow | [Audit Workflow roles](roles/2026-10-30/audit-workflow-roles.md) | Current; Audit, Planner, Improvement Verifier |

## Local workspace

`data/` is a local workspace. Its directories, naming rules, and defaults are
documented in [`data/README.md`](../data/README.md); only those README files are
committed, and the candles, charts, reports, and databases stay local.

## Developer tools

| Tool | Purpose | Path |
|---|---|---|
| Progression replay oracle | Regenerates and checks the Progression golden fixtures independently of the Rust implementation. | [`tools/oracle/README.md`](../tools/oracle/README.md) |
| Experiment matrix runner | Runs one isolated `backtest` per manifest cell in deterministic order. | [`tools/experiment/README.md`](../tools/experiment/README.md) |
| Improvement run store | Manifest validation, run execution, evidence extraction, and deterministic index (Python). Baselines and invalidations are recorded in `docs/improvement/`. | `tools/improvement/` |
| Research model-cell runner | Runs four instrument specs, one pooled prediction evaluation, and the root model result. | `tools/research/model_cell.py` |

## Releases

| Release | Tag | Notes |
|---|---|---|
| Workflow reset baseline | `v0.5.0` | [`docs/releases/2026-11-01/v0.5.0.md`](releases/2026-11-01/v0.5.0.md) |
| Lab Summary and Help | `v0.3.0` | `docs/releases/2026-10-30/v0.3.0.md` |
| MVP v1 | `v0.2.0` | `docs/releases/2026-10-28/v0.2.0.md` |

## Current programs

| Program | Status | Plan |
|---|---|---|
| Help, tutorial, and quant role manuals | Delivered (#208); tagged `v0.3.0` | `docs/execution-plans/2026-10-29/208-help-tutorial-role-manuals.md` |
| MVP v1: config-driven engines, five reports, and the full-execution 2025 example run | Delivered (#141); tagged `v0.2.0` | `docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md` |
| Market analysis and full-execution backtest | Superseded for MVP v1 closeout by the 2026-10-28 plan; history preserved | `docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md` |
| Audit and improvement workflow (pre-reset) | Superseded by the 2026-11-01 workflow reset; retained as history | `docs/execution-plans/2026-10-30/audit-improvement-workflow.md` |
| Workflow reset: Audit Workflow, Improvement Workflow, and run contract | Program green; first model run not green | `docs/execution-plans/2026-11-01/workflow-reset.md` |
| Workflow-reset cleanup | Partially verified; pre-reset experiment and audit directories removed; lab path references remain | `docs/execution-plans/2026-11-01/workflow-reset-cleanup.md` |
| AUD-2026-10-30-01: reject non-M1 ladder inputs | Merged as `98cf9d5` in [#221](https://github.com/quant-rule/quant-rule/pull/221); finding `Verified` | `docs/execution-plans/2026-10-30/AUD-2026-10-30-01-m1-source-validation.md` |
| Remaining MVP audit remediation (AUD-02…11) | Merged; all findings `Verified` | `docs/execution-plans/2026-10-30/audit-remaining-mvp-remediation.md` |
| AUD-06 step-precision regression | Merged as `a006a17`; finding `Verified` | `docs/execution-plans/2026-10-30/aud-06-step-precision.md` |
| Unified timeframe ladder | Complete; Phases 1–9 merged through PRs #238–#246; #246 merged as `7c865de` | `docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md` |
| Cross-engine diagnosis v1 | Implemented; awaiting human acceptance | `docs/execution-plans/2026-10-31/diagnosis-v1.md` |
| Model v3: flat indicator/event model, validation gate, and benchmarks | Stopped at the model gate 2026-11-03; no edge; v8 implementation proceeds separately | `docs/execution-plans/2026-11-03/342-model-v3.md` |
| M2.x model programme | Approved 2026-11-05; P0–P5 merged; P5 `no_edge`; P7 closeout complete; legacy removal #463 complete; validation not opened | `docs/execution-plans/2026-11-05/410-m2-model-programme.md` |
| M2.x model charter | Approved 2026-11-05; P5 `no_edge`; checklist signed; legacy removal #463 complete; strategy tier closed | `docs/governance/2026-11-05/M2-CHARTER.md` |

Execution plans are repository documents. An external draft is a scratch copy; see `AGENTS.md` section 5.

## Current architecture decisions

| Decision | Status | Path |
|---|---|---|
| Flat indicator/event model; axes removed from decisions | Accepted 2026-11-03 | `docs/adr/2026-11-03/flat-indicator-event-model.md` |
| Four-axis analytical model | Superseded for axes 2026-11-03; retained as history | `docs/adr/2026-10-21/four-axis-analytical-model.md` |
| SQLite per-bar store and research schema | Accepted | `docs/adr/2026-10-24/sqlite-store.md` |
| History, retention, and display policy | Accepted | `docs/adr/2026-10-25/history-retention-and-display-policy.md` |
| Market analysis and full-execution backtest boundaries | Accepted (#145); superseded in part by the 2026-10-28 decision | `docs/adr/2026-10-27/analysis-and-backtest-boundaries.md` |
| Extended-range M1 window | Accepted (#144) | `docs/adr/2026-10-27/extended-range-window.md` |
| Config-driven engines and per-engine reports | Accepted (#168) | `docs/adr/2026-10-28/config-driven-engines-and-reports.md` |
| Abstraction levels and engine boundaries | Accepted 2026-10-30 | `docs/adr/2026-10-30/abstraction-levels-and-engine-boundaries.md` |
| Improvement and audit workflow (historical) | Superseded 2026-11-01 by the workflow reset | `docs/adr/2026-10-30/governing-swarm-and-improvement-loop.md` |
| Workflow reset: Audit Workflow and Improvement Workflow | Accepted 2026-11-01 | `docs/adr/2026-11-01/workflow-reset.md` |
| Rule purity and statistics boundary | Accepted 2026-10-30 | `docs/adr/2026-10-30/rule-purity-and-statistics-boundary.md` |
| Stateless strategy and the position parameter | Accepted 2026-10-30 | `docs/adr/2026-10-30/stateless-strategy-position-parameter.md` |
| One-way timeframe ladder and trade-intent boundary | Accepted 2026-10-30 (#228) | `docs/adr/2026-10-30/one-way-timeframe-ladder.md` |
| Retirement of the axis-agreement strategy | Accepted 2026-10-30 (#235) | `docs/adr/2026-10-30/axis-agreement-retirement.md` |
| Separate cross-engine diagnosis artifact | Accepted 2026-10-31 | `docs/adr/2026-10-31/diagnosis-artifact.md` |
| Trustworthy backtest kata and architecture characteristics | Accepted 2026-10-31 | `docs/adr/2026-10-31/adopt-architecture-baseline.md` |

## Reading a work item

A work item's plan and its transparency log have the same base name and sit in two sibling folders:

```
docs/execution-plans/2026-10-21/14-model-engine.md
docs/ai-transparency-log/2026-10-21/14-model-engine.md
```

The plan says what was decided and why. The log entry says what the AI actually did, what was verified, and what was rejected or corrected. Each log entry links to its plan, and each plan names its log entry in its acceptance criteria.

AI transparency logs are point-in-time records. Status lines such as “pull request is open” describe the moment the entry was written; the current state is in Git, GitHub, and the execution plan.

A work item may also have documents in the other kinds. The model contract and the architecture decision record are referenced from the plan and from each other, and the contract is also referenced from `AGENTS.md` section 2. They carry their own date and name rather than the issue's.

## Dates

Every date segment is a UTC date, per `AGENTS.md` section 12. A work item is filed under the UTC date on which it was recorded, which is not necessarily the local date of the machine that produced it.
