# Strategy Configuration v8 — Event Composite

- Date: 2026-11-03 (UTC)
- Status: Implemented; validation pending
- Schema: `quantrule.strategy/v8`
- Rule: `event-composite`

## 1. Thesis

A trade is justified by a small conjunction of published model events and
states across the active ladder. The strategy reads only the event/state
snapshot, confirmed levels, bar identity, and its own position. It never reads
an axis value, an indicator value, a percentile, cash, or P&L.

## 2. Shape

```json
{
  "schema": "quantrule.strategy/v8",
  "rule": "event-composite",
  "thesis": "Trade a DI cross when the execution EMA stack agrees.",
  "ladder": ["15m", "1h"],
  "window_bars": 2,
  "entry": {
    "all": [
      { "kind": "event", "timeframe": "15m", "event_type": 3, "subtype": 2, "source": 17, "direction": 1 },
      { "kind": "state", "timeframe": "15m", "field": "ema_stack", "value": 1 }
    ]
  },
  "exit": {
    "any": [
      { "kind": "state", "timeframe": "15m", "field": "ema_stack", "value": -1 }
    ]
  },
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

## 3. Conditions

| Kind | Fields | Rule |
|---|---|---|
| `event` | `timeframe`, `event_type`, `subtype`, `source`, `direction` | Matches a published event whose availability is inside `window_bars` and is at or before the selected bar. |
| `state` | `timeframe`, `field`, `value` | Matches a published v3 state exactly. |

Entry is the conjunction of `entry.all`. It must contain at least one
directional event, and every entry event must agree on `direction`. State-only
entries are rejected. Exit is the disjunction of `exit.any`; an empty list means
only the risk bracket exits.

The condition list is bounded to six entries per side. There is no expression
language, no weighted score, and no axis input.

## 4. State fields

`ema_stack`, `rsi_zone`, `macd_state`, `macd_histogram_state`, `adx_regime`,
`adx_slope`, `di_state`, `progression_stage`, `progression_leg_direction`.

## 5. Risk and execution

The entry intent is an ATR stop and a reward-to-risk target. Execution resolves
the fill, the bracket, fees, and slippage. The strategy never chooses a fill
price, quantity, or P&L value.

## 6. Verification

- `cargo test` covers the condition parser, direction conflict, state matching,
  entry, exit, and the runner path.
- `policy check` and `strategy explain` print the v8 rule.
- The boundary test proves the strategy input exposes only events, states,
  confirmed levels, bar identity, progression fields, and its own position.

## 7. Limitations

- The strategy is implemented; it has no accepted model or strategy baseline.
- The event window is a bar count, not a clock duration.
- Axis removal remains out of scope; v8 does not read axes.
