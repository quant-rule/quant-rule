# AI Transparency Log — ESANDP500 Pivot-Level Structure Pilot

- Date: 2026-11-03 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#363](https://github.com/quant-rule/quant-rule/issues/363)
- Plan:
  [`docs/execution-plans/2026-11-03/363-structure-pilot.md`](../../execution-plans/2026-11-03/363-structure-pilot.md)
- Claim type: model prediction evidence; no strategy or edge claim

## Intent and boundary

Test the structure family on a fresh asset after the RSI and MACD stops. The
primary question is whether a confirmed pivot Level test predicts the next-1h
return sign better than the directional base rate. The work is prediction-only:
no strategy, costs, execution, or portfolio. No RSI/MACD conjunction, no
cross-timeframe aggregation, and no threshold or horizon tuning.

## Work

- Wrote the pre-registered plan before running the structure result.
- Created five ESANDP500 1h one-year configs (2018–2022).
- Generated the five model bundles and recorded their provenance.
- Added a structure harness that filters confirmed pivot Level tests, crosses,
  and breaks, applies the fresh-test and one-vote-per-bar rules, and reports the
  directional base rate, hit rate, lift, and lower bounds.
- Added unit tests for the event filter, freshness rule, bar de-duplication, and
  decision logic.
- Recorded the development result first and the validation result once.

## Verification and evidence

- Five ESANDP500 1h model bundles are under `data/structure-pilot/` (local,
  gitignored).
- Primary confirmed pivot Level test, development: raw 3,445, fresh 2,338, bars
  1,822, valid n 1,423, hit 0.4814, base 0.4998, lift -0.0184, lift LCB -0.0457.
- Validation: raw 4,036, fresh 2,627, bars 1,983, valid n 1,741, hit 0.4963,
  base 0.4993, lift -0.0030, lift LCB -0.0278.
- The secondary cross has a positive point lift on both partitions but a
  negative lift LCB on both; it is not promoted. The break is negative.
- Decision: stop. No progression pilot and no strategy were run.
- An independent 2018 calculation matches the harness exactly: raw 825, fresh
  560, bars 438, valid n 329, hits 153, hit 0.4650, base 0.4960, lift -0.0310.
- `python3 -m unittest discover -s tools/improvement`: 39 tests pass.

## Learning

- A Level test event can repeat while price stays in the proximity zone, so the
  raw event count is not the number of independent tests. The harness reports
  raw, fresh, and prepared counts.
- The event set mixes long and short directions, so a plain 0.50 hit-rate bound
  is not a fair comparison. The directional base rate is the correct benchmark.
- Structure is a different source from RSI and MACD; a result here is new
  evidence, not a recombination of the failed pilots.
- The confirmed pivot Level test also failed. The classic level-test thesis did
  not survive a fresh asset, a directional base rate, and a fresh-test filter.
- The secondary cross was the only positive point estimate, but its lower bound
  did not clear the base rate. A point estimate is not a prediction.

## Rejected suggestions

- Reusing EURUSD, XAGUSD, BTC, ETH, or XAU as "fresh" evidence.
- Adding RSI or MACD to a structure result to rescue it.
- Aggregating direction, momentum, structure, and progression into a score.
- Changing the test threshold, horizon, or sign after seeing validation.
- Counting every Level event as an independent observation.

## Limitations

- One asset, one timeframe, one period; a pilot, not a model baseline.
- Sign outcomes only; no costs or magnitude.
- The 1h series contains complete M1 hours only; the valid-transition rule keeps
  gaps out of the target, but the series is not a continuous clock.
- Level tests cluster; the fresh-test and one-vote-per-bar rules reduce the
  dependence but do not make the events independent.
- The validation partition is one-shot for this hypothesis.
- Same-session analysis, not an independent Quant Validator record.
