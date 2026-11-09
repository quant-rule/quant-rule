# Model Change — `vol_regime` State

- Date: 2026-11-09 (UTC)
- Status: Current; context state added; existing event values unchanged
- Programme: [`sr9-15m-structure`](../../execution-plans/2026-11-09/sr9-model-states.md)
- Registry: [`indicators.md`](../../registry/indicators.md), `atr_term`
- Claim type: model measurement; no directional claim

## Definition

`vol_regime` discretises the existing ATR term structure:

| Code | Condition | Meaning |
|---:|---|---|
| `0` | `ATR14 / ATR_slow <= 0.7` | compressed |
| `1` | `0.7 < ATR14 / ATR_slow < 1.3` | normal |
| `2` | `ATR14 / ATR_slow >= 1.3` | expanded |

`ATR_slow` is the existing slow ATR at the configured `atr_slow_window` (100
bars by default). The state is available on the same closed bar as the extended
`vol_regime` reading and is published in `readings.csv`, `report.json`, and
`states.vol_regime` beside the other discrete states.

The state is **context**. It does not claim direction and does not alter any
event. It exists so a strategy can condition on volatility without reading a raw
indicator value.

## Contract

| Item | Value |
|---|---|
| Role | context |
| Codes | 0, 1, 2 |
| Thresholds | 0.7 and 1.3, fixed in the model |
| Availability | the ATR term-structure bar |
| Causality | only the current and prior bars |
| Output | `states.vol_regime` and the existing `vol_regime` readings column |
| Event changes | none |

## Verification

- Mapping unit test: exact -1/0/1/2 codes; anything else is null.
- Integration tests in `crates/quantrule-model/tests/context_states.rs`:
  the published state equals the extended reading at every bar; causality
  across every prefix; determinism across repeated runs; empty, single-candle,
  short-series, time-gap, and zero-volume cases.
- Strategy parser test: `vol_regime` in 0..=2 parses; 3 is rejected.
- Model-spec test: a spec can read the state.
- CSV/report state is read by the strategy bundle; an old bundle without the
  column still loads.
- Existing model tests pass; no event value changes.

## Boundary

A strategy may read `vol_regime` as a context state. It may not read the raw
ATR ratio or the extended `vol_regime` column through the strategy input. The
state is the model's published feature; the thresholds are fixed by the model
contract, not re-derived at each call site.
