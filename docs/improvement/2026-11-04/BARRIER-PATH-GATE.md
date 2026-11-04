# BARRIER-PATH-GATE — 5m Onset Directional Path Decision

- Date: 2026-11-04 (UTC)
- Diagnostic run: [`MODEL-2026-11-04-12`](MODEL-2026-11-04-12-barrier-path.md)
- Protocol: [`docs/model/2026-11-04/barrier-path-5m-onset.md`](../../model/2026-11-04/barrier-path-5m-onset.md)
- Gate: **no directional edge**
- Claim type: model/analytics diagnostic; no strategy or profitability claim

## Decision

The 5m fresh-onset event has no directional barrier edge. The compression
bracket program stops.

- 10,349 entries; momentum and fade sides; 1–4 ATR targets; 1 ATR stop; 20 bars.
- 1 ATR target hit rate: 47.6% momentum, 47.6% fade, against 50% break-even.
- 2 ATR target hit rate: 31.5% momentum, 29.8% fade, against 33.3% break-even.
- Gross R is near zero or negative for every side/target.
- The 12 bps cost is about 1.21 R per trade, so no small gross edge survives.

No side/target is promising; no instrument has a positive net mean.

## What this means

The compression event is a range-expansion/timing measurement, not a directional
signal. The 1m and 5m brackets tested the momentum use; the fade side is no
better. The accepted model baselines remain valid.

## Next step

The bracket program stops. A new program would need a different market
hypothesis (for example a trend or mean-reversion event family) and a new
pre-registered model gate. That is outside compression and needs a new issue and
human approval.

## Limitations

- The path scan is an OHLC approximation.
- The cost estimate is an approximation in R.
- The diagnostic is not a strategy result.
- Same-session analysis, not an independent Quant Validator record.
