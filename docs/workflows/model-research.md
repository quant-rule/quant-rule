# Model Research Workflow

- Date: 2026-11-07 (UTC)
- Status: Current
- Scope: model hypotheses, exploratory cells, confirmatory cells, model trust grades, and model baselines
- Owner: Quant Researcher (hypothesis), Quant Developer (implementation), Quant Validator (independent reproduction)
- Contract: [`improvement-workflow.md`](improvement-workflow.md), [`run-contract.md`](run-contract.md), [`research-swarm.md`](research-swarm.md)
- Mandate: [`docs/governance/2026-11-04/research-mandate.md`](../governance/2026-11-04/research-mandate.md)
- Charter template: [`docs/governance/2026-11-04/program-charter-template.md`](../governance/2026-11-04/program-charter-template.md)
- Claim type: model research process; no strategy or edge claim.

## 1. Purpose

The Model Research Workflow answers one question: **which model measurement,
event, or event family has enough forward evidence to become a model baseline?**

It is the front half of the model tier. It turns an observation into a
falsifiable premise, screens the premise before spending a charter, pre-registers
a finite cell set, explores on `model_dev`, promotes at most the declared primary
cells, confirms on `model_validation` once, and records the outcome as a
baseline or `no edge`.

It does not implement a new indicator or event. That is
[`model-change.md`](model-change.md). It does not decide a strategy. That is
[`strategy-research.md`](strategy-research.md).

## 2. When to use

Use it when:

- a registry gap, audit finding, model-confidence report, market observation, or
  data finding produces a candidate premise;
- a model family needs a new event or state hypothesis;
- a failed family is being re-examined under a genuinely new premise;
- a model baseline must be established before a strategy run.

Do not use it when:

- the change is an implementation defect: use the Audit Workflow;
- the change is an indicator or event implementation: use Model Change;
- the change is a strategy rule: use Strategy Research;
- the change is a policy envelope: use Policy Change;
- the data is invalid: use Data Lifecycle before any model run.

## 3. Stages

```text
Intake -> Screen -> Pre-register -> Explore -> Promote -> Confirm -> Accept/Retire
```

| Stage | Question | Owner | Output | Gate |
|---|---|---|---|---|
| 1. Intake | What observation, gap, or finding produced this premise? | Quant Researcher | Hypothesis note | Deduplicated against baselines and invalidations |
| 2. Screen | Is this a new premise or a variant of a failed one? Is the sample plausible? Is the data admissible? | Quant Researcher + Quant Developer | Screening record | Human accepts the premise for a charter |
| 3. Pre-register | What is the finite cell set, correction, budget, and stop rule? | Quant Researcher + Coordinator | Program charter | Human approves the charter |
| 4. Explore | What does `model_dev` say about coverage and the premise? | Cell agents | Exploratory cell results | Coverage and power preflight pass |
| 5. Promote | Which cells, if any, become primary? | Human + Quant Researcher | Promoted cell list | Human promotes at most the declared cells |
| 6. Confirm | Does the frozen candidate survive `model_validation` once? | Coordinator + Quant Validator | Validation result and grade | Quant Validator record; human accepts or rejects |
| 7. Accept/Retire | Is this a baseline or `no edge`? | Human + Triage Officer | Baseline or no-edge record | Registry updated; downstream tier opened or kept closed |

## 4. Intake and screening

### 4.1 Intake sources

- A registry gap: an indicator or event with no event, or a claim with no test.
- A model-confidence report: an event family that looks promising but has no
  pre-registered protocol.
- An audit finding: a construct or measurement defect that blocks a family.
- A data finding: a new source or a repaired source that makes a previously
  untestable family testable.
- A market observation or economic premise: a mechanism, counterparty, risk
  premium, structural flow, or behavioral effect.
- A literature or cross-asset idea, stated as a deterministic rule over OHLCV.

### 4.2 Screening record

Before a charter, record:

- the premise in one falsifiable sentence;
- the economic rationale, including the counterparty or mechanism;
- the event/state family and its registry role (`signal`, `context`,
  `diagnostic`);
- the instrument set, timeframes, and data versions;
- the expected sample size and cluster count;
- the data-admissibility result;
- whether the premise is new or a variant of a failed hypothesis;
- the prior records reviewed (baselines, invalidations, programmes).

A premise that is only "the backtest shows it" is not ready. A variant of a
failed hypothesis is retired, not re-registered.

### 4.3 Power preflight

A count-only preflight runs before a development run:

- run the raw model for each primary cell, instrument, year, and timeframe;
- count `prediction != 0` rows that match the cell's state gate;
- count clusters;
- select a timeframe only if **every** primary cell reaches the declared
  minimum sample and cluster count;
- otherwise stop as `underpowered`.

The preflight reads counts only. It does not compute a hit rate, base rate, lift,
or control. The selected timeframe is frozen in the manifest.

### 4.4 Data admissibility

Before a volume, participation, or flow family:

- verify the volume unit stability across the development partition;
- record the ratio of the largest to the smallest monthly median and the largest
  month-to-month step;
- exclude an instrument-year that fails the declared rule;
- do not re-admit an instrument by changing the threshold after a result.

The rule is a data-quality gate, not a signal. The current volume admissibility
example is in the M3 charter.

## 5. Pre-registration

The charter uses
[`docs/governance/2026-11-04/program-charter-template.md`](../governance/2026-11-04/program-charter-template.md)
and records:

