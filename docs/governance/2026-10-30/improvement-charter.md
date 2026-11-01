# Improvement Charter

- Date: 2026-10-30 (UTC)
- Updated: 2026-11-01 for the workflow reset
- Work item: `audit-improvement-workflow`
- Status: Approved; terminology and autonomy aligned with the 2026-11-01 reset
- Workflows: [Audit](../../workflows/audit-workflow.md) and
  [Improvement](../../workflows/improvement-workflow.md)

## Objective

Keep the program correct and look for a genuine edge without weakening the
deterministic, rule-based system. “Green” is an out-of-sample, cost-inclusive,
pre-registered result, not a single in-sample backtest.

## Ownership

| Level | Owner | What the owner decides |
|---|---|---|
| Policy | Human | Account, performance target, max drawdown, position size, leverage, tickets, timeframes |
| Strategy | Human | Thesis, entry, exit, risk/reward |
| Model/execution/portfolio/analytics | Engineering/Quant | Technical contracts inside the policy |
| Improvement run | Improvement Workflow | Tier, matrix, baseline, splits, costs, green criteria, review, triage |
| Program fix | Audit Workflow + Engineering | Audit, remediation plan, implementation, verification |
| Trading change | Quant Swarm | Hypothesis, implementation, independent validation |
| Baseline acceptance | Human | Material changes, `Won't fix`, release, residual risk |

## Three tiers

| Tier | Changed | Frozen | Development | Validation |
|---|---|---|---|---|
| Model | Model | Data, policy, strategy, execution, portfolio, analytics | `model_dev` | `model_validation` |
| Strategy | Strategy | Data, policy, accepted model, execution, portfolio, analytics | `strategy_dev` | `strategy_validation` |
| Portfolio | Execution, portfolio | Data, policy, accepted model, accepted strategy, analytics | `portfolio_dev` | `portfolio_holdout` |

The tiers run in order. A model axis graded D blocks the strategy run; no
strategy green means no portfolio optimization; a failed portfolio holdout means
no portfolio baseline.

## Data partitions

| Partition | Window | Use |
|---|---|---|
| `model_dev` | 2018-01-01..2020-12-31 | Model development |
| `model_validation` | 2021-01-01..2022-12-31 | Model confidence |
| `strategy_dev` | 2018-01-01..2022-12-31 | Strategy candidates |
| `strategy_validation` | 2024-01-01..2024-12-31 | One-shot strategy out-of-sample |
| `portfolio_dev` | 2024-01-01..2024-12-31 | Portfolio variants |
| `portfolio_holdout` | 2025-01-01..2025-12-31 | Final portfolio holdout |

2023 is excluded from every partition. Validation and holdout are one-shot per
hypothesis.

## Green criteria

A run must define before it executes:

- data versions and coverage;
- tickets and timeframes;
- in-sample and out-of-sample or validation split;
- cost assumptions (fees, slippage, funding, commission);
- minimum trades and exposure;
- return target and maximum drawdown;
- stability across the pre-registered cells;
- pass/fail rule and maximum number of variations.

No post-hoc threshold changes. If the criteria change, it is a new hypothesis.

## Triage

| Finding | Route |
|---|---|
| Wrong value, crash, silent corruption, contradiction | Audit Workflow → Remediation Plan → Engineering |
| Model implementation wrong | Quant investigation; audit if it is also a program defect |
| Model configuration | Quant Researcher → Developer → Validator |
| Strategy configuration | Quant Researcher → Developer → Validator |
| Execution configuration | Quant Researcher → Developer → Validator |
| Portfolio configuration | Quant Researcher → Validator |
| Analytics/measurement | Fix measurement; never tune it to green |
| Data/source | Data audit and fix |
| No out-of-sample edge | Stop, report, or new hypothesis |

## Autonomy

Level 2: the Improvement Workflow may run approved protocols, write manifests,
reviews, and decision proposals, and regenerate the index. It may recommend a
baseline, but the human accepts it. It may not merge, release, accept a trading
change, or declare validation. Human approval is required for policy, strategy,
engine-contract changes, release, baseline acceptance, and residual risk. A
second portfolio holdout access needs a new human decision.

## Stop conditions

- Iteration cap or compute budget reached.
- No out-of-sample improvement after the pre-registered number of attempts.
- A Critical or Major audit finding is open on the affected path.
- Data failure or unexplained degradation.
- The hypothesis is falsified.
- A threshold, cost, split, or criterion changed after the result was seen.
- Human stop.

## Release gate

No release with an open Critical finding, an unvalidated model/strategy change,
or a green claim that lacks out-of-sample evidence.
