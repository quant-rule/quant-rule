# STRATEGY-2026-11-04-02 — 1m Compression Bracket

- Date: 2026-11-04 (UTC)
- Tier: strategy
- Changed: strategy
- Frozen: model, portfolio
- Partition: `strategy_dev` (2018-01-01..2022-12-31)
- Code commit: `8e8a481` plus the 377 working tree
- Manifest: `data/improvement/2026-11-04/STRATEGY-2026-11-04-02/manifest.json`
- Contract: [`docs/config/2026-11-04/strategy-v10-compression-bracket.md`](../../config/2026-11-04/strategy-v10-compression-bracket.md)
- Local artifacts: `data/improvement/2026-11-04/STRATEGY-2026-11-04-02/artifacts/`
  and the fifteen one-year bundles under `data/compression-bracket/dev/`
- Verdict: **no edge — every fold halted at the 10% drawdown limit**
- Claim type: strategy evidence after costs; no profitability claim

## Hypothesis

A two-sided stop-entry bracket at the frozen 1m compression-range boundaries has
a positive cost-aware edge when bracketed by a 1 ATR stop, a 4 ATR target, and a
20-bar time stop. The first fill chooses the side; OCO cancels the other leg.

## Matrix

Fifteen one-year folds: BTCUSD, ETHUSD, XAUUSD × 2018–2022. The model baseline
`MODEL-2026-11-04-06..07` is frozen. Costs are 4 bps commission per operation,
2 bps slippage, and no funding. The entry bracket is valid for the next 1m bar
only.

## Results

| Instrument | Compounded return | Max DD | Trades | PF | Win rate | Halted folds |
|---|---:|---:|---:|---:|---:|---:|
| BTCUSD | −41.23% | 10.18% | 729 | 0.558 | 20.44% | 5/5 |
| ETHUSD | −26.03% | 10.18% | 822 | 0.815 | 27.62% | 5/5 |
| XAUUSD | −41.18% | 10.10% | 458 | 0.006 | 1.31% | 5/5 |

Average instrument return: −36.14%. Total trades: 2,009. Pooled profit factor:
0.592. Positive instruments: 0/3. Halted folds: 15/15.

## Review

Every fold halted at the 10% drawdown limit. ETHUSD 2018 finished +9.16% from the
initial balance but still halted at a 10.02% drawdown from its peak; no fold is
clean. XAUUSD is the worst at a 1.31% win rate and a 0.006 profit factor.

The aggregate gross P&L across the fifteen folds is **+$2,887**, while the
commission is **$15,753**. The 4 bps commission is about 5.5× the gross edge and
about $7.84 per trade against a gross edge of about $1.44 per trade. The bracket
captures some expansion, but not enough to clear the 1m cost floor. The model's
next-bar touch claim does not imply a 4 ATR payoff.

## Decision

No strategy baseline is accepted. The 2024 validation partition is not opened.
No portfolio work follows. The 1m rule is not tuned after the result.

## Next hypothesis

The 1m cost floor dominates the gross edge. The logical structural change is the
same bracket rule on 5m, where 1 ATR is about five times larger relative to the
fixed commission and the trade count is lower. That needs a new 5m
compression-range model gate and a new pre-registered strategy protocol. It is
not a threshold change to this rule.

## Limitations

- One-year folds reset the account; a continuous account would halt earlier.
- The 1m timeframe is cost-sensitive by construction.
- The bracket places an order on every completed coiling window, including
  overlapping windows.
- Same-session review, not an independent Quant Validator record.
