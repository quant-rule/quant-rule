# Strategy Configuration v7 — Failed-Level-Test Fade

- Date: 2026-11-01 (UTC)
- Status: Experimental; validation failed
- Schema: `quantrule.strategy/v7`
- Rule: `level-fade`

## 1. Thesis

A confirmed level is a rejection zone. A failed test — price pierces the level
and closes back inside — attracts mean-reverting flow. This is the inverse of the
v4/v6 continuation family.

## 2. Rule

- **Long:** the execution low pierces confirmed support and the close finishes
  at or above it; the support is within `max_distance_fraction`.
- **Short:** the execution high pierces confirmed resistance and the close
  finishes at or below it.
- **No trend gate:** Direction, Momentum, and Structure are not entry conditions.
  Context completeness is still required.
- **Stop:** beyond the level by `buffer_fraction`.
- **Target:** next confirmed level, or `min_reward_to_risk`.
- **Exit:** `level_lost` and the structural bracket.

## 3. Shape

```json
{
  "schema": "quantrule.strategy/v7",
  "rule": "level-fade",
  "thesis": "Fade a failed test of a confirmed level.",
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
    "level": { "max_distance_fraction": 0.02, "scale": "major" }
  },
  "exit": { "on": ["level_lost"] },
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

The `entry.execution` and `entry.context` blocks are still parsed for schema
uniformity; the v7 rule does not read them. The level context and the structural
bracket are the whole rule.
