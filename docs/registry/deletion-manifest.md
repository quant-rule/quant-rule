# Deletion Manifest — Clean Current Architecture

- Date: 2026-11-05 (UTC)
- Status: **Partially executed**; P3 removed the four-axis, analytics, and old
  shell surfaces. P4 executes the remaining config and docs cleanup.
- Issue: [#467](https://github.com/quant-rule/quant-rule/issues/467)
- Plan: [`467-clean-current-architecture.md`](../execution-plans/2026-11-05/467-clean-current-architecture.md)
- Freeze / recovery commit: `8acf857`
- Exact file list: [`deletion-manifest.tsv`](deletion-manifest.tsv)

## 1. Purpose

This is the file-by-file boundary between the current architecture and the
history that remains. It is the authority for P3 and P4. A path that is not in
the `delete` set is not deleted by those phases.

The TSV is the exact list. This page is the summary and the decision record.
Every path exists at `8acf857`; a deleted path can be recovered from that
commit or from the commit immediately before its deletion.

## 2. Summary

| Fate | Files | Meaning |
|---|---:|---|
| `delete` | 126 | Removed from the working surface in P3/P4 |
| `move` | 5 | Retained logic moves to a flat module or the M1.0 contract |
| `rewrite` | 49 | Retained surface is rewritten for the flat model or the surviving commands |
| `decide` | 169 | Blocked by D2/D3/D4; keep is recommended for the accepted M1.0 evidence |
| `keep` | 3 | `report/flat.rs` and the generic backtest bracket/event engines |

| Class | Files |
|---|---:|
| Config | 172 |
| Code | 114 |
| Fixtures | 29 |
| Tests | 14 |
| Docs | 23 |

## 3. Delete — clear legacy

| Group | Files | What is removed | Replacement |
|---|---:|---|---|
| `crates/quantrule-model/src/axes` | 6 | Direction, momentum, structure axes; the axis wrapper | Flat readings/events |
| `crates/quantrule-model/src/dashboard` | 14 | Four-axis dashboard assembler | Report model / flat events |
| `crates/quantrule-model/src/state.rs`, `state/` | 2 | MarketState view | Flat per-timeframe snapshot |
| `crates/quantrule-model/src/consensus.rs` | 1 | Cross-axis consensus | None; no model-level vote |
| `crates/quantrule-model/src/aggregate/` | 5 | Cross-timeframe reporting aggregates | Strategy as-of context (S3.x) |
| `crates/quantrule-model/src/report/` | 2 | Axis report assembler and chart | `report/flat.rs` |
| `crates/quantrule-shell/src/command/` | 13 | `market-state`, `market-dashboard`, `market-chart`, `market-report`, `axis`, `legs`, `model-series`, `research export`, `research summary`, store, strategy, backtest, lab | Surviving shell surface |
| `crates/quantrule-shell/src/chart/` | 8 | Old market charts and backtest diagnosis | Report model |
| `crates/quantrule-shell/src/store/` | 14 | SQLite store | None; run evidence stays in `data/improvement` |
| `crates/quantrule-shell/src/` | 7 | Dashboard/state/series/lab/diagnosis renderers | Report model / flat events |
| `crates/quantrule-shell/tests/` | 34 | 11 old-command tests and 23 fixtures | Surviving shell tests |
| `config/v5-swing/` | 1 | Old programme config | Current programme configs |
| `docs/model/` | 6 | Old axis, dashboard, report, per-timeframe, evidence-protocol, and v3 contracts | M1.0 and current model contracts |
| `docs/config/` | 3 | Old model v2, migration, and per-engine contracts | Current config contracts |
| `docs/tutorials/` | 10 | Old full-pipeline, unified-ladder, and config-fixture tutorials | Current tutorials |

The exact path list is in `deletion-manifest.tsv`; each row carries the class,
replacement, and `8acf857` recovery commit.

## 4. Move and rewrite — retained logic

| Path | Fate | Destination |
|---|---|---|
| `crates/quantrule-model/src/axes/progression/{leg,mod,target,tests}.rs` | move | `crates/quantrule-model/src/progression/` |
| `crates/quantrule-model/src/report/{mod,tests}.rs` | rewrite | Flat report module and tests |
| `docs/model/2026-11-04/model-v4.md` | move | `docs/model/2026-11-05/model-m1.0.md` |
| `crates/quantrule-shell/src/command/{dispatch,types,help,parse,tests}.rs` | rewrite | Surviving command surface |
| `crates/quantrule-shell/src/{chart/mod,pipeline,export,labels}.rs` | rewrite | Surviving shell modules |
| `crates/quantrule-shell/src/render_report/**` | rewrite | Current report bundle |
| `crates/quantrule-shell/src/command/report_run/**` | rewrite | Current report bundle |
| `crates/quantrule-shell/src/command/help/topics.rs` | rewrite | Surviving help topics |
| `crates/quantrule-model/src/lib.rs` | rewrite | Flat model module surface |
| `crates/quantrule-shell/tests/{cli,architecture,report_command}.rs` | rewrite | Surviving shell tests |
| `crates/quantrule-shell/tests/fixtures/{help*,config-run/strategy.json}` | rewrite | Surviving command fixtures |
| `config/defaults/model.json` | rewrite | Current flat model schema |
| `config/examples/**` | rewrite | Current model-spec/strategy schema |
| `docs/README.md` | rewrite | Current architecture index |
| `docs/registry/{README,history}.md` | rewrite | Link this manifest |

## 5. Keep — current surface and history

These are not deletion candidates. They are listed so a later phase cannot
remove them by accident.

| Path | Why |
|---|---|
| `crates/quantrule-data/**` | Data engine |
| `crates/quantrule-model/src/readings/**` | Flat indicator readings |
| `crates/quantrule-model/src/events/**` | Level, divergence, and signal events |
| `crates/quantrule-model/src/{panel,sessions,spec,timeframe,run,input,kernel}.rs` | Flat model and spec |
| `crates/quantrule-model/src/report/flat.rs` | Flat report projection |
| `crates/quantrule-backtest/src/{execution,portfolio}/**` | Execution and portfolio engines |
| `crates/quantrule-backtest/src/strategy/{bracket,event}.rs` | Generic bracket and event-composite engines for S3.x |
| `crates/quantrule-shell/src/{config,command,source}/**` after P3/P4 | Surviving shell surface |
| `config/defaults/**`, `config/research/**` | Current configs |
| `config/compression-range/**`, `config/compression-range-5m/**`, `config/compression-range-onset-5m/**` | Accepted M1.0 evidence, subject to D4 |
| `config/model-v4/**` | M1.0 baseline, subject to D2/D4 |
| `docs/registry/**` | Current registry |
| `docs/model/2026-11-05/**` | Current model contracts |
| `docs/model/2026-11-03/{adx-verification,claim-validation,path-validation}.md` | Model evidence |
| `docs/model/2026-11-04/{compression-*,barrier-path-5m-onset,path-validation-v4}.md` | Model evidence |
| `docs/adr/**` | Decision history |
| `docs/audits/**` | Audit evidence |
| `docs/improvement/**` | Improvement and programme records |
| `docs/ai-transparency-log/**` | AI usage history |
| `docs/execution-plans/**` | Plans of record |
| `docs/releases/**`, `docs/reports/**` | Release and report history |
| `data/improvement/**`, `data/research/**` | Run evidence |

## 6. Open decisions

| ID | Decision | Affected rows | Recommendation |
|---|---|---|---|
| D1 | P1 validation shape for context/diagnostic events | None in this manifest; blocks P1 | `signal`: predictive; `context`: descriptive construct test; `diagnostic`: construct only |
| D2 | Current model schema: keep `quantrule.model/v4` or migrate to `model-spec/v2` | `config/model-v4/**`, `config/defaults/model.json`, `config/examples/**` | Keep v4 as the current flat model config; delete v1–v3 |
| D3 | P3 strategy boundary: remove the axis ladder or rewrite it | `crates/quantrule-backtest/src/{context,runner,strategy}/**`, except the generic `bracket` and `event` engines | Remove the old axis ladder; build the multi-timeframe context with S3.x |
| D4 | Compression evidence: delete or keep | `config/compression*` | Keep as accepted M1.0 evidence; move only with a manifest |

The `decide` rows in the TSV are blocked by D2/D4. They are not authorized for
deletion by P0.1.

## 7. Verification

```bash
python3 tools/check_links.py
python3 tools/check_terms.py
python3 tools/check_registry.py
git status --short --branch
```

P0.1 is complete when these pass and the manifest covers every path in the
delete, move, rewrite, and decide sets. P3/P4 must re-run the manifest against
the tree before deleting; a path that gains a live caller is removed from the
delete set until the caller is resolved.
