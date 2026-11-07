# Portfolio and Execution Research Workflow

- Date: 2026-11-07 (UTC)
- Status: Current
- Scope: execution mechanics, portfolio accounting, costs, sizing, and the final holdout
- Owner: Quant Researcher (mechanics hypothesis), Quant Developer (implementation), Quant Validator (independent reproduction)
- Contract: [`improvement-workflow.md`](improvement-workflow.md), [`run-contract.md`](run-contract.md), [`docs/config/2026-10-30/policy-and-strategy.md`](../config/2026-10-30/policy-and-strategy.md)
- Qualification: [`docs/config/2026-11-06/qualification-criteria.md`](../config/2026-11-06/qualification-criteria.md)
- Claim type: execution and portfolio research process; no edge claim.

## 1. Purpose

The Portfolio and Execution Research Workflow answers one question: **do
execution and portfolio mechanics improve the account outcome of an accepted
strategy without changing the strategy's decisions?**

It is the portfolio tier of the Improvement Workflow. It changes execution and
portfolio together, freezes the model, strategy, data, policy, and analytics,
runs on `portfolio_dev`, freezes the candidate, and opens the one-shot
`portfolio_holdout` only after human approval.

Execution is not a separate tier. Execution and portfolio are one coupled
mechanism: execution fills the decision, portfolio accounts for the fill. A
separate execution workflow would fragment a single boundary.

## 2. When to use

Use it when:

- an accepted strategy baseline exists;
- the hypothesis is about fills, slippage, fees, funding, sizing, leverage,
  position limits, drawdown halts, exposure, or accounting;
- the policy envelope and prop-firm rules are declared;
- the candidate can be frozen before the holdout.

Do not use it when:

- there is no accepted strategy baseline; the portfolio tier is closed;
- the change is a strategy rule: use Strategy Research;
- the change is a model measurement: use Model Research or Model Change;
- the change is a policy envelope: use Policy Change;
- the change is a defect in the execution or portfolio engine: use the Audit
  Workflow.

## 3. Stages

```text
Hypothesis -> Design -> Implement -> Development -> Freeze -> Holdout -> Accept/Retire
```

| Stage | Question | Owner | Output | Gate |
|---|---|---|---|---|
| 1. Hypothesis | Which mechanic is claimed to improve the account outcome? | Quant Researcher | Hypothesis note | Human accepts the premise |
| 2. Design | What changes, what stays frozen, and what is the cost model? | Quant Researcher + Quant Developer | Mechanics design | Strategy baseline accepted; policy declared |
| 3. Implement | Do the execution and portfolio configs parse and run? | Quant Developer | Configs and tests | `execution run`, `portfolio replay`, `backtest run` pass |
| 4. Development | Does the mechanic improve the account outcome on `portfolio_dev`? | Quant Developer | Backtest, qualification | Green criteria on the development partition |
| 5. Freeze | Is the candidate frozen by config hash? | Quant Researcher | Frozen candidate | No post-hoc change |
| 6. Holdout | Does the frozen candidate survive the one-shot holdout? | Coordinator + Quant Validator | Holdout result | Human approves the access; Quant Validator record |
| 7. Accept/Retire | Is this a portfolio baseline or `no edge`? | Human + Triage Officer | Baseline or no-edge record | Registry updated; program stops or a new hypothesis starts |

## 4. Entry conditions

A portfolio run may start only when:

- a strategy baseline is accepted in
  [`docs/improvement/baselines.md`](../improvement/baselines.md);
- the strategy, model, data, policy, and analytics are frozen;
- the execution and portfolio configurations are declared;
- the policy envelope and prop-firm limits are declared;
- the cost model names commission, slippage, funding, and any other charge;
- the candidate can be frozen before the holdout;
- the human approves the one-shot holdout access.

## 5. What changes and what stays frozen

| Component | Portfolio run |
|---|---|
| Data | Frozen |
| Policy | Frozen |
| Model | Frozen at the accepted baseline |
| Strategy | Frozen at the accepted baseline |
| Execution | **Changed** |
| Portfolio | **Changed** |
| Analytics | Frozen |

A change that spans strategy and portfolio is two runs with two gates, not one
confounded run. A policy change is not a portfolio run; it is a Policy Change
and a re-baseline.

## 6. Mechanics and accounting

### Execution

- Fill timeframe, order type, tie-break, fee rate, slippage, zero-volume policy,
  and end-of-data policy are explicit.
- Execution receives a trade intent from the strategy; the stop and target
  specifications live in the intent, not in the execution config.
- Execution resolves the fill; it never chooses direction or sizing.
- A rejected entry does not re-run the strategy in a standalone run; the coupled
  `backtest run` supplies the actual position.

### Portfolio

- Sizing is `fixed_notional` or `equity_fraction`.
- Commission is `rate × max(equity_before_operation, 0)`.
- Funding is charged at the declared interval.
- The drawdown limit is checked after an entry, an exit, a funding charge, and a
  mark.
- A metric that is undefined is `null`, never zero.
- `final_equity = initial_cash + net_pnl` must hold.

The contracts are
[`docs/config/2026-10-30/policy-and-strategy.md`](../config/2026-10-30/policy-and-strategy.md)
and
[`docs/config/2026-11-06/engine-artifacts.md`](../config/2026-11-06/engine-artifacts.md).

## 7. Partitions and holdout

