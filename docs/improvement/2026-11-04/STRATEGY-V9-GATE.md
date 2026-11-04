# STRATEGY-V9-GATE — Directional Compression Strategy Decision

- Date: 2026-11-04 (UTC)
- Development run: [`STRATEGY-2026-11-04-01`](STRATEGY-2026-11-04-01-compression-strategy.md)
- Validation run: none — not opened
- Contract: [`docs/config/2026-11-04/strategy-v9-compression.md`](../../config/2026-11-04/strategy-v9-compression.md)
- Gate: **no edge**
- Claim type: strategy evidence after costs; no profitability claim

## Decision

The directional 1m compression-breakout strategy fails `strategy_dev`.

- All 15 one-year folds halted at the 10% drawdown limit.
- Compounded returns: BTC −40.97%, ETH −31.80%, XAU −41.17%.
- Pooled profit factor 0.481; positive instruments 0/3.
- The only positive fold, ETH 2018 (+1.23%), still halted at a 10.13% drawdown.

The 2024 validation partition is not opened. No strategy baseline is accepted
and no portfolio work follows.

## What this means

The accepted model baseline is a **range-expansion** result, not a direction
result. The directional entry was a separate hypothesis, and the development
evidence rejects it. The model baseline remains valid; this is a strategy-tier
failure.

The 1m costs are material, but the failure is larger than cost drag: the XAU
profit factor is 0.014 and every instrument is negative. Lowering costs or
changing the target after the result would be post-hoc tuning.

## Next step

The logical next hypothesis is a **bracket / stop-entry volatility strategy**:
enter on either side of the compression range, let the expansion choose the
side, and bracket the position. The execution engine currently supports
market-next-open entries and stop-loss exits, not a two-sided stop-entry
bracket. That is a new execution feature and a new pre-registered strategy
hypothesis. It needs a new issue and human approval.

## Limitations

- One-year folds reset the account; a continuous account would halt earlier.
- The model baseline and the strategy are different claims; no model
  invalidation is implied.
- Same-session review, not an independent Quant Validator record.
