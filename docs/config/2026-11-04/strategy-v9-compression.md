# Strategy v9 — 1m Compression Breakout

- Date: 2026-11-04 (UTC)
- Status: **Implemented; strategy_dev no edge; validation not opened**
- Issue: [#375](https://github.com/quant-rule/quant-rule/issues/375)
- Plan: [`docs/execution-plans/2026-11-04/375-compression-strategy.md`](../../execution-plans/2026-11-04/375-compression-strategy.md)
- Model baseline: `MODEL-2026-11-04-04..05` (accepted 2026-11-04)
- Claim type: strategy evidence after costs; no profitability claim.

## 1. Rule

The strategy reads the published 1m compression-breakout event and takes its
direction at the next open. It never computes an indicator, reads a statistic,
or uses the model's context directly.

| Element | Rule |
|---|---|
| Event | type 3, subtype 7, source 5, on the 1m execution timeframe |
| Trigger | The event is available on the current closed 1m bar |
| Entry | Market at the next 1m open, side = event direction |
| Stop | 1 ATR from the fill |
| Target | 4 ATR from the fill (reward-to-risk 4.0) |
| Time stop | 20 bars |
| Contiguity | A non-contiguous 1m bar blocks a new entry |
| Opposite event | Not an exit; the stop, target, or time stop closes the position |

The strategy is symmetric: long and short use the same rule and the same risk
distances. There are no extra filters, no threshold variants, and no score.

## 2. Costs and execution

| Item | Value |
|---|---|
| Fee | 4 bps per side |
| Slippage | 2 bps |
| Fill | Market next open |
| Tie-break | Stop first |
| Initial cash | 10,000 USD |
| Max position fraction | 1.0 |
| Max drawdown | 10% |

## 3. Partitions and folds

| Partition | Window | Instruments | Use |
|---|---|---|---|
| `strategy_dev` | 2018-01-01..2022-12-31 | BTCUSD, ETHUSD, XAUUSD | Development |
| `strategy_validation` | 2024-01-01..2024-12-31 | BTCUSD, ETHUSD, XAUUSD | One-shot validation |

Development is five one-year folds per instrument. The strategy config is fixed
before the first run; there is no parameter variation and no selection among
variants.

## 4. Green criteria

Development and validation use the same criteria:

1. Total return is positive after costs.
2. Maximum drawdown is at most 10%.
3. At least 30 closed trades.
4. Profit factor is greater than 1.0.
5. Positive return on at least 2 of 3 instruments.

A development pass freezes the rule; validation is opened once. A validation
pass is a candidate strategy baseline pending human acceptance, not a portfolio
go. A development or validation failure records no edge and stops.

## 5. Boundaries

- The model claim is expansion, not direction; the strategy's directional entry
  is a separate hypothesis and may fail after costs.
- The 1m timeframe is cost-sensitive; the fee and slippage assumptions are part
  of the claim.
- No portfolio, leverage, or live-trading claim is made.

## 6. Result

The directional rule failed `strategy_dev`. All 15 one-year folds halted at the
10% drawdown limit; compounded returns were BTC −40.97%, ETH −31.80%, XAU
−41.17%; the pooled profit factor was 0.481 and no instrument was positive. The
only positive fold (ETH 2018, +1.23%) still halted at a 10.13% drawdown.

The validation partition was not opened. No strategy baseline is accepted. The
accepted model baseline remains valid; the model claim is expansion, not
direction.

- Gate: [`STRATEGY-V9-GATE`](../../improvement/2026-11-04/STRATEGY-V9-GATE.md)
- Run record: [`STRATEGY-2026-11-04-01`](../../improvement/2026-11-04/STRATEGY-2026-11-04-01-compression-strategy.md)
