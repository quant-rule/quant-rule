# Policy Change Workflow (Historical)

> Superseded 2026-11-11 by run v3 and `run check`. The policy layer is retired;
> portfolio constraints remain in `portfolio.json`.

- Date: 2026-11-07 (UTC)
- Status: Historical
- Scope: the human-owned account, objective, risk, universe, and prop-firm envelope
- Owner: Human (decision), Planner (impact analysis), Quant Validator (evidence review)
- Contract: [`docs/config/2026-10-30/policy-and-strategy.md`](../config/2026-10-30/policy-and-strategy.md)
- Qualification: [`docs/config/2026-11-06/qualification-criteria.md`](../config/2026-11-06/qualification-criteria.md)
- Claim type: policy governance; no model, strategy, or edge claim.

## 1. Purpose

The Policy Change Workflow answers one question: **does a change to the
human-owned envelope invalidate an existing baseline, and what must be
re-established before a new claim is made?**

The policy is the account, objective, risk, and universe envelope. It never
emits a signal. A policy change is not an improvement run; it changes the
conditions under which every tier is evaluated. The result is a policy change
record, an impact analysis, and a re-baseline sequence.

## 2. When to use

Use it when:

- the account currency, initial cash, performance target, evaluation period, or
  drawdown ceiling changes;
- the risk limits, leverage, position limit, or maximum open positions change;
- the universe (tickets or timeframes) changes;
- the prop-firm rules change (daily loss, maximum loss, profit target,
  consistency rule, time limit);
- a policy field is added, removed, or reinterpreted.

Do not use it when:

- the change is a strategy rule: use Strategy Research;
- the change is a model measurement: use Model Research or Model Change;
- the change is an execution or portfolio mechanic: use Portfolio Research;
- the change is a data source or partition: use Data Lifecycle.

## 3. Stages

```text
Proposal -> Impact Analysis -> Human Decision -> Apply -> Re-baseline -> Record
```

| Stage | Question | Owner | Output | Gate |
|---|---|---|---|---|
| 1. Proposal | What changes, and why? | Human + Planner | Policy change proposal | The change is within the human-owned envelope |
| 2. Impact analysis | Which baselines, runs, partitions, costs, and criteria are affected? | Planner + Quant Validator | Impact record | Every affected baseline is named |
| 3. Human decision | Is the change approved? | Human | Approved or rejected change | Human decision recorded |
| 4. Apply | Is the policy file updated and validated? | Implementation + Test | Updated policy | `policy check` passes |
| 5. Re-baseline | Which tier must be re-run? | Planner + Quant roles | Re-baseline plan | The affected tier is re-run in order |
| 6. Record | Is the change traceable? | Orchestrator | Policy change record, changelog, AI log | Links resolve; status is `Applied` |

## 4. Impact analysis

The impact record names:

- the changed fields and their old and new values;
- every accepted baseline that used the old envelope;
- every run that used the old envelope;
- every partition, cost assumption, qualification criterion, and prop-firm limit
  that is affected;
- the tier that must be re-run first;
- whether the model, strategy, or portfolio baseline is retired, superseded, or
  re-established;
- the residual risk the human accepts.

A policy change that lowers the drawdown ceiling can retire a portfolio baseline
without changing the strategy. A policy change that changes the universe can
invalidate the model and strategy baselines. The impact analysis states which.

## 5. Human gates

| Gate | Human action |
|---|---|
| Proposal | Decide whether the change is in scope |
| Impact | Accept or reject the affected-baseline list |
| Approval | Approve the policy change |
| Re-baseline | Approve the sequence and the validation access |
| Residual risk | Accept or reject the remaining risk |
| Closeout | Mark the change `Applied` or `Rejected` |

## 6. Green criteria

A policy change is green when:

- the proposal states the old and new values and the reason;
- the impact analysis names every affected baseline, run, partition, cost, and
  criterion;
- the human approves the change and the residual risk;
- `policy check` passes on the updated configuration;
- the affected tier is re-run in order, or the human records why it is not;
- the affected baselines are retired or superseded in
  [`docs/improvement/baselines.md`](../improvement/baselines.md);
- the change is recorded in the changelog and the AI log.

## 7. Artifacts and records

| Artifact | Path |
|---|---|
| Policy change record | `docs/config/<date>/policy-change-<id>.md` |
| Policy file | `config/policy/<name>.json` |
| Impact record | The policy change record |
| Baseline registry | `docs/improvement/baselines.md` |
| Invalidation | `docs/improvement/INVALIDATIONS.md` |
| Changelog | `CHANGELOG.md` |
| AI log | `docs/ai-transparency-log/<date>/<issue>-<slug>.md` |

## 8. Stop conditions

Stop and request human direction when:

- the change is actually a strategy rule or a model measurement;
- the change requires a new field that the policy schema does not support;
- an affected baseline cannot be identified;
- the re-baseline sequence would open a validation partition twice;
- the human has not accepted the residual risk;
- the change would hide a failed hypothesis.

## 9. Handoff

| To | Packet |
|---|---|
| Improvement Workflow | The re-baseline sequence and the affected tiers |
| Model Research | The new envelope and the partitions that must be re-run |
| Strategy Research | The new envelope and the retired strategy baseline |
| Portfolio Research | The new envelope, drawdown limit, and prop-firm limits |
| Recordkeeping | The policy change ID and the affected baseline IDs |

## 10. Related documents

- [`improvement-workflow.md`](improvement-workflow.md)
- [`run-contract.md`](run-contract.md)
- [`model-research.md`](model-research.md)
- [`strategy-research.md`](strategy-research.md)
- [`portfolio-research.md`](portfolio-research.md)
- [`../config/2026-10-30/policy-and-strategy.md`](../config/2026-10-30/policy-and-strategy.md)
- [`../config/2026-11-06/qualification-criteria.md`](../config/2026-11-06/qualification-criteria.md)
- [`../../AGENTS.md`](../../AGENTS.md)
