# ADR: Governing Swarm and Continuous Improvement Loop

- Date: 2026-10-30
- Status: Accepted
- Work item: `audit-improvement-workflow`

## Context

Engineering and Quant can fix or tune a component, but neither owns the question
“given these results, should we change the program, the trading, the data, or
stop?” A closed loop needs an explicit governing layer.

## Decision

Add a thin **Governing / Experiment Control Swarm** above the delivery swarms:

```text
Human
  ↓ policy, strategy thesis, green criteria, budget, approval
Governing Swarm
  Loop Governor · Result Analyst · Triage Officer
  ↓
Continuous Improvement Swarm        Quant Swarm
  Audit · Planner · Verifier         Researcher · Developer · Validator
  ↓                                  ↓
             Engineering Swarm
             Orchestrator · Architect · Implementation · Test · Review · Release
```

The Governing Swarm:

- defines the experiment matrix, baseline, splits, costs, and green criteria;
- runs the matrix and analyzes results;
- classifies the failure as program, model, strategy, execution, portfolio,
  analytics, data, or no edge;
- routes program findings to Audit → Planner → Engineering;
- routes trading changes to Quant Researcher → Developer → Validator;
- stops when the hypothesis fails, the budget is exhausted, or the iteration
  cap is reached.

It may not implement, merge, accept a trading change, release, or declare
validation. The human remains the decision owner.

## Alternatives considered

1. **Let the Planner own the loop.** Rejected: the Planner turns known findings
   into plans; it does not decide whether the failure is program or trading.
2. **Let the Quant Researcher own the loop.** Rejected: it would own engineering
   audits and program fixes, blurring the engineering/quant separation.
3. **Fully autonomous optimization.** Rejected for now: the system is
   deterministic and rule-based, and unlimited tuning is data snooping. The loop
   runs and recommends; the human approves material changes.

## Consequences

- The loop can conclude “no edge”; it is not required to turn red into green.
- One loop owner exists per objective; market/timeframe cells can run in
  parallel under isolated output roots.
- Experiments are recorded in `docs/experiments/<date>/`; program findings in
  `docs/audits/<date>/`; fixes in execution plans and issues.
- Engineering and Quant remain the two delivery sides; Governing and Continuous
  Improvement are cross-cutting oversight.
