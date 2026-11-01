# Improvement Cycle 003 — Context-Conditioned Model Evidence

- Date: 2026-11-01 (UTC)
- Status: Stopped at the model gate; no strategy or portfolio baseline
- Model run: [`MODEL-2026-11-01-03`](MODEL-2026-11-01-03-context.md)
- Prior cycle: [`CYCLE-002`](CYCLE-002.md)
- Claim type: model evidence; no edge claim

## Question

Do Direction and Momentum work inside the model's own volatility or
participation context?

## Result

Two of 120 pre-registered regime buckets have a lower bound above 0.50:

- ETHUSD 1h Direction in `code=2` coiling: 0.5261 (LCB 0.5080), n=2,914.
- XAUUSD 15m Momentum in `code=3` settling: 0.5185 (LCB 0.5011), n=3,196.

They are on different instruments, axes, and regimes, and neither repeats. The
result is a candidate hypothesis, not a model baseline.

## Review

The context contains some information, but not a consistent cross-instrument
rule. A strategy built on one bucket would fit one instrument-year. The model
gate remains closed.

## Decision

Stop at the model gate. No baseline is accepted.

## New Hypothesis

Choose one deliberate contract change before the next run:

1. Condition the model axes on context.
2. Expose a filtered regime/participation view to the strategy.
3. Declare Direction/Momentum descriptive only and build on Structure,
   Progression, or levels.

No option is implemented in this cycle. The next run must pre-register one and
keep the other tiers frozen.
