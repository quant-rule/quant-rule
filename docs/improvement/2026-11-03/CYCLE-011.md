# Improvement Cycle 011 — ESANDP500 Structure Pilot

- Date: 2026-11-03 (UTC)
- Status: Stopped at the structure step; no strategy
- Result: [`STRUCTURE-PILOT-SPX`](STRUCTURE-PILOT-SPX.md)
- Prior cycle: [`CYCLE-010`](CYCLE-010.md)
- Claim type: model prediction evidence; no edge claim

## Question

Does a confirmed pivot Level test on ESANDP500 1h predict the next-hour return
sign better than the directional base rate?

## Result

No. Development: hit `0.4814`, base `0.4998`, lift `−1.84pp`, lift LCB `−4.57pp`
(n=1,423). Validation: hit `0.4963`, base `0.4993`, lift `−0.30pp`, lift LCB
`−2.78pp` (n=1,741).

The secondary pivot-Level cross has a small positive point lift on both
partitions (`+1.00pp` development, `+1.03pp` validation), but the lift LCB is
negative on both (`−4.19pp`, `−3.69pp`). It is not promoted. The break is
negative on both partitions.

## Review

The primary event fails development, so the pre-registered rule stops the
structure step. The result is not a prompt to change the test threshold, the
horizon, the fresh-test rule, or the direction sign. The cross is the only
positive point estimate, and it is not statistically distinguishable from the
base rate.

## Decision

Stop the structure step. No progression pilot, no strategy, and no RSI/MACD
conjunction. A new event definition or a different source is required before the
next step.

## New Hypothesis

None from this sample. Do not re-test the same Level-test event on the same
ESANDP500 2018–2022 validation. The next hypothesis must be a genuinely
different event definition or a different asset/timeframe, pre-registered before
the run.
