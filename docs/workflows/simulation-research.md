# Simulation Research Workflow

- Date: 2026-11-10 (UTC)
- Status: Current; the harness is ported to the `simulation` service in v1.0.0
- Scope: chart-first, hindsight-labeled hypothesis generation on the development partition
- Owner: Quant Researcher (premise), AI labeler (marks), Quant Validator (check), Human (acceptance)
- Contract: [`improvement-workflow.md`](improvement-workflow.md), [`run-contract.md`](run-contract.md), [`strategy-research.md`](strategy-research.md)
- Claim type: research process; no edge claim

## 1. Purpose

Simulation Research turns a visual market question into one simple, falsifiable
rule. The AI marks the entries and exits it would have taken on a historical
chart with the future visible, using one or two model events or indicators. The
marks are saved in the same schema as the human annotator, rendered as PNGs for
human review, and compared against a check window that was not marked.

It is an upstream hypothesis generator. It is not a backtest, not a fill
simulation, and not evidence of an edge. A pattern that survives the check is
handed to Strategy Research with a new pre-registration.

## 2. When to use

Use it when:

- a rule-first search has stalled and the next question is what the chart
  implies, not which threshold to tune;
- the human marks or the model events suggest a visual pattern that is not yet
  stated as a rule;
- the hypothesis can be stated with one or two registered events or readings and
  one exit;
- the work stays on `strategy_dev` 2018–2022 and the 2024 validation and 2025
  holdout remain closed.

Do not use it when:

- the change is a model measurement, an indicator, or an event: use Model Change;
- the change is a strategy rule or a backtest: use Strategy Research;
- the change is execution or portfolio mechanics: use Portfolio Research;
- the question needs validation or holdout data: it is a confirmation run, not
  exploration;
- the result would be used to accept a baseline or claim an edge.

## 3. Stages

```text
Premise -> Select -> Mark -> Review -> Check -> Decide
```

| Stage | Question | Owner | Output | Gate |
|---|---|---|---|---|
| 1. Premise | What pattern are we looking for, and with which one or two inputs? | Quant Researcher | Run premise | Human accepts the run list |
| 2. Select | Which windows show the candidate context? | AI labeler | `windows.json` | Deterministic, seeded, no overlap |
| 3. Mark | Where are the best and worst entries and exits? | AI labeler + human review | `marks-ai.csv`, `marks-human.csv` | Both sides, both qualities |
| 4. Review | What distinguishes the good marks from the bad ones? | Quant Researcher | `review.md` | The candidate rule has at most two conditions |
| 5. Check | Does the rule hold on years that were not marked? | Quant Validator | Check table | At least 30 events; the pre-registered thresholds |
| 6. Decide | Test, refine, or stop? | Human | `decision.md` | A survivor goes to Strategy Research |

## 4. Entry conditions

A simulation run may start only when:

- the model identity is `M1.3` and the registry is `2026-11-10.2`;
- the run uses a recorded model bundle, not a live source;
- the label window is `strategy_dev` 2018–2019 and the check window is
  2020–2022;
- the run names at most two inputs and one exit;
- the mark criterion, sample size, seed, and check thresholds are recorded
  before the first mark;
- 2023, 2024, and 2025 are not read.

## 5. Mark rules

- A mark uses the human annotator schema:
  `chart,bar,marked_time_utc,action,quality,side,note`.
- A mark never uses a bar earlier than the event's `available_bar_index`.
- The same run marks both sides and both `good` and `bad` examples. A set with
  only good marks cannot teach the boundary.
- The oracle proposes; the AI labeler reviews; the human corrects. Rejected
  marks and the reason are recorded.
- The `note` carries the run id, the visual reason, and the indicator or event
  reason. It is at most 500 characters.
- A mark whose visual reason and indicator reason disagree is kept and flagged;
  the disagreement is a finding, not a defect to hide.

## 6. Check rules

The check is an event study, not a backtest:

- at least 30 non-overlapping events in 2020–2022;
- mean net 20-bar return > 0 after a 6 bps round-trip cost proxy;
- hit rate > 50%;
- the filtered set beats the unfiltered event control on mean return and hit
  rate;
- the direction holds in at least 3 of 5 half-years.

A passing screen is a hypothesis for Strategy Research. It is not a baseline and
it is not an edge. If the rule cannot be expressed in the current strategy
config, route it to Requirements & Design.

## 7. Records

One run is one folder under `data/simulation/<date>/<run-id>/`:

```text
manifest.json       premise, inputs, windows, seed, model/data identity, criteria
windows.json        deterministic window selection
charts/             PNGs and manifest.json with the plot geometry
marks-ai.csv        the AI marks
marks-human.csv     the human corrections, when present
review.md           what the marks show and the candidate rule
decision.md         test, refine, or stop
```

The run folder is local data, not committed. The synthesis report and any
workflow change are committed under `docs/`.

## 8. Boundaries

- No model, strategy, execution, or portfolio code change.
- No new indicator or event; the run reads what the registry already publishes.
- No statistics in the signal path; statistics is limited to the check.
- No post-hoc change to the mark criterion, sample, exit, or check thresholds.
- No claim of an edge from a simulation run. The check is a screen; the backtest
  is the next gate.
- The human remains the final authority for the run list, the check acceptance,
  and any promotion to Strategy Research.

## 9. Verification

- The harness selection, rendering, marks, and review have unit tests.
- The same seed produces the same `windows.json`; the same window produces the
  same PNG hash.
- The check report names the event count, the cost proxy, the control, and the
  half-year stability.
- The run's `manifest.json` records the model identity, registry, data version,
  code commit, seed, and the closed partitions.

## 10. Stop conditions

Stop and report when:

- the run needs model or strategy code;
- the check needs 2023, 2024, or 2025 data;
- the sample is underpowered and the floor would have to be lowered;
- the rule cannot be expressed in the current strategy config;
- a data-quality defect or a missing event code appears;
- the result is positive only after a post-hoc criterion change.

## 11. Related documents

- [`improvement-workflow.md`](improvement-workflow.md)
- [`run-contract.md`](run-contract.md)
- [`strategy-research.md`](strategy-research.md)
- [`recordkeeping.md`](recordkeeping.md)
- [`../execution-plans/2026-11-10/simulation-runs.md`](../execution-plans/2026-11-10/simulation-runs.md)
- [`../ai-transparency-log/2026-11-10/simulation-runs.md`](../ai-transparency-log/2026-11-10/simulation-runs.md)
- [`templates/simulation-run.md`](templates/simulation-run.md)
- [`../../tools/simulation/README.md`](../../tools/simulation/README.md)

## 12. Records and links

- Execution plan: [`docs/execution-plans/2026-11-10/simulation-runs.md`](../execution-plans/2026-11-10/simulation-runs.md)
- AI transparency log: [`docs/ai-transparency-log/2026-11-10/simulation-runs.md`](../ai-transparency-log/2026-11-10/simulation-runs.md)
- Run template: [`templates/simulation-run.md`](templates/simulation-run.md)
- Strategy handoff: [`strategy-research.md`](strategy-research.md)
- Record rules: [`recordkeeping.md`](recordkeeping.md)
