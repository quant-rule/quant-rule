# ADR: Improvement Workflow and Audit Workflow (Superseded)

- Date: 2026-10-30 (UTC)
- Status: Superseded on 2026-11-01 by
  [`docs/adr/2026-11-01/workflow-reset.md`](../2026-11-01/workflow-reset.md)
- Work item: `audit-improvement-workflow`

> This record is retained for history. The workflow names below are the current
> names; the original decision was renamed by the 2026-11-01 reset. The current
> decision is the workflow reset ADR.

## Context

Engineering and Quant can fix or tune a component, but neither owns the question
“given these results, should we change the program, the trading, the data, or
stop?” A closed loop needs an explicit oversight layer.

## Decision

Add a thin **Improvement Workflow** above the delivery swarms, with a separate
**Audit Workflow** for program defects:

```text
Human
  ↓ policy, strategy thesis, green criteria, budget, approval
Improvement Workflow
  Loop Governor · Result Analyst · Triage Officer
  ↓
Audit Workflow                         Quant Swarm
  Audit · Planner · Verifier           Researcher · Developer · Validator
  ↓                                     ↓
             Engineering Swarm
             Orchestrator · Architect · Implementation · Test · Review · Release
```

The Improvement Workflow:

- defines the run matrix, baseline, partitions, costs, and green criteria;
- runs the matrix and analyzes results;
- classifies the failure as program, model, strategy, execution, portfolio,
  analytics, data, or no edge;
- routes program findings to the Audit Workflow: Audit → Planner → Engineering;
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
   deterministic and rule-based, and unlimited tuning is data snooping. The
   workflow runs and recommends; the human approves material changes.

## Consequences

- The workflow can conclude “no edge”; it is not required to turn red into green.
- One loop owner exists per objective; market/timeframe cells can run in
  parallel under isolated output roots.
- Runs are recorded under the run contract; program findings in
  `docs/audits/<date>/`; fixes in execution plans and issues.
- Engineering and Quant remain the two delivery sides; the Improvement Workflow
  and Audit Workflow are cross-cutting oversight.
