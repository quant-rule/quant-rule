# AI Transparency Log — Model Regime 003

- Date: 2026-11-01 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: `model-regime-003`
- Plan: [`docs/execution-plans/2026-11-01/model-regime-003.md`](../../execution-plans/2026-11-01/model-regime-003.md)
- Claim type: model evidence; no edge claim

## Intent and boundary

Add pre-registered volatility/participation regime conditioning to the
model-confidence harness and run `MODEL-2026-11-01-03` on the same 2021
partition. The model formula, strategy, and portfolio stay frozen. A positive
bucket is a new hypothesis, not a validated edge.

## Work

- Added pre-registered `atr_ratio`, `rvol_ratio`, and `context.code` bucket
  evidence to `model_confidence.py`, with a unit test.
- Ran `MODEL-2026-11-01-03` on BTCUSD, ETHUSD, and XAUUSD 2021 at 15m and 1h.
- Two of 120 buckets have a lower bound above 0.50: ETHUSD 1h Direction in
  coiling and XAUUSD 15m Momentum in settling. They do not repeat across
  instruments.
- No model, strategy, or portfolio baseline was accepted.

## Rejected suggestions

- Rejected: adding a new model reading in this run. The evidence first asks
  whether the existing published context already conditions the axes.
- Rejected: choosing a bucket after seeing the result and calling it an edge.

## Verification

- `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` — 7 tests pass.
- `MODEL-2026-11-01-03` run store — completed.
- Regime table: `artifacts/regime-summary.csv`.

## Limitations

- Same-session review, not an independent model judgment.
- One year, three instruments, two timeframes.
