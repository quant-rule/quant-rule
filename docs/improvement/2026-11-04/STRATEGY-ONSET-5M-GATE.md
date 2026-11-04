# STRATEGY-ONSET-5M-GATE — 5m Fresh-Onset Bracket Decision

- Date: 2026-11-04 (UTC)
- Development run: [`STRATEGY-2026-11-04-03`](STRATEGY-2026-11-04-03-compression-onset-bracket.md)
- Validation run: none — not opened
- Contract: [`docs/config/2026-11-04/strategy-v10-compression-bracket-onset-5m.md`](../../config/2026-11-04/strategy-v10-compression-bracket-onset-5m.md)
- Gate: **no edge**
- Claim type: strategy evidence after costs; no profitability claim

## Decision

The 5m fresh-onset compression bracket fails `strategy_dev`.

- All 15 one-year folds halted at the 10% drawdown limit.
- Compounded returns: BTC −37.47%, ETH −24.97%, XAU −40.96%.
- Pooled profit factor 0.654; positive instruments 0/3.
- Aggregate gross P&L −$2,286 versus $9,960 commission.

The 2024 validation partition is not opened. No strategy baseline is accepted
and no portfolio work follows.

## What this means

The accepted onset model baseline is a **next-bar boundary-touch** claim. It is
direction-neutral and does not claim a 4 ATR follow-through. The bracket turns
the touch into a momentum bet on the touched side; the development data rejects
that bet. Unlike the 1m bracket, the gross edge is negative before costs, so the
failure is not only the 4 bps commission.

The model baseline remains valid; this is a strategy-tier failure.

## Next step

A **barrier path diagnostic** should measure, for each touched side on the 5m
onset event, whether the path reaches +1, +2, or +4 ATR before −1 ATR within 20
bars. That is evaluation, not a strategy. It would show whether any directional
bracket has a positive gross expectancy before another execution feature or
strategy is built.

No target/stop change is permitted from this result. A new hypothesis needs a
new issue and human approval.

## Limitations

- One-year folds reset the account; a continuous account would halt earlier.
- The 5m bracket has fewer trades per fold.
- Same-session review, not an independent Quant Validator record.
