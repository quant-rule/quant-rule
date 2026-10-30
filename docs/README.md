# QuantRule Documentation

The documentation tree is `docs/<kind>/<YYYY-MM-DD>/<file>.md`: the kind comes first, then the UTC date of the work item, then the file. A kind directory appears when it is first used.

## Kinds

| Kind | Holds | Path |
|---|---|---|
| Execution plans | The plan for a substantial work item, per `AGENTS.md` section 11. | `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md` |
| AI transparency log | The record of material AI contributions for a work item, per `AGENTS.md` section 6. One file per work item, named to match its execution plan. | `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md` |
| Agent roles | The quant and engineering roles, their handoffs, and when each is required. | `docs/roles/<YYYY-MM-DD>/<file>.md` |
| Tutorials | End-to-end procedures that run the product through shell commands, including role labs. | `docs/tutorials/<YYYY-MM-DD>/<file>.md` |
| Model | The analytical model's contract: the four axes, the numeric output fields, the event model, and the aggregation rules. The implementation follows it. | `docs/model/<YYYY-MM-DD>/<file>.md` |
| Analytics and research | The research output contract and the dataset definition: layers, columns, null rules, alignment, and export metadata. | `docs/analytics/<YYYY-MM-DD>/<file>.md` |
| Configuration | The per-engine JSON configuration: files, fields, validation, and cross-config rules. | `docs/config/<YYYY-MM-DD>/<file>.md` |
| Reports | The per-engine report bundles: artifacts, JSON and CSV schemas, boundaries, and availability. | `docs/reports/<YYYY-MM-DD>/<file>.md` |
| Releases | The notes for a tagged release: commands, configuration, reports, accounting rules, and evidence. | `docs/releases/<YYYY-MM-DD>/<file>.md` |
| Architecture decision records | A decision, the alternatives considered, and the consequences. | `docs/adr/<YYYY-MM-DD>/<file>.md` |
| Audits | Read-only findings, evidence, and remediation options. | `docs/audits/<YYYY-MM-DD>/<scope>-audit.md` |
| Experiments | Pre-registered experiment records and triage. | `docs/experiments/<YYYY-MM-DD>/<EXP-ID>-<slug>.md` |
| Governance | The improvement charter, green criteria, autonomy, and stop conditions. | `docs/governance/<YYYY-MM-DD>/<file>.md` |

## Current model contracts

