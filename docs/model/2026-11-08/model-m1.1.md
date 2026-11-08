# M1.1 — Opening-Range Sweep Events

- Date: 2026-11-08 (UTC)
- Model ID: `M1.1`
- Registry version: `2026-11-08.1`
- Predecessor: [`M1.0`](../2026-11-05/model-m1.0.md)
- Registry: [`docs/registry/indicators.json`](../../registry/indicators.json)
- Claim type: model implementation contract; no strategy or edge claim

## 1. Identity and scope

`M1.1` is `M1.0` plus two session signal events. The flat indicator/event
architecture, the reading kernels, the confirmed-level registry, the progression
series, and the causal event contract are unchanged. No indicator is added.

| Item | M1.0 | M1.1 |
|---|---:|---:|
| Indicators | 46 | 46 |
| Active events | 143 | 145 |
| Validation claims | 189 | 191 |
| Parameters | 101 | 102 |

The new events are `signal.opening_range_sweep` (144) and
`signal.opening_range_sweep_control` (145), both from source 51
(`opening_range`), event type 3 (signal), subtype 23 (session signal), role
`signal`.

## 2. New reading parameter

`SessionConfig` gains `sweep_atr: f64`, default `0.0`, non-negative and finite.
The model spec session object accepts `sweep_atr`; the shell parser rejects a
negative or non-finite value. The existing `opening_range_bars` default remains
2. The opening range is the first `opening_range_bars` bars of the London
session.

## 3. Event definitions

Let `ORH` and `ORL` be the completed opening-range high and low, `ATR` the ATR at
the bar, and `sweep = sweep_atr * ATR`. The events are evaluated only after the
opening range is complete and only inside the session.

| Event | Formula | Direction |
|---|---|---:|
| `signal.opening_range_sweep` | `H > ORH + sweep and C < ORH` | `-1` |
| `signal.opening_range_sweep` | `L < ORL - sweep and C > ORL` | `+1` |
| `signal.opening_range_sweep_control` | `H > ORH + sweep and C >= ORH` | `+1` |
| `signal.opening_range_sweep_control` | `L < ORL - sweep and C <= ORL` | `-1` |

If both sides exceed the threshold, the larger excursion wins. An exact tie
emits no event. A touch at the level with zero excursion is not a sweep. The
event occurs and becomes available at the bar close; `primary_value` is the
close and `reference_value` is the opening-range level.

The reclaim event is a reversal claim. The control event is a continuation
claim and exists to falsify the reclaim premise; it is not a candidate in the
MR7 programme.

## 4. Availability and causality

The opening-range high and low are known once the opening range is complete.
The sweep uses only the current bar's high, low, close, and the ATR at that bar.
No later bar influences the event. The event record carries `occurred_bar_index
= available_bar_index = bar`. The event is not upgraded in place.

## 5. Registry and claims

| Event | Code | Registry role | Validation outcome | Status |
|---|---:|---|---|---|
| `signal.opening_range_sweep` | 144 | signal | reversal | pending |
| `signal.opening_range_sweep_control` | 145 | signal | path_directional | pending |

Both claims use the matched barrier: 20 bars, +2 ATR target, −1 ATR stop, and a
matched-barrier baseline. The claims are `pending`; the MR7 programme is the
first predictive test.

## 6. Tests and evidence

- Row contracts: `events::session::row_tests::signal_opening_range_sweep_row_contract`
  and `signal_opening_range_sweep_control_row_contract`.
- Causality and determinism are exercised by the row contract and the
  `event_validation` fixture.
- Golden fixture: `crates/quantrule-model/tests/fixtures/session_events_golden.csv`
  is regenerated deliberately for M1.1.
- Edge cases: no completed opening range, zero-width range, equal high/low
  excursion, zero volume, and a session boundary are covered by the session
  fixture and the event contract.
- Emitter check: `check_event_emitters.py` sees both constants in production.

## 7. Boundary

The model publishes the events and their numeric values. It does not publish a
strategy rule, an entry, an exit, a position, or a P&L. The strategy tier remains
closed. A development result is not an edge claim.
