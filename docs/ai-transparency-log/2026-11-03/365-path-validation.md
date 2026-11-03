# AI Transparency Log — Path-Based Model Validation Pilot

- Date: 2026-11-03 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#365](https://github.com/quant-rule/quant-rule/issues/365)
- Plan:
  [`docs/execution-plans/2026-11-03/365-path-validation.md`](../../execution-plans/2026-11-03/365-path-validation.md)
- Claim type: model validation-contract evidence; no strategy or edge claim

## Intent and boundary

Replace the fixed-horizon sign thesis with a claim-aligned path thesis. The
primary question is whether a confirmed pivot Level test reaches a 1 ATR target
before a 1 ATR stop within 20 bars more often than the unconditional barrier
rate on COPPERCMD 1h. The work is prediction-only: no strategy, costs,
execution, or portfolio. No new indicator, threshold, or signal-path change.

## Status

Implementation completed under the approved plan. The pilot stopped: the primary
path lift is negative on both partitions. No replication, strategy, or further
tuning was run.

## Work

- Updated the model contract's validation boundary, the path-validation protocol,
  and the registry claim wording to the path outcome.
- Created five COPPERCMD 1h one-year configs and generated the five model bundles.
- Added a path harness with tests for target, stop, timeout, same-bar tie, ATR
  null/zero, short history, gaps, cluster formation, and the decision rule.
- Ran the development result and the one-shot validation result.
- Recorded the stop in `PATH-PILOT-COPPER` and `CYCLE-012`.

## Verification and evidence

- Development: raw 2,927, fresh 1,982, prepared 1,549, valid n 1,539, clusters
  20, hit 0.4847, base 0.4989, lift -0.0141, cluster SE 0.0156, lift LCB -0.0447.
- Validation: raw 3,922, fresh 2,578, prepared 1,979, valid n 1,964, clusters
  33, hit 0.4659, base 0.4962, lift -0.0303, cluster SE 0.0118, lift LCB -0.0533.
- Decision: stop. The secondary cross has a positive point lift on both
  partitions but a negative lift LCB on both; it is not promoted.
- An independent 2018 calculation matches the harness exactly: raw 1,163, fresh
  790, prepared 621, valid n 617, hits 318, hit 0.5154, base 0.4970, lift
  +0.0184, clusters 5, cluster SE 0.0238, lift LCB -0.0282.
- `python3 -m unittest discover -s tools/improvement`: 64 tests pass.
- `cargo fmt`, `clippy`, `test`, and `doc` pass; registry, links, and terms pass.

## Learning

- The model contract already asks for path outcomes for level, structure,
  divergence, and progression claims; the gate's sign protocol was the gap.
- A path test needs a different sample-size treatment: overlapping 20-bar paths
  are not independent observations, so the bound must be cluster-robust.
- The first harness run pooled event bar indices across one-year bundles. Bar
  indices restart at zero each year, so the cluster boundaries were wrong and the
  result was an invalid `inconclusive`. The harness now clusters per bundle. The
  corrected run is the evidence and the first run was discarded.
- The corrected development cluster count is exactly the pre-registered minimum
  of 20. The cluster-robust bound is retained, and the point estimate is
  negative; the stop does not depend on a borderline bound.
- A pass is not a strategy go. A single fresh asset is a candidate for
  replication, not an edge claim.

## Rejected suggestions

- Re-testing the sign thesis on another asset.
- Tuning the barrier, horizon, or direction after the result.
- Using the ESANDP500 validation again for the path test.
- Treating every level event as an independent observation.
- Running a strategy if the pilot passes.
- Adding RSI, MACD, or a new event family to rescue a failed path test.

## Limitations

- One asset and one timeframe; a pilot, not a model baseline.
- The entry reference is the event close; a strategy would use the next open and
  costs.
- Gaps are part of the 20-bar path.
- The base rate is a matched barrier baseline, not a matched price level.
- The development cluster count is at the minimum of 20; the bound is reported,
  but a larger sample would estimate it more precisely.
- Same-session analysis, not an independent Quant Validator record.
