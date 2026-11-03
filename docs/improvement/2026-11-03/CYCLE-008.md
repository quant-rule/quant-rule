# Improvement Cycle 008 — Model Gate

- Date: 2026-11-03 (UTC)
- Status: Stopped at the model gate; no model, strategy, or portfolio baseline
- Combined gate: [`MODEL-GATE`](MODEL-GATE.md)
- Development: [`MODEL-2026-11-03-01`](MODEL-2026-11-03-01-model-gate-dev.md)
- Validation: [`MODEL-2026-11-03-02`](MODEL-2026-11-03-02-model-gate-validation.md)
- Prior cycle: [`CYCLE-007`](../2026-11-01/CYCLE-007.md)
- Claim type: model evidence; no edge claim

## Question

Does any v3 model state or event — including the new MACD family — have a stable
forward edge on the development and validation partitions?

## Result

No. The development partition has 48 primary cells, all grade C. The validation
partition has one B cell, `divergence.regular_bullish` 5m, which was C on
development. The combined gate is **no edge**.

## Review

The model is correct and descriptive, not predictive. The level events, RSI
divergences, RSI thresholds, EMA crossovers, MACD line/signal cross, histogram
slope flip, zero cross, EMA stack, RSI zone, MACD state, and progression leg
direction all fail the same gate. The new MACD family does not rescue the model.

## Decision

No model baseline is accepted. No strategy v8 run is authorised from this model.
The loop stops at “no edge” on this data and thesis. A new thesis, dataset, or
regime is required; threshold tuning is not authorised.

## New Hypothesis

None. The result is a stop condition, not a prompt for another variant on the
same sample.
