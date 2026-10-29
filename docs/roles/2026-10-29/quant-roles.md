# Quant Roles

- Date: 2026-10-29 (UTC)
- Status: Current
- Normative source: [`AGENTS.md` section 10](../../../AGENTS.md#10-agentic-engineering-and-role-definitions)
- Scope: candles-only, rule-based signals

This page explains the three quant roles in plain language. `AGENTS.md` is the
normative source; if the two disagree, `AGENTS.md` wins and this page is
corrected.

## Why three roles

A strategy can be built correctly and still be wrong. The code may pass every
test while the research behind it selected the best-looking result from many
attempts, ignored costs, or evaluated on the data used to tune it. Separating
research, implementation, and validation makes that failure visible.

The roles are procedural hats for agents, not separate people or independent
model judgments. One agent may perform more than one role, but the orchestrator
must disclose that the passes were sequential.

## The roles

| Role | Owns | Inputs | Outputs | Must not |
|---|---|---|---|---|
| **Quant Researcher** | The hypothesis, experiment protocol, evaluation metrics, and interpretation | A market question, data coverage, the model contract | A rule-based hypothesis, a pre-registered protocol, a validation request | Write production code, change thresholds after seeing results, or approve its own conclusions |
| **Quant Developer** | The implementation, tests, determinism, and performance | The approved protocol, model contract, and configuration schemas | Code, tests, a reproducible run configuration, reported deviations | Change the hypothesis, thresholds, or evaluation criteria to improve results, or call its own work validated |
| **Quant Validator** | Independent reproduction and the evidence verdict | The recorded data version, configuration, and candidate result | A validation record: commands, observed results, limitations, and verdict | Modify the work it reviews, waive a control, or change the criteria to turn a failure into a pass |

## Handoff

```
market question
    -> Quant Researcher: hypothesis + protocol + metrics
    -> Quant Developer: implementation + tests + reproducible run
    -> Quant Validator: independent reproduction + bias and cost audit
    -> Human: accept, revise, or reject
```

The handoff is one way. The researcher does not implement the strategy, and the
developer does not certify the result. The validator can send the result back
with findings; it does not repair it.

## When each role is required

| Change | Researcher | Developer | Validator |
|---|---|---|---|
| New strategy hypothesis | Required | Required | Required |
| Strategy threshold or configuration change | Required | Required | Required |
| Model behavior that affects signals | Required when a behavior claim is made | Required | Required |
| Bug fix that does not change model or strategy output | Not required | Required | Not required |
| Documentation or tooling only | Not required | Not required | Not required |
| Release that changes signal generation | Required when a claim is made | Required | Required |

The Quant Validator is required before a strategy result is called promising,
before a model or strategy behavior change is merged, and before a release that
changes signal generation. A docs-only change does not need a validation pass.

## Relationship to the engineering roles

| Existing role | Difference |
|---|---|
| Domain / Requirements Analyst | Defines the expected behavior and scope. The Quant Researcher forms and tests a market hypothesis within that scope. |
| Implementation Agent | Implements any approved change. The Quant Developer specializes in model and strategy code and its numerical correctness. |
| Test Engineer | Proves that the code does what the tests say. The Quant Validator asks whether the tests and the evidence justify the conclusion. |
| Code Reviewer | Reviews correctness and maintainability. The Quant Validator audits research bias, costs, and risk. |
| Software Architect, Security Reviewer, Release Verifier | Still apply. The quant roles do not replace them. |

## Scope rules

- Signals are deterministic rules over OHLCV candles.
- Statistical, machine-learning, and neural-network signal generation are out
  of scope.
- Statistics is used to evaluate and audit rule-based signals, not to generate
  them.
- A profitable claim reports in-sample and out-of-sample results, cost
  assumptions, and every variation tried.
- A backtest on the tuning sample is not validation.
- The human remains the final authority; a validator verdict is evidence, not
  approval.

## Worked example: changing a strategy threshold

1. **Researcher:** states the hypothesis ("a higher long threshold reduces
   false entries"), fixes the metric, the split, the costs, and the failure
   condition before running anything. Records every threshold tried.
2. **Developer:** implements the configuration change, adds or updates the
   tests, and produces a run with a recorded data version and configuration.
3. **Validator:** reproduces the run from those inputs, checks the split and
   the costs, compares in-sample and out-of-sample behavior, and records the
   verdict and residual risk.
4. **Human:** accepts, revises, or rejects the result.

## What these roles do not do

- They do not guarantee a profit or a working strategy.
- They do not create independent agents or independent model judgments.
- They do not replace tests, code review, security review, or human acceptance.
- They do not authorize commits, pushes, merges, or releases.

See the [container plan](../../execution-plans/2026-10-29/201-quant-roles.md)
for the adoption work and the [docs index](../../README.md) for the rest of the
documentation.