- the premise and economic rationale;
- the tier and frozen inputs;
- the finite cell list and exact configs;
- the development and validation partitions;
- costs and green criteria;
- the multiple-testing correction and primary-cell count;
- the iteration cap and stop rule;
- the validation access;
- the power preflight result and the data-admissibility result.

A cell is one hypothesis, one config, one worktree or output directory, and one
result. A failed cell is retired; it is not retried with a variant.

## 6. Exploration and confirmation

| Mode | Cells | Partition | Evidence | Ranked | Validation |
|---|---|---|---|---|---|
| Exploratory | One cell per information area, or the declared exploration set | `model_dev` only | Coverage and hypothesis generation | No | Never |
| Confirmatory | The declared primary cells | `model_dev`, then one-shot `model_validation` | Corrected primary evidence | Yes | Only after human promotion |

- Exploratory cells are labelled `exploratory` and receive no multiple-testing
  credit.
- A positive exploratory cell is a hypothesis for a primary cell, not a
  candidate.
- The human freezes the exploratory and primary specs before the first run.
- A cell agent may not add, remove, or retune an event after seeing a result.
- A failed exploratory cell is retired; it is not retried with a variant.

## 7. Model trust grades

| Grade | Meaning | Allowed use |
|---|---|---|
| A | Passes the model checks and the pre-registered validation criteria | Usable in a strategy |
| B | Passes with a documented limitation | Usable in a strategy; the limitation is recorded in the baseline |
| C | Diagnostic only; not trusted enough to gate a trade | Research and diagnosis only; cannot feed a strategy decision |
| D | Fails a model check or the validation criteria | Blocks promotion; no strategy run until a new model hypothesis is accepted |

A model run with any D family stops the sequence. The next action is a model
hypothesis, not a strategy run.

## 8. Roles and boundaries

| Role | Responsibility | Boundary |
|---|---|---|
| Quant Researcher | Premise, protocol, metrics, interpretation | No production code; no post-hoc threshold; no self-approval |
| Quant Developer | Implementation, tests, reproducibility | No hypothesis change; no validation claim |
| Quant Validator | Independent reproduction and verdict | Read-only; no repair; no criteria change |
| Coordinator | Program, worktrees, cell briefs, aggregation | No tuning, voting, or promotion |
| Cell agent | One cell in one worktree/output | No reading another cell's result; no config change after a result |
| Human | Charter, validation access, baseline acceptance | Sole final authority |

## 9. Human gates

| Gate | Human action |
|---|---|
| Premise | Accept the premise for a charter |
| Charter | Approve the cells, correction, budget, and stop rule |
| Development results | Review every cell; promote at most the declared primary cells |
| Validation access | Approve the one-shot access and the promoted cells |
| Candidate | Require a Quant Validator record; accept or reject |
| Closeout | Record `no_edge`, `development_only`, `candidate`, or `accepted` |

## 10. Green criteria

A model run is green when:

- the hypothesis, tier, partition, costs, and pass/fail rule were pre-registered;
- the power preflight and data-admissibility gate passed;
- the model checks pass: contract, causality, determinism, availability,
  coverage, and descriptive review;
- the corrected primary evidence meets the declared lift, lower confidence
  bound, control, back-lag, and replication criteria;
- no family grades D;
- the Quant Validator record is present;
- the run is reproducible byte for byte;
- no open Critical or Major audit finding is on the changed path;
- the human accepts the model baseline.

## 11. Artifacts and records

| Artifact | Path |
|---|---|
| Program charter | `docs/governance/<date>/<PROGRAM-ID>.md` |
| Program manifest | `config/research/<PROGRAM-ID>/program.json` |
| Cell result | `data/research/<PROGRAM-ID>/<phase>/<CELL-ID>/cell-result.json` |
| Program report | `data/research/<PROGRAM-ID>/program-results/` |
| Run record | `data/improvement/<date>/<run-id>/` |
| Baseline | `docs/improvement/baselines.md` |
| Invalidation | `docs/improvement/INVALIDATIONS.md` |
| Model family registry | `docs/model/2026-11-05/model-families.md` |

## 12. Stop conditions

Stop and record the reason when:

- no primary cell passes the development gate;
- a validation cell fails;
- the power preflight is underpowered;
- a data-admissibility gate fails and cannot be repaired within the charter;
- a shared input, coverage check, or evaluator prerequisite is missing;
- the iteration cap or compute budget is reached;
- a threshold, cost, split, or criterion changed after a result was seen;
- the human stops the program.

A stopped program retires the premise. It is not re-registered with a different
event, timeframe, horizon, target, stop, filter, cost, instrument set, or
threshold.

## 13. Handoff

| To | Packet |
|---|---|
| Model Change | The measurement gap, the contract change, and the evidence the change must enable |
| Strategy Research | The accepted model baseline, the signal/context families, the grades, and the limitations |
| Improvement Workflow | The run record, baseline, and closeout verdict |
| Recordkeeping | The programme, run, baseline, and invalidation IDs |

## 14. Related documents

- [`improvement-workflow.md`](improvement-workflow.md)
- [`run-contract.md`](run-contract.md)
- [`research-swarm.md`](research-swarm.md)
- [`model-change.md`](model-change.md)
- [`strategy-research.md`](strategy-research.md)
- [`../governance/2026-11-04/research-mandate.md`](../governance/2026-11-04/research-mandate.md)
- [`../governance/2026-11-04/program-charter-template.md`](../governance/2026-11-04/program-charter-template.md)
- [`../model/2026-11-05/model-families.md`](../model/2026-11-05/model-families.md)
- [`../registry/README.md`](../registry/README.md)
- [`../../AGENTS.md`](../../AGENTS.md)
