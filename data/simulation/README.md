# Simulation run store

`data/simulation/` holds local simulation-run artifacts. A simulation run is a
chart-first, hindsight-labeled exploration record; it is not an improvement run,
a backtest, or evidence of an edge.

## Layout

```text
data/simulation/<YYYY-MM-DD>/<run-id>/
  manifest.json       premise, inputs, model identity, data version, seed, criteria
  windows.json        deterministic window selection
  charts/             PNGs and a manifest with the plot geometry
  marks-ai.csv        the AI marks
  marks-human.csv     the human corrections, when present
  review.md           what the marks show and the candidate rule
  decision.md         test, refine, or stop
```

The `_data/` directory holds the recorded model reports the runs consume:

```text
data/simulation/_data/M1.3-2018-2022/
  config/             the run, data, model, execution, portfolio, and policy files
  report/             the model bundle written by `report model`
  reproduce.md        the exact command and provenance
```

## Rules

1. Everything here is local data; it is not committed. Only this README is
   tracked.
2. A run uses a recorded model bundle. It does not reach a live source or
   recompute a model value.
3. The model identity is `M1.3` and the registry is `2026-11-10.2`.
4. The label window is `strategy_dev` 2018–2019; the check is 2020–2022. 2023,
   2024, and 2025 are closed.
5. The marks use the annotator schema and never use a bar earlier than the
   event's `available_bar_index`.
6. A run with fewer than 30 non-overlapping candidates is recorded as
   underpowered and makes no rule claim.
7. The same manifest, seed, and bundle reproduce the same windows, marks, and
   PNG hashes.

## Contract

The workflow is [`docs/workflows/simulation-research.md`](../../docs/workflows/simulation-research.md).
The execution plan is
[`docs/execution-plans/2026-11-10/simulation-runs.md`](../../docs/execution-plans/2026-11-10/simulation-runs.md).
