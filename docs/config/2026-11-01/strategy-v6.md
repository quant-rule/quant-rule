# Strategy Configuration v6 — Confirmed-Level Breakout

- Date: 2026-11-01 (UTC)
- Status: Implemented; experimental candidate, not validated
- Supersedes: none; v3, v4, and v5 remain available candidates

## 1. Purpose

`quantrule.strategy/v6` adds the `breakout` rule: enter when the execution
timeframe closes beyond a confirmed level in the direction of the higher-
timeframe context. It reuses the v4 confirmed-level context and structural
price bracket. It does not change the model.

## 2. Shape

```json
{
  "schema": "quantrule.strategy/v6",
  "rule": "breakout",
  "thesis": "Enter on a confirmed-level breakout in the direction of the higher-timeframe context.",
  "ladder": ["15m", "1h"],
  "weights": {
    "direction": { "1h": 1.0 },
    "momentum": { "1h": 1.0 },
    "structure": { "1h": 1.0 },
    "progression": { "1h": 1.0 }
  },
  "entry": {
    "execution": {
      "direction_min": 0.1,
      "momentum_min": 0.1,
      "structure_min": 0.0,
      "structure_shift": { "from_max": 0, "to_min": 1 }
    },
    "context": {
      "direction": { "kind": "bullish", "share_min": 0.5 },
      "momentum": { "kind": "not_bearish", "bearish_share_max": 0.5 },
      "structure": { "conflict_count_max": 0 },
      "progression": { "direction": "bullish", "late_or_terminal_count_max": 0 }
    },
    "level": {
      "max_distance_fraction": 0.02,
      "scale": "major"
    }
  },
  "exit": { "on": ["context_lost", "level_lost"] },
  "risk": {
    "stop": { "kind": "level", "buffer_fraction": 0.005 },
    "target": { "kind": "next_level", "min_reward_to_risk": 2.0 }
  },
  "no_trade": {
    "missing_context": "reject",
    "require_contiguous_execution": true
  }
}
```

## 3. Rule

- **Long:** the execution close crosses above the nearest confirmed resistance
  by at least `buffer_fraction`, the close is within `max_distance_fraction` of
  the broken level, and the context is bullish.
- **Short:** the mirror at a confirmed support.
- **Stop:** the broken level minus/plus `buffer_fraction`.
- **Target:** the next confirmed level, or `min_reward_to_risk` when none is
  available.
- **Exit:** `context_lost`, `level_lost`, or the bracket.
- The breakout is knowable only at the execution close; no later bar or future
  level is read.

## 4. Validation

- The schema is exactly `quantrule.strategy/v6`.
- The rule is exactly `breakout`.
- The ladder, weights, entry, exit, risk, and no-trade rules match the v4
  level shape.
- `level_lost` is accepted; `structure_lost` remains optional.
- The v3, v4, and v5 parsers remain byte-compatible.
