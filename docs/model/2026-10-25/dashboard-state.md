# DashboardState — Four-Axis CLI View

- Date: 2026-10-25 (UTC)
- Issue: [#123](https://github.com/quant-rule/quant-rule/issues/123)
- Container: [#122](https://github.com/quant-rule/quant-rule/issues/122)
- Status: Approved — reviewed 2026-10-25. Implementation may proceed within the approved plan.
- Plan: [`docs/execution-plans/2026-10-25/122-market-dashboard.md`](../../execution-plans/2026-10-25/122-market-dashboard.md)
- Core model contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../2026-10-21/four-axis-analytical-model.md)
- View contract version: `2026-10-25`
- No architecture decision record is required: this is an additive view over the existing model pipeline; it does not change the pipeline stages, crate dependencies, axis formulas, or store schema.

## 1. Purpose

`DashboardState` is the model's typed, numeric projection for the terminal four-track dashboard. It makes one instrument's four timeframes legible in one snapshot: one row per timeframe, four identical tracks, with position, size, readiness, and evidence. The text renderer is the shell's presentation of the same value.

The view is descriptive. It is not a strategy, forecast, entry, exit, stop, target, confidence, probability, verdict, or recommendation.

The view does not change the existing model. `market-state`, `axis`, `events`, `legs`, `model-series`, and `market-chart` keep their contracts.

## 2. Boundary

| Layer | Responsibility |
|---|---|
| `quantrule-data` | Candle normalization, timeframe construction, ordering, gaps. Unchanged. |
| `quantrule-model` | Existing readings, axes, events, and aggregates; plus the pure `DashboardState` projection and view-only measurements. |
| `quantrule-shell` | Parse `market-dashboard`, load the CSV and ladder, call the model, render text or JSON. No axis computation. |
| Store | Unchanged. No dashboard view is persisted; no migration. |

The projection is single-instrument. There is no `instances` map, no pair identity in the view, and no `cross_pair_state`. The command's CSV identifies the one series.

## 3. Conventions

- All timestamps are UTC. The view publishes bar open times and availability bars; it never reads the wall clock.
- Every measurement is a pure function of the input candles and the explicit configuration.
- The percentile convention is the existing model convention: prior valid samples only, most recent 252, minimum 50, inclusive comparison, sorted by `f64::total_cmp`.
- A missing value is `Undefined` with an unavailable reason. It is never `0`, `NaN`, a default stage, or an origin dot.
- A valid zero is a value. A measured neutral Direction or Structure is not missing.
- Ordinal stages are snapped and never interpolated or averaged.
- A reading is not owned by an axis. The view assigns a reading to one axis for display; this does not create an extra vote.
- Numeric model fields use only numbers, nulls, lists of numbers, and codes. The shell maps codes to words.
- The view is causal: no future bar, event, pivot confirmation, or leg availability may influence an earlier view.

## 4. Envelope

```text
DashboardState
  ladder: Vec<u64>                     // ascending, the configured four timeframes
  timeframes: Vec<TimeframeState>      // one per ladder member, ascending
  cross_tf_state: CrossTfState
```

The JSON output wraps the value with provenance:

```json
{
  "model_version": "<CARGO_PKG_VERSION>",
  "view_contract": "2026-10-25",
  "dashboard_state": { "ladder": [], "timeframes": [], "cross_tf_state": {} }
}
```

The Rust type is `DashboardState`, not `ModelState`, because `MarketState` already exists.

### 4.1 Configuration

```text
DashboardConfig
  churn_threshold: u32       // default 6
  min_cross_tf_members: u8   // default 2
```

`recent_pace_window_bars` is fixed at 3 in this version; it is not a command flag. The shell uses the default configuration. Tests may vary the two config fields to verify boundaries.

## 5. Per-timeframe state

```text
TimeframeState
  timeframe_secs: u64
  present: bool
  latest_bar: Option<usize>
  latest_open_time: Option<i64>
  window_start: usize
  window_len: usize
  axes: Vec<AxisState>              // exactly four, AxisCode::ALL order
  diagnostics: DiagnosticsView
  derived: TimeframeDerived
```

- `present = false` when the ladder member has no closed bar. Every axis is `Undefined`, `latest_bar` and `latest_open_time` are null, and `window_len` is the number of loaded bars (possibly 0).
- `present = true` when the model has a closed bar for that timeframe.
- The `as-of` time is the open time of the latest closed bar. Timeframes are not synchronized; the text header prints each one.
- `axes` always has four entries, one per `AxisCode`: Direction, Momentum, Structure, Progression.

### 5.1 Diagnostics

```text
DiagnosticsView
  atr_ratio: Option<f64>
  rvol_ratio: Option<f64>
  context_code: Option<u8>          // 1 expanding, 2 coiling, 3 settling, 4 high-range/low-participation
```

Diagnostics are context only. They never vote on an axis and are never part of `AxisValue`.

### 5.2 Timeframe-derived diagnostics

```text
TimeframeDerived
  timeframe_consensus: TimeframeConsensusView

TimeframeConsensusView
  bullish_axis_count: u8
  bearish_axis_count: u8
  neutral_axis_count: u8
  weakest_directional_axis_code: Option<u8>
```

This is the existing per-timeframe `TimeframeConsensus`: Direction, Momentum, and Structure counts at the ±0.1 threshold; Progression excluded; missing is not neutral. It is not the cross-timeframe agreement in §9.2.

## 6. Axis state

```text
AxisState
  axis: AxisCode
  position: AxisValue
  size: AxisValue
  readiness: AxisReadiness
  readiness_code: u8               // 0 unavailable, 1 provisional, 2 ready
  readings: Vec<DatedReading>      // dated Event/Transition readings, oldest first, max 50
  detail: AxisDetail
```

```text
AxisValue =
  | Signed { value: f64, domain: Domain }
  | Rank { pct: f64 }
  | Ordinal { stage: Stage }
  | Undefined { reason: UnavailableReason }

Domain = DirectionalBias | Acceleration | StructureRelation
Stage = Against | Early | EarlyMid | Mid | Late | Terminal
AxisReadiness = Solid | Dashed
```

- `position` is `Signed` or `Ordinal`; it is never `Rank`.
- `size` is `Rank` or `Undefined`; it is never `Signed`.
- `readiness`: `Ready` maps to `Solid`; `Provisional` and `Unavailable` map to `Dashed`. The exact code remains in `readiness_code`.
- A missing view-only size is `—` in the text size column; it does not change the model readiness.
- `readings` holds dated Event and Transition readings. Current Level references are separate `LevelReading` values in `StructureDetail` (§6.3 and §7.1). The typed vocabulary is Level/Event/Transition; Level is a current reference, not a dated occurrence.

### 6.1 Direction

```text
DirectionDetail
  bias_state: BiasState                 // bullish, bearish, neutral, undefined
  bias_continuity: BiasContinuity       // continuous, interrupted, undefined
  bias_tenure_bars: Option<u64>
  tenure_origin_kind: TenureOriginKind  // none, first_establishment, reversal, new_origin_after_unavailable
  tenure_origin_bar: Option<usize>
  magnitude_pct: Option<f64>            // existing direction size percentile
  compact_state_age_bars: u64
  compact_state_changes_50: u64
```

- **Position:** the existing signed `direction` score, domain `DirectionalBias`.
- **Size:** the bias-tenure percentile.
- **Bias state:** bullish above +0.1, bearish below -0.1, neutral inside the inclusive corridor, undefined when the direction value is unavailable.
- **Bias tenure:** bars since the last genuine establishment or reversal, including neutral bars. The counter does not increment on an unavailable bar.
- **Neutral pause:** the origin and tenure are preserved; the current state is `neutral`; the text must say `neutral · prior bias tenure N`, never imply an active bullish or bearish bias.
- **Reversal:** a genuine `+ ↔ -` change resets the origin and sets tenure to 1.
- **Unavailable interval:** preserve the stored tenure and origin; publish `bias_continuity = interrupted` and `bias_tenure_bars = null` for the unavailable bar. On the first available bar:
  - same sign as the stored origin side: resume the stored tenure, increment by one, keep the origin;
  - opposite sign: set a new origin at the first observed bar, tenure 1, `tenure_origin_kind = new_origin_after_unavailable`; the actual reversal time during the gap is unknown and is never backdated.
- **Percentile:** prior valid tenure samples only, 252 cap, minimum 50, inclusive ties. Fewer than 50 samples → `Undefined`; the raw tenure may still be shown.
- **Detail:** `magnitude_pct` is the existing size percentile; `compact_state_age_bars`/`compact_state_changes_50` are the model's compact-state counters and are not bias tenure.

### 6.2 Momentum

```text
MomentumDetail
  force: Option<f64>
  acceleration: Option<f64>
  intensification: Option<f64>
  compact_state_age_bars: u64
  compact_state_changes_50: u64
```

- **Position:** the existing signed `momentum.acceleration`, domain `Acceleration`.
- **Size:** the existing percentile of absolute `momentum.force`.
- **Readiness:** the model's existing readiness.
- **Detail:** force, acceleration, intensification, compact-state counters.
- If acceleration is unavailable but force is available, the position is `Undefined`; the dot has no position. The size may still be present. The model readiness is used unchanged.

### 6.3 Structure

```text
StructureDetail
  score: Option<f64>
  high_relation_code: Option<i8>       // -1, 0, +1
  low_relation_code: Option<i8>        // -1, 0, +1
  scale_code: u8                       // 1 minor, 2 major
  ema_distance_atr_20: Option<f64>
  ema_distance_atr_50: Option<f64>
  ema_distance_atr_200: Option<f64>
  ema_cross_code_20: Option<i8>
  ema_cross_code_50: Option<i8>
  ema_cross_code_200: Option<i8>
  ema_retest_code_20: Option<i8>
  ema_retest_code_50: Option<i8>
  ema_retest_code_200: Option<i8>
  level_references: Vec<LevelReading>  // fixed order: EMA20, EMA50, EMA200, minor high, minor low, major high, major low; missing omitted
  confirmed_level_count: usize
  activity_count: u32
  activity_window: usize
  activity_per_bar: Option<f64>
  compact_state_age_bars: u64
  compact_state_changes_50: u64
```

- **Position:** the existing signed `structure.score`, domain `StructureRelation`.
- **Size:** the existing percentile of absolute structure score. Call it the **swing-alignment magnitude percentile**. Do not call it event density or confidence.
- **Evidence:** high/low relations, pivot scale, EMA distances/crosses/retests, current reference prices, and confirmed-level count.
- **Activity:** Structure-mapped Event readings with `available_bar_index` in the last `min(5, window_len)` bars; `activity_per_bar = activity_count / window`. A zero count is a measured zero. Activity never changes position or size.
- The score has five possible values and its absolute magnitude has three; the percentile can be stepwise. That is a property of the model, not a renderer defect.

### 6.4 Progression

```text
ProgressionDetail
  progress: Option<f64>
  stage_code: Option<i8>
  leg_direction: Option<i8>             // +1 up, -1 down
  leg_scale_code: Option<u8>            // 1 minor, 2 major
  lifecycle_code: Option<u8>            // 0 forming, 1 active, 2 completed, 3 invalidated, 4 superseded
  target_version: Option<u32>
  target_stable: Option<bool>
  origin_bar: Option<usize>
  origin_available_bar: Option<usize>
  displacement_pct: Option<f64>         // existing percentile of absolute displacement
  recent_pace_signed_atr_per_bar: Option<f64>
  recent_pace_direction: Option<PaceDirection> // with_leg, against_leg, flat
  recent_pace_window_bars: u8           // 3
  average_net_displacement_atr_per_bar: Option<f64>
  recent_pace_frozen_terminal: bool
```

- **Position:** the six ordinal anchors, mapped 1:1 from the model's stage codes: `0 against`, `1 early`, `2 early_mid`, `3 mid`, `4 late`, `5 terminal`.
- **Size:** the percentile of absolute recent pace.
- **Recent pace:** for the same confirmed published leg at `t` and `t - 3`:

```text
recent_pace_signed_atr_per_bar = leg_direction × (displacement_atr(t) - displacement_atr(t-3)) / 3
```

  `recent_pace_direction` is `with_leg` when positive, `against_leg` when negative, `flat` when zero.
- **Recent pace percentile:** percentile of `abs(recent_pace_signed_atr_per_bar)` against prior valid absolute values, 252 cap, minimum 50, inclusive ties. The radius uses this percentile. The signed pace remains a detail.
- **Same-leg rule:** both endpoints must publish the same `leg_id`; a leg change resets the measurement. A forming leg, a missing displacement, or a different leg is `Undefined`.
- **Availability:** a confirmed-leg measurement is never published before `origin_available_at`.
- **Terminal freeze:** when the lifecycle becomes terminal, freeze recent pace and average net displacement at that bar. Later bars do not decay either value. The lifecycle detail distinguishes completed, invalidated, and superseded.
- **Average historical pace:** `average_net_displacement_atr_per_bar = abs(displacement_atr) / max(1, t - origin_occurred_at)`. It measures net displacement per elapsed bar, not total path distance. It is detail only.
- **Displacement percentile:** the existing displacement percentile remains a separate magnitude detail.
- **No categorical states:** no coiling, ignition, exhaustion, or similar labels are published in this version. The text may state `with_leg`, `against_leg`, or `flat`, which are factual signs.

## 7. Readings

### 7.1 Types

```text
LevelReading
  reference_kind_code: u8              // 1 EMA20, 2 EMA50, 3 EMA200, 4 minor high, 5 minor low, 6 major high, 7 major low
  price: f64
  available_bar: usize                 // the view bar at which this current reference is knowable

DatedReading =
  | Event { event_type_code: u8, event_subtype_code: u8, source_code: u16,
            occurred_bar: usize, available_bar: usize, direction_code: i8,
            reference_code: Option<u32>, primary_value: Option<f64>,
            reference_value: Option<f64> }
  | Transition { axis: AxisCode, kind: TransitionKind, from_code: Option<i8>,
                 to_code: Option<i8>, occurred_bar: usize, available_bar: usize }

TransitionKind = compact_state | progression_stage
```

- The typed vocabulary is Level/Event/Transition. The dated history holds Event/Transition; Level is a current persistent reference.
- `LevelReading` values are EMA 20/50/200 or the latest confirmed minor/major high/low. They are point references, not price zones. They live in `StructureDetail.level_references` and are not repeated in the dated 50-item history. `available_bar` is the current view bar at which the reference is knowable.
- `Event` readings are dated occurrences from the event batch.
- `Transition` readings are compact-state changes and Progression stage changes.

### 7.2 Event-to-axis mapping

| Event | Axis |
|---|---|
| Level interaction | Structure |
| Divergence | Structure |
| Signal threshold crossing | Momentum |
| Signal EMA crossover | Structure |
| Signal compact state change | The axis identified by its source code |
| Progression stage change | Progression |

Each event appears in exactly one axis list. This is a display assignment, not a second vote.

### 7.3 Ordering and retention

- Only events with `available_bar_index <= current bar` are included.
- On the same availability and occurrence bar an Event reading precedes a Transition reading. Events sort by a fixed key `(event_type_code, event_subtype_code, source_code, reference_code, primary_value)`, with `direction_code` and `reference_value` as final tie-breakers so batch order cannot change the result.
- Keep the last 50 per axis, oldest first; trim the oldest.
- Progression stage transitions are synthesized from the per-bar stage series and are dated at the bar the stage changes; they are available at that closed bar.
- No event is upgraded in place or emitted twice.

## 8. Cross-timeframe state

```text
CrossTfState
  configured_timeframes: Vec<u64>
  present_timeframes: Vec<u64>
  aggregates: CrossTimeframeAggregates
  agreement: CrossTfAgreement
  churn: CrossTfChurn
  progression_leg_direction: Vec<LegDirectionReading>

LegDirectionReading
  timeframe_secs: u64
  direction: Option<i8>                // +1 or -1 when a published leg exists
```

### 8.1 Aggregates

The existing `CrossTimeframeAggregates` is carried unchanged: Direction, Momentum, Structure, Progression, and Events. Every aggregate keeps its source timeframes. Ordinal codes are counted, never averaged.

### 8.2 Cross-timeframe agreement

```text
CrossTfAgreement
  of: u8                               // 3
  agreeing_axes: u8
  axes: Vec<AxisAgreement>
  coverage: Coverage                   // present / configured
  min_valid_timeframes: u8             // default 2

Coverage
  present: u32
  configured: u32

MajorityClass = bullish | bearish | neutral

AxisAgreement
  axis: AxisCode                       // Direction, Momentum, Structure
  valid_timeframes: u8
  bullish_count: u8
  bearish_count: u8
  neutral_count: u8
  majority: Option<MajorityClass>      // bullish, bearish, neutral
  majority_share: Option<f64>          // majority_count / valid_timeframes
  agrees: bool
  insufficient_coverage: bool
```

- Direction, Momentum, and Structure only; Progression is excluded and its leg directions are published separately.
- Bullish > +0.1; bearish < -0.1; neutral inside the inclusive ±0.1 corridor.
- A strict majority requires one class to have more than half of the valid timeframes.
- At least `min_cross_tf_members` valid timeframes are required. Fewer → `insufficient_coverage = true`, `majority = null`, `agrees = false`.
- Missing timeframes are absent and lower coverage; they are never neutral.
- A tie is not a strict majority.
- `agreeing_axes` counts axes with `agrees = true`.
- The existing per-timeframe `TimeframeConsensus` is not this field and is not re-used as if it were.

### 8.3 Churn

```text
CrossTfChurn
  max_changes_50: u32
  sources: Vec<ChurnSource>            // all ties, sorted by timeframe then axis
  threshold: u32                       // default 6
  window: u32                          // 50
  history_complete: bool
  churn_class: ChurnClass              // Building, Holding, Unstable
  coverage: Coverage

ChurnClass = Building | Holding | Unstable

ChurnSource
  timeframe_secs: u64
  axis: AxisCode
```

- `max_changes_50` is the maximum `state_changes_50` across all present axes/timeframes.
- `sources` lists every `(timeframe, axis)` tied at the maximum. If there is no code-bearing history, it is empty.
- `history_complete` requires every required timeframe to have at least 50 code-bearing bars for the compared axes.
- `Building` when history is incomplete; `Unstable` at or above the threshold; otherwise `Holding`.
- The field is churn, not a universal market-stability verdict. The text must label it `churn`.

## 9. Absence, readiness, and labels

- A missing timeframe is a `TimeframeState` with `present = false`; all four axes are `Undefined`; the text prints `— no closed bar`.
- A missing axis value is `Undefined { reason }`. The reason registry is the model's existing six codes.
- A valid zero is preserved: a neutral Direction/Structure, a zero activity count, a zero displacement, or a flat pace is a value.
- Readiness is the model's mechanical completeness, not confidence. `Ready` → solid; `Provisional`/`Unavailable` → dashed.
- The shell maps codes to labels; the model never publishes words.

## 10. Text rendering contract

- Header: window, ladder, model/view identity, and the as-of line for each timeframe.
- Cross-TF line: `cross-TF agreement <n>/3 axes have a strict majority (...) · coverage <present>/<configured>`.
- Churn line: `churn max <n> state changes (<source>) · threshold <n> · history <present>/<configured>`.
- Four identical tracks, one per axis, in Direction, Momentum, Structure, Progression order.
- One row per configured timeframe, ascending. Missing rows are present as absent.
- Track geometry is identical across axes. No axis reverses.
- Position is the horizontal coordinate. Size is a three-step radius. Readiness is a solid or hollow outline. Fill is uniform except for the readiness outline. No axis-specific color; the first release is monochrome.
- A `Rank` size is printed as a percentage; a missing size is `—`.
- An `Undefined` position has no dot and prints `—` with the reason code. It never sits at the origin.
- The text evidence line shows the axis-specific detail; the readings line shows the latest dated reading per axis or `—`.
- The exact track width, glyph set, spacing, and legend are fixed by the command reference and locked by a golden test. Any change is a command-contract decision.
- The same command, input, and configuration print the same bytes, interactively or non-interactively.

Reference layout (the renderer may use fixed spacing; this is not a second contract):

```text
market-dashboard  window=full  ladder=1m,5m,15m,1h
as-of  1m 12:34 | 5m 12:30 | 15m 12:30 | 1h 12:00
cross-TF agreement  2/3 axes have a strict majority (direction neutral, structure bullish) · coverage 4/4
churn  max 2 state changes (1h direction) · threshold 6 · history 4/4

            DIRECTION              MOMENTUM               STRUCTURE              PROGRESSION
            -1 ◄────●────► +1      decel ◄──◌──► accel    bear ◄───●───► bull    against ◄─●─► terminal
  1m        ─────●──── 71%  ●       ───◌───── 34%  ◌       ──●────── 62%  ●       ──●────── 88%  ◌
  5m        ───●────── 40%  ●       ──●────── 88%  ●       ──────●── 10%  ●       ─────●─── 12%  ●
  1h        — no closed bar
  diag      1m coiling (atr 0.92, rvol 0.81)
  evidence  1m DIR bearish · tenure 46 (92%) · magnitude 71% | STRUCT +0.5 · HH +1 HL 0 · major · events 3/5 | PROG late · pace +0.12 with leg · speed 88% · avg net 0.08 · disp 72% · v2 stable
  readings  1m DIR transition neutral→bearish age=2 | MOM event threshold age=4 | STRUCT event level test ema20 age=0 | PROG transition mid→late age=1
```

The percentages are, in order: Direction bias-tenure percentile, Momentum force percentile, Structure swing-alignment percentile, Progression absolute recent-pace percentile. They are not event density, a confidence score, or a probability.

## 11. JSON contract

- One object with `model_version`, `view_contract`, and `dashboard_state`.
- `dashboard_state` is the numeric `DashboardState`; no labels, no confidence, no probability.
- Enum codes are stable:
  - `AxisCode`: `direction`, `momentum`, `structure`, `progression`.
  - `Domain`: `directional_bias`, `acceleration`, `structure_relation`.
  - `Stage`: `Against`, `Early`, `EarlyMid`, `Mid`, `Late`, `Terminal`.
  - `AxisReadiness`: `solid`, `dashed`.
  - `BiasState`: `bullish`, `bearish`, `neutral`, `undefined`.
  - `BiasContinuity`: `continuous`, `interrupted`, `undefined`.
  - `TenureOriginKind`: `none`, `first_establishment`, `reversal`, `new_origin_after_unavailable`.
  - `PaceDirection`: `with_leg`, `against_leg`, `flat`.
  - `ChurnClass`: `Building`, `Holding`, `Unstable`.
- `AxisValue` JSON:
  ```json
  {"kind":"signed","value":-0.42,"domain":"directional_bias"}
  {"kind":"rank","pct":71.5}
  {"kind":"ordinal","stage":"Late"}
  {"kind":"undefined","reason_code":6}
  ```
- Numbers use full precision. Missing is `null` or `Undefined`, never 0.
- Key order is stable. Collections are `Vec` in the order stated in this contract.
- The JSON is deterministic and has no clock, path, or environment dependency beyond `model_version`.

## 12. Invariants

1. Causality: no future bar, event, pivot, or leg availability influences an earlier view.
2. Determinism: repeated runs, reordered input timeframes, and reordered event lists produce the same output.
3. No unordered iteration: no `HashMap`/`HashSet` iteration affects output.
4. No double counting: one event appears in one axis's dated history; tenure, pace, and activity are not votes.
5. Axis independence: no cross-axis score; agreement is a count.
6. Numeric-only model output: the model publishes numbers and codes; the shell maps words.
7. Missing is not zero: `Undefined` is never replaced by 0 or NaN.
8. Ordinals are snapped, counted, and never averaged.
9. Readiness is mechanical completeness, never confidence.
10. The view is single-instrument; no pair map or cross-pair state.

## 13. Edge cases

- Empty input or no closed timeframe: all rows absent; no fabricated axis values.
- Single candle: positions may be undefined; sizes undefined; the dashboard still prints.
- Short history: positions may be available while sizes are `Undefined`; the radius is `—`.
- Unaligned timeframe boundary: each timeframe's latest closed bar is independent; the as-of line shows it.
- Gap: no special view behavior; no future fill.
- Zero volume: context ratios may be unavailable; axes are unchanged.
- Equal highs or lows: relation code 0 is a measured zero; the structure score is 0.0, not undefined.
- Direction neutral pause: tenure preserved, state neutral, text explicit.
- Direction unavailable interval: continuity interrupted; opposite return starts a new origin at the first observed bar.
- Progression leg change: recent pace resets; old terminal pace is frozen; new pace is undefined until the same-leg 3-bar window exists.
- Terminal lifecycle: completed, invalidated, and superseded freeze pace and remain distinguishable.
- Fewer than 50 percentile samples: size undefined; raw detail may remain.
- Missing timeframe: absent, not neutral; coverage lowered.
- Agreement tie or insufficient coverage: no majority; `agrees = false`.
- Churn history incomplete: `Building`; sources may be empty.

## 14. Deliberate relationship to the external pasted model

This view uses the QuantRule model's meanings, not a verbatim copy of the pasted external four-axis model:

- Structure position is the confirmed swing relation, not net event bias. Event activity is a text detail.
- Structure size is the swing-alignment magnitude percentile, not event density.
- Direction size is bias tenure, a view-only role; the existing magnitude percentile stays in detail.
- Progression has six ordinal anchors, including `against`; the external model had five.
- Progression size is absolute recent pace, a view-only role; the existing displacement percentile stays in detail.
- Categorical coiling/ignition/exhaustion states are not published.
- The projected type is `DashboardState`, not `ModelState`.
- There is no multi-pair envelope or `cross_pair_state`.

These are deliberate, approved decisions. They do not change the existing axis contract.

## 15. Verification requirements

The implementation must cover:

- Unit tests for all view types, mappings, readiness, reasons, and absence.
- Direction tenure: neutral pause, reversal, unavailable freeze, gap return, percentile window and ties.
- Progression pace: six anchors, signed/absolute pace, same-leg, leg change, forming leg, terminal freeze for completed/invalidated/superseded, average net displacement, displacement percentile.
- Structure activity: zero and non-zero, never changes position/size; evidence fields.
- Readings: total mapping, 50 cap, availability, transition, ordering.
- Agreement: strict majority, tie, neutral majority, minimum valid timeframes, missing not neutral, coverage.
- Churn: max and sources, threshold boundary, history incomplete, threshold parameter.
- Causality: earlier view unchanged after future bars/events; `origin_available_at` gate.
- Determinism: repeated runs, reordered timeframes/events, no unordered iteration.
- Text and JSON goldens; interactive/non-interactive byte identity.
- Edge cases from §13.
- Existing commands unchanged; no store schema change; no new dependency.

The approved execution plan's verification table is the release checklist.

## 16. Open items

None. Decisions D1–D14 are approved. This document was approved on 2026-10-25 and is the Phase 1 contract. Rust implementation may proceed within the approved plan.
