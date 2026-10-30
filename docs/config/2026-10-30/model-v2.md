# Model Configuration v2 — Per-Timeframe Readings

- Date: 2026-10-30 (UTC)
- Status: Implemented (#231); first delivery
- Issues: [#228](https://github.com/quant-rule/quant-rule/issues/228), [#231](https://github.com/quant-rule/quant-rule/issues/231)
- Plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Model contract: [`docs/model/2026-10-30/per-timeframe-model.md`](../../model/2026-10-30/per-timeframe-model.md)

## 1. Purpose

`model.json` configures the scalar reading periods for each available
timeframe. Defaults apply everywhere; a timeframe may override the readings it
needs.

The version 1 `structure_scale` shape remains accepted for existing configs.

## 2. Shape

```json
{
  "schema": "quantrule.model/v2",
  "defaults": {
    "ema": [20, 50, 200],
    "rsi": 14,
    "atr": 14,
    "rvol": 20
  },
  "timeframes": {
    "1m": { "rsi": 21 },
    "5m": { "rsi": 21 },
    "15m": { "rsi": 14 },
    "1h": { "rsi": 14 },
    "4h": { "rsi": 14 }
  },
  "structure": {
    "scale": "major"
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
| `timeframes` | Optional map; keys must be supported timeframe codes |
| `timeframes.<tf>.ema` | Optional replacement of all three EMA periods |
| `timeframes.<tf>.rsi` | Optional positive RSI period |
| `timeframes.<tf>.atr` | Optional positive ATR period |
| `timeframes.<tf>.rvol` | Optional positive RVOL period |
| `structure.scale` | `major` or `minor` |

Unknown fields and unknown timeframe keys are rejected.

## 4. Semantics

- A missing override field inherits the default.
- A timeframe without an override uses the defaults.
- An override changes only that timeframe.
- The model records the configured periods in the run configuration and the
  config fingerprint.
- The `Readings` field names are stable role labels: `ema20`, `ema50`, and
  `ema200` are the fast, middle, and slow EMA roles; `rsi14`, `atr14`, and
  `rvol20` are the RSI, ATR, and relative-volume roles. The configured period
  may differ from the role label.
- The native axis scales are unchanged: Direction and Momentum are continuous
  `[-1, +1]`; Structure is signed `-2..+2`; Progression is `0..5`.

## 5. Validation

- EMA periods are positive and strictly increasing.
- RSI, ATR, and RVOL periods are positive.
- Every override timeframe is a supported timeframe and is in
  `data.available_timeframes`.
- The model computes only the timeframes the runner passes to it.
- The model does not know the strategy’s active ladder.

## 6. Control and variations

- The **control** config uses the current values on every timeframe.
- The first experiment may test at most one per-timeframe variation, such as a
  faster RSI on the execution timeframes.
- Do not tune all periods on the sample. Every variation is pre-registered.

## 7. Reserved for a follow-up

The following model knobs remain fixed in this delivery and are not accepted by
the v2 parser yet. They need separate event/progression golden review:

- percentile window and minimum samples;
- progression target multiple, stability bars, and invalidation buffer;
- Level-event proximity, Signal thresholds, and EMA pair selection;
- context volatility/participation thresholds.

The schema will add them in a follow-up sub-issue rather than accepting fields
that do not affect the computation.
