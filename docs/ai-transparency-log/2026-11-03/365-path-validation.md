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

The human approved the path-based thesis and the exact protocol, estimator, and
pass rule. Implementation begins under the approved plan. No model bundle,
harness result, or validation result has been produced yet.

## Planned work

- Update the model contract's validation boundary and the registry claim wording
  to the path outcome.
- Add the path-validation protocol.
- Create five COPPERCMD 1h one-year configs.
- Generate the five model bundles.
- Add a path harness with unit tests for target, stop, timeout, same-bar tie,
  ATR null/zero, short history, gaps, cluster formation, and the decision rule.
- Run the development result, then the one-shot validation result.
- Record the decision as stop, inconclusive, development-only, or replication.

## Verification and evidence

No result is claimed before it is run. The plan fixes the verification commands
and the independent one-year check. The pull request will report the actual
commands and observed output.

## Learning

- The model contract already asks for path outcomes for level, structure,
  divergence, and progression claims; the gate's sign protocol was the gap.
- A path test needs a different sample-size treatment: overlapping 20-bar paths
  are not independent observations, so the bound must be cluster-robust.
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
- Same-session analysis, not an independent Quant Validator record.
