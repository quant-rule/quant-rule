# AI Transparency Log — XAGUSD MACD-Only Pilot

- Date: 2026-11-03 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#361](https://github.com/quant-rule/quant-rule/issues/361)
- Plan:
  [`docs/execution-plans/2026-11-03/361-macd-pilot.md`](../../execution-plans/2026-11-03/361-macd-pilot.md)
- Claim type: model prediction evidence; no strategy or edge claim

## Intent and boundary

Test MACD alone on XAGUSD 1h as a fresh-asset prediction step after the EURUSD
RSI failure. Primary event: line/signal cross, next-1h return sign. No strategy,
costs, or portfolio.

## Work

- Wrote the plan and five XAGUSD 1h configs (2018–2022).
- Added a MACD event filter and prediction harness with dev/validation metrics.
- Recorded the decision after the primary result.

## Verification and evidence

- Five XAGUSD 1h model bundles are under `data/macd-pilot/`.
- Primary MACD line/signal cross, development: hit 0.4586, base 0.5030, lift
  -0.0444, LCB 0.4243 (n=809).
- Validation: hit 0.4831, base 0.4926, lift -0.0095, LCB 0.4471 (n=739).
- Zero cross and histogram slope flip are also below base on both partitions.
- Decision: stop; the RSI+MACD conjunction was not run.
- `python3 -m unittest discover -s tools/improvement`: 26 tests pass.

## Learning

- The RSI failure is not a reason to combine indicators; MACD is tested alone on
  a fresh asset.
- The MACD line/signal cross is a distinct event from the zero cross and the
  histogram slope flip.
- One asset/timeframe is a pilot; a positive result still needs replication.

## Rejected suggestions

- RSI+MACD conjunction on EURUSD after the RSI failure.
- Reusing BTC/ETH/XAU or EURUSD validation.
- Tuning the horizon after the result.

## Limitations

- One asset and one timeframe.
- Sign outcomes only.
- Same-session analysis, not an independent Quant Validator record.
