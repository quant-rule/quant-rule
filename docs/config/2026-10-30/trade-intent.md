# Trade Intent Contract

- Date: 2026-10-30 (UTC)
- Status: Implemented (#234, PR #243); wired into the unified strategy (#235, PR #244)
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
    pub valid_until: Option<usize>,
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

pub struct StrategyPosition {
    pub side: Option<Side>,
    pub entry_bar: Option<usize>,
    pub bars_held: u64,
}
```

`StrategyPosition` is the strategy-facing view. Execution's own `PositionState`
enum remains execution-internal: it tracks flat, open, and just-closed states
and carries stop and target prices the strategy must not see.

The strategy also has a human-owned `RiskPolicy` (stop ATR multiple and
reward-to-risk). `RiskPolicy::intent(side)` builds the market-next-open intent
with an ATR stop and a reward-to-risk target.

## 3. Strategy output

```rust
pub enum Decision {
    Hold,
    Enter(Entry),
    Exit(Exit),
}

pub struct Entry {
    pub bar: StrategyInput,
    pub intent: TradeIntent,
}

pub struct Exit {
    pub side: Side,
    pub bar: StrategyInput,
    pub reason: ExitReason,
}
```

The bar snapshot preserves the decision bar for reporting; it is not a fill and
carries no price, quantity, fee, or P&L.

The strategy receives:

- The execution snapshot: timeframe, bar, open time, axes, structure code,
  progression fields.
- The as-of `LadderContext` (Phase 7 wiring).
- The actual `StrategyPosition`.

It does not receive cash, equity, P&L, execution config, or raw indicator
parameters.

## 4. Execution resolution

```text
ATR stop:  stop_distance = stop_multiple * ATR(decision_bar)
Price stop: stop_distance = |fill_price - stop_price|

long:  stop = fill - stop_distance          (ATR) or stop_price
       target = fill + stop_distance * reward_to_risk (R:R) or target_price
short: stop = fill + stop_distance          (ATR) or stop_price
       target = fill - stop_distance * reward_to_risk (R:R) or target_price
```

An ATR is required only when the stop is ATR-based. An absolute stop with a
reward-to-risk target uses the absolute stop distance and needs no ATR.

Execution validates:

- Stop and target are finite and positive.
- Long: stop < entry < target.
- Short: target < entry < stop.
- A required ATR is available and positive.
- The fill timeframe equals the execution timeframe.
- The entry style is supported (`market_next_open` in this phase).
- The submission bar is not past `valid_until`.
- Execution's configured slippage does not exceed `max_slippage_bps`.
- The bar is contiguous and has volume.

A rejected entry produces a rejection record and leaves the position unchanged.
The rejection reasons are `unsupported_entry_style`, `expired_intent`,
`slippage_limit`, `missing_atr`, `invalid_bracket`, `non_contiguous_bar`, and
the existing sizing and position reasons.

## 5. Invariants

- The strategy is stateless with respect to position.
- Execution and portfolio own the position.
- A rejected entry cannot create a virtual position.
- A bracket exit is applied to the portfolio before the next strategy decision.
- The strategy cannot set a fill price, fee, or slippage.
- Execution cannot change the side or risk distance.
- A `TradeIntent` is a request, not a fill.
- A `StrategyPosition` is the strategy's own state, not a market reading.
- `valid_until` is the last submission bar; an intent is never silently
  deferred to a later decision.

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
