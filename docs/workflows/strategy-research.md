# Strategy Research Workflow

- Date: 2026-11-07 (UTC)
- Status: Current
- Updated: 2026-11-08 — feasibility preflight gate (human-approved)
- Scope: strategy hypotheses, strategy configuration, strategy development, and strategy validation
- Owner: Quant Researcher (hypothesis), Quant Developer (configuration and implementation), Quant Validator (independent reproduction)
- Contract: [`improvement-workflow.md`](improvement-workflow.md), [`run-contract.md`](run-contract.md), [`docs/config/2026-11-06/strategy-config.md`](../config/2026-11-06/strategy-config.md)
- Qualification: [`docs/config/2026-11-06/qualification-criteria.md`](../config/2026-11-06/qualification-criteria.md)
- Claim type: strategy research process; no edge claim.

## 1. Purpose

The Strategy Research Workflow answers one question: **does a deterministic rule
over accepted model measurements produce a cost-inclusive edge on data it was not
tuned on?**

It is the strategy tier of the Improvement Workflow. It consumes an accepted
model baseline, writes a human-owned strategy rule, runs it on `strategy_dev`,
freezes the candidate, runs `strategy_validation` once, and records the outcome
as a strategy baseline or `no edge`.

It does not compute an indicator, read a statistic, or touch cash or P&L. Those
belong to the model, evaluation, and portfolio stages.

## 2. When to use

Use it when:

- a model baseline has been accepted and publishes `signal` or `context`
  families that can support a rule;
- a strategy hypothesis is stated as a deterministic rule over published
  events, states, confirmed levels, progression, and bar identity;
- a strategy configuration changes a condition, threshold, exit, stop, target,
  or timeframe.

Do not use it when:

- there is no accepted model baseline; the strategy tier is closed;
- the change is an indicator or event: use Model Change;
- the change is execution or portfolio mechanics: use Portfolio Research;
- the change is a policy envelope: use Policy Change;
- the change is a defect in the strategy engine: use the Audit Workflow.

## 3. Stages

```text
Hypothesis -> Design -> Implement -> Feasibility -> Development -> Freeze -> Validation -> Accept/Retire
```

| Stage | Question | Owner | Output | Gate |
|---|---|---|---|---|
| 1. Hypothesis | What is the rule, the thesis, and the falsifier? | Quant Researcher | Hypothesis note | Human accepts the premise |
| 2. Design | Which published events/states does it read, and what is the rule budget? | Quant Researcher + Quant Developer | Strategy design | Model baseline and event roles are valid |
| 3. Implement | Does the strategy config parse and explain one decision? | Quant Developer | `strategy.json`, tests | `strategy explain` and `strategy run` pass |
| 3.5 Feasibility | Can the declared stop/target pay after costs without a win rate above 50%? | Quant Validator | `feasibility.json` | Net target > 0; breakeven win rate <= 50%; every timeframe <= 1h |
| 4. Development | Does the rule produce a cost-inclusive result on `strategy_dev`? | Quant Developer | Decisions, backtest, qualification | Green criteria on the development partition |
| 5. Freeze | Is the candidate frozen by config hash? | Quant Researcher | Frozen candidate | No post-hoc change |
| 6. Validation | Does the frozen candidate survive `strategy_validation` once? | Coordinator + Quant Validator | Validation result | Quant Validator record; human accepts or rejects |
| 7. Accept/Retire | Is this a strategy baseline or `no edge`? | Human + Triage Officer | Baseline or no-edge record | Registry updated; portfolio tier opened or kept closed |

## 4. Entry conditions

A strategy run may start only when:

- a model baseline is accepted in
  [`docs/improvement/baselines.md`](../improvement/baselines.md);
- the strategy reads only `signal` and `context` events and states;
- every event identity is registered and is not `diagnostic`;
- the model bundle publishes the timeframe and fields the strategy reads;
- the strategy rule is human-owned and explainable;
- the data version, costs, and partitions are declared;
- the rule complexity budget is declared.

A strategy may not rescue a failed model cell. A strategy may not read a
qualification record, an analytics metric, a fill, cash, equity, or P&L.

## 5. Rule design

- The rule is a deterministic conjunction of entry conditions and a disjunction
  of exit conditions, or the registered compression-bracket engine.
- The event-composite engine supports one to six entry conditions; at least one
  must be an event.
- Each event identity is named once. A correlated event is not a second vote.
- A context event may filter a decision; it cannot be the sole entry signal.
- A diagnostic never enters a decision.
- The rule names its execution timeframe and optional context timeframes.
- The rule declares its stop, target, and time stop.
- The rule complexity budget is part of the hypothesis; adding a condition after
  a result is a new hypothesis.

The strategy configuration contract is
[`docs/config/2026-11-06/strategy-config.md`](../config/2026-11-06/strategy-config.md).

## 6. Development and validation

### Feasibility preflight

Before the first backtest cell, the Quant Validator runs the deterministic
feasibility preflight on the declared rule and data:

```bash
python3 tools/improvement/feasibility.py \
  --strategy config/strategy/<name>.json \
  --data config/<run>/<cell>/data.json \
  --cost-bps 6 \
  --out <run>/artifacts/preflight/<cell>
```

The screen reads only the strategy risk block, the data config, and the raw
candles. It does not read a backtest result. It computes:

```text
cost_atr = round-trip cost bps / median ATR(14) as bps of price
net target = target ATR - cost_atr
net stop = stop ATR + cost_atr
breakeven win rate = net stop / (net stop + net target)
```

