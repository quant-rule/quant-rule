# Model Evidence Protocol

- Date: 2026-11-01 (UTC)
- Status: Current for improvement runs
- Scope: model-tier evidence; evaluation only, never the signal path
- Tooling: `tools/improvement/model_confidence.py`

## Purpose

A model run must show that the published measurements mean what the model
contract says before a strategy may use them. The protocol has two layers:

1. **Contract evidence:** formulas, ranges, nulls, availability, causality,
   determinism, golden fixtures, and coverage. These are program checks.
2. **Behavioral evidence:** whether the published axis state is followed by the
   price behavior the axis claims to describe.

Statistics is confined to this evaluation layer. No value from this protocol
enters a model axis, a strategy decision, or a portfolio rule.

## Primary metrics

| Axis | Primary metric | Horizon | Grade unit |
|---|---|---|---|
| Direction | `P(sign(forward return) = sign(direction))` | 5 bars | per instrument/timeframe |
| Momentum | `P(sign(forward return) = sign(momentum))` | 3 bars | per instrument/timeframe |
| Structure | ordered bucket monotonicity and shift continuation | 5 bars | per instrument/timeframe |
| Progression | target hit before invalidation; stage vs remaining move | leg lifecycle | per instrument/timeframe |

A hit rate is reported with its sample size, coverage, and a 95%
normal-approximation lower bound. The lower bound is the confidence number; it
is not a p-value. A direction or momentum hit rate whose lower bound is at or
below 0.50 is not evidence of a positive edge.

Structure and Progression are ordinal. They are reported as ordered buckets and
lifecycle counts; codes are never averaged. A bucket table with no ordering is
inconclusive, not neutral.

## Correctness baseline vs signal usability

Two questions are separate:

1. **Model correctness:** does the model publish the right numbers, causally,
   deterministically, with the documented availability and coverage?
2. **Signal usability:** is an axis or event a standalone predictor at a chosen
   horizon?

The model correctness baseline is accepted on the first question. The grades
below answer the second. A C grade does not make the model incorrect; it means
the axis must be used as an explicit strategy hypothesis, with the limitation
recorded, and the strategy run must prove the edge. A D grade still blocks.

This separation prevents a measurement layer from being rejected because it is
not, by itself, a trading signal.

## Grades

| Grade | Rule | Use |
|---|---|---|
| A | Contract checks pass; coverage ≥ 95%; primary lower bound > 0.55; stable across at least three instruments or folds | Usable in a strategy |
| B | Contract checks pass; coverage ≥ 90%; primary lower bound > 0.50; stable across at least two instruments or folds | Usable with the limitation recorded |
| C | Lower bound ≤ 0.50, unstable, or the primary metric is not computed | Diagnostic only; cannot gate a trade |
| D | Contract failure, coverage below 80%, or a wrong-sign direction | Blocks promotion |

A model run with any D blocks the strategy tier. An A/B axis may be used as a
standalone signal; a C axis may be used only as an explicit strategy hypothesis,
and the strategy run records the grade. The strategy run, not the model run,
decides whether the C-axis hypothesis has a cost-inclusive OOS edge.

## Evidence table

The model run writes one row per axis, instrument, and timeframe:

| Axis | TF | Instrument | n | Coverage | Estimate | 95% lower bound | Grade | Role |
|---|---|---:|---:|---|---:|---:|---|---|

`Role` is `signal`, `context`, or `diagnostic`. A strategy may read a `signal`
or `context` axis only when the grade and role allow it.

## Multiple testing

- Pre-register one primary metric and horizon per axis.
- Treat all other horizons, instruments, and timeframes as secondary.
- Apply a multiple-testing correction when more than one primary cell is used
  for a promotion decision; record the method before the run.
- A single positive cell is not a model baseline.

## Coverage and availability

- Report non-null coverage, the first available bar, and the availability delay
  for every axis and event.
- A missing measurement stays null; it is never replaced by zero or neutral.
- A level is usable only at or after its `available_at` bar.
- Progression coverage is part of the grade; a low-coverage axis cannot be a
  strategy gate.

## Limitations of the current harness

- `model_confidence.py` grades Direction and Momentum hit rates and reports
  Structure/Progression buckets and coverage. It does not yet grade events,
  levels, divergences, or signals.
- The first run is a 2021, 15m/1h, three-instrument harness pilot. It is not a
  multi-year validation.
- The review is a same-session pass until an independent Quant Validator record
  exists.
