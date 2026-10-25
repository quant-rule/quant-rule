# The Research Output Contract

- Date: 2026-10-24 (UTC)
- Issue: [#76](https://github.com/quant-rule/quant-rule/issues/76) — docs: define the research output contract
- Status: Contract. The analytics engine and the export follow this document.
- Related: [model contract](../../model/2026-10-21/four-axis-analytical-model.md), [plan](../../execution-plans/2026-10-24/76-research-output.md), [transparency log](../../ai-transparency-log/2026-10-24/76-research-output.md)

## 1. Purpose

This document defines the research dataset: a deterministic, file-based export of the model's published numbers in three cumulative layers, plus the record files and metadata that make the dataset self-describing.

The dataset is the input to research, not a trading interface. It contains no entries, exits, stops, position sizes, or recommendations, and no field combines the four axes into one score. Every field is a number, a null, a list of numbers, or a record of numbers.

The analytics engine that builds the layers is pure: no I/O, no clock, no randomness. The shell owns every read and write, and the export is performed by `research export <csv-path> <dataset> --data-version <id> [--root <dir>]` ([#80](https://github.com/quant-rule/quant-rule/issues/80)).

## 2. Dataset layout

```text
data/
  raw/
    <dataset>/
      m1.csv            required
      m5.csv            optional, when the source supplied it
      m15.csv           optional
      h1.csv            optional
  processed/
    <dataset>/
      layer1.csv
      layer2.csv
      layer3.csv
      events.csv
      legs.csv
      levels.csv
      consensus.csv
      manifest.json
      data_dictionary.csv
      data_dictionary.md
      codes.csv
      README.md
```

`<dataset>` is a caller-supplied directory name. The same name is recorded in the manifest. Raw files hold the accepted source candles; processed files are derived from them. A higher timeframe is derived by the data engine when the source supplies only M1, and its bars then appear in Layers 1 and 2 even though no raw file exists for it.

The raw file of a timeframe is `<lowercase timeframe>.csv`, using the same timeframe names as the model ladder. The processed file set is fixed; a file is always written, even when it has zero data rows, so that the file list in the manifest never depends on the data.

The layer export implemented by #80 writes `m1.csv`, `layer1.csv`, `layer2.csv`, `layer3.csv`, and the metadata. The event, leg, level, and consensus files in this layout are the dataset's record contract; their persistence belongs to the store epic (#64) and they are not written by the layer export.

## 3. Common representation

| Rule | Representation |
|---|---|
| Encoding | UTF-8, no byte-order mark |
| Line ending | LF (`\n`) |
| Delimiter | `,` (RFC 4180 quoting) |
| Null | an empty field |
| Float | the shortest decimal that round-trips to the same `f64`, as produced by Rust's `f64` `Display`; no exponent, no thousands separator, `.` as the decimal mark |
| Negative zero | written as `0`, so zero has one representation |
| Integer | decimal, no leading zeros, no `+` sign |
| List | `;`-separated elements with no spaces, each formatted by its own rule; an empty list is an empty field |
| Boolean-like code | `0` or `1`; never `true`, `false`, or a word |
| Timestamp | integer Unix seconds in UTC in the processed layers; the raw file keeps the source `YYYY-MM-DD HH:MM:SS+00:00` form |
| Non-finite value | never written; `NaN` and `±inf` are a contract violation and fail the export |

A null is not a list: a list field is never null, and an empty list is written as an empty field. The two list-bearing aggregate fields are the only list fields in the dataset, and both are documented in §7.3.

A valid zero is written as `0` and is never replaced by a null. A null is never replaced by `0`, by the previous value, or by another axis's value. A missing volume and a zero volume are different: `volume` is null when the source supplied no volume, and `0` when the source supplied a zero.

Descriptions in the metadata files may contain commas, quotes, and semicolons; they are quoted with RFC 4180 rules. The layer and record files contain no such fields, so they are never quoted.

## 4. Data version and identity

The data version is the identity of the input series: its source, its timeframe, and its coverage. It is set by the caller and recorded verbatim in the manifest and in the store's dataset row. It is never inferred from the filesystem, the clock, or the process.

A recommended form is `<source>:<symbol>:<timeframe>:<first_open_time>-<last_open_time>`, for example `binance:BTCUSDT:60:1704067200-1710115200`. The form is a convention; the contract requires only that the value is non-empty, stable for the same input, and different for different input.

The model version is the workspace package version. The configuration recorded in the manifest is the timeframe ladder, the structure scale, and the 100,000-bar cap. The contract dates are recorded as well, so a later reader can tell which field set produced the files.

## 5. Layer 1 — candle, readings, and metrics

`layer1.csv` has one row per closed bar per timeframe, ordered by `timeframe_secs` ascending and then `bar_index` ascending. It has 61 columns.

The row key is `(timeframe_secs, bar_index)`. `bar_index` is the 0-based position of the bar in its timeframe series. Bars are preserved in source order; a gap is a missing position in time, not a synthetic bar and not a null row, so `open_time` is not required to be an arithmetic function of `bar_index`.

### 5.1 Identity and candle

| Column | Type | Range | Null | Meaning |
|---|---|---|---|---|
| `timeframe_secs` | int | > 0 | no | Source timeframe in seconds |
| `bar_index` | int | >= 0 | no | 0-based closed-bar position in the timeframe series |
| `open_time` | int | UTC seconds | no | Bar open time |
| `open` | float | finite | no | Open price |
| `high` | float | finite | no | High price |
| `low` | float | finite | no | Low price |
| `close` | float | finite | no | Close price |
| `volume` | float | >= 0 | yes | Volume; null when the source supplied none, `0` when it supplied zero |

### 5.2 Readings

Readings are the indicator calculations the model consumes. They are published for every bar, so the metrics can be audited without recomputation.

| Column | Type | Range | Null | Meaning |
|---|---|---|---|---|
| `ema20` | float | finite | yes | EMA 20 of the close |
| `ema50` | float | finite | yes | EMA 50 of the close |
| `ema200` | float | finite | yes | EMA 200 of the close |
| `rsi14` | float | 0..100 | yes | RSI 14 |
| `atr14` | float | >= 0 | yes | ATR 14 |
| `rvol20` | float | >= 0 | yes | Relative volume, 20-bar baseline |

A reading is null while its window is incomplete. `rvol20` is also null when a candle in its window has no volume or when the 20-bar volume baseline is zero. The model publishes no per-reading reason code; the four axes' `unavailable_reason_code` fields are the published reasons, and the dictionary states each reading's null condition. A reading is never replaced by zero.

### 5.3 Direction

| Column | Type | Range | Null | Meaning |
|---|---|---|---|---|
| `direction` | float | -1..+1 | yes | Directional alignment |
| `direction_state_code` | int | -1, 0, +1 | yes | Compact state: -1 below -0.1, 0 in the inclusive ±0.1 band, +1 above +0.1 |
| `direction_size_percentile` | float | 0..100 | yes | Empirical percentile of `abs(direction)` against up to 252 prior valid values; null below 50 samples |
| `direction_readiness_code` | int | 0..2 | no | 0 unavailable, 1 provisional, 2 ready |
| `direction_unavailable_reason_code` | int | 1..6 | yes | Reason `direction` is null; null when `direction` is a value |
| `direction_state_age_bars` | int | >= 0 | no | Closed bars the compact state has held |
| `direction_state_changes_50` | int | >= 0 | no | Compact-state changes in the last 50 code-bearing closed bars |

### 5.4 Momentum

| Column | Type | Range | Null | Meaning |
|---|---|---|---|---|
| `momentum_force` | float | -1..+1 | yes | Signed RSI force |
| `momentum_acceleration` | float | -1..+1 | yes | Signed change in force over three bars |
| `momentum_intensification` | float | -1..+1 | yes | Signed change in the force magnitude over three bars |
| `momentum_state_code` | int | -1, 0, +1 | yes | Compact state of `momentum_force`, same bucketing as Direction |
| `momentum_size_percentile` | float | 0..100 | yes | Empirical percentile of `abs(momentum_force)` |
| `momentum_readiness_code` | int | 0..2 | no | 0 unavailable, 1 provisional, 2 ready |
| `momentum_unavailable_reason_code` | int | 1..6 | yes | Reason `momentum_force` is null |
| `momentum_state_age_bars` | int | >= 0 | no | Closed bars the compact state has held |
| `momentum_state_changes_50` | int | >= 0 | no | Compact-state changes in the last 50 code-bearing closed bars |

### 5.5 Structure

| Column | Type | Range | Null | Meaning |
|---|---|---|---|---|
| `structure_score` | float | -1..+1 | yes | Swing-relation score, `(high_relation_code + low_relation_code) / 2` |
| `structure_code` | int | -2..+2 | yes | `round(2 * structure_score)` |
| `structure_scale_code` | int | 1..2 | no | Scale the swing fields were computed at: 1 minor, 2 major |
| `high_relation_code` | int | -1, 0, +1 | yes | Relation of the latest two confirmed swing highs |
| `low_relation_code` | int | -1, 0, +1 | yes | Relation of the latest two confirmed swing lows |
| `ema_distance_atr_20` | float | finite | yes | `(close - ema20) / atr14` |
| `ema_distance_atr_50` | float | finite | yes | `(close - ema50) / atr14` |
| `ema_distance_atr_200` | float | finite | yes | `(close - ema200) / atr14` |
| `ema_cross_code_20` | int | -1, 0, +1 | yes | Close/EMA 20 cross |
| `ema_cross_code_50` | int | -1, 0, +1 | yes | Close/EMA 50 cross |
| `ema_cross_code_200` | int | -1, 0, +1 | yes | Close/EMA 200 cross |
| `ema_retest_code_20` | int | -1, 0, +1 | yes | EMA 20 zone retest |
| `ema_retest_code_50` | int | -1, 0, +1 | yes | EMA 50 zone retest |
| `ema_retest_code_200` | int | -1, 0, +1 | yes | EMA 200 zone retest |
| `structure_size_percentile` | float | 0..100 | yes | Empirical percentile of `abs(structure_score)` |
| `structure_readiness_code` | int | 0..2 | no | 0 unavailable, 1 provisional, 2 ready |
| `structure_unavailable_reason_code` | int | 1..6 | yes | Reason `structure_score` is null |
| `structure_state_age_bars` | int | >= 0 | no | Closed bars the compact state has held |
| `structure_state_changes_50` | int | >= 0 | no | Compact-state changes in the last 50 code-bearing closed bars |

The EMA distance, cross, and retest fields are auxiliary published measurements. They do not enter the structure score and are never counted as a second structural feature.

### 5.6 Progression

| Column | Type | Range | Null | Meaning |
|---|---|---|---|---|
| `progress` | float | unclamped | yes | Progress of the published leg, `(close - origin) / (target - origin)`; the raw value is retained |
| `progression_stage_code` | int | 0..5 | yes | Stage of the published leg |
| `progression_leg_direction` | int | -1, +1 | yes | Direction of the published leg: +1 upward from a low, -1 downward from a high |
| `progression_leg_scale_code` | int | 1..2 | yes | Scale of the published leg |
| `progression_size_percentile` | float | 0..100 | yes | Empirical percentile of `abs(displacement_atr)` |
| `progression_readiness_code` | int | 0..2 | no | 0 unavailable, 1 provisional, 2 ready |
| `progression_unavailable_reason_code` | int | 1..6 | yes | Reason `progress` is null |
| `progression_state_age_bars` | int | >= 0 | no | Closed bars the published stage has held |
| `progression_state_changes_50` | int | >= 0 | no | Published-stage changes in the last 50 code-bearing closed bars |

`progress` is the raw, unclamped value and may be negative or greater than one. The clamped value is published separately as `progression_continuous` in Layer 2. An ordinal stage is never averaged and never used in place of `progress`.

### 5.7 Context

| Column | Type | Range | Null | Meaning |
|---|---|---|---|---|
| `atr_ratio` | float | >= 0 | yes | `atr14 / SMA20(atr14)` |
| `rvol_ratio` | float | >= 0 | yes | `rvol20` |
| `context_code` | int | 1..4 | yes | 1 expanding, 2 coiling, 3 settling, 4 high-range/low-participation |

Context never votes on an axis. The code is null when either ratio is null; the raw ratios are retained even then. The four code conditions are in the model contract §12.

## 6. Layer 2 — axis resolutions

`layer2.csv` has the same row set and row order as `layer1.csv`. Its first 61 columns are exactly the Layer 1 columns in the same order; it then appends 12 resolution columns, for 73 columns in total.

### 6.1 Continuous and ordinal values

| Axis | Continuous source | Ordinal | Reason when null |
|---|---|---|---|
| Direction | `direction` | bucket of `direction` | `direction_unavailable_reason_code` |
| Momentum | `momentum_force` | bucket of `momentum_force` | `momentum_unavailable_reason_code` |
| Structure | `structure_score` | bucket of `structure_score` | `structure_unavailable_reason_code` |
| Progression | `clamp(progress, -1, +1)` | bucket of the clamped value | `progression_unavailable_reason_code` |

The four appended groups are:

| Column | Type | Range | Null | Meaning |
|---|---|---|---|---|
| `direction_continuous` | float | -1..+1 | yes | Direction resolution value |
| `direction_ordinal` | int | 0..9 | yes | Direction equal-width bucket |
| `direction_resolution_reason_code` | int | 1..6 | yes | Reason both resolution values are null |
| `momentum_continuous` | float | -1..+1 | yes | Momentum resolution value |
| `momentum_ordinal` | int | 0..9 | yes | Momentum equal-width bucket |
| `momentum_resolution_reason_code` | int | 1..6 | yes | Reason both resolution values are null |
| `structure_continuous` | float | -1..+1 | yes | Structure resolution value |
| `structure_ordinal` | int | 0..9 | yes | Structure equal-width bucket |
| `structure_resolution_reason_code` | int | 1..6 | yes | Reason both resolution values are null |
| `progression_continuous` | float | -1..+1 | yes | `clamp(progress, -1, +1)` |
| `progression_ordinal` | int | 0..9 | yes | Progression equal-width bucket |
| `progression_resolution_reason_code` | int | 1..6 | yes | Reason both resolution values are null |

### 6.2 Bucket mapping

The ordinal range is ten equal-width buckets over `[-1, +1]`. Buckets are closed on the left and open on the right, except the last, which is closed on both ends:

| Bucket | Interval |
|---|---|
| 0 | `[-1.00, -0.80)` |
| 1 | `[-0.80, -0.60)` |
| 2 | `[-0.60, -0.40)` |
| 3 | `[-0.40, -0.20)` |
| 4 | `[-0.20, 0.00)` |
| 5 | `[0.00, 0.20)` |
| 6 | `[0.20, 0.40)` |
| 7 | `[0.40, 0.60)` |
| 8 | `[0.60, 0.80)` |
| 9 | `[0.80, 1.00]` |

The bucket is the count of the nine boundaries `{-0.8, -0.6, -0.4, -0.2, 0.0, 0.2, 0.4, 0.6, 0.8}` that are less than or equal to the value:

```text
bucket(v) = |{b in {-0.8, -0.6, -0.4, -0.2, 0.0, 0.2, 0.4, 0.6, 0.8} : v >= b}|
```

The boundaries are compared directly rather than recomputed by division, so the mapping is exact and platform-independent: `-0.8` is bucket 1, `0.0` is bucket 5, and `+0.8` is bucket 9. A value outside `[-1, +1]` is clamped first; a non-finite value is a contract violation and fails the export.

Boundary examples:

| Value | Bucket |
|---|---|
| -1.0 | 0 |
| -0.8 | 1 |
| -0.79 | 1 |
| -0.2 | 4 |
| -0.0 | 5 |
| 0.0 | 5 |
| 0.199 | 5 |
| 0.2 | 6 |
| 0.8 | 9 |
| 1.0 | 9 |

### 6.3 Null and reason rules

1. If the continuous value is null, the ordinal is null and the reason column carries the axis's `unavailable_reason_code`.
2. The reason column is null exactly when the continuous value is a value. A null continuous value with a null reason is a contract violation and fails the export; every axis publishes a reason when it publishes no value.
3. A valid zero is a value. It resolves to bucket 5 and has a null reason.
4. Progression is resolved from the clamped value, but the raw `progress` remains in Layer 1. A raw progress of `1.5` resolves to `1.0` and bucket 9; a raw progress of `-0.25` resolves to `-0.25` and bucket 3.
5. An axis's null never fills another axis. There is no cross-axis imputation and no composite resolution.
6. Ordinals are counted, compared, and never averaged. No field in the dataset is a mean of ordinal codes.

## 7. Layer 3 — as-of higher timeframes and aggregates

`layer3.csv` has one row per M1 bar, ordered by `bar_index` ascending. Each row has:

1. the 73 Layer 2 columns of that M1 bar, unchanged in name and order;
2. one as-of block per configured higher timeframe, in ladder order;
3. the 82 cross-timeframe aggregate columns.

With three higher timeframes (M5, M15, H1) the file has `73 + 3 * 67 + 82 = 356` columns. In general, with `n` higher timeframes it has `155 + 67 * n` columns.

The file requires an M1 base. A dataset whose fastest configured timeframe is not M1 has no Layer 3, and the file is written empty with its header.

### 7.1 As-of block template

Each block is prefixed with the timeframe name (`m5_`, `m15_`, `h1_`) and has 67 columns. The prefix is the only difference between the blocks; the column set is fixed.

| Group | Count | Columns before the prefix |
|---|---|---|
| Identity | 1 | `bar_index` |
| Candle | 6 | `open_time`, `open`, `high`, `low`, `close`, `volume` |
| Direction | 7 | `direction`, `direction_state_code`, `direction_size_percentile`, `direction_readiness_code`, `direction_unavailable_reason_code`, `direction_state_age_bars`, `direction_state_changes_50` |
| Momentum | 9 | `momentum_force`, `momentum_acceleration`, `momentum_intensification`, `momentum_state_code`, `momentum_size_percentile`, `momentum_readiness_code`, `momentum_unavailable_reason_code`, `momentum_state_age_bars`, `momentum_state_changes_50` |
| Structure | 19 | `structure_score`, `structure_code`, `structure_scale_code`, `high_relation_code`, `low_relation_code`, `ema_distance_atr_20`, `ema_distance_atr_50`, `ema_distance_atr_200`, `ema_cross_code_20`, `ema_cross_code_50`, `ema_cross_code_200`, `ema_retest_code_20`, `ema_retest_code_50`, `ema_retest_code_200`, `structure_size_percentile`, `structure_readiness_code`, `structure_unavailable_reason_code`, `structure_state_age_bars`, `structure_state_changes_50` |
| Progression | 9 | `progress`, `progression_stage_code`, `progression_leg_direction`, `progression_leg_scale_code`, `progression_size_percentile`, `progression_readiness_code`, `progression_unavailable_reason_code`, `progression_state_age_bars`, `progression_state_changes_50` |
| Context | 3 | `atr_ratio`, `rvol_ratio`, `context_code` |
| Resolutions | 12 | `direction_continuous`, `direction_ordinal`, `direction_resolution_reason_code`, `momentum_continuous`, `momentum_ordinal`, `momentum_resolution_reason_code`, `structure_continuous`, `structure_ordinal`, `structure_resolution_reason_code`, `progression_continuous`, `progression_ordinal`, `progression_resolution_reason_code` |
| Staleness | 1 | `staleness_secs` |

The block's `open_time` is the as-of bar's own open time. It is not repeated as a separate identity column. The block carries the higher bar's candle, metrics, and resolutions, as the store and the model contract define them; the higher bar's readings are not repeated, because they remain available in Layer 2 at `(timeframe_secs, bar_index)`.

When no higher bar has closed by the base bar's close, every column of that block is null, including `staleness_secs`. The block is still present in the row: its absence is expressed by nulls, not by a missing column and not by a shorter row.

### 7.2 As-of formula

Let the base bar open at `t_b` with duration `d_b`, so it closes at `c_b = t_b + d_b`. Let the higher timeframe have duration `d_h` and let its open times be `T`. The eligible higher bars are those whose close is at or before the base close:

```text
eligible(T) = { T : T + d_h <= c_b }
T* = max(eligible(T))
staleness_secs = c_b - (T* + d_h)
```

The selected bar is the greatest closed higher bar, and it is never the containing bar while that bar is still open. `staleness_secs` is non-negative by construction. A block uses only bars closed at or before `c_b`, so appending later higher bars cannot change an earlier row.

Boundary examples for M5 higher bars (`d_h = 300`):

| Base (M1) | Base close | Latest eligible M5 bar | Staleness |
|---|---|---|---|
| 08:54 | 08:55 | 08:50–08:55 (closes 08:55) | 0 s |
| 08:59 | 09:00 | 08:55–09:00 (closes 09:00) | 0 s |
| 09:00 | 09:01 | 08:55–09:00 | 60 s |
| 09:03 | 09:04 | 08:55–09:00 (09:00–09:05 is open) | 240 s |
| 09:04 | 09:05 | 09:00–09:05 (closes 09:05) | 0 s |
| 09:05 | 09:06 | 09:00–09:05 | 60 s |
| first bar | before the first M5 close | none | null block |

The unaligned case follows the same rule. An M5 base bar opening at 09:05 closes at 09:10; an H1 bar opening at 09:00 closes at 10:00 and is open, so the latest eligible H1 is 08:00–09:00 and the staleness is 600 s.

### 7.3 Cross-timeframe aggregates

The 82 aggregate columns are published for every Layer 3 row. They are computed from the as-of blocks of that row and the M1 base, so they change only when an eligible bar changes. No aggregate is a composite across axes: each group belongs to one axis or to events.

Lists are `;`-separated with no spaces. A list field is never null; an empty list is an empty field.

| Column | Type | Range | Null | Meaning |
|---|---|---|---|---|
| `agg_ladder` | list<int> | non-empty | no | Timeframes aggregated in this row in seconds, fastest first; a higher timeframe with no closed bar yet is absent |
| `agg_direction_timeframes` | list<int> | may be empty | no | Timeframes with a valid Direction value |
| `agg_direction_timeframe_count` | int | >= 0 | no | Number of timeframes aggregated in this row |
| `agg_direction_mean` | float | -1..+1 | yes | Mean of the valid Direction values |
| `agg_direction_bullish_share` | float | 0..1 | yes | Share of valid values above +0.1 |
| `agg_direction_bearish_share` | float | 0..1 | yes | Share of valid values below -0.1 |
| `agg_direction_neutral_share` | float | 0..1 | yes | Share of valid values in the inclusive ±0.1 band |
| `agg_direction_gradient` | float | -2..+2 | yes | Slowest value minus fastest value |
| `agg_direction_ready_count` | int | >= 0 | no | Timeframes whose Direction readiness is 2 |
| `agg_momentum_timeframes` | list<int> | may be empty | no | Timeframes with a valid force |
| `agg_momentum_timeframe_count` | int | >= 0 | no | Number of timeframes aggregated in this row |
| `agg_momentum_force_mean` | float | -1..+1 | yes | Mean of the valid forces |
| `agg_momentum_acceleration_mean` | float | -1..+1 | yes | Mean of the valid accelerations |
| `agg_momentum_positive_share` | float | 0..1 | yes | Share of valid forces above +0.1 |
| `agg_momentum_negative_share` | float | 0..1 | yes | Share of valid forces below -0.1 |
| `agg_momentum_force_fast_slow_gap` | float | -2..+2 | yes | Fastest force minus slowest force |
| `agg_momentum_acceleration_fast_slow_gap` | float | -2..+2 | yes | Fastest acceleration minus slowest acceleration |
| `agg_momentum_gradient` | float | -2..+2 | yes | Slowest force minus fastest force |
| `agg_momentum_ready_count` | int | >= 0 | no | Timeframes whose Momentum readiness is 2 |
| `agg_structure_timeframes` | list<int> | may be empty | no | Timeframes with a valid structure score |
| `agg_structure_timeframe_count` | int | >= 0 | no | Number of timeframes aggregated in this row |
| `agg_structure_code_minus2_count` | int | >= 0 | no | Timeframes with `structure_code = -2` |
| `agg_structure_code_minus1_count` | int | >= 0 | no | Timeframes with `structure_code = -1` |
| `agg_structure_code_zero_count` | int | >= 0 | no | Timeframes with `structure_code = 0` |
| `agg_structure_code_plus1_count` | int | >= 0 | no | Timeframes with `structure_code = +1` |
| `agg_structure_code_plus2_count` | int | >= 0 | no | Timeframes with `structure_code = +2` |
| `agg_structure_bullish_share` | float | 0..1 | yes | Share of valid scores above +0.1 |
| `agg_structure_bearish_share` | float | 0..1 | yes | Share of valid scores below -0.1 |
| `agg_structure_neutral_share` | float | 0..1 | yes | Share of valid scores in the inclusive ±0.1 band |
| `agg_structure_state_span` | int | 0..4 | yes | Maximum minus minimum valid `structure_code` |
| `agg_structure_gradient` | float | -2..+2 | yes | Slowest score minus fastest score |
| `agg_structure_ready_share` | float | 0..1 | yes | Share of the aggregated timeframes whose Structure readiness is 2 |
| `agg_structure_conflict_timeframe_count` | int | >= 0 | no | Timeframes whose high and low relations have opposite signs |
| `agg_level_cluster_count` | int | >= 0 | no | Single-linkage level clusters, including singletons |
| `agg_largest_level_cluster_timeframe_count` | int | >= 0 | no | Distinct timeframes in the largest cluster |
| `agg_largest_level_cluster_span_atr` | float | >= 0 | yes | Price span of the largest cluster divided by its maximum ATR |
| `agg_progression_timeframes` | list<int> | may be empty | no | Timeframes with a retained leg |
| `agg_progression_timeframe_count` | int | >= 0 | no | Number of timeframes aggregated in this row |
| `agg_progression_minor_stage0_count` | int | >= 0 | no | Timeframes whose minor leg stage is 0 |
| `agg_progression_minor_stage1_count` | int | >= 0 | no | Timeframes whose minor leg stage is 1 |
| `agg_progression_minor_stage2_count` | int | >= 0 | no | Timeframes whose minor leg stage is 2 |
| `agg_progression_minor_stage3_count` | int | >= 0 | no | Timeframes whose minor leg stage is 3 |
| `agg_progression_minor_stage4_count` | int | >= 0 | no | Timeframes whose minor leg stage is 4 |
| `agg_progression_minor_stage5_count` | int | >= 0 | no | Timeframes whose minor leg stage is 5 |
| `agg_progression_minor_stage_mode` | int | 0..5 | yes | Most frequent minor stage; lowest stage wins a tie |
| `agg_progression_minor_stage_min` | int | 0..5 | yes | Minimum minor stage |
| `agg_progression_minor_stage_max` | int | 0..5 | yes | Maximum minor stage |
| `agg_progression_minor_stage_span` | int | 0..5 | yes | Maximum minus minimum minor stage |
| `agg_progression_minor_modal_stage_share` | float | 0..1 | yes | Share of valid minor legs at the modal stage |
| `agg_progression_major_stage0_count` | int | >= 0 | no | Timeframes whose major leg stage is 0 |
| `agg_progression_major_stage1_count` | int | >= 0 | no | Timeframes whose major leg stage is 1 |
| `agg_progression_major_stage2_count` | int | >= 0 | no | Timeframes whose major leg stage is 2 |
| `agg_progression_major_stage3_count` | int | >= 0 | no | Timeframes whose major leg stage is 3 |
| `agg_progression_major_stage4_count` | int | >= 0 | no | Timeframes whose major leg stage is 4 |
| `agg_progression_major_stage5_count` | int | >= 0 | no | Timeframes whose major leg stage is 5 |
| `agg_progression_major_stage_mode` | int | 0..5 | yes | Most frequent major stage; lowest stage wins a tie |
| `agg_progression_major_stage_min` | int | 0..5 | yes | Minimum major stage |
| `agg_progression_major_stage_max` | int | 0..5 | yes | Maximum major stage |
| `agg_progression_major_stage_span` | int | 0..5 | yes | Maximum minus minimum major stage |
| `agg_progression_major_modal_stage_share` | float | 0..1 | yes | Share of valid major legs at the modal stage |
| `agg_progression_active_minor_leg_count` | int | >= 0 | no | Active minor legs across the ladder |
| `agg_progression_active_major_leg_count` | int | >= 0 | no | Active major legs across the ladder |
| `agg_progression_ready_share` | float | 0..1 | yes | Share of the aggregated timeframes whose Progression readiness is 2 |
| `agg_progression_leg_direction_agreement_share` | float | 0..1 | yes | Share of active legs pointing in the most common direction |
| `agg_progression_cross_timeframe_leg_match_count` | int | >= 0 | no | Active leg pairs that match across timeframes |
| `agg_event_count_level` | int | >= 0 | no | Level events in the five-bar windows, summed over timeframes |
| `agg_event_count_divergence` | int | >= 0 | no | Divergence events in the five-bar windows |
| `agg_event_count_signal` | int | >= 0 | no | Signal events in the five-bar windows |
| `agg_event_count_level_test_retest` | int | >= 0 | no | Level subtype 1 events |
| `agg_event_count_level_cross` | int | >= 0 | no | Level subtype 2 events |
| `agg_event_count_level_break` | int | >= 0 | no | Level subtype 3 events |
| `agg_event_count_divergence_regular_bullish` | int | >= 0 | no | Divergence subtype 1 events |
| `agg_event_count_divergence_regular_bearish` | int | >= 0 | no | Divergence subtype 2 events |
| `agg_event_count_divergence_hidden_bullish` | int | >= 0 | no | Divergence subtype 3 events |
| `agg_event_count_divergence_hidden_bearish` | int | >= 0 | no | Divergence subtype 4 events |
| `agg_event_count_signal_threshold_crossing` | int | >= 0 | no | Signal subtype 1 events |
| `agg_event_count_signal_series_crossover` | int | >= 0 | no | Signal subtype 2 events |
| `agg_event_count_signal_state_change` | int | >= 0 | no | Signal subtype 3 events |
| `agg_divergence_timeframe_count` | int | >= 0 | no | Timeframes with at least one divergence in the window |
| `agg_divergence_direction_agreement_share` | float | 0..1 | yes | Largest share of eligible timeframes whose latest divergence has the same direction |
| `agg_level_event_timeframe_count` | int | >= 0 | no | Timeframes with at least one Level event in the window |
| `agg_signal_event_timeframe_count` | int | >= 0 | no | Timeframes with at least one Signal event in the window |

The model's `event_count_by_subtype.divergence` and `divergence_count_by_subtype` carry the same four values. The export publishes them once, as `agg_event_count_divergence_*`; it does not duplicate the columns. The model's `EventAggregates.timeframes` is the ladder and is published once as `agg_ladder`.

The per-timeframe event window summary is not duplicated in Layer 3. It is derived from `events.csv` with the documented rule: an event counts in a timeframe's window at a row when its `available_bar_index` is within the last five closed bars of that timeframe at the row's as-of bar.

## 8. Record files

The record files are long-form and join to the layer files on `(timeframe_secs, bar_index)`. Every bar index is 0-based and refers to the same series as Layer 1.

### 8.1 `events.csv`

One row per event record, ordered by `timeframe_secs`, then `available_bar_index`, then `event_type_code`, then `event_subtype_code`, then `source_code`, then `occurred_bar_index`. Ten columns.

| Column | Type | Range | Null | Meaning |
|---|---|---|---|---|
| `timeframe_secs` | int | > 0 | no | Source timeframe |
| `event_type_code` | int | 1..3 | no | 1 Level, 2 Divergence, 3 Signal |
| `event_subtype_code` | int | 1..4 | no | Subtype within the type |
| `source_code` | int | 1..12 | no | Reading or comparison that produced the event |
| `occurred_bar_index` | int | >= 0 | no | Bar the relationship occurred on |
| `available_bar_index` | int | >= `occurred_bar_index` | no | First closed bar it became knowable |
| `direction_code` | int | -1, 0, +1 | no | -1 bearish, 0 neutral, +1 bullish |
| `reference_code` | int | 1..3 or >= 1001 | yes | EMA reference or `1000 + n` swing level |
| `primary_value` | float | finite | yes | Main measured value at the occurrence |
| `reference_value` | float | finite | yes | Compared level or value |

An event is written once, at its availability bar, and is never upgraded in place. `available_bar_index` is never earlier than `occurred_bar_index`.

### 8.2 `legs.csv`

One row per retained leg, ordered by `timeframe_secs`, then `leg_id`. The ledger is bounded to the most recent 1024 legs per scale per timeframe; the bound is a retention rule, not a research filter, and the per-bar published leg values remain in Layers 1 and 2. Eighteen columns.

| Column | Type | Range | Null | Meaning |
|---|---|---|---|---|
| `timeframe_secs` | int | > 0 | no | Source timeframe |
| `leg_id` | int | >= 1 | no | Creation-order identifier within the timeframe |
| `scale_code` | int | 1..2 | no | 1 minor, 2 major |
| `origin_price` | float | finite | no | Origin pivot price |
| `origin_occurred_bar_index` | int | >= 0 | no | Bar the origin pivot occurred on |
| `origin_available_bar_index` | int | >= 0 | yes | Bar the origin became knowable; null while forming |
| `origin_confirmed_code` | int | 0, 1 | no | 1 when the origin pivot is confirmed |
| `leg_direction_code` | int | -1, +1 | no | +1 upward from a low, -1 downward from a high |
| `target_price` | float | finite | yes | Selected target |
| `target_basis_code` | int | 1..2 | yes | 1 structural level, 2 ATR fallback |
| `target_version` | int | >= 0 | no | Target revisions since creation |
| `target_stable_code` | int | 0, 1 | no | 1 when the target is stable |
| `progress` | float | unclamped | yes | Raw progress |
| `displacement_price` | float | finite | yes | `close - origin_price` |
| `displacement_atr` | float | finite | yes | `displacement_price / atr_at_origin` |
| `lifecycle_code` | int | 0..4 | no | 0 forming, 1 active, 2 completed, 3 invalidated, 4 superseded |
| `stage_code` | int | 0..5 | yes | Ordinal stage |
| `target_reached_code` | int | 0, 1 | yes | 1 when progress reached the target |

### 8.3 `levels.csv`

One row per confirmed level, ordered by `timeframe_secs`, then `reference_code`. Seven columns.

| Column | Type | Range | Null | Meaning |
|---|---|---|---|---|
| `timeframe_secs` | int | > 0 | no | Source timeframe |
| `reference_code` | int | >= 1001 | no | `1000 + n`, assigned in confirmation order |
| `scale_code` | int | 1..2 | no | 1 minor, 2 major |
| `kind_code` | int | 1..2 | no | 1 swing high, 2 swing low |
| `price` | float | finite | no | Swing price |
| `occurred_bar_index` | int | >= 0 | no | Bar the extreme occurred on |
| `available_bar_index` | int | >= `occurred_bar_index` | no | First bar it became knowable |

### 8.4 `consensus.csv`

One row per closed bar per timeframe, ordered by `timeframe_secs`, then `bar_index`. Six columns.

| Column | Type | Range | Null | Meaning |
|---|---|---|---|---|
| `timeframe_secs` | int | > 0 | no | Source timeframe |
| `bar_index` | int | >= 0 | no | Closed bar |
| `bullish_axis_count` | int | 0..3 | no | Direction, Momentum, and Structure values above +0.1 |
| `bearish_axis_count` | int | 0..3 | no | Values below -0.1 |
| `neutral_axis_count` | int | 0..3 | no | Values in the inclusive ±0.1 band |
| `weakest_directional_axis_code` | int | 1..3 | yes | Axis with the lowest valid size percentile; 1 Direction, 2 Momentum, 3 Structure |

Progression is excluded from the consensus. A missing axis is excluded from all three counts and is never counted as neutral. A tie for the weakest percentile is broken toward the lower axis code.

### 8.5 Raw files

One row per accepted source candle, ordered by `Date` ascending. The columns are the source format the shell reads: `Date,Open,High,Low,Close,Volume`. `Date` is `YYYY-MM-DD HH:MM:SS+00:00` in UTC; `Volume` is empty when the source supplied no volume. Raw files are the input, not a derived product: they are written by the export from the accepted source series, and their row count is the source count.

## 9. Manifest

`manifest.json` is a UTF-8, LF-terminated JSON object with a fixed key order and fixed list order. It contains no wall-clock timestamp, no host name, no absolute path, and no random value.

| Key | Type | Meaning |
|---|---|---|
| `dataset` | string | Dataset directory name |
| `data_version` | string | Caller-supplied identity from §4 |
| `model_version` | string | Workspace package version |
| `model_contract` | string | Model contract date, `2026-10-21` |
| `research_contract` | string | This contract's date, `2026-10-24` |
| `timeframes_secs` | list<int> | Configured ladder, fastest first |
| `structure_scale_code` | int | 1 minor, 2 major |
| `cap` | object | `timeframe_secs` (60) and `max_bars` (100000) |
| `rows` | object | Row count per layer and record file, keyed by file stem |
| `files` | list<object> | One entry per file, in the order of §2 |

Each `files` entry has `path` (relative to the working directory, `/` separators), `rows`, and `bytes`. The list covers the raw and layer data files; the manifest, dictionary, code table, and README are metadata and are not listed. `bytes` is the exact file size; no hash is recorded, and byte identity is verified by comparing two exports directly.

The manifest is deterministic: the same input and configuration produce the same bytes. The `rows` object lists `layer1`, `layer2`, `layer3`, and `raw`, where `raw` is an object keyed by timeframe seconds.

## 10. Data dictionary

`data_dictionary.csv` has one row per column of every file the layer export writes, in file order and then column position. It documents the dataset rather than the model: a column that appears in three layer files has three rows, because its position and file differ.

| Column | Meaning |
|---|---|
| `file` | File name, for example `layer3.csv` |
| `position` | 1-based column position within the file |
| `column` | Column name |
| `layer` | `raw`, `layer1`, `layer2`, `layer3`, or `record` |
| `axis` | `meta`, `candle`, `reading`, `direction`, `momentum`, `structure`, `progression`, `context`, `resolution`, `aggregate`, `consensus`, `event`, `leg`, or `level` |
| `type` | `int`, `float`, `string`, or `list<int>` |
| `range` | The documented range, or `finite` / `uncalibrated` for an unbounded number |
| `nullable` | `yes` or `no` |
| `reason_column` | The column carrying the reason when this column is null, or empty |
| `codes_registry` | The `codes.csv` registry for this column's codes, or empty |
| `description` | One sentence stating the quantity |

`data_dictionary.md` carries the same information as one Markdown table per file, in the same order. The two files are generated from one column registry, so they cannot disagree.

The dictionary is complete by construction: the export fails if a column in a data file has no registry entry, and it fails if a registry entry names a column that the file does not have.

## 11. Codes

`codes.csv` has one row per code and registry, ordered by registry and then code. Columns: `registry`, `code`, `label`, `description`. The registries are:

| Registry | Codes |
|---|---|
| `event_type` | 1 Level, 2 Divergence, 3 Signal |
| `event_subtype.level` | 1 test/retest, 2 cross, 3 break |
| `event_subtype.divergence` | 1 regular bullish, 2 regular bearish, 3 hidden bullish, 4 hidden bearish |
| `event_subtype.signal` | 1 threshold crossing, 2 series crossover, 3 compact state change |
| `source` | 1 EMA 20, 2 EMA 50, 3 EMA 200, 4 RSI 14, 5 ATR 14, 6 RVOL 20, 7 confirmed minor pivots, 8 confirmed major pivots, 9 EMA spread alignment, 10 swing relations, 11 RSI force, 12 leg progression |
| `unavailable_reason` | 1 insufficient bars, 2 insufficient confirmed pivots, 3 invalid denominator, 4 no target, 5 target unstable, 6 origin unconfirmed |
| `readiness` | 0 unavailable, 1 provisional, 2 ready |
| `axis` | 1 Direction, 2 Momentum, 3 Structure, 4 Progression |
| `scale` | 1 minor, 2 major |
| `pivot_kind` | 1 swing high, 2 swing low |
| `target_basis` | 1 structural level, 2 ATR fallback |
| `lifecycle` | 0 forming, 1 active, 2 completed, 3 invalidated, 4 superseded |
| `stage` | 0 below the origin, 1 `[0, 0.25)`, 2 `[0.25, 0.50)`, 3 `[0.50, 0.75)`, 4 `>= 0.75`, 5 terminal |
| `state` | -1 bearish, 0 neutral, +1 bullish |
| `structure_state` | -2..+2, `round(2 * structure_score)` |
| `relation` | -1 lower, 0 equal, +1 higher |
| `cross` | -1 downward cross, 0 no cross, +1 upward cross |
| `retest` | -1 finished below, 0 no test, +1 finished above |
| `context` | 1 expanding, 2 coiling, 3 settling, 4 high-range/low-participation |
| `confirmed` | 0 no, 1 yes |
| `event_direction` | -1 bearish, 0 neutral, +1 bullish |
| `reference` | 1 EMA 20, 2 EMA 50, 3 EMA 200, `1000 + n` confirmed swing level |
| `resolution_bucket` | 0..9, the §6.2 intervals |

The `code` field is written as a decimal integer, except the `reference` row for a confirmed swing level, whose code is the literal `1000+n`. A registry that is not used by a file is still written, so the code table is a complete reference rather than a per-dataset subset.

## 12. Determinism and validation

The export is deterministic: the same raw candles and configuration produce byte-identical files, including the manifest and the dictionary. The following rules are part of the contract:

1. Column order is fixed by this document. A new column is appended to its group; renaming, removing, or reordering a column changes the contract date and the manifest's `research_contract`.
2. Row order is fixed by §5 through §8.
3. Float formatting, null representation, list representation, and line endings follow §3.
4. No value depends on the clock, the host, the process, or the iteration order of an unordered collection. The timeframe ladder is sorted ascending.
5. No `NaN` or infinity is written. A non-finite value fails the export rather than being replaced.
6. A missing value is null with its published reason, never zero. A valid zero stays zero.
7. An ordinal code is counted or compared, never averaged. No aggregate is computed from ordinals.
8. The four axes are never combined into one score. The consensus is a count.

The export validates the dataset before publishing it:

| Check | Rule |
|---|---|
| Row counts | `layer1` = `layer2` = consensus rows; `layer3` = M1 rows |
| Bar identity | Every `(timeframe_secs, bar_index)` is unique and every `bar_index` is in `0..n-1` for its timeframe |
| Resolution range | Every continuous value is in `-1..+1` or null; every ordinal is in `0..9` or null |
| Resolution consistency | A null continuous implies a null ordinal and a non-null reason; a non-null continuous implies a null reason |
| As-of causality | Every block's higher close is at or before the base close; every `staleness_secs` is non-negative |
| Event availability | `available_bar_index >= occurred_bar_index` for every event and level |
| Leg availability | `origin_available_bar_index >= origin_occurred_bar_index` when the origin is confirmed |
| Dictionary completeness | Every column has exactly one dictionary row, and every dictionary row names a column that exists |
| Code completeness | Every code value in a data file has a row in `codes.csv` |
| 100k cap | The M1 row count is at most 100,000; a larger dataset is rejected before any file is written |

A failed render or validation writes no file. Each file is written through a temporary sibling and an atomic rename, so an interrupted write never leaves a partial file under the requested name. A file write that fails after earlier files were renamed leaves those earlier files in place; the export does not claim an all-or-nothing directory rename.

## 13. Non-goals

- Trade entries, exits, stops, targets for a trade, position sizing, and buy or sell recommendations.
- A single score formed by averaging the four axes.
- Ordinal averages, ordinal composites, or ordinal-to-continuous conversions beyond the §6.2 bucket.
- A live feed, a server, or a second surface of any kind.
- Persisting a backtest run; the store's backtest tables belong to the execution, strategy, and portfolio engines ([#10](https://github.com/quant-rule/quant-rule/issues/10), [#11](https://github.com/quant-rule/quant-rule/issues/11), [#12](https://github.com/quant-rule/quant-rule/issues/12)).