A hypothesis passes only when the net target is positive and the breakeven win
rate is at most 50% after the declared cost. Every execution and context
timeframe must be at or below 1 hour. A fail stops the hypothesis before the
first cell; the result is `infeasible`, not `no_edge`, and no backtest compute
is spent.

A human may approve an exception before the run, with the reason and the
expected win-rate evidence recorded in the charter. The exception is not a
post-hoc waiver.

| Partition | Window | Use |
|---|---|---|
| `strategy_dev` | 2018-01-01..2022-12-31 | Strategy candidates |
| `strategy_validation` | 2024-01-01..2024-12-31 | One-shot out-of-sample |

- Development runs may not read `strategy_validation` or the portfolio holdout.
- The candidate is frozen by configuration hash before validation.
- Validation is one-shot per hypothesis.
- A change after the freeze is a new hypothesis, not a retry.
- The validation run records the access in the run manifest and decision.
- A failed validation is `no edge`; it is not tuned into a pass.

## 7. Commands and artifacts

```bash
quantrule strategy explain --model <model-bundle> --strategy <strategy.json> --bar <index>
quantrule strategy run --model <model-bundle> --strategy <strategy.json> --out <dir>
quantrule backtest run --config <run.json> --strategy <strategy.json> --out <dir>
python3 tools/improvement/feasibility.py --strategy <strategy.json> --data <data.json> --cost-bps <bps> --out <dir>
quantrule qualify --evidence <backtest-bundle> --criteria <qualification.json> --out <dir>
quantrule analytics evaluate --evidence <backtest-bundle> --out <dir>
```

| Artifact | Path |
|---|---|
| Decisions bundle | `<out>/strategy/` |
| Backtest bundle | `<out>/backtest/` |
| Qualification bundle | `<out>/qualification/` |
| Analytics bundle | `<out>/analytics/` |
| Run record | `data/improvement/<date>/<run-id>/` |

## 8. Green criteria

A strategy run is green when:

- the hypothesis, costs, partition, trade count, drawdown, profit factor, and
  pass/fail rule were pre-registered;
- the model baseline is accepted and the rule reads only `signal` and `context`;
- the rule complexity budget is respected;
- the feasibility preflight passes before the first backtest cell;
- the development result is cost-inclusive and stable across the declared cells;
- the candidate is frozen before validation;
- the one-shot validation meets the pre-registered criteria;
- the qualification record has no mandatory `fail` or `insufficient_evidence`;
- the Quant Validator record is present;
- the run is reproducible byte for byte;
- no open Critical or Major audit finding is on the changed path;
- the human accepts the strategy baseline.

## 9. Roles and boundaries

| Role | Responsibility | Boundary |
|---|---|---|
| Quant Researcher | Hypothesis, rule, metrics, split, costs | No production code; no post-hoc threshold |
| Quant Developer | Strategy config, engine change, tests | No hypothesis change; no validation claim |
| Quant Validator | Independent reproduction and verdict | Read-only; no repair |
| Human | Strategy ownership, validation access, baseline acceptance | Sole final authority |

## 10. Human gates

| Gate | Human action |
|---|---|
| Strategy premise | Accept the thesis and rule budget |
| Development result | Review the candidate; freeze or reject |
| Validation access | Approve the one-shot access |
| Candidate | Require a Quant Validator record; accept or reject |
| Baseline | Accept the strategy baseline |
| Next tier | Open or keep closed the portfolio tier |

## 11. Artifacts and records

| Artifact | Path |
|---|---|
| Strategy config | `config/strategy/<name>.json` or the declared path |
| Qualification criteria | `config/qualification/<name>.json` |
| Run record | `data/improvement/<date>/<run-id>/` |
| Baseline | `docs/improvement/baselines.md` |
| Invalidation | `docs/improvement/INVALIDATIONS.md` |
| Changelog | `CHANGELOG.md` |
| AI log | `docs/ai-transparency-log/<date>/<issue>-<slug>.md` |

## 12. Stop conditions

Stop and record the reason when:

- there is no accepted model baseline;
- the rule reads a diagnostic or an unregistered event;
- the rule needs a new engine or condition language;
- the feasibility preflight fails or the hypothesis needs a win rate above 50%;
- the candidate is not green on `strategy_dev`;
- the one-shot validation fails;
- a threshold, cost, split, or criterion changed after a result was seen;
- the iteration cap is reached;
- the human stops the hypothesis.

A failed strategy is retired. It is not re-registered as a variant with a
different threshold, stop, target, timeframe, or filter.

## 13. Handoff

| To | Packet |
|---|---|
| Portfolio Research | The accepted strategy baseline, its costs, and its limitations |
| Improvement Workflow | The run record, review, decision, and baseline |
| Recordkeeping | The strategy run ID, baseline ID, and invalidation if any |
| Strategy baseline | The frozen config hash, data version, and qualification record |

## 14. Related documents

- [`improvement-workflow.md`](improvement-workflow.md)
- [`run-contract.md`](run-contract.md)
- [`model-research.md`](model-research.md)
- [`portfolio-research.md`](portfolio-research.md)
- [`../config/2026-11-06/strategy-config.md`](../config/2026-11-06/strategy-config.md)
- [`../config/2026-11-06/qualification-criteria.md`](../config/2026-11-06/qualification-criteria.md)
- [`../config/2026-11-06/engine-artifacts.md`](../config/2026-11-06/engine-artifacts.md)
- [`../../AGENTS.md`](../../AGENTS.md)
