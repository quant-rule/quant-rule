# Strategy v10 — Fresh-Onset 5m Compression Bracket

- Date: 2026-11-04 (UTC)
- Status: **Implemented; strategy_dev no edge; validation not opened**
- Issue: [#381](https://github.com/quant-rule/quant-rule/issues/381)
- Plan: [`docs/execution-plans/2026-11-04/381-compression-onset.md`](../../execution-plans/2026-11-04/381-compression-onset.md)
- Model event: [`signal.compression_range_onset`](../../model/2026-11-04/compression-range-onset-5m.md)
- Candidate: [`COMPRESSION-ONSET-5M-CANDIDATE`](../../improvement/2026-11-04/COMPRESSION-ONSET-5M-CANDIDATE.md)
- Claim type: strategy evidence after costs; no profitability claim.

## 1. Rule

The rule is the v10 two-sided bracket on 5m, using the fresh-onset event. It
reads the neutral 5m onset event and places a stop entry at each published
boundary. The first fill chooses the side.

| Element | Rule |
|---|---|
| Event | type 3, subtype 9, source 5, direction 0, on the 5m execution timeframe |
| Trigger | The event is available on the current closed 5m bar |
| Upper entry | Buy stop at `range_high` |
| Lower entry | Sell stop at `range_low` |
| Validity | The next 5m bar only |
| OCO | The first fill cancels the other entry |
| Gap | An entry fills at the open when the open is beyond its stop |
| Same-bar ambiguity | If both stops would trigger and no gap resolves one side, neither fills |
| Stop | 1 ATR from the fill |
| Target | 4 ATR from the fill |
| Time stop | 20 bars |
| Contiguity | A non-contiguous next 5m bar cancels the bracket |

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
5m onset event are fixed before the first run; no parameter variation.

## 4. Green criteria

1. Average instrument return is positive after costs.
2. No one-year fold halts at the 10% drawdown limit.
3. At least 30 closed trades.
4. Pooled profit factor is greater than 1.0.
5. Positive return on at least 2 of 3 instruments.

A development pass freezes the rule; validation is opened once. A validation
pass is a candidate strategy baseline pending human acceptance. A failure
records no edge and stops.

## 5. Rationale

The 1m bracket had aggregate gross P&L +$2,887 against $15,753 of commission. On
5m, one ATR is about five times larger for the same notional, and the fresh
onset fires once per coiling run instead of every rolling window, so the entry
count is lower. The target, stop, and costs are unchanged.

## 6. Boundaries

- The model claim is next-bar range resolution, not direction.
- The strategy may still fail after costs even if the model gate passes.
- No portfolio, leverage, or live-trading claim is made.

## 7. Result

The 5m onset bracket failed `strategy_dev`: all 15 folds halted, BTC −37.47%,
ETH −24.97%, XAU −40.96%, pooled PF 0.654, 0/3 positive. The aggregate gross
P&L was −$2,286 against $9,960 of commission, so the 4 ATR target / 1 ATR stop
payoff is negative before costs. The 2024 validation was not opened; no strategy
baseline is accepted.

- Gate: [`STRATEGY-ONSET-5M-GATE`](../../improvement/2026-11-04/STRATEGY-ONSET-5M-GATE.md)
- Run record: [`STRATEGY-2026-11-04-03`](../../improvement/2026-11-04/STRATEGY-2026-11-04-03-compression-onset-bracket.md)
