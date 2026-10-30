# Improvement Charter

- Date: 2026-10-30
- Work item: `audit-improvement-workflow`
- Status: Approved

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
| Experiment | Governing Swarm | Matrix, baseline, splits, costs, green criteria, triage |
| Program fix | Continuous Improvement + Engineering | Audit, plan, implementation, verification |
| Trading change | Quant Swarm | Hypothesis, implementation, independent validation |
| Acceptance | Human | Material changes, `Won't fix`, release, residual risk |

## Green criteria

An experiment must define before it runs:

- data versions and coverage;
- tickets and timeframes;
- in-sample and out-of-sample or walk-forward split;
- cost assumptions (fees, slippage, funding, commission);
- minimum trades and exposure;
- return target and maximum drawdown;
- stability across the pre-registered cells;
- pass/fail rule and maximum number of variations.

No post-hoc threshold changes. If the criteria change, it is a new experiment.

## Triage

| Finding | Route |
|---|---|
| Wrong value, crash, silent corruption, contradiction | Audit → Plan → Engineering |
| Model implementation wrong | Quant investigation; audit if it is also a program defect |
| Model configuration | Quant Researcher → Developer → Validator |
| Strategy configuration | Quant Researcher → Developer → Validator |
| Execution configuration | Quant Researcher → Developer → Validator |
| Portfolio configuration | Quant Researcher → Validator |
| Analytics/measurement | Fix measurement; never tune it to green |
| Data/source | Data audit and fix |
| No out-of-sample edge | Stop, report, or new hypothesis |

## Autonomy

Level 2: the Governing Swarm may run approved matrices and recommend the next
action. It may not merge, accept a trading change, release, or declare
validation. Human approval is required for policy, strategy, engine-contract
changes, release, and residual risk.

## Stop conditions

- Iteration cap or compute budget reached.
- No out-of-sample improvement after the pre-registered number of attempts.
- A critical audit finding is open.
- Data failure or unexplained degradation.
- The hypothesis is falsified.
- Human stop.

## Release gate

No release with an open Critical finding, an unvalidated model/strategy change,
or a green claim that lacks out-of-sample evidence.
