# STRATEGY-BRACKET-GATE — 1m Compression Bracket Decision

- Date: 2026-11-04 (UTC)
- Development run: [`STRATEGY-2026-11-04-02`](STRATEGY-2026-11-04-02-compression-bracket.md)
- Validation run: none — not opened
- Contract: removed in #463; this record is the durable evidence
- Gate: **no edge**
- Claim type: strategy evidence after costs; no profitability claim

## Decision

The 1m compression-bracket strategy fails `strategy_dev`.

- All 15 one-year folds halted at the 10% drawdown limit.
- Compounded returns: BTC −41.23%, ETH −26.03%, XAU −41.18%.
- Pooled profit factor 0.592; positive instruments 0/3.
- Aggregate gross P&L +$2,887 versus $15,753 commission.

The 2024 validation partition is not opened. No strategy baseline is accepted
and no portfolio work follows.

## What this means

The accepted range model baseline is a **next-bar boundary-touch** claim. It
does not claim a 4 ATR follow-through, and the bracket's 4 ATR target / 1 ATR
stop does not convert the touch into a cost-aware edge. The model baseline
remains valid; this is a strategy-tier failure.

The 1m cost floor is the dominant cause: the gross edge per trade is about
$1.44, while the 4 bps commission is about $7.84 per trade. Changing the target,
stop, or filters after this result would be post-hoc tuning.

## Next step

The logical structural change is the same bracket rule on **5m**, where 1 ATR is
about five times larger relative to the fixed commission and the trade count is
lower. That needs:

- a new 5m compression-range model gate; and
- a new pre-registered strategy protocol.

It is a new hypothesis, not a threshold change to this rule. It needs a new
issue and human approval.

## Limitations

- One-year folds reset the account; a continuous account would halt earlier.
- The model baseline and the strategy are different claims; no model
  invalidation is implied.
- Same-session review, not an independent Quant Validator record.
