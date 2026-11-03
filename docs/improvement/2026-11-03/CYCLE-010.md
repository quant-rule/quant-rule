# Improvement Cycle 010 — XAGUSD MACD Pilot

- Date: 2026-11-03 (UTC)
- Status: Stopped at the MACD step; no strategy
- Result: [`MACD-PILOT-XAG`](MACD-PILOT-XAG.md)
- Prior cycle: [`CYCLE-009`](CYCLE-009.md)
- Claim type: model prediction evidence; no edge claim

## Question

Does the MACD line/signal cross on XAGUSD 1h predict the next-hour return sign
better than the base rate?

## Result

No. Development: hit `0.4586`, base `0.5030`, lift `−4.44pp`, LCB `0.4243`
(n=809). Validation: hit `0.4831`, base `0.4926`, lift `−0.95pp`, LCB `0.4471`
(n=739). The zero cross and histogram slope flip are also below base on both
partitions.

## Review

The fresh asset did not rescue MACD. The primary event fails development, so the
RSI+MACD conjunction is not run. This is a stop, not a prompt for a new threshold.

## Decision

Stop the MACD step. No strategy. A new event definition or thesis is required
before the next step.

## New Hypothesis

None from this sample. The next hypothesis must be genuinely different, not a
recombination of RSI and MACD on the same evidence.
