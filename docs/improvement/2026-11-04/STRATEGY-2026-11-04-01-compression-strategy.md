# STRATEGY-2026-11-04-01 — 1m Compression Breakout

- Date: 2026-11-04 (UTC)
- Tier: strategy
- Changed: strategy
- Frozen: model, portfolio
- Partition: `strategy_dev` (2018-01-01..2022-12-31)
- Code commit: `73b9ddd` plus the uncommitted strategy working tree
- Manifest: `data/improvement/2026-11-04/STRATEGY-2026-11-04-01/manifest.json`
- Contract: removed in #463; this record is the durable evidence
- Local artifacts: `data/improvement/2026-11-04/STRATEGY-2026-11-04-01/artifacts/`
  and the fifteen one-year bundles under `data/strategy-v9/dev/`
- Verdict: **no edge — every fold halted at the 10% drawdown limit**
- Claim type: strategy evidence after costs; no profitability claim

## Hypothesis

The accepted 1m 4 ATR compression-breakout event has a positive cost-aware
strategy edge when entered in the event direction with a 1 ATR stop, 4 ATR
target, and 20-bar time stop.

## Matrix

Fifteen one-year folds: BTCUSD, ETHUSD, XAUUSD × 2018–2022. The model baseline
`MODEL-2026-11-04-04..05` is frozen. Costs are 4 bps commission per operation,
2 bps slippage, and no funding.

## Results

| Instrument | Compounded return | Max DD | Trades | PF | Win rate | Halted folds |
|---|---:|---:|---:|---:|---:|---:|
| BTCUSD | −40.97% | 10.16% | 698 | 0.512 | 24.36% | 5/5 |
| ETHUSD | −31.80% | 10.16% | 670 | 0.669 | 26.72% | 5/5 |
| XAUUSD | −41.17% | 10.09% | 454 | 0.014 | 2.64% | 5/5 |

Average instrument return: −37.98%. Total trades: 1,822. Pooled profit factor:
0.481. Positive instruments: 0/3.

## Review

Every fold halted at the 10% drawdown limit. The only positive fold, ETHUSD
2018 (+1.23%), still halted at a 10.13% drawdown. The strategy is consistently
stopped, not merely noisy. XAUUSD is the worst at a 2.64% win rate and a 0.014
profit factor.

The model's expansion claim is not a direction claim. Taking the breakout
direction does not preserve the validated range edge after 1m costs. The model
baseline remains valid; this is a strategy failure, not a model invalidation.

## Decision

No strategy baseline is accepted. The 2024 validation partition is not opened.
No portfolio work follows. The directional rule is not tuned.

## Next hypothesis

The expansion claim is direction-neutral. A bracket / stop-entry volatility
strategy would enter on either side of the compression range, letting the
expansion choose the side. The execution engine does not yet support a
two-sided stop-entry bracket; that is a new execution feature and a new
pre-registered strategy hypothesis. It needs a new issue and human approval.

## Limitations

- One-year folds reset the account; a continuous account would halt earlier.
- The 1m costs are material; the rule is cost-sensitive by design.
- Same-session review, not an independent Quant Validator record.
