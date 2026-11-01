# Improvement Workflow Roles

- Date: 2026-10-30 (UTC)
- Updated: 2026-11-01 for the workflow reset
- Work item: `audit-improvement-workflow`
- Workflow: [`docs/workflows/improvement-workflow.md`](../../workflows/improvement-workflow.md)
- Contract: [`docs/workflows/run-contract.md`](../../workflows/run-contract.md)
- Normative source: [`AGENTS.md` section 10](../../../AGENTS.md)

## Mission

Run the model, strategy, and portfolio tiers over a pre-registered universe, read
the results, decide whether the problem is the program, the trading
model/configuration, the data, or “no edge”, route the work, and repeat within
guardrails. The Improvement Workflow changes trading behavior one tier at a time;
it does not fix program defects.

## Roles

### Loop Governor / Experiment Orchestrator

Owns the cycle, the matrix, the baseline, the budget, the iteration cap, and the
stop decision. It runs the tier's protocol, writes the manifest, and keeps the
changed/frozen boundary. It does not implement or approve.

### Result Analyst

Reads the run artifacts, compares against the accepted baseline and the
validation/holdout result, checks costs, sample size, and stability, and reports
the observed result. It writes `review.md`. It does not decide the remediation.

### Triage Officer

Classifies the result and routes it to the Audit Workflow, a Quant hypothesis,
Data, or Stop. It writes `decision.md` with the next action. It does not
implement, approve, or validate.

## Inputs

- Policy and strategy blueprints.
- Data versions, tickets, timeframes, partitions, costs.
- The accepted model, strategy, and portfolio baselines.
- Prior runs, audits, plans, and reviews.
- Green criteria and stop conditions recorded before the run.

## Outputs

- `data/improvement/<date>/<run-id>/` with `manifest.json`, `config/`,
  `artifacts/`, `results.json`, `review.md`, and `decision.md`.
- A regenerated `data/improvement/index.json` and `index.md`.
- An updated baseline registry and, when needed, an invalidation tombstone.
- A route: program defect, model hypothesis, strategy hypothesis, portfolio
  hypothesis, data fix, or stop/no-edge.

## Tiers

| Tier | Changes | Frozen | Gate |
|---|---|---|---|
| Model | Model | Data, policy, strategy, execution, portfolio, analytics | Axis grades A/B on strategy-read axes; no D; C diagnostic only |
| Strategy | Strategy | Data, policy, accepted model, execution, portfolio, analytics | Green on the one-shot validation; Quant Validator record |
| Portfolio | Execution, portfolio | Data, policy, accepted model, accepted strategy, analytics | Green on the one-shot holdout; Quant Validator record |

A tier may not start without the previous tier's accepted baseline. The human
accepts the baseline; the workflow only recommends it.

## Parallel rules

- Independent market/timeframe/config cells may run in parallel under isolated
  run folders.
- One Loop Governor per objective.
- The Result Analyst may analyze cells in parallel.
- Triage is serial for one run.
- Program fixes and trading changes are never mixed in one run.
- A program fix blocks affected re-runs until it is merged, verified, and
  versioned.

## Boundaries

The Improvement Workflow cannot implement code, change model/strategy configs
directly, create a PR, merge, release, accept a baseline, or declare validation.
It may execute an approved protocol and report. The human owns the policy and
strategy, approves the protocol and the one-shot holdout, accepts baselines, and
accepts residual risk.
