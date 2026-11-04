# MODEL-2026-11-04-12 — 5m Onset Barrier Path

- Date: 2026-11-04 (UTC)
- Tier: model (diagnostic)
- Changed: model
- Frozen: strategy, portfolio
- Partition: `model_dev` (2018-01-01..2020-12-31)
- Code commit: `f9b77d8` plus the 383 working tree
- Manifest: `data/improvement/2026-11-04/MODEL-2026-11-04-12/manifest.json`
- Protocol: [`docs/model/2026-11-04/barrier-path-5m-onset.md`](../../model/2026-11-04/barrier-path-5m-onset.md)
- Baseline: `MODEL-2026-11-04-10..11`
- Local artifacts: `data/improvement/2026-11-04/MODEL-2026-11-04-12/artifacts/`
- Verdict: **no directional edge — the compression bracket program stops**
- Claim type: model/analytics diagnostic; no strategy or profitability claim

## Path

| Side | Target | n | Target rate | Stop rate | Gross R | Cost R | Net R | Net LCB | Positive |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| momentum | 1 | 10,349 | 47.62% | 51.59% | −0.0405 | 1.2116 | −1.2521 | −2.0575 | 0/3 |
| momentum | 2 | 10,349 | 31.54% | 63.80% | +0.0101 | 1.2116 | −1.2015 | −2.0297 | 0/3 |
| momentum | 3 | 10,349 | 21.02% | 69.06% | +0.0121 | 1.2116 | −1.1995 | −2.0272 | 0/3 |
| momentum | 4 | 10,349 | 14.55% | 71.27% | +0.0185 | 1.2116 | −1.1930 | −2.0356 | 0/3 |
| fade | 1 | 10,349 | 47.59% | 51.59% | −0.0393 | 1.2116 | −1.2509 | −2.0426 | 0/3 |
| fade | 2 | 10,349 | 29.76% | 63.25% | −0.0004 | 1.2116 | −1.2120 | −2.0137 | 0/3 |
| fade | 3 | 10,349 | 19.38% | 67.04% | +0.0277 | 1.2116 | −1.1839 | −1.9931 | 0/3 |
| fade | 4 | 10,349 | 12.26% | 68.54% | +0.0300 | 1.2116 | −1.1815 | −1.9914 | 0/3 |

## Review

No side/target is promising. The 1 ATR target hit rate is 47.6% against a 50%
break-even; the 2 ATR target is 31.5% against 33.3%. The path is close to a
symmetric random walk, with no directional follow-through for either the
momentum or the fade side. The gross R is near zero or negative for every
side/target.

The 12 bps round-trip cost is about 1.21 R per trade, so even a small gross edge
would not survive. The 5m ATR is not five times the 1m ATR; volatility scales
roughly with the square root of time, so the fixed commission remains a large
fraction of one risk unit.

## Decision

The compression bracket program stops. No further target/stop, timeframe, or
filter variants are supported by the evidence. The accepted model baselines
remain valid; none claims direction.

## Next hypothesis

A new program would need a different market hypothesis (for example a trend or
mean-reversion event family) and a new pre-registered model gate. That is
outside the compression program and needs a new issue and human approval.

## Limitations

- The path scan is an OHLC approximation; the fade entry-bar target is suppressed
  to avoid a pre-entry favorable low.
- The cost estimate is an approximation in R.
- The diagnostic is not a strategy result and does not model account state.
- Same-session analysis, not an independent Quant Validator record.
