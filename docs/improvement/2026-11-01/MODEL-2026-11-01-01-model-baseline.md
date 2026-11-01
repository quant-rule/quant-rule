# MODEL-2026-11-01-01 — Model Baseline Harness Pilot

- Date: 2026-11-01 (UTC)
- Tier: model
- Changed: model evaluation
- Frozen: strategy, portfolio
- Partition: `model_validation` (2021)
- Manifest: [`manifests/MODEL-2026-11-01-01.json`](manifests/MODEL-2026-11-01-01.json)
- Local artifacts: `data/improvement/2026-11-01/MODEL-2026-11-01-01/`
- Verdict: `not green — no model baseline accepted`
- Claim type: harness pilot; no edge claim
- Protocol: [`docs/model/2026-11-01/model-evidence-protocol.md`](../../model/2026-11-01/model-evidence-protocol.md)

## Hypothesis

The model baseline can be graded from the published axes before a strategy run.
The pre-registered trust rule is A/B for every axis a strategy reads; C is
diagnostic only; D blocks.

## Run

The run store executed the BTCUSD 2021 configuration through the release shell
and wrote the five bundles. The model confidence harness then evaluated
BTCUSD/ETHUSD/XAUUSD 2021 at 15m and 1h.

| Cell | Direction hit (95% LCB) | Momentum hit (95% LCB) | Structure cov. | Progression cov. |
|---|---|---|---:|---:|
| BTCUSD 15m | 0.4824 (0.4771) | 0.4727 (0.4675) | 0.999 | 0.211 |
| BTCUSD 1h | 0.4874 (0.4768) | 0.4661 (0.4556) | 0.994 | 0.355 |
| ETHUSD 15m | 0.4891 (0.4838) | 0.4754 (0.4702) | 0.999 | 0.319 |
| ETHUSD 1h | 0.5051 (0.4944) | 0.4882 (0.4775) | 0.997 | 0.478 |
| XAUUSD 15m | 0.4942 (0.4878) | 0.4888 (0.4824) | 0.998 | 0.254 |
| XAUUSD 1h | 0.4875 (0.4745) | 0.4910 (0.4782) | 0.993 | 0.413 |

Direction is below 0.50 in five of six cells; ETHUSD 1h is not distinguishable
from chance. Momentum is below 0.50 in every cell. Structure and Progression are
not yet graded by the harness; Progression coverage is low.

## Review

- `data/improvement/2026-11-01/MODEL-2026-11-01-01/review.md`
- `data/improvement/2026-11-01/MODEL-2026-11-01-01/decision.md`
- Artifacts: `results.json`, `artifacts/evidence.json`,
  `artifacts/model-confidence-summary.csv`, `artifacts/confidence/*.json`

## Decision

No model baseline is accepted. The strategy and portfolio tiers are blocked. The
next hypothesis is a model-improvement run that grades more than one horizon,
adds Structure monotonicity and Progression target-before-invalidation, and
tests whether a regime/volatility reading is needed.

## Limitations

- One year and three instruments; not a full multi-year validation.
- The confidence harness does not grade events, levels, divergences, or signals.
- The model-series command contract was extended to 100,000 output bars and
  600,000 input M1 candles for the harness.
- This is a same-session review, not an independent Quant Validator record.