| Contract | Status | Path |
|---|---|---|
| Four-axis analytical model | Contract | `docs/model/2026-10-21/four-axis-analytical-model.md` |
| Per-timeframe model | Approved (#228); readings implemented (#231); reporting aggregates pending (#232) | `docs/model/2026-10-30/per-timeframe-model.md` |
| DashboardState four-axis CLI view | Approved (#123) | `docs/model/2026-10-25/dashboard-state.md` |
| MarketReport full-period descriptive view | Implemented (#150); `market-report` command (#151); verified on SOLUSD 2025 (#152) | `docs/model/2026-10-27/market-report.md` |

## Current run contracts

| Contract | Status | Path |
|---|---|---|
| Per-engine configuration (v1) | Historical; superseded by the policy/strategy v2 contract | `docs/config/2026-10-28/per-engine-config.md` |
| Policy and strategy blueprints | Current; `run v2`, `policy v1`, `strategy v2`, `execution v2` | `docs/config/2026-10-30/policy-and-strategy.md` |
| Per-engine reports | Approved (#168); implemented by #173–#176; verified by #177 | `docs/reports/2026-10-28/per-engine-reports.md` |
| SOLUSD 2025 example run | Verified by #177: five bundles, 29 artifacts, two byte-identical runs | `config/examples/solusd-2025/` |

### Approved target contracts

The unified ladder contracts are approved under #228. The data v3 contract is
implemented by #230; the remaining target contracts wait for their code phases.

| Contract | Status | Path |
|---|---|---|
| Available timeframes | `quantrule.data/v3`; implemented by #230 | `docs/config/2026-10-30/data-v3.md` |
| Per-timeframe model | `quantrule.model/v2`; readings implemented (#231) | `docs/config/2026-10-30/model-v2.md` |
| Active ladder and context | `quantrule.strategy/v3`; implementation pending (#233, #235) | `docs/config/2026-10-30/strategy-v3.md` |
| Fill mechanics | `quantrule.execution/v3`; implementation pending (#234) | `docs/config/2026-10-30/execution-v3.md` |
| Strategy/execution boundary | `TradeIntent`; implementation pending (#234) | `docs/config/2026-10-30/trade-intent.md` |

## Current evidence

| Evidence | Status | Path |
|---|---|---|
| MVP v1 verification | Verified (#177); re-verified on current `main` after #189 and #196; release notes `v0.2.0` | `docs/reports/2026-10-28/mvp-v1-verification.md` |

## Current tutorials

| Tutorial | Status | Path |
|---|---|---|
| Run the full pipeline, with one lab per quant role | Current; verified 2026-10-30 | [`docs/tutorials/2026-10-29/full-pipeline.md`](tutorials/2026-10-29/full-pipeline.md) |
| The human improvement loop | Current; policy → strategy → experiment → triage | [`docs/tutorials/2026-10-30/improvement-loop.md`](tutorials/2026-10-30/improvement-loop.md) |

## Current roles

| Role | Manual | Status |
|---|---|---|
| Quant Researcher | [Quant Researcher Manual](roles/2026-10-29/quant-researcher-manual.md) | Current; normative in `AGENTS.md` section 10 |
| Quant Developer | [Quant Developer Manual](roles/2026-10-29/quant-developer-manual.md) | Current; normative in `AGENTS.md` section 10 |
| Quant Validator | [Quant Validator Manual](roles/2026-10-29/quant-validator-manual.md) | Current; normative in `AGENTS.md` section 10 |
| All three | [Quant roles overview](roles/2026-10-29/quant-roles.md) | Current; role table and handoff chain |
| Governing Swarm | [Governing Swarm](roles/2026-10-30/governing-swarm.md) | Current; Loop Governor, Result Analyst, Triage Officer |
| Continuous Improvement | [Continuous Improvement roles](roles/2026-10-30/continuous-improvement-roles.md) | Current; Audit, Planner, Improvement Verifier |

## Developer tools

| Tool | Purpose | Path |
|---|---|---|
| Progression replay oracle | Regenerates and checks the Progression golden fixtures independently of the Rust implementation. | [`tools/oracle/README.md`](../tools/oracle/README.md) |
| Experiment matrix runner | Runs one isolated `backtest` per manifest cell in deterministic order. | [`tools/experiment/README.md`](../tools/experiment/README.md) |

## Releases

| Release | Tag | Notes |
|---|---|---|
| Lab Summary and Help | `v0.3.0` | `docs/releases/2026-10-30/v0.3.0.md` |
| MVP v1 | `v0.2.0` | `docs/releases/2026-10-28/v0.2.0.md` |

## Current programs

| Program | Status | Plan |
|---|---|---|
| Help, tutorial, and quant role manuals | Delivered (#208); tagged `v0.3.0` | `docs/execution-plans/2026-10-29/208-help-tutorial-role-manuals.md` |
| MVP v1: config-driven engines, five reports, and the full-execution SOLUSD 2025 run | Delivered (#141); tagged `v0.2.0` | `docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md` |
| Market analysis and full-execution backtest | Superseded for MVP v1 closeout by the 2026-10-28 plan; history preserved | `docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md` |
| Audit and continuous-improvement workflow | In progress; policy/strategy v2 and swarm model | `docs/execution-plans/2026-10-30/audit-improvement-workflow.md` |
| AUD-2026-10-30-01: reject non-M1 ladder inputs | Merged as `98cf9d5` in [#221](https://github.com/quant-rule/quant-rule/pull/221); finding `Verified` | `docs/execution-plans/2026-10-30/AUD-2026-10-30-01-m1-source-validation.md` |
| Remaining MVP audit remediation (AUD-02…11) | Merged; all findings `Verified` | `docs/execution-plans/2026-10-30/audit-remaining-mvp-remediation.md` |
| AUD-06 step-precision regression | Merged as `a006a17`; finding `Verified` | `docs/execution-plans/2026-10-30/aud-06-step-precision.md` |
| MVP pre-release audit closure | All findings `Verified`; AUD-04 validation recorded | `docs/audits/2026-10-30/mvp-pre-release-closure.md` |
| Unified timeframe ladder | Approved (#228); Phase 1 merged; Phase 2 (#230) merged; Phase 3 (#231) implemented, PR pending; phases #232–#237 pending | `docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md` |

Execution plans are repository documents. An external draft is a scratch copy; see `AGENTS.md` section 5.

## Current architecture decisions

| Decision | Status | Path |
|---|---|---|
| Four-axis analytical model | Accepted | `docs/adr/2026-10-21/four-axis-analytical-model.md` |
| SQLite per-bar store and research schema | Accepted | `docs/adr/2026-10-24/sqlite-store.md` |
| History, retention, and display policy | Accepted | `docs/adr/2026-10-25/history-retention-and-display-policy.md` |
| Market analysis and full-execution backtest boundaries | Accepted (#145); superseded in part by the 2026-10-28 decision | `docs/adr/2026-10-27/analysis-and-backtest-boundaries.md` |
| Extended-range M1 window | Accepted (#144) | `docs/adr/2026-10-27/extended-range-window.md` |
| Config-driven engines and per-engine reports | Accepted (#168) | `docs/adr/2026-10-28/config-driven-engines-and-reports.md` |
| Abstraction levels and engine boundaries | Accepted 2026-10-30 | `docs/adr/2026-10-30/abstraction-levels-and-engine-boundaries.md` |
| Governing swarm and improvement loop | Accepted 2026-10-30 | `docs/adr/2026-10-30/governing-swarm-and-improvement-loop.md` |
| Rule purity and statistics boundary | Accepted 2026-10-30 | `docs/adr/2026-10-30/rule-purity-and-statistics-boundary.md` |
| Stateless strategy and the position parameter | Accepted 2026-10-30 | `docs/adr/2026-10-30/stateless-strategy-position-parameter.md` |
| One-way timeframe ladder and trade-intent boundary | Accepted 2026-10-30 (#228) | `docs/adr/2026-10-30/one-way-timeframe-ladder.md` |

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
