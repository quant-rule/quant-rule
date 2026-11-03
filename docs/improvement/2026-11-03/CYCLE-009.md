# Improvement Cycle 009 — RSI Prediction Pilot

- Date: 2026-11-03 (UTC)
- Status: Stopped at the first prediction step; no strategy
- Result: [`PREDICTION-PILOT-RSI`](PREDICTION-PILOT-RSI.md)
- Prior cycle: [`CYCLE-008`](CYCLE-008.md)
- Claim type: model prediction evidence; no edge claim

## Question

On EURUSD 1h, does the RSI 30/70 reversion event predict the next-hour return
sign better than the base rate?

## Result

No. Development: hit `0.4663`, base `0.5007`, lift `−3.44pp`, LCB `0.4257`
(n=579). Validation: hit `0.4590`, base `0.4886`, lift `−2.97pp`, LCB `0.4051`
(n=329).

The secondary RSI-zone observation is positive at 3h on development (`0.5415`,
LCB `0.5196`) but not on validation (`0.5098`, LCB `0.4818`). It is not promoted.

## Review

The primary event fails on the development partition, so MACD is not added. The
first harness run counted non-reversion crossings as misses; that bug was fixed
before the decision. The corrected result is a stop.

## Decision

Stop the RSI step. Do not add MACD to rescue a failed primary event. Do not tune
the threshold, horizon, or sign after the result. A new event definition or
thesis is required before the next step.

## New Hypothesis

None from this sample. The next hypothesis must be a genuinely different
event definition or a different asset/timeframe, pre-registered before the run.
