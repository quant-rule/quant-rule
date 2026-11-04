# Strategy v10 — 1m Compression Bracket

- Date: 2026-11-04 (UTC)
- Status: **Implemented; strategy_dev no edge; validation not opened**
- Issue: [#377](https://github.com/quant-rule/quant-rule/issues/377)
- Plan: [`docs/execution-plans/2026-11-04/377-compression-bracket.md`](../../execution-plans/2026-11-04/377-compression-bracket.md)
- Model event: [`signal.compression_range`](../../model/2026-11-04/compression-range.md)
- Model baseline: `MODEL-2026-11-04-04..05` (accepted 2026-11-04)
- Claim type: strategy evidence after costs; no profitability claim.

## 1. Rule

The strategy reads the neutral 1m compression-range event and places a
two-sided stop-entry bracket at the published range boundaries. It never
predicts the breakout direction, computes an indicator, or reads a statistic.

| Element | Rule |
|---|---|
| Event | type 3, subtype 8, source 5, direction 0, on the 1m execution timeframe |
| Trigger | The event is available on the current closed 1m bar |
| Upper entry | Buy stop at `range_high` |
| Lower entry | Sell stop at `range_low` |
| Validity | The next 1m bar only |
| OCO | The first entry fill cancels the other entry |
| Gap | An entry fills at the open when the open is beyond its stop |
| Same-bar ambiguity | If both stops would trigger and no gap resolves one side, neither fills |
| Stop | 1 ATR from the fill |
| Target | 4 ATR from the fill (reward-to-risk 4.0) |
| Time stop | 20 bars after the fill |
| Contiguity | A non-contiguous next bar cancels the bracket |

There are no extra filters, no direction input, no threshold variants, and no
score. The strategy is symmetric: long and short use the same rule and the same
risk distances.

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
range-event gate are fixed before the first strategy run; there is no parameter
variation and no selection among variants.

## 4. Green criteria

Development and validation use the same criteria:

1. Average instrument return is positive after costs.
2. No one-year fold halts at the 10% drawdown limit.
3. At least 30 closed trades.
4. Pooled profit factor is greater than 1.0.
5. Positive return on at least 2 of 3 instruments.

A development pass freezes the rule; validation is opened once. A validation
pass is a candidate strategy baseline pending human acceptance, not a portfolio
go. A development or validation failure records no edge and stops.

## 5. Boundaries

- The model claim is next-bar range resolution, not direction. The bracket is a
  separate cost-aware hypothesis and may fail.
- The event is available only after the coiling window closes; the bracket is
  placed before the next bar opens.
- The same-bar ambiguity rule is conservative: it forgoes a bar when OHLC cannot
  order the two triggers.
- The 1m timeframe is cost-sensitive; the fee and slippage assumptions are part
  of the claim.
- No portfolio, leverage, or live-trading claim is made.

## 6. Result

The 1m bracket failed `strategy_dev`. All 15 one-year folds halted at the 10%
drawdown limit; compounded returns were BTC −41.23%, ETH −26.03%, XAU −41.18%;
the pooled profit factor was 0.592 and no instrument was positive. The aggregate
gross P&L was +$2,887 against $15,753 of commission: the 4 bps cost was about
5.5× the gross edge.

The 2024 validation partition was not opened. No strategy baseline is accepted.
The accepted range model baseline remains valid; the model claim is next-bar
boundary touch, not a 4 ATR payoff.

- Gate: [`STRATEGY-BRACKET-GATE`](../../improvement/2026-11-04/STRATEGY-BRACKET-GATE.md)
- Run record: [`STRATEGY-2026-11-04-02`](../../improvement/2026-11-04/STRATEGY-2026-11-04-02-compression-bracket.md)
