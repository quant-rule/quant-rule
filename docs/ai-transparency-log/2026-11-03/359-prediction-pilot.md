# AI Transparency Log — Single-Timeframe Prediction Pilot

- Date: 2026-11-03 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#359](https://github.com/quant-rule/quant-rule/issues/359)
- Plan:
  [`docs/execution-plans/2026-11-03/359-prediction-pilot.md`](../../execution-plans/2026-11-03/359-prediction-pilot.md)
- Claim type: model prediction evidence; no strategy or edge claim

## Intent and boundary

Run a small, additive prediction pilot on a fresh asset: EURUSD 1h, 2018–2022,
RSI 30/70 first, next-1h return sign. No strategy, no costs, no portfolio. The
pilot tests whether one indicator event predicts better than the base rate.

## Work

- Wrote the approved plan and the pilot configs.
- Generated five one-year model bundles (2018–2022) with MACD active.
- Added a dedicated prediction harness that filters RSI 30/70 events and RSI
  zone states, and reports n, base rate, hit rate, lift, and lower bound.
- The development result is reported first; validation is one-shot and is not
  used for tuning.

## Verification and evidence

- Five EURUSD 1h model bundles are under `data/prediction-pilot/`.
- Primary RSI 30/70 reversion, development: hit 0.4663, base 0.5007, lift
  -0.0344, LCB 0.4257 (n=579).
- Validation: hit 0.4590, base 0.4886, lift -0.0297, LCB 0.4051 (n=329).
- Decision: stop. MACD was not added.
- The first harness run counted non-reversion crossings as misses; the filter was
  corrected before the decision. The corrected result is the evidence.
- `python3 -m unittest discover -s tools/improvement`: 22 tests pass.

## Learning

- RSI/MACD were already tested on BTC/ETH/XAU in the v3 gate. EURUSD is a fresh
  asset; the pilot is new evidence for this asset, not a new test of the old
  data.
- A 1h RSI 30/70 crossing sample may be small; the confidence bound decides
  whether the result is conclusive.
- The pilot is prediction evidence, not a strategy pretest. A positive result
  still needs costs and a strategy run later.
- The primary reversion event failed; the secondary RSI-zone 3h development cell
  was positive but the validation LCB was below 0.50, so it was not promoted.

## Rejected suggestions

- Re-running RSI/MACD on BTC/ETH/XAU 2018–2022 and calling it new.
- Aggregating the axes into a score.
- Adding MACD to rescue a failed RSI result.
- Tuning the threshold or horizon after seeing validation.

## Limitations

- One asset and one timeframe; a pilot, not a baseline.
- Yearly folds are pooled.
- Same-session analysis, not an independent Quant Validator record.
