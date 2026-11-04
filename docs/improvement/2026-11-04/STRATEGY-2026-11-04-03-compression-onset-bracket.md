# STRATEGY-2026-11-04-03 — 5m Fresh-Onset Compression Bracket

- Date: 2026-11-04 (UTC)
- Tier: strategy
- Changed: strategy
- Frozen: model, portfolio
- Partition: `strategy_dev` (2018-01-01..2022-12-31)
- Code commit: `2c85b41` plus the 381 working tree
- Manifest: `data/improvement/2026-11-04/STRATEGY-2026-11-04-03/manifest.json`
- Contract: [`docs/config/2026-11-04/strategy-v10-compression-bracket-onset-5m.md`](../../config/2026-11-04/strategy-v10-compression-bracket-onset-5m.md)
- Local artifacts: `data/improvement/2026-11-04/STRATEGY-2026-11-04-03/artifacts/`
  and the fifteen one-year bundles under
  `data/compression-bracket-onset-5m/dev/`
- Verdict: **no edge — every fold halted; negative gross edge**
- Claim type: strategy evidence after costs; no profitability claim

## Hypothesis

A two-sided stop-entry bracket at the fresh-onset 5m compression boundaries has
a positive cost-aware edge with a 1 ATR stop, a 4 ATR target, and a 20-bar time
stop. The first fill chooses the side; OCO cancels the other leg.

## Matrix

Fifteen one-year folds: BTCUSD, ETHUSD, XAUUSD × 2018–2022. The model baseline
`MODEL-2026-11-04-10..11` is frozen. Costs are 4 bps commission per operation,
2 bps slippage, and no funding. The entry bracket is valid for the next 5m bar
only.

## Results

| Instrument | Compounded return | Max DD | Trades | PF | Win rate | Halted folds |
|---|---:|---:|---:|---:|---:|---:|
| BTCUSD | −37.47% | 10.23% | 417 | 0.636 | 23.50% | 5/5 |
| ETHUSD | −24.97% | 10.29% | 447 | 0.843 | 24.83% | 5/5 |
| XAUUSD | −40.96% | 10.15% | 421 | 0.084 | 7.60% | 5/5 |

Average instrument return: −34.47%. Total trades: 1,285. Pooled profit factor:
0.654. Positive instruments: 0/3. Halted folds: 15/15.

## Review

Every fold halted at the 10% drawdown limit. The aggregate gross P&L is
**−$2,286** against $9,960 of commission, so the failure is not only the fixed
cost: the 4 ATR target / 1 ATR stop payoff is directionally wrong on this event.
The model's next-bar touch is direction-neutral; the bracket's momentum bet on
the touched side does not follow through often enough to pay for the stop.

XAUUSD is the worst at a 7.60% win rate and a 0.084 profit factor. The 4 ATR
target rarely fills before the 1 ATR stop.

## Decision

No strategy baseline is accepted. The 2024 validation partition is not opened.
No portfolio work follows. The rule is not tuned after the result.

## Next hypothesis

The negative gross edge suggests the touched boundary may mean-revert, but that
is not established. A **barrier path diagnostic** on the 5m onset event should
measure, for each touched side, whether the path reaches +1, +2, or +4 ATR
before −1 ATR within 20 bars. That is evaluation, not a strategy. It would show
whether any directional bracket has a positive gross expectancy before another
execution feature or strategy is built.

## Limitations

- One-year folds reset the account; a continuous account would halt earlier.
- The 5m bracket has fewer trades per fold, so each fold's estimate is noisy.
- The model baseline remains valid; this is a strategy-tier failure.
- Same-session review, not an independent Quant Validator record.
