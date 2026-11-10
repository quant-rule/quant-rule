# M1.2 — Pivot Sequence, Fibonacci, and Horizontal Bands

- Date: 2026-11-10 (UTC)
- Model ID: `M1.2`
- Predecessor: [`M1.1`](../2026-11-08/model-m1.1.md)
- Registry: [`indicators.md`](../../registry/indicators.md), version `2026-11-10.1`
- Claim type: model measurement and event contract; no edge claim.

## 1. Scope

`M1.2` is `M1.1` plus three structure measurements and four events. Existing
readings, events, and their values are unchanged. All inputs are OHLCV candles;
the model remains pure, causal, and deterministic.

| Addition | Code | Role | Outputs |
|---|---:|---|---|
| `pivot_sequence` | 47 | signal | lower/higher high/low counts |
| `fibonacci` | 48 | signal | causal retracement levels and nearest level |
| `horizontal_band` | 49 | signal | multi-touch band edges, touches, age, width, position |

Sources: 55 pivot sequence, 56 Fibonacci, 57 horizontal band.
Events: 147 `signal.lower_high_sequence`, 148 `signal.lower_low_sequence`,
149 `level.fib_test`, 150 `level.band_test`.

## 2. Pivot sequence

The sequence counts use confirmed major pivots only. A pivot is usable on its
`available_at` bar, never on its occurrence bar before confirmation.

For confirmed major highs in occurrence order, `lower_high_count` is the number
of consecutive pairs where the newer high is strictly lower than the previous
high, ending at the latest confirmed high. `higher_high_count` is the mirror.
The low counts mirror with confirmed major lows. Equal highs or lows break the
sequence and produce zero, not a directional count. Fewer than two confirmed
pivots of a kind is `None`, not zero.

`signal.lower_high_sequence` is appended on the latest high's confirmation bar
when `lower_high_count >= 2`; `signal.lower_low_sequence` mirrors on a low. The
event carries the count as `primary_value` and `2` as `reference_value`.

## 3. Fibonacci

The leg is the latest confirmed major pivot and the most recent confirmed major
pivot of the opposite kind before it:

- latest high: leg is the prior confirmed low to that high (`+1`);
- latest low: leg is the prior confirmed high to that low (`−1`).

Both anchors must be confirmed and available; otherwise every Fibonacci output
is `None`. Levels use fixed ratios `0.236`, `0.382`, `0.5`, `0.618`, `0.786`:

- up leg `L -> H`: `level(r) = H - r * (H - L)`;
- down leg `H -> L`: `level(r) = L + r * (H - L)`.

`fib_position` is `(close - L) / (H - L)` for an up leg and
`(H - close) / (H - L)` for a down leg. `fib_nearest_ratio` is the ratio of the
nearest level to close; `fib_nearest_atr` divides that distance by ATR.

`level.fib_test` is appended when the bar high or low is within
`fib_test_atr * ATR` of a level. At most one level is emitted per bar: the
nearest tested level to close. The event carries the ratio code in
`reference_code`, the level in `reference_value`, and the leg direction.

## 4. Horizontal bands

Confirmed major levels available at or before the bar and within the existing
`level_lookback` are sorted by price. A new cluster starts when the gap to the
previous level exceeds `band_atr * ATR`. The selected cluster has the most
levels; ties go to the cluster center nearest close, then to the lower center.
At least two levels are required; otherwise all band outputs are `None`.

- `band_upper` / `band_lower`: maximum / minimum price in the cluster;
- `band_center`: arithmetic mean of the cluster prices;
- `band_touches`: cluster size;
- `band_age`: bars since the youngest level in the cluster;
- `band_width_atr`: `(upper - lower) / ATR`;
- `band_position`: `(close - lower) / (upper - lower)`, null when width is zero.

`level.band_test` is appended when the bar range intersects the band expanded by
`band_atr * ATR`. Direction is `+1` when close is at or above the center
(support test) and `−1` below it (resistance test).

## 5. Parameters

| Parameter | Value | Kind |
|---|---:|---|
| `model.pivots.major` | 5 | fixed |
| `model.fibonacci.ratios` | 0.236, 0.382, 0.5, 0.618, 0.786 | fixed |
| `model.fibonacci.test_atr` | 0.2 | fixed |
| `model.band.cluster_atr` | 0.25 | fixed |
| `model.band.min_touches` | 2 | fixed |
| `model.level.lookback` | 500 | fixed |

## 6. Availability and causality

- A pivot candidate never affects a reading or event before confirmation.
- The Fibonacci leg changes only when a new confirmed pivot becomes available.
- Band membership uses `available_at <= bar`; future levels are never read.
- Counts and levels are recomputed from the closed series and do not depend on
  the order in which the caller supplies unrelated timeframes.
- Equal highs/lows and missing ATR are covered by unit tests.

## 7. Verification

- `per_row_extended_indicators::pivot_sequence_row_contract`
- `per_row_extended_indicators::fibonacci_row_contract`
- `per_row_extended_indicators::horizontal_band_row_contract`
- `per_row_extended_session_events::signal_lower_high_sequence_row_contract`
- `per_row_extended_session_events::signal_lower_low_sequence_row_contract`
- `per_row_extended_session_events::level_fib_test_row_contract`
- `per_row_extended_session_events::level_band_test_row_contract`
- `event_validation::appending_bars_does_not_change_past_events`
- `event_validation::repeated_model_runs_publish_the_same_events`
- `event_validation::one_occurrence_is_recorded_once`
- Golden fixtures: `extended_readings_golden.csv`, `extended_events_golden.csv`

## 8. Boundary

The three indicators are `signal` because each carries a forward claim. Their
raw readings are available to reports and analysis; strategies read the
registered events, not the raw columns. `fib_nearest_ratio`, `band_touches`, and
the sequence counts are numeric readings, not labels.
