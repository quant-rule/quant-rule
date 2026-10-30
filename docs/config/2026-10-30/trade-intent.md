# Trade Intent Contract

- Date: 2026-10-30 (UTC)
- Status: Approved; implementation pending
- Issues: [#228](https://github.com/quant-rule/quant-rule/issues/228), [#234](https://github.com/quant-rule/quant-rule/issues/234)
- Plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- ADR: [`docs/adr/2026-10-30/one-way-timeframe-ladder.md`](../../adr/2026-10-30/one-way-timeframe-ladder.md)

## 1. Purpose

The strategy owns the trade thesis and risk intent. Execution owns fill
mechanics. `TradeIntent` is the typed boundary between them.

The strategy does not set a fill price, fee, or slippage. Execution does not set
direction, stop distance, or target distance.

## 2. Types

```rust
pub struct TradeIntent {
    pub side: Side,
    pub entry: EntryStyle,
    pub stop: StopSpec,
    pub target: TargetSpec,
    pub valid_until: Option<BarId>,
    pub max_slippage_bps: Option<f64>,
}

pub enum EntryStyle {
    MarketNextOpen,
    Limit { price: f64, valid_for_bars: u32 },
}

pub enum StopSpec {
    AtrMultiple { multiple: f64 },
    Price { price: f64 },
}

pub enum TargetSpec {
    RewardToRisk { multiple: f64 },
    Price { price: f64 },
}

pub struct PositionState {
    pub side: Option<Side>,
    pub entry_bar: Option<usize>,
    pub bars_held: u64,
}
```

## 3. Strategy output

```rust
pub enum Decision {
    Hold,
    Enter(TradeIntent),
    Exit(ExitIntent),
}

pub struct ExitIntent {
    pub side: Side,
    pub reason: ExitReason,
}
```

The strategy receives:

- The execution snapshot: timeframe, bar, open time, axes, structure code,
  progression fields.
- The as-of `LadderContext`.
- The actual `PositionState`.

It does not receive cash, equity, P&L, execution config, or raw indicator
parameters.

## 4. Execution resolution

```text
long:  stop_price = fill_price - stop_multiple * ATR(decision_bar)
       target_price = fill_price + stop_distance * reward_to_risk
short: stop_price = fill_price + stop_multiple * ATR(decision_bar)
       target_price = fill_price - stop_distance * reward_to_risk
```

Execution validates:

- Stop and target are finite and positive.
- Long: stop < entry < target.
- Short: target < entry < stop.
- The ATR is available and positive.
- The fill timeframe equals the execution timeframe.
- The bar is contiguous and has volume.

A rejected entry produces a rejection record and leaves the position unchanged.

## 5. Invariants

- The strategy is stateless with respect to position.
- Execution and portfolio own the position.
- A rejected entry cannot create a virtual position.
- A bracket exit is applied to the portfolio before the next strategy decision.
- The strategy cannot set a fill price, fee, or slippage.
- Execution cannot change the side or risk distance.
- A `TradeIntent` is a request, not a fill.
- A `PositionState` is the strategy's own state, not a market reading.

## 6. Example

```json
{
  "side": "long",
  "entry": { "kind": "market_next_open" },
  "stop": { "kind": "atr", "multiple": 2.0 },
  "target": { "kind": "reward_to_risk", "multiple": 1.5 },
  "valid_until": null,
  "max_slippage_bps": 5.0
}
```

The strategy emits the intent. Execution resolves it against the decision bar's
ATR and the fill price.