| Partition | Window | Use |
|---|---|---|
| `portfolio_dev` | 2024-01-01..2024-12-31 | Portfolio variants |
| `portfolio_holdout` | 2025-01-01..2025-12-31 | Final one-shot holdout |

- The holdout is opened once for the reset.
- Only a `PORTFOLIO` run with `partition: portfolio_holdout` may set
  `holdout_access: final`.
- The candidate is frozen before the access.
- A byte-identical rerun for determinism is allowed and recorded.
- A rerun with a changed config is a new candidate, not another holdout access.
- If the holdout fails, no portfolio baseline is accepted and the 2025 window is
  not reused to tune a later candidate.

## 8. Commands and artifacts

```bash
quantrule execution run --decisions <strategy-bundle> --config <run.json> --out <dir>
quantrule portfolio replay --fills <execution-bundle> --config <run.json> --out <dir>
quantrule backtest run --config <run.json> --strategy <strategy.json> --out <dir>
quantrule qualify --evidence <backtest-bundle> --criteria <qualification.json> --out <dir>
quantrule analytics evaluate --evidence <backtest-bundle> --out <dir>
```

| Artifact | Path |
|---|---|
| Fills bundle | `<out>/execution/` |
| Portfolio bundle | `<out>/portfolio/` |
| Backtest bundle | `<out>/backtest/` |
| Qualification bundle | `<out>/qualification/` |
| Analytics bundle | `<out>/analytics/` |
| Run record | `data/improvement/<date>/<run-id>/` |

## 9. Green criteria

A portfolio run is green when:

- the hypothesis, costs, partition, drawdown limit, trade count, and pass/fail
  rule were pre-registered;
- the strategy baseline is accepted and frozen;
- the execution and portfolio mechanics are the only changed components;
- the development result is cost-inclusive and stable across the declared cells;
- the candidate is frozen before the holdout;
- the one-shot holdout meets the pre-registered criteria;
- the qualification record has no mandatory `fail` or `insufficient_evidence`;
- the Quant Validator record is present;
- the run is reproducible byte for byte;
- no open Critical or Major audit finding is on the changed path;
- the human accepts the portfolio baseline.

## 10. Prop-firm limits

The policy envelope and the qualification criteria map the intended account
rules:

- daily loss;
- maximum loss;
- profit target;
- consistency rule;
- time limit;
- leverage and position limits;
- drawdown ceiling and halt behavior.

A prop-firm limit is a policy constraint and a qualification criterion. It is
not a strategy signal and it does not enter the model.

## 11. Roles and boundaries

| Role | Responsibility | Boundary |
|---|---|---|
| Quant Researcher | Mechanics hypothesis, costs, metrics, split | No production code; no post-hoc threshold |
| Quant Developer | Execution/portfolio config, engine change, tests | No strategy change; no validation claim |
| Quant Validator | Independent reproduction and verdict | Read-only; no repair |
| Human | Strategy and policy ownership, holdout access, baseline acceptance | Sole final authority |

## 12. Human gates

| Gate | Human action |
|---|---|
| Mechanics premise | Accept the thesis and cost model |
| Development result | Review the candidate; freeze or reject |
| Holdout access | Approve the one-shot access |
| Candidate | Require a Quant Validator record; accept or reject |
| Baseline | Accept the portfolio baseline |
| Closeout | Record `no_edge`, `development_only`, `candidate`, or `accepted` |

## 13. Artifacts and records

| Artifact | Path |
|---|---|
| Execution config | `config/execution/<name>.json` |
| Portfolio config | `config/portfolio/<name>.json` |
| Qualification criteria | `config/qualification/<name>.json` |
| Run record | `data/improvement/<date>/<run-id>/` |
| Baseline | `docs/improvement/baselines.md` |
| Invalidation | `docs/improvement/INVALIDATIONS.md` |
| Changelog | `CHANGELOG.md` |
| AI log | `docs/ai-transparency-log/<date>/<issue>-<slug>.md` |

## 14. Stop conditions

Stop and record the reason when:

- there is no accepted strategy baseline;
- the policy envelope or prop-firm rules are undeclared;
- the execution or portfolio engine is defective;
- the candidate is not green on `portfolio_dev`;
- the one-shot holdout fails;
- a threshold, cost, split, or criterion changed after a result was seen;
- the holdout was already opened for this hypothesis;
- the iteration cap is reached;
- the human stops the hypothesis.

A failed portfolio is retired. It is not re-registered as a variant with a
different cost, sizing, leverage, stop, or holdout window.

## 15. Handoff

| To | Packet |
|---|---|
| Improvement Workflow | The run record, review, decision, and baseline |
| Recordkeeping | The portfolio run ID, baseline ID, and invalidation if any |
| Release & Operations | The accepted baseline and the remaining operational risks |

## 16. Related documents

- [`improvement-workflow.md`](improvement-workflow.md)
- [`run-contract.md`](run-contract.md)
- [`strategy-research.md`](strategy-research.md)
- [`policy-change.md`](policy-change.md)
- [`../config/2026-10-30/policy-and-strategy.md`](../config/2026-10-30/policy-and-strategy.md)
- [`../config/2026-11-06/qualification-criteria.md`](../config/2026-11-06/qualification-criteria.md)
- [`../config/2026-11-06/engine-artifacts.md`](../config/2026-11-06/engine-artifacts.md)
- [`../../AGENTS.md`](../../AGENTS.md)
