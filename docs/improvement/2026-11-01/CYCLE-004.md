# Improvement Cycle 004 — Event Confidence

- Date: 2026-11-01 (UTC)
- Status: Stopped at the model gate; no strategy or portfolio baseline
- Model run: [`MODEL-2026-11-01-04`](MODEL-2026-11-01-04-events.md)
- Prior cycle: [`CYCLE-003`](CYCLE-003.md)
- Claim type: model evidence; no edge claim

## Question

Do the published Level, Divergence, and Signal events carry a forward edge?

## Result

Four of 300 buckets are positive, all on BTCUSD: level cross at 1h/1 bar and
15m/3/10 bars, and level test at 15m/1 bar. ETHUSD and XAUUSD do not repeat them.
Divergence and Signal do not produce a stable bucket.

## Review

The event layer contains a small BTCUSD level effect, but it is one instrument
and one year. Selecting it would be post-hoc. The model gate remains closed.

## Decision

Stop at the model gate. No baseline is accepted.

## New Hypothesis

The evidence now covers axes, horizons, regimes, and events. None produces a
stable model baseline. The next step must be a deliberate contract decision:

1. Condition the model axes on context.
2. Expose a filtered regime/event view to the strategy.
3. Declare Direction/Momentum descriptive only and build on a different rule
   family.

No option is implemented in this cycle.
