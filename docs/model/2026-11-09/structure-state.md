# Model Change — `structure_state` State

- Date: 2026-11-09 (UTC)
- Status: Current; context state added; existing event values unchanged
- Programme: [`sr9-15m-structure`](../../execution-plans/2026-11-09/sr9-model-states.md)
- Registry: [`indicators.md`](../../registry/indicators.md), `swing_structure`
- Claim type: model measurement; no directional claim

## Definition

`structure_state` publishes the confirmed major swing structure:

| Code | Condition | Meaning |
|---:|---|---|
| `+1` | last two confirmed major highs higher and last two confirmed major lows higher | higher highs and higher lows |
| `-1` | last two confirmed major highs lower and last two confirmed major lows lower | lower highs and lower lows |
| `0` | mixed, equal, or not enough confirmed pivots | no directional structure |

The state uses only confirmed major pivots. A pivot becomes usable on its
availability bar; the state never reads a candidate or a future pivot. The
confirmation delay is part of the definition.

The state is **context**. It does not claim direction and does not alter any
event. It exists so a strategy can condition on the confirmed structure without
reading raw pivot values.

## Contract

| Item | Value |
|---|---|
| Role | context |
| Codes | -1, 0, +1 |
| Inputs | confirmed major pivot highs and lows |
| Availability | the bar on which the second pivot pair is confirmed |
| Causality | only confirmed pivots available at or before the bar |
| Output | `states.structure_state` and the existing `structure_state` readings column |
| Event changes | none |

## Verification

- Mapping unit test: exact -1/0/1 codes; anything else is null.
- Structure tie test: equal highs or lows map to 0, not a directional code.
- Integration tests in `crates/quantrule-model/tests/context_states.rs`:
  the published state equals the extended reading at every bar; causality
  across every prefix; determinism across repeated runs; empty, single-candle,
  short-series, time-gap, and zero-volume cases.
- Strategy parser test: `structure_state` in -1..=1 parses; out-of-range values
  are rejected.
- Model-spec test: a spec can read the state.
- CSV/report state is read by the strategy bundle; an old bundle without the
  column still loads.
- Existing model tests pass; no event value changes.

## Boundary

A strategy may read `structure_state` as a context state. It may not read the
raw pivot values or the extended `structure_state` column through the strategy
input. The state is the model's published feature; the pivot confirmation rule
is fixed by the model contract, not re-derived at each call site.
