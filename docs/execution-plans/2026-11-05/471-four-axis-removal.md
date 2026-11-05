# P3 — Remove the Four-Axis Architecture

- Date: 2026-11-05 (UTC)
- Issue: [#471](https://github.com/quant-rule/quant-rule/issues/471)
- Status: **Implemented** 2026-11-05; PR pending. The boundary below was approved
  by the human on 2026-11-05
- Claim type: architecture and model-output cutover; behavior change
- Parent: [`467-clean-current-architecture.md`](467-clean-current-architecture.md)
- AI transparency log: [`docs/ai-transparency-log/2026-11-05/471-four-axis-removal.md`](../../ai-transparency-log/2026-11-05/471-four-axis-removal.md)

## 1. Intent

Make the flat per-timeframe model the only model implementation. The Direction,
Momentum, and Structure scores, their bars, the dashboard, the old report
assembler, the market state, the consensus, and the cross-timeframe aggregate
leave the working product. The retained level registry and progression logic
move to flat modules. The strategy axis ladder leaves; execution, portfolio, and
the generic bracket/event engines stay as libraries for S3.x.

## 2. Approved boundary

| Area | Decision |
|---|---|
| Direction, Momentum, Structure score series | Delete |
| `dashboard`, `state`, `consensus`, `aggregate` | Delete |
| `report/assemble.rs`, `report/chart.rs` | Delete; keep `report/flat.rs` |
| `ConfirmedLevelRegistry` | Move to `crates/quantrule-model/src/levels.rs` |
| `ProgressionSeries` | Move to `crates/quantrule-model/src/progression/` |
| Registered events | Keep all 143; the axis state changes are `LEGACY_UNREGISTERED` code 0 |
| `quantrule-analytics` | Delete with the `report analytical`, `research export`, and `research summary` surfaces |
| Backtest axis ladder/context | Delete; keep execution, portfolio, `bracket`, and `event` engines |
| Shell | Keep data/fetch, `report model`, `events`, `policy check`, `help` |

## 3. Work breakdown

### P3a — model core

- Move levels and progression to flat modules (done in the first slice).
- Rewrite `TimeframeModel`, `ModelBar`, `TimeframeSnapshot`, and `ModelRun`
  without axis fields.
- Remove the Direction, Momentum, and Structure series.
- Remove the legacy axis state events from `SignalEventSeries`.
- Rewrite `FlatReport` around the flat model.
- Delete `dashboard`, `state`, `consensus`, `aggregate`, and the old report
  assembler/chart.

### P3b — shell

- Remove `market-state`, `market-dashboard`, `market-chart`, `market-report`,
  `axis`, `legs`, and `model-series`.
- Rewrite `report model` and `events` around the flat model.
- Remove `report analytical`, research export/summary, store/db, lab, strategy,
  and backtest commands.
- Update help, the command reference, and fixtures.

### P3c — analytics and backtest

- Remove the `quantrule-analytics` crate and its shell dependency.
- Remove the axis ladder/context and the old strategy parser; keep execution,
  portfolio, `bracket`, and `event` engines.
- Remove the axis/dashboard/report/store tests and fixtures.

### P3d — contract and docs

- Update the M1.0 contract output section to the flat result.
- Remove the old axis/dashboard/report/state docs and tutorials.
- Update the history index and legacy scan allowlist.
- Record the Quant Developer and Quant Validator passes.

## 4. Acceptance criteria

- No `axes`, `dashboard`, `state`, `consensus`, or `aggregate` module compiles.
- `TimeframeModel` publishes readings, states, levels, events, progression, and
  context only.
- `report model` and `events` use the flat model.
- Shell help lists only the surviving command set.
- `quantrule-analytics` is gone.
- The backtest axis ladder/context is gone; execution, portfolio, bracket, and
  event engines remain.
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt
  --check`, registry, validation, links, terms, and legacy checks pass.
- The golden fixtures are updated only where the intentional model-output change
  requires it.

## 5. Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
python3 tools/check_legacy.py
python3 tools/check_registry.py
python3 tools/check_validation.py
python3 tools/check_links.py
python3 tools/check_terms.py
```

The model-output change needs the Quant Developer record and the Quant
Validator reproduction; the passes are sequential if separate agents are not
available.

## 6. Risks and recovery

| Risk | Mitigation |
|---|---|
| Removing axes breaks a retained event | The axis state changes are unregistered code 0; the 143 registry events are checked |
| Removing the old ladder breaks the shell | P3b rewrites the surviving commands in the same cutover |
| Deleting accepted evidence | History stays in ADRs, audits, improvement records, and run records |
| Golden fixtures change | Regenerate only after the flat output is reviewed; record the change |
| Analytics deletion removes a needed research path | The research workflow uses `report model` and `events.csv`, not the axis analytics crate |

Recovery: every deleted path exists at `0e3ba07`.

## 7. Stop conditions

- A retained registered event depends on an axis value.
- A surviving command cannot be expressed on the flat model.
- A model-output change lacks the Quant Developer or Quant Validator record.
- A deletion removes history or run evidence.

## 8. Execution record

### P3a — model core

- Moved `axes/structure/registry.rs` to `levels.rs`; moved `axes/progression/*`
  to `progression/` (P3a-1).
- Rewrote `TimeframeModel`, `ModelBar`, and `ModelRun` without Direction,
  Momentum, Structure, consensus, snapshot, or cross-timeframe aggregate fields.
- Removed the Direction, Momentum, and Structure series, `dashboard`, `state`,
  `consensus`, `aggregate`, and the old report assembler/chart.
- Removed the legacy axis state events from `SignalEventSeries`; the detector now
  reads the readings and MACD series only. The removed events used
  `LEGACY_UNREGISTERED` code 0 and were never registry events.
- Rewrote `FlatReport` around `levels` and `progression`; coverage is local to the
  flat report.

### P3b — shell

- Removed `market-state`, `market-dashboard`, `market-chart`, `market-report`,
  `axis`, `legs`, `model-series`, `store`, `db`, `research`, `lab`, strategy, and
  backtest commands.
- Rewrote `report model` around `FlatReport`; the bundle is
  `summary.txt`, `report.json`, `result.json`, `readings.csv`, `events.csv`,
  `levels.csv`, `legs.csv`, `provenance.json`, plus spec artifacts.
- Removed the report charts (`price-axes.png`, `events.png`).
- Rewrote `events` to publish the flat registered and extended event batch.
- Rewrote help, dispatch, command types, config tests, and CLI tests around the
  surviving surface.
- Removed `quantrule-analytics` from the workspace and the shell.

### P3c — backtest

- Removed the axis ladder, as-of context, strategy walk, and runner.
- Kept execution, portfolio, the bracket engine, and the event-composite engine.
- Rewrote `StrategyInput`, `EventStrategyInput`, and the `Strategy` trait without
  the axis context. Higher-timeframe event views are added explicitly.
- Trimmed `StrategyConfigError` to the errors the retained engines return.

### P3d — contract and docs

- Updated the M1.0 result contract to the flat output and bundle.
- Rewrote the README, docs index, architecture entry point, kata, and
  characteristics to the flat model and surviving surface.
- Deleted the old model, config, and tutorial documents in the approved
  deletion set. Historical links now point to `docs/registry/history.md`.
- Updated the registry board and history boundary.

### Intentional behavior and command changes

| Change | Reason |
|---|---|
| Axis state events (code 0, sources 9–11) are no longer emitted | Only legacy axis states; no registry event was removed |
| `SignalEventSeries::compute` takes `(readings, macd, config)` | The axis series no longer exist |
| `events` publishes registered and extended events | The flat model has one event surface |
| `report model` writes no PNG charts | The chart surface was deleted |
| `report model` writes `result.json` and the CSV artifacts | The flat report is the only report |
| `fetch` writes CSV only; `--png` and `--candles` are removed | The chart surface was deleted |
| `run.json` no longer loads strategy or analytics | Their parsers and surfaces were deleted |
| `backtest`, `strategy`, `store`, `research`, and `lab` are no longer commands | The approved surviving surface is data/fetch, `report model`, `events`, `policy check`, `help` |

### Verification

| Check | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `cargo test` | 977 passed, 0 failed, 7 ignored across 28 suites |
| `python3 tools/check_legacy.py` | 179 current files; no retired labels |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |
| `python3 tools/check_validation.py` | 188 claims for 46 indicators and 143 events |
| `python3 tools/check_links.py` | 538 Markdown files; no broken relative links |
| `python3 tools/check_terms.py` | 324 files; no banned terms |
| `report model` fixture | Flat bundle written; `report.json` has `flat_report`, no `market_report`; `result.json` has no `charts` |
| `events` fixture | Deterministic event JSON; no axis state events |
| Signal golden fixture | 69 legacy axis-state rows removed; 49 registered rows remain and pass |

### Quant Developer record

The Quant Developer pass implemented the flat model cutover, preserved the 143
registered events, and removed only code-0 axis state events. The signal golden
fixture was filtered for those removed rows; no registered event, reading,
level, or progression value changed. The pass changed model output by removing
the legacy axis state events and changing the public `SignalEventSeries`
signature. No strategy or edge claim is made.

### Quant Validator record

The Quant Validator pass was performed sequentially by the same agent, not by an
independent agent or model judgment. It checked the cutover from the current
working tree:

- Re-ran `cargo test` (977 passed, 0 failed), clippy, fmt, and the Python gates.
- Confirmed the registry still carries 143 events; only code-0 axis state events
  were removed.
- Confirmed `events` output is deterministic.
- Confirmed `report model` is deterministic and carries no axis, consensus,
  cross-timeframe aggregate, or chart field.

Residual risk: the validator reproduced the working tree before the commit, so
the final commit is the frozen artifact; the PR and CI run are the authoritative
reproduction after commit.
