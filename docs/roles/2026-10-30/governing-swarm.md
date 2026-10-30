# Governing / Experiment Control Swarm

- Date: 2026-10-30
- Work item: `audit-improvement-workflow`

## Mission

Run the full system over a pre-registered universe, read the results, decide
whether the problem is the program, the trading model/configuration, the data,
or “no edge”, route the work, and repeat within guardrails.

## Roles

### Loop Governor / Experiment Orchestrator

Owns the cycle, the matrix, the baseline, the budget, the iteration cap, and the
stop decision. It does not implement or approve.

### Result Analyst

Reads labs and experiment records, compares against baseline and out-of-sample,
checks costs and stability, and reports the observed result. It does not decide
the remediation.

### Triage Officer

Classifies the failure and routes it to Audit, Quant, Data, or Stop. It records
the decision and the next experiment. It does not implement, approve, or
validate.

## Inputs

- Policy and strategy blueprints.
- Data versions, tickets, timeframes, splits, costs.
- Prior experiments, audits, plans, and labs.
- Green criteria and stop conditions.

## Outputs

- `docs/experiments/<date>/<EXP-ID>-<slug>.md`.
- A run matrix with isolated output roots.
- A triage decision and the next action.
- A stop/no-edge report when the hypothesis fails.

## Parallel rules

- Independent market/timeframe/config cells may run in parallel.
- One Loop Governor per objective.
- The Result Analyst may analyze cells in parallel.
- Triage is serial for one experiment.
- Program fixes and config changes are never mixed in one experiment.
- A program fix blocks affected re-runs until merged and versioned.

## Boundaries

The Governing Swarm cannot implement code, change model/strategy configs
directly, create a PR, merge, release, or declare validation. Human approval is
required for material changes and residual risk.
