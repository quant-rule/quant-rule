# Strategy Configuration v3 — Active Ladder and Ladder Context

- Date: 2026-10-30 (UTC)
- Status: Context builder (#233) and trade-intent boundary (#234) implemented; strategy wiring pending (#235)
- Issues: [#228](https://github.com/quant-rule/quant-rule/issues/228), [#233](https://github.com/quant-rule/quant-rule/issues/233), [#235](https://github.com/quant-rule/quant-rule/issues/235)
- Plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- ADR: [`docs/adr/2026-10-30/one-way-timeframe-ladder.md`](../../adr/2026-10-30/one-way-timeframe-ladder.md)

## 1. Purpose

`strategy.json` is the human-owned rule. It selects the active ladder, declares
the per-axis context weights, defines the entry and exit, and states the risk
intent.

It does not contain fill mechanics, indicator formulas, statistics, cash, or
P&L.

## 2. Shape

```json
{
  "schema": "quantrule.strategy/v3",
  "rule": "ladder-context",
  "thesis": "Trade with the higher-timeframe regime after a lower-timeframe pullback and reclaim.",
  "ladder": ["5m", "15m", "1h", "4h"],
  "weights": {
    "direction": { "15m": 1.0, "1h": 1.0, "4h": 1.0 },
    "momentum": { "15m": 1.0, "1h": 1.0, "4h": 1.0 },
    "structure": { "15m": 1.0, "1h": 1.0, "4h": 1.0 },
    "progression": { "15m": 1.0, "1h": 1.0, "4h": 1.0 }
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
      "progression": {
        "direction": "bullish",
        "late_or_terminal_count_max": 0
      }
    }
  },
  "exit": { "on": ["context_lost", "structure_lost"] },
  "risk": {
    "stop": { "kind": "atr", "multiple": 2.0 },
    "target": { "kind": "reward_to_risk", "multiple": 1.5 }
  },
  "no_trade": {
    "missing_context": "reject",
    "require_contiguous_execution": true
  }
}
```

## 3. Ladder

| Field | Rule |
|---|---|
| `ladder` | Ascending, unique, at least two members |
| `ladder[0]` | Execution timeframe |
| `ladder[1..]` | Context timeframes, strictly higher |
| Membership | Every member is in `data.available_timeframes` and `policy.universe.timeframes` |
| Execution | `execution.fill_timeframe` must equal `ladder[0]` |

The active ladder is the only place the strategy states its timeframes. The data
file states what can be built.

## 4. Weights

- Keys are context timeframes only.
- Values are finite and positive.
- Missing block means equal weights for every context timeframe.
- Weights are normalized in active-ladder order.
- Missing context defaults to `reject`.
- Weights are fixed before the run and recorded in provenance.
- Weights express importance, not calibration.

Continuous axes use weighted means. Structure and Progression use weighted
shares and histograms; ordinal codes are never averaged.

### 4.1 Context output

For each execution bar the builder publishes:

- execution timeframe, bar index, open time, and close time;
- one context identity per selected timeframe: timeframe, bar index, open time,
  and close time;
- the context timeframes with no closed bar, if any;
- Direction and Momentum: weighted value, bullish/bearish/neutral counts,
  weighted shares, and an agreement code;
- Structure: the same score context plus a conflict count;
- Progression: the leg-direction context, a weighted stage histogram `0..5`, and
  a late-or-terminal count.

Agreement codes are numeric: 0 unavailable, 1 bullish, 2 bearish, 3 mixed. A
missing context timeframe makes the axis context unavailable; the weights are
not silently renormalized. A null axis value is excluded from that axis's valid
set; it is never treated as neutral or zero.

The builder reads only a context bar whose close is at or before the execution
bar's close. A bar that closes exactly at the execution close is available.

## 5. Entry

### Execution conditions

- Long: `direction > direction_min`, `momentum_force > momentum_min`,
  `structure_score >= structure_min`, and a structure shift from
  `structure_code <= from_max` to `structure_code >= to_min`.
- Short: the mirrored comparisons.

### Context conditions

- Direction: weighted bullish share at least `share_min` and greater than the
  bearish share.
- Momentum: weighted bearish share at most `bearish_share_max`.
- Structure: conflict count at most `conflict_count_max`.
- Progression: weighted leg direction agrees and the late/terminal count is at
  most `late_or_terminal_count_max`.

The agreement condition is part of this rule. There is no separate
`axis-agreement` strategy.

## 6. Exit

- `context_lost`: the context condition that justified the entry no longer
  holds.
- `structure_lost`: the execution structure flips against the position.
- The ATR bracket can also close the position.

## 7. Risk intent

| Field | Rule |
|---|---|
| `risk.stop.kind` | `atr` in v1 of this rule |
| `risk.stop.multiple` | Finite, `> 0` |
| `risk.target.kind` | `reward_to_risk` in v1 of this rule |
| `risk.target.multiple` | Finite, `> 0` |

The strategy emits a `TradeIntent`; execution resolves the ATR specification.
The execution engine also supports absolute stop and target prices, but this
rule uses the ATR form. Structural stop/target is future work.

## 8. Migration

`quantrule.strategy/v2` is retired. A v2 file is rejected with:

```text
quantrule.strategy/v2 is retired; use quantrule.strategy/v3 with rule "ladder-context"
```

The old `axis-agreement` behavior is preserved by a historical golden fixture
and an ADR, not by a live config path.
