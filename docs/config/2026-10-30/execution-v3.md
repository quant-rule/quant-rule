# Execution Configuration v3 — Fill Mechanics Only

- Date: 2026-10-30 (UTC)
- Status: Approved; bracket resolution and risk-free execution config implemented (#234, PR #243). The current loader still reads `quantrule.execution/v2`; the v3 schema remains a target for a later work item.
- Issues: [#228](https://github.com/quant-rule/quant-rule/issues/228), [#234](https://github.com/quant-rule/quant-rule/issues/234)
- Plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Trade intent: [`docs/config/2026-10-30/trade-intent.md`](trade-intent.md)

## 1. Purpose

`execution.json` owns fill mechanics, costs, and bracket handling. It does not
own direction, strategy thresholds, stop distance, or target distance.

## 2. Shape

```json
{
  "schema": "quantrule.execution/v3",
  "fill_timeframe": "5m",
  "order_type": "market_next_open",
  "bracket_order_type": "attached_stop_target",
  "tie_break": "stop_first",
  "fee_rate": 0.0,
  "slippage_bps": 2.0,
  "max_slippage_bps": 5.0,
  "zero_volume": "reject",
  "end_of_data": "leave_open",
  "partial_fills": "reject"
}
```

## 3. Fields

| Field | Rule |
|---|---|
| `schema` | Exactly `quantrule.execution/v3` |
| `fill_timeframe` | Equals `strategy.ladder[0]` |
| `order_type` | `market_next_open` in v3 |
| `bracket_order_type` | `attached_stop_target` in v3 |
| `tie_break` | `stop_first` in v3 |
| `fee_rate` | Finite, `>= 0` |
| `slippage_bps` | Finite, `>= 0` |
| `max_slippage_bps` | Finite, `>= slippage_bps` |
| `zero_volume` | `reject` in v3 |
| `end_of_data` | `leave_open` or `close_at_last` |
| `partial_fills` | `reject` in v3 |

Unknown fields are rejected.

## 4. Boundary

Execution may:

- Resolve a `StopSpec` and `TargetSpec` into prices.
- Fill at the next open.
- Apply slippage and fees.
- Apply the bracket tie-break.
- Reject a zero-volume fill or an invalid bracket.
- Return fills and rejections.

Execution may not:

- Decide direction.
- Read strategy thresholds or context weights.
- Choose stop or target distances.
- Read cash, equity, or P&L.
- Change a strategy decision.

## 5. Bracket resolution

Execution resolves the strategy's `StopSpec` and `TargetSpec`:

```text
ATR stop:  stop_distance = stop_multiple * ATR(decision_bar)
Price stop: stop_distance = |fill - stop_price|

long:  stop = fill - stop_distance                 (ATR) or stop_price
       target = fill + stop_distance * reward_to_risk (R:R) or target_price
short: stop = fill + stop_distance                 (ATR) or stop_price
       target = fill - stop_distance * reward_to_risk (R:R) or target_price
```

An ATR is required only for an ATR stop. An absolute stop with a
reward-to-risk target uses the absolute stop distance. The strategy never reads
ATR; it emits the specification. Execution never chooses a distance.

## 6. Position state

Execution reports the actual position as a `StrategyPosition` (`side`,
`entry_bar`, `bars_held`) to the runner. The runner passes that position to the
strategy on the next bar. A rejected entry cannot become a strategy position. A
bracket exit updates the position before the next decision.

## 7. Compatibility

`quantrule.execution/v2` is retired. It owned the stop/target multiples, which
now belong to the strategy risk intent. A v2 file is rejected with a migration
message.
