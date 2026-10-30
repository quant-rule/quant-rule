# Model Configuration v2 — Per-Timeframe Readings

- Date: 2026-10-30 (UTC)
- Status: Approved; implementation pending
- Issues: [#228](https://github.com/quant-rule/quant-rule/issues/228), [#231](https://github.com/quant-rule/quant-rule/issues/231)
- Plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Model contract: [`docs/model/2026-10-30/per-timeframe-model.md`](../../model/2026-10-30/per-timeframe-model.md)

## 1. Purpose

`model.json` configures the low-level measurement of each available timeframe.
Defaults apply everywhere; a timeframe may override the readings it needs.

## 2. Shape

```json
{
  "schema": "quantrule.model/v2",
  "defaults": {
    "ema": [20, 50, 200],
    "rsi": 14,
    "atr": 14,
    "rvol": 20,
    "percentile_window": 252,
    "percentile_min_samples": 50
  },
  "timeframes": {
    "1m": { "rsi": 21 },
    "5m": { "rsi": 21 },
    "15m": { "rsi": 14 },
    "1h": { "rsi": 14 },
    "4h": { "rsi": 14 }
  },
  "structure": {
    "scale": "major",
    "minor_pivot_bars": 2,
    "major_pivot_bars": 5,
    "level_proximity_atr": 0.2
  },
  "progression": {
    "atr_target_multiple": 2.0,
    "target_stability_bars": 3,
    "invalidation_atr": 0.2
  },
  "events": {
    "divergence_scale": "major",
    "rsi_thresholds": [30, 50, 70],
    "ema_pairs": [[20, 50], [50, 200], [20, 200]]
  },
  "context": {
    "atr_ratio_expanding": 1.0,
    "rvol_expanding": 1.0
  }
}
```

## 3. Fields

| Field | Rule |
|---|---|
| `schema` | Exactly `quantrule.model/v2` |
| `defaults.ema` | Exactly three positive periods, strictly increasing |
| `defaults.rsi` | Positive integer |
| `defaults.atr` | Positive integer |
| `defaults.rvol` | Positive integer |
| `defaults.percentile_window` | Positive integer; at least `percentile_min_samples` |
| `defaults.percentile_min_samples` | Positive integer |
| `timeframes` | Optional map; keys must be available timeframes |
| `timeframes.<tf>` | Optional override of `ema`, `rsi`, `atr`, `rvol` |
| `structure.scale` | `major` or `minor` |
| `structure.minor_pivot_bars` | Positive integer |
| `structure.major_pivot_bars` | Positive integer |
| `structure.level_proximity_atr` | Finite, `>= 0` |
| `progression.atr_target_multiple` | Finite, `> 0` |
| `progression.target_stability_bars` | Positive integer |
| `progression.invalidation_atr` | Finite, `>= 0` |
| `events.divergence_scale` | `major` or `minor` |
| `events.rsi_thresholds` | Three ascending values in `(0, 100)` |
| `events.ema_pairs` | Three non-empty pairs of positive periods |
| `context.*` | Finite, `> 0` |

Unknown fields and unknown timeframe keys are rejected.

## 4. Semantics

- A missing override uses the default.
- An override of `ema` replaces all three periods.
- Per-timeframe settings change the measurement of that timeframe, not its
  native scale.
- Direction and Momentum remain continuous `[-1, +1]`.
- Structure remains signed `-2..+2`; Progression remains `0..5`.
- Per-timeframe settings are recorded in provenance and in the decision
  evidence.
- The model computes only the timeframes the runner passes to it; it does not
  know the active ladder.

## 5. Control and variations

- The **control** config uses the current values on every timeframe.
- The first experiment may test at most one per-timeframe variation, such as a
  faster RSI on the execution timeframes.
- Do not tune all periods on the sample. Every variation is pre-registered.
