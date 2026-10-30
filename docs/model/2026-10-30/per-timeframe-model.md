# Per-Timeframe Model Contract

- Date: 2026-10-30 (UTC)
- Status: Approved; implementation pending
- Issues: [#228](https://github.com/quant-rule/quant-rule/issues/228), sub-issues [#231](https://github.com/quant-rule/quant-rule/issues/231), [#232](https://github.com/quant-rule/quant-rule/issues/232)
- Plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- ADR: [`docs/adr/2026-10-30/one-way-timeframe-ladder.md`](../../adr/2026-10-30/one-way-timeframe-ladder.md)
- Base contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../2026-10-21/four-axis-analytical-model.md)

## 1. Purpose

This document is the approved delta to the four-axis model contract. It defines
the model boundary for the one-way timeframe ladder.

The model measures each timeframe independently and publishes per-timeframe
measurements. It does not know the strategy's active ladder and does not compute
the decision context.

The four axes, their formulas, the event model, the numeric output contract, and
the native ordinal scales remain in the base contract unless this document says
otherwise.

## 2. Boundary

| Layer | Responsibility |
|---|---|
| Data engine | Build the available timeframes from M1 |
| Model engine | Compute readings, axes, events, per-timeframe context, and per-timeframe consensus |
| Strategy layer | Select the active ladder, build the as-of context, and decide |
| Analytics | Compute descriptive aggregates over the available ladder |
| Execution | Resolve trade intents and fill orders |
| Portfolio | Account for positions, cash, and P&L |

The model never reads strategy, execution, portfolio, policy, or analytics.

## 3. Output

For each timeframe the runner passes to the model, the model publishes:

- `ModelReadings`: the configured EMA, RSI, ATR, and RVOL values.
- `DirectionBar`.
- `MomentumBar`.
- `StructureBar`.
- `ProgressionBar`.
- `ContextBar`: volatility and participation diagnostics.
- `TimeframeConsensus`: the per-timeframe cross-axis counts for Direction,
  Momentum, and Structure.
- `EventRecord` values for Level, Divergence, and Signal.
- Confirmed levels and progression leg records.
- The per-timeframe snapshot.

The model does **not** publish:

- An active ladder.
- An active execution timeframe.
- A cross-timeframe decision context.
- Strategy weights.
- Entry, exit, stop, target, position, cash, or P&L.
- A single composite score across axes or timeframes.

## 4. Native scales

The model retains its native scales:

| Axis | Output | Scale |
|---|---|---|
| Direction | `direction` | continuous `[-1, +1]` |
| Momentum | `momentum_force` | continuous `[-1, +1]` |
| Momentum | `momentum_acceleration` | continuous `[-1, +1]` |
| Momentum | `momentum_intensification` | continuous `[-1, +1]` |
| Structure | `structure_score` | discrete-valued continuous `[-1, +1]` |
| Structure | `structure_code` | ordinal `-2..+2` |
| Progression | `progression_stage_code` | ordinal `0..5` |
| Progression | `progress` | unclamped continuous |

No `0..9` conversion. A display mapping belongs in the shell. A strategy mapping
belongs in the strategy layer; it does not change the model contract.

## 5. Per-timeframe configuration

`model.json` v2 provides:

- `defaults`: EMA periods, RSI period, ATR period, RVOL period, percentile window,
  and minimum samples.
- `timeframes`: optional per-timeframe overrides for the same readings.
- `structure`: pivot scale, pivot bars, level proximity.
- `progression`: target multiple, stability bars, invalidation buffer.
- `events`: divergence scale, RSI thresholds, EMA pairs.
- `context`: volatility and participation thresholds.

The model config is a technical contract validated against the available
timeframes. Per-timeframe overrides are explicit and recorded in provenance.

## 6. Causality and determinism

- A per-timeframe value at bar `t` depends only on bars at or before `t`.
- A confirmed pivot is used only at or after its availability bar.
- An event is published only at or after `available_bar_index`.
- The model computes each timeframe independently; it never reads a lower or
  higher timeframe to produce that timeframe's axes.
- The same candles and the same model config produce the same output.
- Container iteration order never affects output.

## 7. Reporting aggregates

Descriptive cross-timeframe aggregation is not part of the model contract.

Reports, the dashboard, and research may aggregate the model's per-timeframe
outputs over the available ladder. Those aggregates are descriptive. They are
not the strategy's decision context and must not be read by a strategy.

The strategy's `LadderContext` is built in `quantrule-backtest` from the
per-timeframe outputs and the strategy-owned active ladder and weights.

## 8. Implementation status

This document is approved but not yet implemented. The code migration is tracked
by:

- [#231](https://github.com/quant-rule/quant-rule/issues/231) — per-timeframe
  model configuration.
- [#232](https://github.com/quant-rule/quant-rule/issues/232) — move reporting
  aggregates out of the model.
- [#233](https://github.com/quant-rule/quant-rule/issues/233) — ladder context
  builder.

Until those issues merge, the current model contract remains the implementation
of record.
