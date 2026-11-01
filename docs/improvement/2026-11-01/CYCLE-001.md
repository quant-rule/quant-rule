# Improvement Cycle 001 — Model Gate

- Date: 2026-11-01 (UTC)
- Status: Stopped at the model gate; no strategy or portfolio baseline
- Model run: [`MODEL-2026-11-01-01`](MODEL-2026-11-01-01-model-baseline.md)
- Audit: [`docs/audits/2026-11-01/workflow-reset-audit.md`](../../audits/2026-11-01/workflow-reset-audit.md)
- Claim type: model evidence; no edge claim
- Protocol: [`docs/model/2026-11-01/model-evidence-protocol.md`](../../model/2026-11-01/model-evidence-protocol.md)

## Run

The new run store executed the model tier on the 2021 window. The confidence
harness evaluated BTCUSD, ETHUSD, and XAUUSD at 15m and 1h.

| Cell | Direction hit (95% LCB) | Momentum hit (95% LCB) | Progression coverage |
|---|---|---|---:|
| BTCUSD 15m | 0.4824 (0.4771) | 0.4727 (0.4675) | 0.211 |
| BTCUSD 1h | 0.4874 (0.4768) | 0.4661 (0.4556) | 0.355 |
| ETHUSD 15m | 0.4891 (0.4838) | 0.4754 (0.4702) | 0.319 |
| ETHUSD 1h | 0.5051 (0.4944) | 0.4882 (0.4775) | 0.478 |
| XAUUSD 15m | 0.4942 (0.4878) | 0.4888 (0.4824) | 0.254 |
| XAUUSD 1h | 0.4875 (0.4745) | 0.4910 (0.4782) | 0.413 |

## Results

The model gate did not pass. Direction and Momentum are below 0.50 in five of
six cells; the sixth is not distinguishable from 0.50. Structure and Progression
are not yet graded, and Progression coverage is low.

## Review

The prior strategy failures are consistent with a measurement problem at the
tested horizons, not only a threshold problem. A strategy run would consume axes
that the current evidence does not show to be predictive. Running it would
violate the tier contract and produce a conclusion that cannot be attributed.

## Decision

Stop at the model gate. Do not run a strategy or portfolio baseline. No baseline
is accepted.

## New Hypothesis

`MODEL-2026-11-02-01` — test whether the axes are defined at the wrong horizon,
whether the model needs a regime/volatility reading, and whether Structure and
Progression add a measurable edge. Pre-register the primary horizon and grade
every axis before the run. Keep strategy and portfolio frozen.

## Limitations

- One year, three instruments, two timeframes.
- The confidence harness does not grade events, levels, divergences, or signals.
- The review is a same-session sequential pass, not an independent Quant
  Validator record.
