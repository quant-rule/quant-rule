# Compression-Range Candidate — Pending Human Acceptance

- Date: 2026-11-04 (UTC)
- Decision: **accepted model baseline (human, 2026-11-04)**
- Event: `signal.compression_range` (type 3, subtype 8, source 5), 1m
- Protocol: [`docs/model/2026-11-04/compression-range.md`](../../model/2026-11-04/compression-range.md)
- Development: [`MODEL-2026-11-04-06`](MODEL-2026-11-04-06-compression-range-dev.md)
- Validation: [`MODEL-2026-11-04-07`](MODEL-2026-11-04-07-compression-range-validation.md)

## What is claimed

A completed 5-bar coiling window is followed by a next-bar touch of at least one
of its published range boundaries more often than a matched non-coiling window
completion. The event publishes `range_high` and `range_low`; it is neutral and
does not claim the direction of the expansion.

## Gate

| Partition | n | J | Hit | Matched base | Lift | Lift LCB | Rand p | Back lag | Rep |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `model_dev` | 346,120 | 60,605 | 0.6474 | 0.5442 | +0.1032 | +0.1011 | 0.0050 | +0.0531 | 3 |
| `model_validation` | 247,989 | 43,937 | 0.5802 | 0.4984 | +0.0818 | +0.0795 | 0.0050 | +0.0291 | 3 |

Both partitions pass every pre-registered criterion. No threshold was tuned
after the development result.

## Second-use disclosure

`model_validation` 2021–2022 was already used for `MODEL-2026-11-04-05`, the
accepted 1m 4 ATR compression-breakout baseline. This candidate uses the same
partition for a different event and outcome. The reuse is disclosed; residual
data-snooping risk is higher than a first use.

## What followed acceptance

The 1m compression-bracket strategy ran `strategy_dev` 2018–2022 and recorded
**no edge**: every one-year fold halted at the 10% drawdown limit, with BTC
−41.23%, ETH −26.03%, and XAU −41.18%. The aggregate gross P&L was +$2,887
against $15,753 of commission. The 2024 `strategy_validation` partition was not
opened.

The model baseline remains accepted. The model claim is next-bar boundary touch,
not a 4 ATR payoff; the strategy failure does not invalidate it.

## Acceptance

- Accepted by (human): Human
- Accepted on: 2026-11-04
- Scope: 1m, 5 coiling bars, next-bar boundary touch
- Supersedes: none
