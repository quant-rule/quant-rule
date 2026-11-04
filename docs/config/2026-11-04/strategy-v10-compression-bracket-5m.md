# Strategy v10 — 5m Compression Bracket

- Date: 2026-11-04 (UTC)
- Status: **Pre-registered; 5m model gate no edge; strategy not run**
- Issue: [#379](https://github.com/quant-rule/quant-rule/issues/379)
- Plan: [`docs/execution-plans/2026-11-04/379-compression-bracket-5m.md`](../../execution-plans/2026-11-04/379-compression-bracket-5m.md)
- Model event: [`signal.compression_range`](../../model/2026-11-04/compression-range-5m.md)
- Prior 1m bracket: [`STRATEGY-BRACKET-GATE`](../../improvement/2026-11-04/STRATEGY-BRACKET-GATE.md) — no edge
- Claim type: strategy evidence after costs; no profitability claim.

## 1. Rule

The rule is the same v10 two-sided bracket, evaluated on 5m. It reads the
neutral 5m compression-range event and places a stop entry at each published
boundary. The first fill chooses the side. The strategy never predicts the
breakout direction.

| Element | Rule |
|---|---|
| Event | type 3, subtype 8, source 5, direction 0, on the 5m execution timeframe |
| Trigger | The event is available on the current closed 5m bar |
| Upper entry | Buy stop at `range_high` |
| Lower entry | Sell stop at `range_low` |
| Validity | The next 5m bar only |
| OCO | The first fill cancels the other entry |
| Gap | An entry fills at the open when the open is beyond its stop |
| Same-bar ambiguity | If both stops would trigger and no gap resolves one side, neither fills |
| Stop | 1 ATR from the fill |
| Target | 4 ATR from the fill |
| Time stop | 20 bars (100 minutes) |
| Contiguity | A non-contiguous next 5m bar cancels the bracket |

There are no extra filters, no direction input, no threshold variants, and no
score. The rule is symmetric.

## 2. Costs and execution

| Item | Value |
|---|---|
| Fee | 4 bps per side (portfolio commission) |
| Slippage | 2 bps |
| Entry fill | Stop-entry price, or the gap open |
| Exit fill | Bracket level, or market next open for the time stop |
| Tie-break | Stop first for the position bracket |
| Initial cash | 10,000 USD |
| Max position fraction | 1.0 |
| Max drawdown | 10%, halt |
| Funding | 0 |

## 3. Partitions and folds

| Partition | Window | Instruments | Use |
|---|---|---|---|
| `strategy_dev` | 2018-01-01..2022-12-31 | BTCUSD, ETHUSD, XAUUSD | Development |
| `strategy_validation` | 2024-01-01..2024-12-31 | BTCUSD, ETHUSD, XAUUSD | One-shot validation |

Development is five one-year folds per instrument. The strategy config and the
5m range event are fixed before the first run; no parameter variation and no
selection among variants.

## 4. Green criteria

1. Average instrument return is positive after costs.
2. No one-year fold halts at the 10% drawdown limit.
3. At least 30 closed trades.
4. Pooled profit factor is greater than 1.0.
5. Positive return on at least 2 of 3 instruments.

A development pass freezes the rule; validation is opened once. A validation
pass is a candidate strategy baseline pending human acceptance, not a portfolio
go. A development or validation failure records no edge and stops.

## 5. Rationale

The 1m bracket had aggregate gross P&L +$2,887 against $15,753 of commission:
the fixed 4 bps cost was about 5.5× the gross edge. On 5m, one ATR is about five
times larger for the same notional, so the commission is a smaller fraction of
one risk unit; the bar count and trade count are lower. The target, stop, and
costs are unchanged.

## 6. Boundaries

- The model claim is next-bar range resolution, not direction.
- A 5m bar is five 1m bars; the data engine owns the aggregation and gap
  handling.
- The strategy may still fail after costs even if the 5m model gate passes.
- No portfolio, leverage, or live-trading claim is made.

## 7. Result

The 5m model gate failed on the pre-registered back-lag control: the cell had a
+8.13pp lift but only a +4.80pp incremental edge over the one-bar-lagged rule,
0.20pp below the +5pp gate. The 5m strategy was not run; the 2024 validation
partition was not opened; no strategy baseline is accepted. See
[`COMPRESSION-RANGE-5M-GATE`](../../improvement/2026-11-04/COMPRESSION-RANGE-5M-GATE.md).
