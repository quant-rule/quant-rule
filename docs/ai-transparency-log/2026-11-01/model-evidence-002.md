# AI Transparency Log — Model Evidence 002

- Date: 2026-11-01 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: `model-evidence-002`
- Plan: [`docs/execution-plans/2026-11-01/model-evidence-002.md`](../../execution-plans/2026-11-01/model-evidence-002.md)
- Claim type: model evidence; no edge claim

## Intent and boundary

Extend the model-confidence harness to a pre-registered multi-horizon grid and
ordered-axis monotonicity, then run `MODEL-2026-11-01-02` on the same 2021
partition. The model formula, strategy, and portfolio stay frozen. A positive
horizon is a new hypothesis, not a validated edge.

## Work

- Extended `model_confidence.py` with the pre-registered horizon grid
  `1,3,5,10,20` and ordered-axis Spearman monotonicity.
- Added `test_model_confidence.py`; six unit tests pass.
- Ran `MODEL-2026-11-01-02` on BTCUSD, ETHUSD, and XAUUSD 2021 at 15m and 1h.
- Direction is below 0.50 at every horizon in five of six cells; Momentum is
  below 0.50 everywhere. Structure and Progression monotonicity are unstable.
- No model, strategy, or portfolio baseline was accepted.

## Rejected suggestions

- Rejected: changing the Direction/Momentum formula in this run. The point is to
  separate a horizon problem from an axis problem.
- Rejected: choosing the best horizon after seeing the result and calling it an
  edge. The horizon grid is pre-registered and descriptive.

## Verification

- `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` — 6 tests pass.
- `python3 -m py_compile tools/improvement/*.py` — pass.
- `MODEL-2026-11-01-02` run store — completed.
- The confidence table is in `artifacts/model-confidence-summary.csv`.

## Limitations

- Same-session review, not an independent model judgment.
- One year and three instruments; no 2023 or 2025 data in this model run.
