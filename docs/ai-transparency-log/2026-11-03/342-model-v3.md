# AI Transparency Log — Model v3, Validation Gate, and Benchmarks

- Date: 2026-11-03 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: epic [#342](https://github.com/quant-rule/quant-rule/issues/342)
- Plan:
  [`docs/execution-plans/2026-11-03/342-model-v3.md`](../../execution-plans/2026-11-03/342-model-v3.md)
- Claim type: model and strategy contracts, implementation, and evidence; no
  edge claim

## Intent and boundary

Implement the five approved decisions: remove axes from model decisions, keep
them only as a reporting taxonomy; gate strategy work on A/B signal-role events;
measure target and drawdown time on an annualised trajectory; add buy-and-hold
and matched random-trade benchmarks; and tombstone invalidated axis-based runs.

The model and strategy contracts change. The execution and portfolio accounting
rules do not, except for the benchmark evaluation path and the new time metrics.

## Work

- Read the current model contract, the four model runs, the strategy runs, the
  portfolio ledger, and the report bundle.
- Confirmed the historical evidence: Direction is a coin flip, Momentum has no
  directional edge, Structure and Progression were not tested against their
  actual claims, and no event family replicated.
- Created the epic and eight sub-issues.
- Wrote the approved execution plan; implementation evidence is added below as
  each phase completes.

## Verification and evidence

- Plan and issue links are recorded in the execution plan.
- Implementation evidence is recorded per phase; no test result is claimed
  before it is run.

## Learning

- The old harness tested Momentum as a direction predictor, but Momentum claims
  acceleration. The v2 validation design, not only the model, needs correction.
- The axis layer hides attribution. A flat event model makes the claim, the
  source, and the outcome directly testable.
- A hard gate is necessary: without it, a descriptive model is used as if it were
  predictive, which is what produced v3–v7.

## Rejected suggestions

- Keeping axes as hidden strategy inputs while reporting flat events: that would
  preserve the double-counting problem.
- Adding indicators beyond MACD in the first batch: each needs its own thesis and
  validation, and the program is already large.
- Tuning `direction_min` or any axis threshold on the 2024 sample: data snooping.
- Treating a single positive cell as a model baseline: it did not replicate.

## Limitations

- Sequential role passes in one session, not independent agents or independent
  model judgments.
- No profitability claim; the gate may return no edge.
- The benchmark replay is an evaluation tool, not a strategy.
