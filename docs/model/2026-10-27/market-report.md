# MarketReport — Full-Period Descriptive View

- Date: 2026-10-27 (UTC)
- Issue: [#149](https://github.com/quant-rule/quant-rule/issues/149)
- Container: [#148](https://github.com/quant-rule/quant-rule/issues/148)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Status: Approved and implemented — 2026-10-27. The A1 contract merged in [PR #153](https://github.com/quant-rule/quant-rule/pull/153) as `d887a18`; the pure view is implemented in `quantrule-model::report` by A2 (#150), and the `market-report` command layer is A3 (#151). A4 (#152) verifies the full-year run.
- Plan: [`docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md`](../../execution-plans/2026-10-27/141-market-analysis-and-backtest.md)
- Core model contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../2026-10-21/four-axis-analytical-model.md)
- Boundaries ADR: [`docs/adr/2026-10-27/analysis-and-backtest-boundaries.md`](../../adr/2026-10-27/analysis-and-backtest-boundaries.md)
- View contract version: `2026-10-27`

## 1. Purpose

`MarketReport` is the model's numeric summary of a completed model run. It answers descriptive questions over the whole requested period: how much data was covered, how often each axis was measurable, what ranges and ordinal distributions the axes produced, which events became knowable, and what the latest published state was.

The view is descriptive. It is not a strategy, entry, exit, stop, target, confidence, probability, verdict, or recommendation. It publishes no order, fill, position, trade, equity, or P&L field. `market-report` is the only command that renders this view, and it never creates a trade.

The view does not change the existing model or commands. `market-state`, `axis`, `events`, `legs`, `model-series`, `market-dashboard`, and `market-chart` keep their contracts.

## 2. Boundary

| Layer | Responsibility |
|---|---|
| `quantrule-data` | Candle normalization, timeframe construction, ordering, gaps. Unchanged. |
| `quantrule-model` | Existing readings, axes, events, and aggregates; plus the pure `MarketReport` projection over `ModelRun`. |
| `quantrule-shell` | Parse `market-report`, load the extended range, call the view, render `summary.txt`, `report.json`, and the two PNGs. No statistic is computed in the shell. |
| Store | Unchanged. The report is not persisted; no migration. |
| `quantrule-backtest` | Not involved. The report is market analysis, not a backtest. |

The projection is single-instrument. The command's CSV identifies the one series. There is no pair map or cross-pair state.

## 3. Conventions

- All timestamps are UTC Unix seconds at bar open. The view never reads the wall clock.
- The view is a pure function of the `ModelRun` and the ladder it is given. It performs no I/O, uses no randomness, and reads no configuration outside them.
- A missing measurement is `null`, never `0`, `NaN`, or a default. A valid zero is a value.
- Ordinal and discrete codes are counted, never averaged. Histograms are indexed by code, and the index mapping is fixed below.
- The model publishes numbers and codes. The shell maps codes to words in `summary.txt`; the JSON carries codes and numbers only.
- The report is a summary of a completed run, not a per-bar signal. Causality is inherited from the model: every per-bar value is a function of bars at or before its bar. A report over a prefix has the same per-bar values for that prefix; only the summary counts and the chart buckets change.
- The report is deterministic: the same run produces the same `MarketReport`, the same JSON bytes, the same text bytes, and the same PNG bytes.

## 4. Envelope

The model type is `MarketReport`. The shell wraps it with provenance in `report.json`:

```json
{
  "model_version": "<CARGO_PKG_VERSION>",
  "view_contract": "2026-10-27",
  "data_version": "<--data-version value>",
  "requested_period": {
    "from": "2025-01-01",
    "to": "2025-12-31"
  },
  "market_report": { }
}
```

- `model_version` is the shell's compiled package version.
- `view_contract` is the fixed string `2026-10-27`.
- `data_version` is the caller-supplied identity of the input series. The view does not infer it from the file path or the filesystem.
- `requested_period.from` and `requested_period.to` are the exact `--from` and `--to` values, or `null` when the flag was omitted. They are not the actual coverage; the actual coverage is in `market_report.period`.
- `market_report` is the numeric view defined in §5.

The shell emits stable JSON: object keys have a fixed order, arrays are ordered as stated in this contract, numbers use full precision, and missing values are `null`.

## 5. MarketReport

```text
MarketReport
  period: ReportPeriod
  ladder: Vec<u64>                       // ascending, normalized configured ladder
  timeframes: Vec<TimeframeReport>       // one per ladder member, ascending
  cross_timeframe: CrossTimeframeAggregates
  events: EventReport
  charts: ChartReport
```

```text
ReportPeriod
  first_open_time: Option<i64>           // earliest first bar across present timeframes
  last_open_time: Option<i64>            // latest last bar across present timeframes
```

`MarketReport::compute(run: &ModelRun, ladder: &[u64]) -> MarketReport` is the only constructor. `ladder` is the configured timeframe ladder in seconds; the shell passes the four supported timeframes. The constructor sorts it ascending and removes duplicates, so the input order cannot change the output. A ladder entry with no series in `run` produces a zero-bar `TimeframeReport`; it is absent, not neutral. The constructor never fails: an empty ladder produces `ladder = []`, `timeframes = []`, null period bounds, zero event counts, and empty charts. The command layer decides whether an empty report is renderable; see §13.

## 6. Per-timeframe coverage

```text
TimeframeReport
  timeframe_secs: u64
  bars: usize
  expected_bars: usize
  missing_bars: usize
  coverage_share: Option<f64>
  first_open_time: Option<i64>
  last_open_time: Option<i64>
  axes: Vec<AxisReport>                  // exactly four, AxisCode::ALL order
  latest: Option<LatestState>
```

- `bars` is the number of closed bars in the model series for this timeframe.
- `first_open_time` and `last_open_time` are the open times of the first and last closed bars, or `null` when `bars = 0`.
- `expected_bars = (last_open_time - first_open_time) / timeframe_secs + 1` when `bars > 0`, and `0` otherwise.
- `missing_bars = expected_bars - bars`. It counts internal missing bars between the first and last closed bar. It cannot see missing bars before the first or after the last bar; the actual first and last bars are published so the reader can see the file's coverage.
- `coverage_share = bars / expected_bars` when `expected_bars > 0`, and `null` otherwise. It is a ratio in `0..=1`.
- Derived timeframes publish a bucket only when it is complete. The final incomplete bucket of the input is therefore not a closed bar: the last published bar ends before it, and `missing_bars` does not count it because it lies after the last bar. That is accurate coverage, not a defect.
- A timeframe with no bars has `bars = 0`, `expected_bars = 0`, `missing_bars = 0`, `coverage_share = null`, null first/last, four empty axis reports, and `latest = null`.

## 7. Axis statistics

```text
AxisReport
  axis: AxisCode                         // 1 Direction, 2 Momentum, 3 Structure, 4 Progression
  present_count: usize
  null_count: usize
  min: Option<f64>
  max: Option<f64>
  mean: Option<f64>
  code_counts: Vec<u32>
  code_null_count: usize
```

The primary value and code histogram per axis are fixed:

| Axis | Primary value | `present_count` | Code | Histogram length | Index |
|---|---|---|---|---|---|
| Direction | `direction.direction` | non-null primary | `direction.state_code` | 3 | `code + 1` for `-1, 0, +1` |
| Momentum | `momentum.force` | non-null primary | `momentum.state_code` | 3 | `code + 1` for `-1, 0, +1` |
| Structure | `structure.score` | non-null primary | `structure.state_code` | 5 | `code + 2` for `-2..=+2` |
| Progression | `progression.progress` | non-null primary | `progression.stage_code` | 6 | `code` for `0..=5` |

- `present_count` counts bars where the primary value is non-null; `null_count = bars - present_count`.
- `min`, `max`, and `mean` are computed over the non-null primary values. They are `null` when `present_count = 0`.
- `mean` accumulates in ascending bar order. `min` and `max` use `f64::total_cmp`. The order is fixed so the result does not depend on an unordered collection.
- `code_counts` counts bars where the code is non-null and within the documented range. `code_null_count` counts bars where the code is null. A code outside its documented range is a model defect; it is not silently folded into a bin.
- `code_counts` and `code_null_count` are independent of `present_count`: a primary value may be present while its code is null (Progression progress is available while the stage is null), and a code may be present while the primary value is null only if the model publishes such a combination. Each count describes its own field.
- Progression's primary `progress` is unclamped and may be negative or greater than one. The histogram counts the ordinal stage, never the progress value.
- An axis with no valid primary values still has a histogram entry: all-zero `code_counts` and `code_null_count = bars` when the code is null on every bar.

## 8. Latest state

`latest` is the model's complete published state at the last closed bar of the timeframe. It is `null` when the timeframe has no bars.

```text
LatestState
  bar_index: usize
  open_time: i64
  readings: ReportReadings
  direction: ReportDirection
  momentum: ReportMomentum
  structure: ReportStructure
  progression: ReportProgression
  context: ReportContext
  consensus: ReportConsensus
```

```text
ReportReadings
  ema20: Option<f64>
  ema50: Option<f64>
  ema200: Option<f64>
  rsi14: Option<f64>
  atr14: Option<f64>
  rvol20: Option<f64>

ReportDirection
  direction: Option<f64>
  state_code: Option<i8>
  size_percentile: Option<f64>
  readiness_code: u8                     // 0 unavailable, 1 provisional, 2 ready
  reason_code: Option<u8>                // 1..6, null when available
  state_age_bars: u64
  state_changes_50: u64

ReportMomentum
  force: Option<f64>
  acceleration: Option<f64>
  intensification: Option<f64>
  state_code: Option<i8>
  size_percentile: Option<f64>
  readiness_code: u8
  reason_code: Option<u8>
  state_age_bars: u64
  state_changes_50: u64

ReportStructure
  high_relation_code: Option<i8>
  low_relation_code: Option<i8>
  score: Option<f64>
  state_code: Option<i8>
  scale_code: u8                         // 1 minor, 2 major
  ema_distance_atr_20: Option<f64>
  ema_distance_atr_50: Option<f64>
  ema_distance_atr_200: Option<f64>
  ema_cross_code_20: Option<i8>
  ema_cross_code_50: Option<i8>
  ema_cross_code_200: Option<i8>
  ema_retest_code_20: Option<i8>
  ema_retest_code_50: Option<i8>
  ema_retest_code_200: Option<i8>
  size_percentile: Option<f64>
  readiness_code: u8
  reason_code: Option<u8>
  state_age_bars: u64
  state_changes_50: u64

ReportProgression
  progress: Option<f64>
  stage_code: Option<i8>
  leg_direction: Option<i8>
  leg_scale_code: Option<u8>
  size_percentile: Option<f64>
  readiness_code: u8
  reason_code: Option<u8>
  state_age_bars: u64
  state_changes_50: u64
  leg: Option<ReportLeg>

ReportLeg
  leg_id: u64
  timeframe_secs: Option<u64>
  scale_code: u8
  origin_price: f64
  origin_occurred_at: usize
  origin_available_at: Option<usize>
  origin_confirmed_code: u8
  direction_code: i8
  target_price: Option<f64>
  target_basis_code: Option<u8>
  target_version: u32
  target_stable_code: u8
  progress: Option<f64>
  displacement_price: Option<f64>
  displacement_atr: Option<f64>
  lifecycle_code: u8                     // 0 forming, 1 active, 2 completed, 3 invalidated, 4 superseded
  stage_code: Option<i8>
  target_reached_code: Option<u8>

ReportContext
  atr_ratio: Option<f64>
  rvol_ratio: Option<f64>
  context_code: Option<u8>               // 1 expanding, 2 coiling, 3 settling, 4 high-range/low-participation

ReportConsensus
  bullish_axis_count: u8
  bearish_axis_count: u8
  neutral_axis_count: u8
  weakest_directional_axis_code: Option<u8> // 1 Direction, 2 Momentum, 3 Structure
```

- `readiness_code` is the model's `Readiness::code()`: 0 unavailable, 1 provisional, 2 ready. It is mechanical completeness, never confidence.
- `reason_code` is the model's `UnavailableReason::code()`: 1 insufficient bars, 2 insufficient confirmed pivots, 3 invalid denominator, 4 no target, 5 target unstable, 6 origin unconfirmed. It is `null` when the axis is available.
- `ReportLeg` is `ProgressionBar::published_leg()`, the more recently created of the two retained scale legs. The minor and major records are not both published in the report; the per-scale records remain available through the existing `legs` command.
- `state_age_bars` and `state_changes_50` are the model's published counters, not view-only measurements.
- `ReportConsensus` is the existing per-timeframe `TimeframeConsensus`: Direction, Momentum, and Structure vote; Progression is excluded; a missing value is not counted as neutral.

## 9. Event report

The report counts every event in the full run, by type and subtype, per timeframe and overall. This is separate from `cross_timeframe.events`, which is the existing five-bar window.

```text
EventReport
  total: u32
  by_type: EventTypeCounts
  by_subtype: EventSubtypeCounts
  timeframes: Vec<TimeframeEventReport>  // ladder order

EventTypeCounts
  level: u32
  divergence: u32
  signal: u32

TimeframeEventReport
  timeframe_secs: u64
  total: u32
  by_type: EventTypeCounts
  by_subtype: EventSubtypeCounts
  latest: Option<LatestEvent>

LatestEvent
  event_type_code: u8                    // 1 level, 2 divergence, 3 signal
  event_subtype_code: u8
  source_code: u16
  occurred_bar_index: usize
  available_bar_index: usize
  direction_code: i8                     // -1, 0, +1
  reference_code: Option<u32>
  primary_value: Option<f64>
  reference_value: Option<f64>
```

`EventSubtypeCounts` is the existing aggregate type:

```text
EventSubtypeCounts
  level: [u32; 3]                        // subtypes 1..3
  divergence: [u32; 4]                   // subtypes 1..4
  signal: [u32; 3]                       // subtypes 1..3
```

`EventSubtypeCounts` arrays are zero-based: index 0 counts subtype 1, index 1 counts subtype 2, and so on. This is the existing aggregate layout; the report does not re-index it.

- Counts include every event in `TimeframeModel::events()`, in every bar of the run. They are not limited to the five-bar aggregate window.
- `timeframes` has one entry per ladder member, in ladder order. A timeframe with no series has zero counts and `latest = null`.
- An event is counted once, at its availability bar. It is never counted at its occurrence bar as well.
- `latest` is the event with the greatest availability bar, then the greatest occurrence bar, then the fixed event key in §12. It is `null` when the timeframe has no events.
- The overall `by_type` and `by_subtype` are the sums of the per-timeframe counts. They are integers; accumulation order cannot change them.
- `total` is the sum of the three type counts and must equal the sum of the subtype counts.

## 10. Cross-timeframe aggregates

`cross_timeframe` is the existing `CrossTimeframeAggregates` at the run's latest bar, carried unchanged. Its semantics are defined by the model contract and the aggregate module; this report does not redefine them.

```text
CrossTimeframeAggregates
  ladder: Vec<u64>
  direction: DirectionAggregates
  momentum: MomentumAggregates
  structure: StructureAggregates
  progression: ProgressionAggregates
  events: EventAggregates
```

The JSON uses the existing field names:

| Object | Fields |
|---|---|
| `direction` | `timeframes`, `direction_timeframe_count`, `direction_mean`, `direction_bullish_share`, `direction_bearish_share`, `direction_neutral_share`, `direction_gradient`, `direction_ready_count` |
| `momentum` | `timeframes`, `momentum_timeframe_count`, `momentum_force_mean`, `momentum_acceleration_mean`, `momentum_positive_share`, `momentum_negative_share`, `momentum_force_fast_slow_gap`, `momentum_acceleration_fast_slow_gap`, `momentum_gradient`, `momentum_ready_count` |
| `structure` | `timeframes`, `structure_timeframe_count`, `structure_code_histogram` (5), `structure_bullish_share`, `structure_bearish_share`, `structure_neutral_share`, `structure_state_span`, `structure_gradient`, `structure_ready_share`, `structure_conflict_timeframe_count`, `level_cluster_count`, `largest_level_cluster_timeframe_count`, `largest_level_cluster_span_atr` |
| `progression` | `timeframes`, `progression_timeframe_count`, `minor`, `major`, `active_major_leg_count`, `active_minor_leg_count`, `progression_ready_share`, `leg_direction_agreement_share`, `cross_timeframe_leg_match_count` |
| `progression.minor` / `major` | `stage_histogram` (6), `stage_mode`, `stage_min`, `stage_max`, `stage_span`, `modal_stage_share` |
| `events` | `timeframes`, `per_timeframe`, `event_count_by_type` (3), `event_count_by_subtype`, `divergence_count_by_subtype` (4), `divergence_timeframe_count`, `divergence_direction_agreement_share`, `level_event_timeframe_count`, `signal_event_timeframe_count` |
| `events.per_timeframe` | `timeframe_secs`, `counts_by_type` (3), `counts_by_subtype`, `latest_event_type_code`, `latest_event_subtype_code`, `latest_occurred_bar_index`, `latest_available_bar_index`, `latest_event_age_bars` |

- `events` here is the five-bar aggregate window. The full-period counts are in §9.
- `cross_timeframe` covers the timeframes present in the run. Its `ladder` may be shorter than the report's configured `ladder` when a timeframe has no bars; the per-timeframe report still lists the absent timeframe with zero bars.
- Ordinal codes are counted, never averaged. Every aggregate names the timeframes it came from.
- `progression.minor` and `progression.major` are separate; the two scales are never blended.

## 11. Chart report and layout

The full-year chart uses the H1 series, not the M1 series (D11). A deterministic bucketing rule reduces the H1 series to at most 240 buckets, so the chart covers the whole period without drawing 8,666 candles.

```text
ChartReport
  timeframe_secs: Option<u64>
  source_bars: usize
  bucket_size: usize
  bars: Vec<ReportChartBar>              // oldest first, at most 240
  events: Vec<ReportChartEvent>          // fixed event order

ReportChartBar
  first_bar_index: usize
  last_bar_index: usize
  open_time: i64
  last_open_time: i64
  open: f64
  high: f64
  low: f64
  close: f64
  volume: Option<f64>
  direction: Option<f64>
  momentum: Option<f64>
  structure: Option<f64>
  progression_stage_code: Option<i8>

ReportChartEvent
  bucket_index: usize
  available_bar_index: usize
  occurred_bar_index: usize
  event_type_code: u8
  event_subtype_code: u8
  direction_code: i8
```

- `timeframe_secs` is H1 (3600) when present, otherwise the slowest present timeframe, otherwise `null`.
- `source_bars` is the number of closed bars in the selected timeframe.
- `bucket_size = max(1, ceil(source_bars / 240))`, computed with integer arithmetic `(source_bars + 239) / 240`.
- `bars.len() = ceil(source_bars / bucket_size)`, at most 240. Bucket `i` covers source bars `[i * bucket_size, min((i + 1) * bucket_size, source_bars))`.
- For each bucket:
  - `first_bar_index` and `last_bar_index` are the absolute indices of the first and last source bar in the bucket.
  - `open_time` and `last_open_time` are their UTC open times.
  - `open` is the first bar's open; `high` is the maximum high; `low` is the minimum low; `close` is the last bar's close.
  - `volume` is the sum of the bucket's volumes when every bar has a volume, and `null` when any bar's volume is missing. A zero volume is a valid sum.
  - `direction`, `momentum`, and `structure` are the last non-null continuous primary value in the bucket. `progression_stage_code` is the last non-null stage code in the bucket. The rule is "last published value", never an average, so an ordinal code is never averaged and the bucket ends on the state the model actually published.
- A bucket with no valid axis value publishes `null` for that axis; it is a gap in the panel.
- `events` includes every event of the selected timeframe. Its `bucket_index` is `available_bar_index / bucket_size`, because the selected series starts at absolute bar 0. The event is drawn at the bucket's x position, at the bucket's last close. Markers in the same bucket may overlap; the chart is a locator, not a count. The exact counts are in §9.
- `events` is sorted by `(available_bar_index, occurred_bar_index, event_type_code, event_subtype_code, source_code, reference_code, primary_value, direction_code, reference_value)`, using `f64::total_cmp` for the two optional numbers.
- If `timeframe_secs` is `null`, `source_bars = 0`, `bucket_size = 1`, and `bars` and `events` are empty. The command refuses to render an empty chart; see §13.

### 11.1 `price-axes.png`

- 1,280 × 960 pixels, RGB PNG.
- The geometry is the existing `market-chart` geometry: price panel 72–400, volume strip 418–478, four axis panels 500–930 with 10-pixel gaps, shared x-axis labels at 938.
- The header states `market-report`, the timeframe, the source bar count, the bucket count, the first bucket's first open time, and the last bucket's last open time in UTC.
- The price panel draws the bucket OHLC as candles. The volume strip draws the bucket volume.
- The four axis panels plot Direction, Momentum, Structure, and Progression in that order. The first three use the bucket's last published values; Progression maps the stage code to six evenly spaced heights, against at the bottom and terminal at the top. A null is a gap, never a point at zero.
- The renderer copies the report's values. It does not compute an axis, an indicator, an event, or a bucket.

### 11.2 `events.png`

- 1,280 × 720 pixels, RGB PNG.
- The geometry is the existing fetch-chart geometry: price panel 72–492, volume strip 522–640, x-axis labels at 652.
- The header states `market-report events`, the timeframe, the source bar count, the bucket count, the first bucket's first open time, and the last bucket's last open time in UTC.
- A marker is drawn at the event's bucket x position, at the bucket's close price. Level is a circle, Divergence is a triangle, and Signal is a square. The marker color follows the event direction: bullish up, bearish down, neutral.
- A legend names the three event types and the three directions.
- The marker position is derived from the availability bar, never the occurrence bar. An event that is not yet knowable is not drawn.

Both charts use the existing deterministic canvas and PNG encoder. No system font, clock, locale, or unordered collection participates. The same `ChartReport` produces the same PNG bytes.

## 12. Deterministic ordering

- `ladder` and `timeframes` are ascending by `timeframe_secs`.
- `axes` is exactly four entries in `AxisCode::ALL` order: Direction, Momentum, Structure, Progression.
- Histograms are indexed by the fixed code mapping in §7.
- `events.timeframes` is in ladder order.
- Per-timeframe event counts are integer sums; order cannot change them.
- `latest` uses the greatest `(available_bar_index, occurred_bar_index, event_type_code, event_subtype_code, source_code, reference_code, primary_value, direction_code, reference_value)` with `f64::total_cmp` for the two optional numbers.
- `charts.bars` is oldest first. `charts.events` uses the same event key as `latest`.
- `mean` accumulates in ascending bar order. `min` and `max` use `f64::total_cmp`.
- JSON object keys have a fixed order. Arrays are ordered as stated in this contract.
- No `HashMap` or `HashSet` iteration affects the output. The view does not depend on the order of the input timeframes; `ModelRun` sorts them, and the constructor sorts the ladder entries.

## 13. Command contract

```text
market-report <csv> --data-version <id> [--from <date>] [--to <date>] --out <dir> [--json]
```

- `<csv>` is the M1 CSV source. The shell reads it; the model never reads a file.
- `--data-version <id>` is required and non-empty. It is copied into `report.json` and printed in `summary.txt`.
- `--from` and `--to` are inclusive UTC dates in `YYYY-MM-DD` form. `from` must not be after `to`. When both are omitted, the whole file is used. When one is given, only that bound is applied.
- The shell filters M1 candles to the requested date range before the ladder build. The model run sees only the selected candles; it does not receive pre-range warm-up. Early bars may therefore have unavailable readings. This is the same behavior as loading a file that starts at `--from`.
- `--out <dir>` is required. The directory must not exist, or must exist and be empty. A non-empty directory is rejected before any file is written.
- The command writes exactly four files, in this fixed order: `summary.txt`, `report.json`, `price-axes.png`, `events.png`.
- `--json` prints the exact `report.json` bytes to stdout. Without `--json`, stdout is the four artifact paths, one per line, in the fixed order. The files are always written.
- The command is atomic from the user's perspective: all bytes are built before the output directory is populated. A failure leaves no partial artifact file. If the directory was created by the command, a failure leaves it absent or empty.
- An empty report, or a chart with no bars, is rejected with a deterministic error and no artifacts. The error names the requested period and the reason; it does not read the clock.
- Errors go to stderr with a non-zero exit. The command never creates an order, fill, position, trade, equity, or P&L field.

### 13.1 `summary.txt`

`summary.txt` is a fixed-section text rendering of the numeric view. It is ASCII, uses LF line endings, and ends with a trailing newline. The A3 implementation fixes the exact column widths and the golden test locks the bytes; this contract fixes the sections and their order. The sections are:

1. Title, `data_version`, requested period, actual coverage, and ladder.
2. Coverage table: timeframe, bars, expected, missing, share, first, last.
3. Axis table: timeframe, axis, present, null, min, max, mean.
4. Histogram table: timeframe, axis, code counts, null count.
5. Event table: timeframe, level, divergence, signal, total, latest event type/subtype and bars.
6. Latest table: timeframe, open time, each axis's primary value and readiness code, context code, and consensus counts.
7. Artifact paths.

The text may map codes to words using the shell's stable label mapping. It must not introduce a measurement that is not in the JSON. The same report prints the same bytes.

## 14. Edge cases

- **Empty run:** an empty ladder produces an empty report with null period bounds, zero event counts, and empty charts. The command rejects it because there is no chart. A non-empty ladder over an empty run produces zero-bar timeframe entries.
- **Single candle:** coverage is 1/1, the latest state exists, most axes are unavailable with a reason, and the chart has one bar.
- **Short history:** axes may have values while percentiles are null; histograms count only the published codes; null counts are explicit.
- **Unaligned timeframe boundary:** each timeframe's first and last bar are independent; `period` is the earliest first and latest last across present timeframes.
- **Gap:** `expected_bars > bars` and `missing_bars > 0`; no value is fabricated and no future bar is read.
- **Zero volume:** volume-dependent readings and context may be null; the axes are unchanged.
- **Equal highs or lows:** a Structure score of `0.0` is a measured value; it is counted as present, not as missing.
- **Progression stage null while progress is present:** `present_count` counts the progress, `code_null_count` counts the null stage, and the histogram does not invent a bin.
- **Timeframe with no bars:** the report keeps the ladder entry with zero bars and empty axis reports; it is absent, not neutral.
- **Partial last bucket:** a derived bucket is not published until complete; the last published bar ends before it, so the coverage table does not count it and the chart ends at the last published bucket.
- **Requested range with no candles:** the command errors before the model run and writes no artifacts.
- **Over-cap input:** the extended loader rejects the file with the deterministic cap error; no artifacts are written.

## 15. Invariants

1. **Descriptive only:** no order, fill, position, trade, equity, drawdown, or P&L field exists anywhere in the report.
2. **No double counting:** an event is counted once, at its availability bar, in one type and one subtype.
3. **Axis independence:** the report summarizes axes separately; it never combines them into one score. Consensus is a count.
4. **Missing is not zero:** a null measurement stays null and is counted as null.
5. **Ordinals are counted, not averaged:** stage and state codes are counted in histograms and published as codes in the latest state and chart; they are never averaged into a continuous number.
6. **Availability, not occurrence:** events are counted and drawn at their availability bar; the chart never draws an event before it is knowable.
7. **Causality:** a value published for a given bar does not change when later bars are appended. The latest state and the chart select a different bar or buckets, but they never rewrite an earlier bar's value.
8. **Determinism:** the same run produces the same view, JSON, text, and PNG bytes.
9. **No shell computation:** the shell renders the view; it does not compute a statistic, a histogram, or an event count.
10. **No new dependency:** the view uses the existing model types and the shell's existing JSON/PNG stack.

## 16. Verification requirements

The implementation must cover:

- Unit tests for every report type and statistic: coverage, min/max/mean, histograms, null counts, event counts, latest state, chart selection.
- A golden fixture of the full `MarketReport` over a small deterministic run, comparing every field.
- A JSON golden: exact bytes, including nulls, arrays, and key order.
- A text golden: exact `summary.txt` bytes.
- A PNG golden: dimensions and byte hash for both charts, and repeated-render byte identity.
- Causality: a report over a prefix has the same per-bar values for that prefix; appending, changing, or removing later bars does not change an earlier value.
- Determinism: repeated runs, reordered timeframe inputs, and reordered event inputs produce the same report.
- Edge cases from §14.
- A schema test that the JSON contains no `order`, `fill`, `position`, `trade`, `equity`, `pnl`, `drawdown`, or `strategy` field.
- The command tests in A3: stdout, files, chart bytes, atomic failure, help, and README.

## 17. Decisions and non-goals

### 17.1 Decisions

| ID | Decision | Alternative | Consequence |
|---|---|---|---|
| R1 | The chart is the H1 series reduced to at most 240 fixed buckets, and each axis bucket carries the last published value. | A recent M15 tail. | The chart covers the whole requested period; resolution is coarser than M15 but no code is averaged. The summary and JSON still carry the full-resolution statistics. |
| R2 | `--from`/`--to` filter M1 before the ladder build; no pre-range warm-up. | Run on the full file and clip the report. | The report covers exactly the requested period; early readings may be unavailable. The full-year run is unaffected. |
| R3 | `cross_timeframe.events` is the existing five-bar window; full-period counts are in `events`. | Replace the aggregate with full-period counts. | Existing aggregate semantics stay unchanged; the report adds a separate full-period section. |

These are contract decisions within A1's approved scope. The pull request is the review gate.

### 17.2 Non-goals

- Strategy, entries, exits, orders, fills, positions, trades, equity, drawdown, or P&L.
- Persisting the report to the store or research dataset.
- Multi-instrument or cross-pair reports.
- Interactive charts, zoom, HTML, or a second output surface.
- A chart with more than 240 buckets or finer than H1 resolution.
- Changing an existing `market-*` command's contract.
- Optimizing the report pass; correctness and determinism come first.

## 18. Open items

None. The view contract, the artifact set, the chart layout, and the command behavior are fixed above. A2 implements the view; A3 implements the command; A4 verifies the SOLUSD 2025 run.
