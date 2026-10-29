# The Four-Axis Analytical Model

- Date: 2026-10-21 (UTC)
- Issue: [#14](https://github.com/quant-rule/quant-rule/issues/14) — feat: model engine, contract and implementation breakdown
- Status: Contract. The implementation follows this document.
- Related: [`docs/adr/2026-10-21/four-axis-analytical-model.md`](../../adr/2026-10-21/four-axis-analytical-model.md), [`docs/execution-plans/2026-10-21/14-model-engine.md`](../../execution-plans/2026-10-21/14-model-engine.md)

## 1. Purpose

This document is the analytical model's contract. It states the four axes, the numeric output fields, the event model, and the aggregation rules. It describes market state, not entries, exits, or trade execution: nothing here decides a trade.

The model layer publishes numbers only — continuous values, ordinal codes, and discrete codes. Words are a presentation mapping, and they live in the shell.

## 2. Design principles

1. The model always keeps four axes: Direction, Momentum, Structure, and Progression.
2. Indicators are calculation inputs, not exclusive owners of an axis. The same input may serve different operational purposes on different axes. EMAs, for example, describe directional alignment through their relative positions and structure through their use as dynamic levels.
3. An input may contribute to more than one axis, but each use has a distinct definition. The same feature is never counted twice inside one axis merely because it has multiple names.
4. The model layer publishes numeric values only. Words are presentation mappings outside the model.
5. The four axes are never combined into one universal score. Their meanings differ.
6. Computation uses closed bars and information available at that time. A pivot's occurrence time is kept separate from the time it became knowable.
7. Missing, provisional, neutral, and zero are different states. An unavailable measurement is null with an unavailable-reason code, and is never replaced by zero. A valid zero is a value.
8. The model keeps three shared operational event types: Level, Divergence, and Signal. An event type describes what happened, not which indicator produced it. Event types and subtypes are encoded numerically. There is no event lifecycle and no event-scoring pipeline.

## 3. The four axes

| Axis | Question | Primary numeric output | Supporting outputs |
|---|---|---|---|
| Direction | Which way is the market biased, and how strongly? | Continuous score [-1, +1] | EMA spreads, persistence, historical percentile |
| Momentum | How strong is directional pressure, and is it strengthening or fading? | Continuous force [-1, +1] | Change in force, intensification, historical percentile |
| Structure | How is price organised: higher/lower swings, dynamic levels, and level crossings or retests? | Discrete swing code and a signed swing score [-1, +1] | EMA distances, crossing/retest codes, confirmed levels |
| Progression | How far has a particular movement travelled relative to its reference? | Unclamped continuous progress plus an ordinal stage code | Leg direction, displacement, reference stability, lifecycle |

The axes describe different things. Direction is not acceleration. Structure is not another trend-strength score. Progression is not a probability and not a forecast of reversal.

## 4. Minimal input set

The model consumes closed OHLCV bars for each configured timeframe. The first version needs only these calculation families.

| Input | Default | Operational uses |
|---|---|---|
| Exponential moving averages | EMA 20, 50, 200 | Directional alignment; dynamic structural levels, distance, crossing, and retest |
| Relative Strength Index | RSI 14 | Momentum force and its change |
| Average True Range | ATR 14 | Scale-normalise price distances; measure displacement and volatility context |
| Confirmed swing pivots | Minor: 2 bars on each side; major: 5 bars on each side | Structural sequence, horizontal levels, and progression-leg origins |
| Relative volume | Volume / SMA20(volume) | Context only; participation is not a directional vote |

No other indicator is required. Volume is optional when the source does not provide it reliably. The model does not require MACD, Stochastic, Williams %R, a separate divergence indicator, chart-pattern catalogues, Fibonacci, SMC, or volume profile. RSI plus confirmed price swings is sufficient for the initial divergence feature.

## 5. Shared calculations

- **EMA:** `alpha = 2/(n+1)`; `EMA_t = alpha * Close_t + (1-alpha) * EMA_(t-1)`. Seed with the SMA of the first `n` closes. The value is unavailable before `n` closed bars.
- **RSI:** Wilder-smoothed average gain and loss; `RSI = 100 - 100/(1 + avg_gain/avg_loss)`. The first average gain and the first average loss are the arithmetic means of the first 14 gains and the first 14 losses, and each is then smoothed as `avg_t = (avg_(t-1) * 13 + value_t) / 14`. The value is unavailable before 15 closed bars. If both averages are zero, RSI is unavailable. If the average loss is zero and the average gain is not, RSI is 100.
- **ATR:** true range is `max(High-Low, abs(High-previous_Close), abs(Low-previous_Close))`. Wilder smoothing, seeded with the mean of the first 14 true ranges. The value is unavailable before 15 closed bars.
- **Relative volume:** `RVOL20 = Volume / SMA20(Volume)`. If the volume baseline is zero, or volume is absent, the value is null.
- **Swing high:** its high is strictly greater than the highs of the configured number of bars before and after it. A swing low is strictly lower than the lows of those bars. A pivot is knowable only when all bars on both sides have closed. If one bar qualifies as both high and low at the same pivot scale, it is discarded at that scale.
- **Pivot candidate:** a bar whose high is strictly greater than the highs of the `k` bars to its left, or whose low is strictly lower than the lows of the `k` bars to its left, where `k` is 2 for the minor scale and 5 for the major scale. A candidate is knowable at the close of its own bar, and it is not a pivot until the `k` bars to its right have closed. If one bar qualifies as both a candidate high and a candidate low at the same scale, it is discarded at that scale.

A minor pivot uses two bars on either side; a major pivot uses five. Both `pivot_occurred_at` and `pivot_available_at` are stored. A pivot is never used before `pivot_available_at`.

The formulas use only addition, subtraction, multiplication, division, comparison, and the absolute value, plus `round` and `sign` as defined. No transcendental function appears in the signal path, so the same input yields the same values under IEEE 754 binary64 with round-to-nearest. The accumulation order is fixed: an exponential average is accumulated from the oldest bar forward, a Wilder average is accumulated in bar order, and a percentile is computed over the samples in bar order after a stable sort.

**Zero denominators.** A denominator that is an ATR, or a multiple of ATR, is zero only on a flat series. When it is, the affected field is null with `unavailable_reason_code` 3, invalid denominator. A null never propagates as zero, and a null is never used as a sample in a percentile.

## 6. Operational map: one input, multiple uses

The map organises calculations by what they do. It does not force each indicator into one axis.

| Input/measurement | Direction use | Momentum use | Structure use | Progression use |
|---|---|---|---|---|
| EMA 20/50/200 | Relative EMA spreads define directional alignment | None required | Dynamic price levels; distance, crossing and retest measurements | Context only; does not define progress arithmetic |
| RSI 14 | None required | Signed force and force change | None required | None required |
| ATR 14 | Normalises EMA spreads | None required in the baseline formula | Normalises distance from EMA levels | Normalises displacement and provides a fallback target distance |
| Confirmed pivots | None required in the baseline formula | None required | Higher/lower swing sequence and horizontal levels | Origin, leg direction, and structural target candidates |
| RVOL20 | No directional vote | No directional vote | Context only | No direct effect on progress |

A calculation may be reused by several axes only through the role defined above. Direction, Momentum, and Structure are never added together to manufacture an overall market score. Any cross-axis agreement output is a count.

## 7. The event model

Three event types are shared by every source.

| `event_type_code` | Type | Operational meaning | Example subtypes (`event_subtype_code`) |
|---|---|---|---|
| 1 | Level | Price interacts with a named price reference | 1 = test/retest, 2 = cross, 3 = break |
| 2 | Divergence | Price swings and an indicator's values disagree directionally | 1 = regular bullish, 2 = regular bearish, 3 = hidden bullish, 4 = hidden bearish |
| 3 | Signal | A defined numerical condition changes | 1 = threshold crossing, 2 = series crossover, 3 = compact state change |

The type vocabulary is operational, not per-indicator. One source may emit different event types, and one event type may have different sources. An event's `source_code` identifies the measurement or comparison that produced it; it does not define the event type. Events are observations, not extra votes: they do not alter an axis score unless a separate, explicit scoring rule is added later.

### 7.1 Common event record

| Field | Numeric representation | Meaning |
|---|---|---|
| `event_type_code` | 1..3 | Level, Divergence, or Signal |
| `event_subtype_code` | integer, defined by event type | Compact subtype |
| `source_code` | integer | Stable identifier for the reading or comparison source |
| `timeframe_secs` | positive integer | Source timeframe |
| `occurred_bar_index` | integer | Bar on which the underlying relationship occurred |
| `available_bar_index` | integer | First closed bar on which the model could know the event |
| `direction_code` | -1, 0, +1 | Bearish, neutral, or bullish implication |
| `reference_code` | integer or null | Identifies the EMA or confirmed swing level for Level events |
| `primary_value` | number or null | Main measured value at the occurrence |
| `reference_value` | number or null | Compared level or value, when applicable |

An event is appended once, when it becomes knowable. It is not upgraded in place. Historical records remain immutable; the current snapshot exposes the latest event and its age in bars. No text labels are required by the model.

### 7.2 Level events

Eligible references in v1 are EMA 20/50/200 and confirmed minor and major swing levels, and the set is bounded: the three EMAs, plus the most recent confirmed swing high and the most recent confirmed swing low at each of the two scales — seven references per timeframe. A longer history is not a reference set, because an unbounded set makes the event store unbounded.

ATR scales proximity, and one predicate serves all three subtypes. A close **crosses** a level when the previous close was on one side of it and the current close is on the other side, where above means strictly greater than the level and below means strictly less.

- A test/retest occurs when the candle range intersects `level ± 0.2 * ATR14`.
- A cross occurs when consecutive closes move from one side of the level to the other.
- A break occurs when the close crosses the level and finishes at least `0.2 * ATR14` beyond it.

When more than one subtype qualifies for the same level and bar, only the highest-priority subtype is emitted: break, then cross, then test/retest. `direction_code` records the side of the level on which price finishes: +1 above, -1 below, and 0 when the close finishes exactly on the level. A level event describes price/level interaction; it is not a buy or sell instruction.

A Level event is evaluated only when ATR 14 is available at that bar. Without it the break and test predicates are undefined and the subtype precedence cannot be resolved, so no Level event is emitted; a measured ATR of zero remains a value. The proximity zone is closed: a range that touches its boundary intersects it. When more than one reference qualifies on the same bar, the events are recorded in the fixed order EMA 20, EMA 50, EMA 200, latest minor high, latest minor low, latest major high, latest major low.

The same interaction is published twice by design: the EMA cross and retest features of the Structure axis (§8.3) and the Level event record describe the same price/level contact from two directions. Neither is a vote, neither alters an axis value, and the Structure features are not added to `structure_score`.

### 7.3 Divergence events

Divergence is a derived relationship between price structure and indicator values, not a separate indicator and not a fifth axis. v1 implements RSI divergence using consecutive confirmed price pivots of the same kind. Around each pivot, use the minimum RSI within two bars either side for a price-low pivot, and the maximum RSI within two bars either side for a price-high pivot. This RSI window is evaluated only once that price pivot is knowable.

| `event_subtype_code` | Condition | `direction_code` |
|---|---|---|
| 1 | Regular bullish: P2 < P1 and R2 > R1 at lows | +1 |
| 2 | Regular bearish: P2 > P1 and R2 < R1 at highs | -1 |
| 3 | Hidden bullish: P2 > P1 and R2 < R1 at lows | +1 |
| 4 | Hidden bearish: P2 < P1 and R2 > R1 at highs | -1 |

The event is emitted only after the newer price pivot is confirmed. `occurred_bar_index` is the newer pivot's bar; `available_bar_index` is the bar on which that pivot becomes knowable. The record retains `price_delta_atr = (P2-P1)/ATR14` and `rsi_delta = R2-R1` as numeric evidence, where the ATR is the ATR14 of the newer pivot's occurrence bar. No additional support/resistance break confirmation stage is required in v1. Divergence is not inferred when the two comparable pivots or either RSI value is unavailable.

The detector runs at one confirmed-pivot scale per run; the default is the major scale, because it is the structural sequence. A run may select the minor scale instead, and the two scales are never compared with each other. The event's `source_code` is 4 (RSI 14); `reference_code` identifies the newer confirmed swing level, whose scale is recorded in the confirmed-level registry. If any bar in either two-bar RSI window lacks a value, the window is unavailable and no event is inferred. `price_delta_atr` is null when ATR 14 at the newer pivot's occurrence is unavailable or zero; the divergence itself does not depend on ATR, so the event is still emitted.

Divergence belongs to the relationship between Structure and Momentum. It is reported as a structural relationship with RSI as its source. It is never a second RSI vote, and it never changes `direction`, `momentum_force`, or `structure_score`.

### 7.4 Signal events

Signals record discrete changes in a reading, not every fluctuation in a continuous score. v1 uses:

- 1 = threshold crossing: RSI crosses 30, 50, or 70 from one side to the other.
- 2 = series crossover: one configured EMA crosses another configured EMA. A price/EMA crossing is a Level event, not a duplicated Signal.
- 3 = compact state change: a published compact Direction, Momentum, or Structure state code changes.

Each signal records its source and direction code. The numeric reading remains authoritative; a signal adds no vote. A single underlying occurrence qualifies for more than one category only when the definitions describe genuinely different relationships; otherwise the most specific event is emitted and duplicates are avoided.

The Signal detector uses three fixed condition families. RSI threshold crossings use 30, 50, and 70; a crossing is strict, so a close exactly on a threshold is not a crossing. Series crossovers use the configured EMA pairs 20/50, 50/200, and 20/200; the source is the faster EMA and the reference code is the slower EMA, and equality on the previous bar counts as a side for the next cross while equality on the current bar is not a cross. Compact state changes use the published Direction, Momentum, and Structure codes and fire only between two code-bearing bars: the first code-bearing bar, a transition from null, and a transition to null are not changes. A price/EMA crossing is a Level event and is never emitted as a Signal. Distinct signal conditions can coincide on one bar; each is stored once and they are not duplicates because each names a different defined relationship. No signal alters an axis value.

## 8. Axis calculations

`clamp(x, -1, +1)` limits a value to the indicated range.

### 8.1 Direction

```
d_short = clamp((EMA20 - EMA50) / (2 * ATR14), -1, +1)
d_long  = clamp((EMA50 - EMA200) / (4 * ATR14), -1, +1)
direction = (d_short + d_long) / 2
```

A positive value is bullish directional alignment, a negative value is bearish alignment, and a value near zero means the EMAs are close together or their alignments offset one another. The magnitude is relative EMA separation scaled by current volatility; it is not a probability.

If an EMA or ATR required by the formula is unavailable, `direction` is null. A valid score of 0 means neutral alignment, not missing data.

### 8.2 Momentum

```
momentum_force = (RSI14 - 50) / 50
momentum_acceleration = clamp((momentum_force_t - momentum_force_(t-3)) / 0.5, -1, +1)
momentum_intensification = clamp(sign(momentum_force_t) * (abs(momentum_force_t) - abs(momentum_force_(t-3))) / 0.5, -1, +1)
```

A positive force indicates positive RSI force; a negative force indicates negative force. Acceleration describes the signed change in force. Intensification above 0 means the force magnitude is strengthening in its current direction, below 0 means it is fading, and exactly 0 means the magnitude is unchanged across the comparison window. These are deterministic change measures, not statistical estimates and not confidence scores. If any required RSI value is unavailable, the affected outputs are null.

`sign(0)` is 0. A force of exactly zero therefore reports an intensification of exactly 0 whatever the comparison value was, and the change in magnitude is read from `momentum_acceleration` instead. This tie is stated so that the value is never ambiguous.

### 8.3 Structure

Structure is based primarily on confirmed price swings. EMA-derived levels are additional numeric context; they do not replace the swing sequence.

The scale is a configuration value of the model run, and the default is the major scale, because the major swing sequence is the structural one. `structure_scale_code` publishes the scale in force — 1 = minor, 2 = major — and the fields below are computed at that scale.

Comparing the latest two confirmed swing highs and the latest two confirmed swing lows at that scale:

```
high_relation_code = +1 if the latest high is higher, 0 if equal, -1 if lower
low_relation_code  = +1 if the latest low is higher,  0 if equal, -1 if lower
structure_score = (high_relation_code + low_relation_code) / 2
structure_code  = round(2 * structure_score)
```

The two relation codes are distinct measurements of distinct price relations, and each is counted once. `structure_score` is a defined summary of the pair, published as a discrete-valued continuous number; it is not an average of a code across timeframes or across axes, and it is never compared with a code from another scale.

| `structure_code` | Meaning |
|---|---|
| +2 | Higher high and higher low |
| +1 | One higher relation and one unchanged relation |
| 0 | Mixed or offsetting relations, or both unchanged |
| -1 | One lower relation and one unchanged relation |
| -2 | Lower high and lower low |

If the latest high or low comparison is unavailable, `structure_score` is null; a swing state is never inferred from an incomplete pair. `high_relation_code` and `low_relation_code` are retained when each is individually available.

For each EMA period `n` in {20, 50, 200}:

```
ema_distance_atr_n = (Close - EMA_n) / ATR14
```

`ema_cross_code_n` is +1 when the close crossed from at or below the EMA to above it, -1 when it crossed from at or above to below, and 0 when no cross occurred on the bar.

`ema_retest_code_n` follows a retest zone, which exists when the bar range intersects `EMA_n ± 0.2 * ATR14`: +1 when the zone was tested and the close finishes above the EMA, -1 when the zone was tested and the close finishes below, and 0 when no qualifying test occurred.

These fields let an EMA have a structural role as a dynamic level as well as a directional role as a relative spread. They are not extra independent votes added to `structure_score` without a separately specified aggregation rule.

### 8.4 Progression

Progression belongs to a specific structural leg, not to the instrument in the abstract. Separate major and minor legs are maintained per timeframe.

**Leg creation.** A leg is created from a pivot *candidate*: a bar whose extreme is strict against the `k` bars to its left at that scale, where `k` is 2 for the minor scale and 5 for the major scale. A candidate is not yet a pivot, so the leg it creates is in lifecycle 0, forming, and `origin_confirmed_code` is 0. When the `k` bars to its right close and the candidate is still a strict extreme, the pivot is confirmed: the leg moves to lifecycle 1, `origin_confirmed_code` becomes 1, and `origin_available_at` is the bar on which the confirmation became knowable. A candidate that fails to confirm is discarded, unless price has closed beyond its origin against the leg by more than `0.2 * ATR_at_origin`, in which case the leg is invalidated. A bar that qualifies as a candidate high and a candidate low at the same scale is discarded at that scale, exactly as a bar that qualifies as both a confirmed swing high and a confirmed swing low is.

The candidate is a reading in its own right (§5), so the leg logic never recomputes it and the candidate and the confirmed pivot can never disagree.

This is why a leg can exist before its origin is confirmed. A leg created from a confirmed pivot would be confirmed the instant it was created, and lifecycle 0 and unavailable-reason 6 would be unreachable.

**Leg transitions.** A confirmed pivot of the opposite kind at the same scale completes the active leg — lifecycle 2 — and starts a new leg. A confirmed pivot of the same kind supersedes the active leg — lifecycle 4 — and starts a new leg. A close beyond the origin against the leg by more than `0.2 * ATR_at_origin` invalidates the leg — lifecycle 3.

**The published leg.** A timeframe publishes one leg: the most recently created retained leg whose origin is confirmed, across both scales. A newer candidate whose origin is not confirmed does not displace a confirmed leg, because its origin may never exist and its progress would describe a pivot that can disappear. Before the first confirmation, the timeframe publishes the most recently created retained leg, so the axis still reports the forming candidate and lifecycle 0 remains reachable. A retained leg is forming, active, or terminal; a terminal leg is published with its measurements frozen at the bar it ended, until a newer confirmed leg is created. Its scale is published as `progression_leg_scale_code` and its direction as `progression_leg_direction`, which is the leg record's own `leg_direction_code` carried into the per-timeframe output. The full record of the published leg is published as `published_leg`; the full record of the leg at each scale is published as well (§9), so a forming candidate and the other scale's leg stay inspectable and the two are never blended.

Each leg stores numeric fields for:

- `leg_id`, `timeframe_secs`, `scale_code` (1 = minor, 2 = major)
- `origin_price`, `origin_occurred_at`, `origin_available_at`, `origin_confirmed_code`
- `leg_direction_code` (+1 = upward leg from a confirmed low, -1 = downward leg from a confirmed high). The leg's direction is its own field, distinct from an event's `direction_code` (§7.1), which carries an implication rather than a leg's orientation
- `target_price`, `target_basis_code`, `target_version`, `target_stable_code`
- `progress`, `displacement_price`, `displacement_atr`
- `lifecycle_code`, `progression_stage_code`, `target_reached_code`

**Target policy.** At leg creation, the nearest confirmed structural level in the leg's direction that lies beyond the origin is chosen. The candidates are the confirmed swing levels of the same scale: for an upward leg, the nearest confirmed swing high above the origin; for a downward leg, the nearest confirmed swing low below it. If no such level exists, the fallback target is `origin_price + leg_direction_code * 2 * ATR_at_origin`. The target and its basis code are stored. The target is a reference for measuring progress, not an entry, exit, stop, or guarantee of completion. `target_basis_code` is 1 when a confirmed structural level was used and 2 when the ATR fallback was used. `origin_confirmed_code` and `target_stable_code` are 0/1 values.

If the selected target is revised after a newly confirmed or invalidated structural level, `target_price` is updated and `target_version` incremented. `target_stable_code` is set to 0 on the revision, and set back to 1 only after the target has remained unchanged for three closed bars. Stage promotion is suppressed while the target is unstable. A target revision may change the calculated progress without any price movement; price displacement and `target_version` distinguish the two.

**Progress arithmetic.**

```
progress = (Close - origin_price) / (target_price - origin_price)
displacement_price = Close - origin_price
displacement_atr = (Close - origin_price) / ATR_at_origin
```

Progress is not clamped. It can be negative, when price moved against the leg, or greater than one, when price moved beyond the reference. The target distance is the absolute distance `abs(target_price - origin_price)`, so the test is the same for an upward and a downward leg. If the target equals the origin, or that absolute distance is below `0.2 * ATR_at_origin`, `progress` is null: with reason 3 when the target degenerates against the origin, and with reason 4 when there is no usable target at all.

**Leg lifecycle.**

| `lifecycle_code` | Meaning |
|---|---|
| 0 | forming — the origin is not yet confirmed |
| 1 | active |
| 2 | completed — an opposing swing at the same scale is confirmed |
| 3 | invalidated — price closes beyond the origin against the leg by more than `0.2 * ATR_at_origin` |
| 4 | superseded — a newer confirmed origin replaces the active leg at the same scale |

**Progression stage.**

| `progression_stage_code` | Meaning |
|---|---|
| 0 | progress < 0 — price is against the leg |
| 1 | 0 <= progress < 0.25 |
| 2 | 0.25 <= progress < 0.50 |
| 3 | 0.50 <= progress < 0.75 |
| 4 | progress >= 0.75 — late; includes reaching or exceeding the target |
| 5 | terminal lifecycle — completed, invalidated, or superseded |

A terminal lifecycle has precedence and sets stage 5. Otherwise the stage is null if the origin is unconfirmed, the reference is unavailable or degenerate, or the target is unstable. Reaching the target sets `target_reached_code = 1` but does not by itself complete the leg. Stage codes are never averaged.

## 9. Numeric output contract

Each timeframe publishes the same output shape. Field names describe the quantity; every model-layer value is numeric, null, or a list or record of numeric values. Display strings are mapped outside the analytical model. The quantities below are the published fields; anything a section of this document uses and does not list here is an internal reading, not a published field.

**Axis values.**

| Field | Type/range | Meaning |
|---|---|---|
| `direction` | continuous [-1, +1] | Directional alignment |
| `direction_state_code` | integer -1, 0, +1 | Compact Direction state: -1 below -0.1, 0 from -0.1 through +0.1, +1 above +0.1 |
| `momentum_force` | continuous [-1, +1] | Signed momentum force |
| `momentum_acceleration` | continuous [-1, +1] | Signed change in force |
| `momentum_intensification` | continuous [-1, +1] | Whether force is strengthening or fading in its current direction |
| `momentum_state_code` | integer -1, 0, +1 | Compact Momentum state, the same bucketing applied to `momentum_force` |
| `structure_score` | discrete-valued continuous [-1, +1] | Score derived from swing relations |
| `structure_code` | integer [-2, +2] | Compact swing-state code |
| `structure_scale_code` | integer 1..2 | Pivot scale the structural fields were computed at |
| `high_relation_code` | integer -1, 0, +1 | Relation of the latest two confirmed swing highs |
| `low_relation_code` | integer -1, 0, +1 | Relation of the latest two confirmed swing lows |
| `ema_distance_atr_20`, `ema_distance_atr_50`, `ema_distance_atr_200` | continuous | (Close - EMA_n) / ATR14 |
| `ema_cross_code_20`, `ema_cross_code_50`, `ema_cross_code_200` | integer -1, 0, +1 | Cross of the close through the EMA |
| `ema_retest_code_20`, `ema_retest_code_50`, `ema_retest_code_200` | integer -1, 0, +1 | Retest of the EMA zone |
| `progress` | continuous, unclamped | Fraction of origin-to-reference displacement |
| `progression_stage_code` | ordinal integer 0..5 or null | Stage of the published leg |
| `progression_leg_direction` | integer -1, +1 | Direction of the published leg |
| `progression_leg_scale_code` | integer 1..2 | Scale of the published leg |

**Per-axis diagnostics.** Every axis carries its own readiness, its own unavailable reason, its own state age, and its own change count, because each axis can be unavailable for a different reason at the same bar.

| Field | Type/range | Meaning |
|---|---|---|
| `direction_size_percentile` | continuous 0..100 or null | Magnitude of the Direction score against its own recent history |
| `momentum_size_percentile` | continuous 0..100 or null | Magnitude of the momentum force against its own recent history |
| `structure_size_percentile` | continuous 0..100 or null | Magnitude of the structure score against its own recent history |
| `progression_size_percentile` | continuous 0..100 or null | Magnitude of the leg's displacement against its own recent history |
| `direction_readiness_code` | discrete 0..2 | 0 = unavailable, 1 = provisional, 2 = ready |
| `momentum_readiness_code` | discrete 0..2 | 0 = unavailable, 1 = provisional, 2 = ready |
| `structure_readiness_code` | discrete 0..2 | 0 = unavailable, 1 = provisional, 2 = ready |
| `progression_readiness_code` | discrete 0..2 | 0 = unavailable, 1 = provisional, 2 = ready |
| `direction_unavailable_reason_code` | integer or null | Reason the Direction axis cannot publish its value |
| `momentum_unavailable_reason_code` | integer or null | Reason the Momentum axis cannot publish its value |
| `structure_unavailable_reason_code` | integer or null | Reason the Structure axis cannot publish its value |
| `progression_unavailable_reason_code` | integer or null | Reason the Progression axis cannot publish its value |
| `direction_state_age_bars` | non-negative integer | Closed bars the Direction compact state has held |
| `momentum_state_age_bars` | non-negative integer | Closed bars the Momentum compact state has held |
| `structure_state_age_bars` | non-negative integer | Closed bars the Structure compact state has held |
| `progression_state_age_bars` | non-negative integer | Closed bars the Progression compact state has held |
| `direction_state_changes_50` | non-negative integer | Direction compact state changes in the last 50 closed bars |
| `momentum_state_changes_50` | non-negative integer | Momentum compact state changes in the last 50 closed bars |
| `structure_state_changes_50` | non-negative integer | Structure compact state changes in the last 50 closed bars |
| `progression_state_changes_50` | non-negative integer | Progression compact state changes in the last 50 closed bars |

**Context.** These are the diagnostic quantities of §12, published per timeframe and read by no axis.

| Field | Type/range | Meaning |
|---|---|---|
| `atr_ratio` | continuous or null | ATR14 / SMA20(ATR14) |
| `rvol_ratio` | continuous or null | RVOL20 |
| `context_code` | integer 1..4 or null | Volatility and participation bucket |

**Records.** Alongside the fields above, each timeframe publishes the full record of the published leg and of the leg at each scale (§8.4), the confirmed-level registry (§8.3), the event records (§7.1), and the per-timeframe event window counts (§10). These are records of numeric values, not strings.

The progression ledger exposed for inspection retains the most recent 1024 legs per scale; the per-scale published record is always the most recent retained leg. The bound keeps the per-bar state machine linear on a long series and does not change the published record. The batch event series may expose every computed event to a consumer that needs the full record; the 1024-record retention bound applies to the per-timeframe live history, and the published five-bar window is always computed from the retained records.

The engine computes a per-bar series for every published field, aligned to the input length, and the published output is the latest bar's snapshot of that series. The snapshot is what the shell renders; the series is what the causality and determinism tests compare.

`size_percentile` is the empirical percentile of the absolute primary magnitude against the preceding 252 valid bars at the same timeframe. It is computed as `100 * count(samples <= value) / n`, where the samples are the up to 252 most recent valid magnitudes strictly before the current bar, in bar order, and `n` is their count. The comparison is inclusive, so a value equal to its history ranks at the upper end. If `n` is below 50, the value is null. The magnitude per axis is the absolute Direction score, the absolute momentum force, the absolute structure score, and, for Progression, `abs(displacement_atr)` rather than the ordinal stage.

`readiness_code` is mechanical completeness, not confidence. A neutral zero is a valid output. Missing data is represented by null and, where useful, an `unavailable_reason_code`. An axis is ready (2) when its primary value is available and its reference is stable. It is provisional (1) when its primary value is available but the axis is not yet complete: the reference is unstable — a progression leg whose target is not yet stable, or whose origin is not yet confirmed — or there are too few valid samples for the percentile. It is unavailable (0) when its primary value is null, in which case the reason code says why.

`state_age_bars` and `state_changes_50` count compact state codes. Direction and Momentum use their published compact state codes, Structure uses `structure_code`, and Progression uses `progression_stage_code`. Changes are counted in these codes, not in every small continuous-value fluctuation. A change is counted on the bar where the compact code differs from the previous bar's code, over the last 50 closed bars. While the compact code is null — an unconfirmed origin, an unstable target, an unavailable reading — the counters hold their last values, and the axis reports readiness 0 or 1. The first closed bar that carries a code has age 1; each later bar with the same code increments the age, and a change resets it to 1. A null bar advances neither the age nor the change window, so `state_changes_50` counts changes over the last 50 closed bars that carried a code.

There is no universal confidence field and no string-valued state label in the model output.

## 10. Cross-timeframe aggregation

An ordered timeframe ladder runs from fastest to slowest. Each timeframe is calculated independently first. Only fields with the same defined meaning are aggregated, and every aggregate keeps its source timeframes available so it can be traced back to its inputs.

**Direction.** `direction_mean` (the arithmetic mean of valid Direction scores), `direction_bullish_share` (score > +0.1), `direction_bearish_share` (score < -0.1), `direction_neutral_share` (abs(score) <= 0.1), `direction_gradient` (slowest-timeframe score minus fastest-timeframe score), `direction_ready_count`, and `direction_timeframe_count`. The mean is allowed because Direction uses the same signed scale on every timeframe.

**Momentum.** `momentum_force_mean`, `momentum_acceleration_mean`, `momentum_positive_share` (force > +0.1), `momentum_negative_share` (force < -0.1), `momentum_force_fast_slow_gap`, `momentum_acceleration_fast_slow_gap`, `momentum_gradient`, `momentum_ready_count`, and `momentum_timeframe_count`. All are numeric; none is a probability. Momentum publishes no neutral share, so the shares do not sum to one: a timeframe whose force is inside ±0.1 is in neither. `momentum_gradient` is the slowest minus the fastest of `momentum_force`, the same convention as `direction_gradient` and `structure_gradient`. The two gap fields are the fastest minus the slowest, so `momentum_gradient` is the negative of `momentum_force_fast_slow_gap`; both are published because the specification names both.

**Structure.** `structure_code_histogram` for codes -2..+2, `structure_bullish_share` (`structure_score` > +0.1), `structure_bearish_share` (`structure_score` < -0.1), `structure_neutral_share` (abs(`structure_score`) <= 0.1), `structure_state_span` (maximum minus minimum valid `structure_code`), `structure_gradient` (slowest `structure_score` minus fastest `structure_score`; a comparison between two timeframes, never a consensus and never an average of the codes), `structure_ready_share` (a share rather than a count, because the Structure axis is compared across timeframes as a state and not as a signed score), `structure_conflict_timeframe_count` (the count of timeframes with conflicting bullish and bearish states), `level_cluster_count`, `largest_level_cluster_timeframe_count`, and `largest_level_cluster_span_atr`. The shares use the same ±0.1 threshold as the cross-axis consensus, and `structure_code` is used for the histogram and the state span.

Two levels belong to the same cluster when their price separation is no greater than `0.5 * max(ATR_i, ATR_j)`. The levels that take part are the same bounded set the Level events use (§7.2), and the clusters are formed across timeframes by single linkage: two levels are linked when they satisfy the separation rule, and a cluster is a connected set of linked levels. `largest_level_cluster_span_atr` is the highest level price minus the lowest, divided by the maximum ATR among the timeframes represented in that cluster. Single linkage can chain levels that are far apart, which is why the span is published: the measures are numeric, and no qualitative convergence label is published.

**Progression.** The progression aggregates are computed per scale, so that a minor counter-move inside a major leg is never blended into the major leg's stage distribution. For each scale, in {minor, major}: `progression_stage_histogram` for codes 0..5, `progression_stage_mode`, `progression_stage_min`, `progression_stage_max`, `progression_stage_span`, and `progression_modal_stage_share` (the share of valid legs at the modal stage). Across scales: `active_major_leg_count`, `active_minor_leg_count`, `progression_ready_share`, `leg_direction_agreement_share` (the share of valid active legs pointing in the most common direction), and `cross_timeframe_leg_match_count` for legs with the same direction, origin timestamps within one bar of the slower timeframe, and origin prices within `0.5 * max(ATR_i, ATR_j)`.

**Events across timeframes.** For each timeframe, counts over the most recent five closed bars are published for each of the three event types and each configured subtype, alongside the latest event's `occurred_bar_index`, `available_bar_index`, and `latest_event_age_bars`. Across timeframes: `event_count_by_type`, `event_count_by_subtype` over the same five-bar per-timeframe window, `divergence_count_by_subtype` for regular and hidden bullish and bearish divergence, `divergence_timeframe_count`, `divergence_direction_agreement_share` (the largest share of eligible timeframes whose latest divergence has the same direction; null when no timeframe has a recent divergence), `level_event_timeframe_count`, and `signal_event_timeframe_count`. Event codes are never averaged, and an event is never treated as an independent indicator vote.

The share denominators are the valid values, so a missing axis is never counted as neutral; `structure_ready_share` and `progression_ready_share` use all configured timeframes. A gradient uses the configured fastest and slowest timeframes, and is null when either endpoint is unavailable. Level clusters use references with a valid ATR; the largest cluster is chosen by distinct timeframe count, then span. The stage histogram uses the retained leg at each scale; the stage mode breaks a tie toward the lower stage. A cross-timeframe leg match requires the same scale, the same direction, origin times within one slower-timeframe bar, and origin prices within `0.5 * max(ATR_i, ATR_j)`. An event counts in a timeframe's five-bar window when its `available_bar_index` is in the last five closed bars; the latest-event age is measured from its availability bar.

## 11. Cross-axis consensus and stability

A consensus diagnostic is a count, never a combined score. It is computed across the timeframes of the ladder, and the per-timeframe axis values are its inputs. For `direction`, `momentum_force`, and `structure_score`, a common sign threshold of ±0.1 applies:

- `bullish_axis_count` — the number above +0.1.
- `bearish_axis_count` — the number below -0.1.
- `neutral_axis_count` — the number in [-0.1, +0.1].
- `weakest_directional_axis_code` — the numeric identifier of the lowest valid size percentile among Direction, Momentum, and Structure. The percentiles are the per-axis percentiles of §9, each measured against its own axis's recent history. This is weakest relative to that axis's own history, not universally weakest in absolute terms. An axis with no valid percentile is excluded from the comparison, and the code is null when no axis has one.

Progression is excluded from the directional consensus, because it describes the state of a named leg rather than an independent market bias. `progression_leg_direction` is published separately, as a field of §9, so consumers can compare it with the consensus without treating the ordinal stage as a score.

The consensus is published per timeframe, so each published state carries the counts for its own three directional axes. A missing axis is excluded from all three counts. `weakest_directional_axis_code` breaks a percentile tie toward the lower axis code. Progression is excluded from the counts; `progression_leg_direction` remains published on the Progression axis.

Stability is reported through `state_age_bars` and `state_changes_50` per axis. Higher change counts mean the compact state has switched more often; they are not confidence values. A missing axis does not count as neutral.

## 12. Context outside the four axes

A small volatility and participation diagnostic is maintained, and it never votes on an axis:

```
atr_ratio  = ATR14 / SMA20(ATR14)
rvol_ratio = RVOL20
```

| `context_code` | Condition |
|---|---|
| 1 | expanding — `atr_ratio >= 1` and `rvol_ratio >= 1` |
| 2 | coiling — `atr_ratio < 1` and `rvol_ratio < 1` |
| 3 | settling — `atr_ratio < 1` and `rvol_ratio >= 1` |
| 4 | high-range/low-participation — `atr_ratio >= 1` and `rvol_ratio < 1` |

These are descriptive threshold buckets, not predictions. The code is null if either ratio is unavailable. `atr_ratio` is null when fewer than 20 ATR values are available or when SMA20(ATR14) is zero; `rvol_ratio` is null when volume or the volume baseline is unavailable. The raw ratios are retained even when `context_code` is null. The code is a compact encoding of the two inputs; consumers retain the underlying values.

## 13. Explicit exclusions

- Trade entries and exits, stops, targets for a trade, position sizing, and automatic buy or sell recommendations.
- A single score formed by averaging the four axes.
- Duplicate votes from correlated indicators.

## 14. Code registries

The implementation uses these numeric registries. A code is stable once published; a retired code is not reused.

**Event types.** 1 = Level, 2 = Divergence, 3 = Signal.

**Event subtypes.** Level: 1 = test/retest, 2 = cross, 3 = break. Divergence: 1 = regular bullish, 2 = regular bearish, 3 = hidden bullish, 4 = hidden bearish. Signal: 1 = threshold crossing, 2 = series crossover, 3 = compact state change.

**Sources.** `source_code` identifies the reading or comparison that produced an event.

| `source_code` | Source |
|---|---|
| 1 | EMA 20 |
| 2 | EMA 50 |
| 3 | EMA 200 |
| 4 | RSI 14 |
| 5 | ATR 14 |
| 6 | Relative volume (RVOL20) |
| 7 | Confirmed minor pivots |
| 8 | Confirmed major pivots |
| 9 | EMA spread alignment (Direction) |
| 10 | Swing relations (Structure) |
| 11 | RSI force (Momentum) |
| 12 | Leg progression (Progression) |

**Unavailable reasons.** 1 = insufficient bars, 2 = insufficient confirmed pivots, 3 = invalid denominator, 4 = no target, 5 = target unstable, 6 = origin unconfirmed.

**Axis codes.** 1 = Direction, 2 = Momentum, 3 = Structure, 4 = Progression. These identify an axis in `weakest_directional_axis_code`.

**Reference codes.** 1 = EMA 20, 2 = EMA 50, 3 = EMA 200. A confirmed swing level uses `1000 + n`, where `n` is the level's sequence number in the timeframe's confirmed-level registry, ordered by confirmation bar. The registry itself holds the level's price, scale code, and pivot occurrence and availability bars.

**Scale codes.** 1 = minor, 2 = major.

**Target basis codes.** 1 = confirmed structural level, 2 = ATR fallback.

**Retention bound.** A per-timeframe event history keeps the most recent 1024 records. The five-bar window counts are computed from the retained records, and the bound is far larger than any published window needs.

## 15. Decisions that refine this contract

Eleven points in the source specification left a choice open. Each is applied where the table below says it is applied, and the plan's Decisions table holds the options, the rationale, and the choice.

| Decision | Applied in |
|---|---|
| A1 — the published leg is the most recently created leg whose origin is confirmed, with its scale published; before the first confirmation, the newest forming leg | §8.4, §9, §10 |
| A2 — a leg is created from a pivot candidate, so lifecycle 0 and reason 6 are reachable | §5, §8.4 |
| A3 — the exact empirical percentile | §9 |
| A4 — the state counters freeze while the compact code is null | §9 |
| A5 — the share thresholds | §10, §11 |
| A6 — the bounded level reference set | §7.2 |
| A7 — the level clustering set and the single-linkage rule | §10 |
| A8 — the bounded event retention | §14 |
| A9 — a per-bar series and a published snapshot | §9 |
| A10 — when the crate names are fixed | `docs/adr/2026-10-21/four-axis-analytical-model.md`, "Crate layout" |
| A11 — the shell command names are proposals, reserved to the human | M13, issue #29 |

The decisions, their options, and their rationale are recorded in [`docs/execution-plans/2026-10-21/14-model-engine.md`](../../execution-plans/2026-10-21/14-model-engine.md) under "Decisions", and are summarised in the [AI transparency log](../../ai-transparency-log/2026-10-21/14-model-engine.md) for the same work item. They are agent decisions, taken under the human's instruction to apply the recommended default and flag it for review. Each one is a short, isolated change if the human overrides it, and the plan names the section it would change.
