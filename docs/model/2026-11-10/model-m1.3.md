# M1.3 — Corrected Pivot Sequence, Fibonacci, and Horizontal-Band Semantics

- Date: 2026-11-10 (UTC)
- Model ID: `M1.3`
- Predecessor: [`M1.2`](model-m1.2.md)
- Registry: [`indicators.md`](../../registry/indicators.md), version `2026-11-10.2`
- Claim type: model behavior correction and event contract; no edge claim.

## 1. Scope

`M1.3` is `M1.2` with the audit corrections below. Every reading, event, claim,
and parameter not named here is unchanged from `M1.2`. The corrections are:

1. the pivot-sequence events record the pivot occurrence and the confirmation
   availability separately;
2. `level.fib_test` is a bar-range intersection test and emits one event per
   tested level;
3. the Fibonacci and band tolerances are fixed model constants and are not
   accepted by the spec-v2 config surface;
4. the band parameter is named `model.band.atr`, and `band_age` is measured from
   the youngest level's confirmation bar.

The corrections close the M1.2 audit findings `AUD-2026-11-10-01`,
`AUD-2026-11-10-02`, `AUD-2026-11-10-04`, and `AUD-2026-11-10-09`. The M1.2
contract remains historical and is not rewritten.

## 2. Pivot sequence

The readings are unchanged from `M1.2` §2: `lower_high_count`,
`lower_low_count`, `higher_high_count`, and `higher_low_count` count consecutive
strictly lower or higher confirmed major pivots; fewer than two pivots of a
kind is `None`; an equal latest pair breaks the sequence and yields zero.

`signal.lower_high_sequence` and `signal.lower_low_sequence` are emitted only on
the confirmation bar of the pivot that extends the sequence. For each event:

- `occurred_bar_index` is the latest pivot's `occurred_at`;
- `available_bar_index` is that pivot's `available_at`;
- `primary_value` is the count;
- `reference_value` is `2.0`;
- `direction_code` is `-1`.

A pivot candidate never affects the event before confirmation. The event is not
re-emitted while the reading is constant between pivots.

## 3. Fibonacci

The leg, ratios, levels, position, and nearest-level readings are unchanged from
`M1.2` §3.

`level.fib_test` is a bar-range intersection test. For each of the five fixed
levels `L` and tolerance `tol = fib_test_atr * ATR`, the event is emitted when:

```text
candle.high >= L - tol  and  candle.low <= L + tol
```

Every tested level emits its own event on that bar. The event carries:

- `occurred_bar_index = available_bar_index = bar`;
- `direction_code` = the leg direction (`+1` up leg, `-1` down leg);
- `primary_value` = close;
- `reference_code` = the ratio code (`236`, `382`, `500`, `618`, `786`);
- `reference_value` = the level price.

Distinct tested levels on one bar are distinct events; the batch deduplication
key is code, source, bar, direction, ratio code, and level price. The detector
does not choose a single "nearest" event.

`fib_test_atr` is fixed at `0.2` and is not a spec-v2 field. A missing or
non-positive ATR suppresses the event; it never replaces the level with zero.

## 4. Horizontal bands

The clustering, selection, edges, touches, width, and position are unchanged
from `M1.2` §4. The parameter is named `model.band.atr` with value `0.25`;
`model.band.min_touches` remains fixed at `2`.

`band_age` is the number of bars since the youngest level in the selected
cluster became available: `bar - max(level.available_at)`. This is the
confirmation age used by the existing `level.age` reading; it is not the pivot
occurrence age.

`level.band_test` is unchanged: a bar-range intersection with the selected band
expanded by `band_atr * ATR`, direction `+1` when close is at or above the
center and `-1` below it, `primary_value` close, `reference_value` center.

`band_atr` is fixed at `0.25` and is not a spec-v2 field.

## 5. Parameters

| Parameter | Value | Kind |
|---|---:|---|
| `model.pivots.major` | 5 | fixed |
| `model.fibonacci.ratios` | 0.236, 0.382, 0.5, 0.618, 0.786 | fixed |
| `model.fibonacci.test_atr` | 0.2 | fixed |
| `model.band.atr` | 0.25 | fixed |
| `model.band.min_touches` | 2 | fixed |
| `model.level.lookback` | 500 | fixed |

The spec-v2 `readings.extended` block rejects `fib_test_atr` and `band_atr`.
The model's own extended-config validator still checks both values are in
`0.0..=2.0` for programmatic callers.

## 6. Availability and causality

- A pivot candidate never affects a reading or event before confirmation.
- Sequence events carry the pivot occurrence and become available only at the
  confirmation bar.
- The Fibonacci leg changes only when a new confirmed pivot becomes available.
- Band membership uses `available_at <= bar`; future levels are never read.
- Every new reading is recomputed from the closed series and does not depend on
  the order in which the caller supplies unrelated timeframes.

## 7. Verification

- `per_row_extended_indicators::pivot_sequence_row_contract`
- `per_row_extended_indicators::fibonacci_row_contract`
- `per_row_extended_indicators::horizontal_band_row_contract`
- `per_row_extended_session_events::signal_lower_high_sequence_row_contract`
- `per_row_extended_session_events::signal_lower_low_sequence_row_contract`
- `per_row_extended_session_events::level_fib_test_row_contract`
- `per_row_extended_session_events::level_band_test_row_contract`
- `event_validation::sequence_events_record_the_pivot_occurrence_and_availability`
- `event_validation::appending_bars_does_not_change_past_events`
- `event_validation::repeated_model_runs_publish_the_same_events`
- `event_validation::one_occurrence_is_recorded_once`
- `readings::extended::tests::fibonacci_reads_the_confirmed_major_leg`
- `readings::extended::tests::fibonacci_levels_publish_without_atr_but_distance_is_null`
- `readings::extended::tests::fibonacci_nulls_a_zero_span_leg`
- `readings::extended::tests::horizontal_band_selects_the_largest_nearest_cluster`
- `readings::extended::tests::horizontal_band_is_null_without_two_levels_or_atr`
- `readings::extended::band::tests::*` for the cluster selection and tie-break
- `events::extended::tests::a_fibonacci_level_inside_the_bar_range_is_tested`
- `events::extended::tests::every_tested_fibonacci_level_emits_an_event`
- `events::extended::tests::a_fibonacci_level_outside_the_bar_range_is_not_tested`
- `spec::v2::tests::a_v2_fixture_rejects_fixed_extended_tolerances`
- Golden fixtures: `extended_readings_golden.csv`, `extended_events_golden.csv`

## 8. Boundary

The three indicators remain `signal`. Strategies read the registered events,
not the raw columns. The Fibonacci and band events are Level events and the
sequence events are Signal events; the event type, subtype, source, and role are
unchanged from `M1.2`. No statistical or fitted field enters a value.

## 9. Records

- Audit: [`model-structures-audit.md`](../../audits/2026-11-10/model-structures-audit.md)
- Remediation plan: [`audit-model-structures-remediation.md`](../../execution-plans/2026-11-10/audit-model-structures-remediation.md)
- Validation: [`validation.json`](../../registry/validation.json)
- Verification: [`verification.json`](../../registry/verification.json)
- Changelog: [`CHANGELOG.md`](../../../CHANGELOG.md)
