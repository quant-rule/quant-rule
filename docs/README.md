# QuantRule Documentation

The documentation tree is `docs/<kind>/<YYYY-MM-DD>/<file>.md`: the kind comes first, then the UTC date of the work item, then the file. A kind directory appears when it is first used.

## Kinds

| Kind | Holds | Path |
|---|---|---|
| Execution plans | The plan for a substantial work item, per `AGENTS.md` section 11. | `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md` |
| AI transparency log | The record of material AI contributions for a work item, per `AGENTS.md` section 6. One file per work item, named to match its execution plan. | `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md` |
| Model | The analytical model's contract: the four axes, the numeric output fields, the event model, and the aggregation rules. The implementation follows it. | `docs/model/<YYYY-MM-DD>/<file>.md` |
| Analytics and research | The research output contract and the dataset definition: layers, columns, null rules, alignment, and export metadata. | `docs/analytics/<YYYY-MM-DD>/<file>.md` |
| Configuration | The per-engine JSON configuration: files, fields, validation, and cross-config rules. | `docs/config/<YYYY-MM-DD>/<file>.md` |
| Reports | The per-engine report bundles: artifacts, JSON and CSV schemas, boundaries, and availability. | `docs/reports/<YYYY-MM-DD>/<file>.md` |
| Releases | The notes for a tagged release: commands, configuration, reports, accounting rules, and evidence. | `docs/releases/<YYYY-MM-DD>/<file>.md` |
| Architecture decision records | A decision, the alternatives considered, and the consequences. | `docs/adr/<YYYY-MM-DD>/<file>.md` |

## Current model contracts

| Contract | Status | Path |
|---|---|---|
| Four-axis analytical model | Contract | `docs/model/2026-10-21/four-axis-analytical-model.md` |
| DashboardState four-axis CLI view | Approved (#123) | `docs/model/2026-10-25/dashboard-state.md` |
| MarketReport full-period descriptive view | Implemented (#150); `market-report` command (#151); verified on SOLUSD 2025 (#152) | `docs/model/2026-10-27/market-report.md` |

## Current run contracts

| Contract | Status | Path |
|---|---|---|
| Per-engine configuration | Approved (#168); implemented by #170; used by `report` and `backtest` (#172–#176); verified by #177 | `docs/config/2026-10-28/per-engine-config.md` |
| Per-engine reports | Approved (#168); implemented by #173–#176; verified by #177 | `docs/reports/2026-10-28/per-engine-reports.md` |
| SOLUSD 2025 example run | Verified by #177: five bundles, 29 artifacts, two byte-identical runs | `config/examples/solusd-2025/` |

## Current evidence

| Evidence | Status | Path |
|---|---|---|
| MVP v1 verification | Verified (#177); release notes `v0.2.0` | `docs/reports/2026-10-28/mvp-v1-verification.md` |

## Releases

| Release | Tag | Notes |
|---|---|---|
| MVP v1 | `v0.2.0` | `docs/releases/2026-10-28/v0.2.0.md` |

## Current programs

| Program | Status | Plan |
|---|---|---|
| MVP v1: config-driven engines, five reports, and the full-execution SOLUSD 2025 run | Delivered (#141); tagged `v0.2.0` | `docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md` |
| Market analysis and full-execution backtest | Superseded for MVP v1 closeout by the 2026-10-28 plan; history preserved | `docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md` |

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
