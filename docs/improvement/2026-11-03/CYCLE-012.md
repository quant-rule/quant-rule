# Improvement Cycle 012 — COPPERCMD Path Pilot

- Date: 2026-11-03 (UTC)
- Status: Stopped at the path step; no strategy
- Result: [`PATH-PILOT-COPPER`](PATH-PILOT-COPPER.md)
- Prior cycle: [`CYCLE-011`](CYCLE-011.md)
- Claim type: model validation-contract evidence; no edge claim

## Question

Does a confirmed pivot Level test on COPPERCMD 1h reach a 1 ATR target before a
1 ATR stop within 20 bars more often than the unconditional barrier rate?

## Result

No. Development: hit `0.4847`, base `0.4989`, lift `−1.41pp`, lift LCB `−4.47pp`
(n=1,539, 20 clusters). Validation: hit `0.4659`, base `0.4962`, lift `−3.03pp`,
lift LCB `−5.33pp` (n=1,964, 33 clusters).

The secondary Level cross has a positive point lift on both partitions
(`+1.07pp` development, `+1.89pp` validation), but the lift LCB is negative on
both (`−3.72pp`, `−2.34pp`). It is not promoted. The break is negative on
validation.

## Review

The path thesis was tested on its own terms: a claim-aligned 1 ATR target/stop
with a 1 ATR/20-bar horizon, a per-year directional barrier base, and a
cluster-robust bound for overlapping paths. The primary event fails development,
so the pre-registered rule stops the pilot. The result is not a prompt to change
the barrier, horizon, event, or direction.

The first harness run pooled bar indices across one-year bundles and produced an
invalid cluster count. The harness now clusters per bundle; the corrected result
is above. The first run is not evidence.

## Decision

Stop the path step. No replication, no strategy, and no RSI/MACD conjunction. A
new event family or a different target requires a new pre-registered hypothesis;
the current Level path claim is not supported on this asset.

## New Hypothesis

None from this sample. The next hypothesis must be a genuinely different event
family or target, not a barrier or horizon variant of this test on the same
asset.
