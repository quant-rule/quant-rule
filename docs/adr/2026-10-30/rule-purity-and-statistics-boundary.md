# ADR: Rule Purity and the Statistics Boundary

- Date: 2026-10-30
- Status: Accepted
- Work item: `audit-improvement-workflow`

## Context

The project is a deterministic, rule-based system over OHLCV candles. Statistics
is allowed to evaluate and validate rules; it must not generate signals. The
model also publishes diagnostic percentile/readiness fields. The boundary needs
to be explicit and testable.

## Decision

- The **signal path** is the model’s axis primary values plus the strategy’s
  deterministic comparisons. It uses addition, subtraction, multiplication,
  division, comparison, clamp, sign, and rounding only.
- No regression, correlation, p-value, optimization, fitted coefficient,
  percentile-driven entry/exit, or machine-learning model is part of the signal
  path.
- The strategy receives only `timeframe_secs`, `bar_index`, `open_time`,
  `direction`, `momentum_force`, and `structure_score`. It cannot read
  percentiles, readiness, events, context, execution, portfolio, or policy.
- Model percentile/readiness diagnostics are output diagnostics. They must not
  affect an axis value or a strategy decision. A boundary test proves the
  strategy cannot access them.
- Statistics is confined to `quantrule-analytics`, validation records, and
  experiment evaluation. It never automatically writes a config or threshold.
- Configs are immutable during a run and recorded in provenance.

## Alternatives considered

1. **Remove every percentile/readiness field now.** Rejected for this work item:
   it would change the model output contract. The fields are isolated first; if
   they cannot be proven outside the signal path, a separate approved model
   change removes them.
2. **Allow statistical filters as long as they are documented.** Rejected:
   statistical, ML, and neural-network signal generation are out of scope.
3. **Let the Improvement Workflow optimize thresholds automatically.** Rejected:
   post-hoc tuning is data snooping. The swarm recommends; the human approves a
   pre-registered hypothesis.

## Consequences

- A strategy that needs a statistical field is outside the product.
- Analytics may compute correlations and distributions for evaluation only.
- Every model/strategy change requires causality, determinism, golden, and
  boundary tests; a model/strategy behavior change requires Quant Validator
  evidence.
