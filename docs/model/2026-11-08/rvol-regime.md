# Model Change — `rvol_regime` State

- Date: 2026-11-08 (UTC)
- Status: Current; context state added; existing event values unchanged
- Programme: [`sr2-feature-programme.md`](../../execution-plans/2026-11-08/sr2-feature-programme.md)
- Registry: [`indicators.md`](../../registry/indicators.md)
- Claim type: model measurement; no directional claim

## Definition

`rvol_regime` discretises the existing RVOL20 reading:

| Code | Condition | Meaning |
|---:|---|---|
| `-1` | `RVOL20 < 1.4` | low participation |
| `0` | `1.4 ≤ RVOL20 < 2.0` | normal participation |
| `+1` | `RVOL20 ≥ 2.0` | high participation |

`RVOL20 = volume / mean(volume, 20)` is the existing `rvol` reading. The state
is available on the same closed bar as RVOL20 and is published in
`readings.csv` and `report.json` beside the other discrete states.

The state is **context**. It does not claim direction and does not alter any
event. It exists so a strategy can condition on participation without reading a
raw indicator value.

## Contract

| Item | Value |
|---|---|
| Role | context |
| Codes | -1, 0, +1 |
| Thresholds | 1.4 and 2.0, fixed in the model |
| Availability | the RVOL20 bar |
| Causality | only the current and prior bars |
| Output | `states.rvol_regime` and the `rvol_regime` readings column |
| Event changes | none |

## Verification

- Threshold unit test: `rvol_regime_uses_the_contract_thresholds`.
- Integration tests in `crates/quantrule-model/tests/rvol_regime.rs`:
  the fixture golden codes, causality across every prefix, determinism across
  repeated runs, and the empty, single-candle, short-series, zero-volume,
  missing-volume, and time-gap cases.
- Strategy parser test: `an_rvol_regime_state_condition_parses`.
- Model-spec test: `a_spec_can_read_the_rvol_regime`.
- CSV/report state is read by the strategy bundle.
- Existing model tests pass; no event value changes.

## Boundary

A strategy may read `rvol_regime` as a context state. It may not read the raw
`rvol` or `rvol_ratio` readings through the strategy input. The state is the
model's published feature; the strategy's threshold is fixed by the model
contract, not re-derived at each call site.
