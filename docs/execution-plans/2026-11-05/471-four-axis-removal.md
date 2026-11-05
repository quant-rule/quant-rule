# P3 — Remove the Four-Axis Architecture

- Date: 2026-11-05 (UTC)
- Issue: [#471](https://github.com/quant-rule/quant-rule/issues/471)
- Status: **Approved**; the boundary below was approved by the human on
  2026-11-05
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
