# Model v3 — Flat Indicator and Event Contract

- Date: 2026-11-03 (UTC)
- Status: **Approved**; flat report, MACD, v3 config, and ADX/DI implemented; axis removal deferred.
- Issue: [#343](https://github.com/quant-rule/quant-rule/issues/343)
- Epic: [#342](https://github.com/quant-rule/quant-rule/issues/342)
- Plan: [`docs/execution-plans/2026-11-03/342-model-v3.md`](../../execution-plans/2026-11-03/342-model-v3.md)
- ADR: [`docs/adr/2026-11-03/flat-indicator-event-model.md`](../../adr/2026-11-03/flat-indicator-event-model.md)
- Supersedes for axes: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../2026-10-21/four-axis-analytical-model.md)
- Registry: [`docs/registry/README.md`](../../registry/README.md)

## 1. Purpose

This document is the analytical model's contract from v3 onward. The model takes
closed OHLCV candles per timeframe and publishes per-timeframe indicator readings,
discrete indicator states, confirmed levels, event records, and progression legs.
It publishes numbers, nulls, lists, and records of numeric values. Words are a
presentation mapping in the shell.

The model does not publish axes. The four axis names survive only as a reporting
taxonomy in documentation and presentation: trend, momentum, structure, and
progression. No model field, analytics aggregate, or strategy decision is an axis
value.

## 2. Principles

1. Every indicator and every event is a module with one definition, one source,
   one role, and one claim. The registry is the index of those definitions.
2. A reading is not owned by an axis. The same reading may feed several events,
   and each event states its own definition. A feature is never counted twice
   inside one strategy condition.
3. The model publishes per-timeframe results only. Cross-timeframe views are
   computed by analytics or the shell and are never read by a strategy.
4. Missing, unavailable, neutral, and zero are different states. A missing value
   is null with a reason code, never zero. A measured zero is a value.
5. Availability is not occurrence. A confirmed pivot, a divergence, and a
   progression target record both the bar they describe and the first bar on
   which they became knowable. Nothing reaches an earlier bar.
6. The signal path uses only addition, subtraction, multiplication, division,
   comparison, clamp, sign, and rounding. No statistical or fitted value enters
   an indicator, state, event, or strategy decision.
7. A strategy may read only events and states whose registry role is `signal` or
   `context`. A `diagnostic` measurement is never readable by a strategy.
8. The model never reads a strategy, execution, portfolio, policy, or analytics
   value.

## 3. Configuration

The model config is `quantrule.model/v3`. It has a `defaults` block and optional
per-timeframe overrides. An absent override inherits the default. An indicator
with `active: false` is not computed and is not published for that timeframe.

```json
{
  "schema": "quantrule.model/v3",
  "defaults": {
    "ema": { "periods": [20, 50, 200], "active": true },
    "rsi": { "period": 14, "active": true },
    "atr": { "period": 14, "active": true },
    "rvol": { "period": 20, "active": true },
    "macd": { "fast": 12, "slow": 26, "signal": 9, "active": true },
    "adx": { "period": 14, "threshold": 25.0, "slope_bars": 3, "active": true },
    "pivots": { "minor": 2, "major": 5, "active": true },
    "events": {
      "level": {
        "references": ["ema", "pivots"],
        "test_atr": 0.2,
        "break_atr": 0.2
      },
      "divergence": {
        "indicator": "rsi",
        "pivot_scale": "major",
        "window": 2
      },
      "signal": {
        "rsi_levels": [30, 50, 70],
        "ema_pairs": [[20, 50], [50, 200], [20, 200]],
        "macd": ["line_signal_cross", "histogram_slope_flip", "zero_cross"],
        "state_changes": true
      }
    },
    "progression": {
      "target_atr": 2.0,
      "stability_bars": 3,
      "invalidation_atr": 0.2,
      "min_target_atr": 0.2
    }
  },
  "timeframes": {
    "15m": { "rsi": { "period": 14, "active": false } }
  }
}
```

Rules:

- `ema.periods` is exactly three positive, strictly increasing periods.
- `macd.fast < macd.slow`; `macd.signal > 0`.
- `pivots.minor > 0`, `pivots.major > 0`.
- `events.level.references` is a subset of `["ema", "pivots"]`. A reference to an
  inactive indicator is rejected.
- `events.divergence.indicator` is `"rsi"`; a divergence with `rsi.active: false`
  is rejected.
- `events.signal` may reference only active indicators.
- `progression.stability_bars >= 1`; `min_target_atr > 0`.
- An unknown indicator, event, field, or timeframe is rejected.
- The effective config is recorded in the report and the run provenance.

## 4. Indicators

### 4.1 EMA

For period `n`, `alpha = 2/(n+1)`. The first value is the arithmetic mean of the
first `n` closes. Thereafter `ema_t = alpha * close_t + (1-alpha) * ema_{t-1}`.
The value is unavailable before `n` closed bars. The three periods are the fast,
middle, and slow roles.

Published readings: `ema_fast`, `ema_mid`, `ema_slow`.
Published state: `ema_stack_state` = +1 when fast > mid > slow, -1 when
fast < mid < slow, 0 otherwise. Equality produces 0 unless the strict ordering
holds. State is null when any required EMA is null.

### 4.2 RSI

Wilder-smoothed RSI with period `n` (default 14). The first average gain and loss
are the arithmetic means of the first `n` gains and losses. Thereafter
`avg_t = (avg_{t-1} * (n-1) + value_t) / n`.
`rsi = 100 - 100/(1 + avg_gain/avg_loss)`.
Unavailable before `n+1` closed bars. If both averages are zero, RSI is null
(reason 3, invalid denominator). If average loss is zero and average gain is not,
RSI is 100. A measured 0 or 100 is a value.

Published reading: `rsi`.
Published state: `rsi_zone` = -1 when `rsi < 30`, +1 when `rsi > 70`, 0 when
`30 <= rsi <= 70`. State is null when RSI is null. The zone thresholds are
configurable in `events.signal.rsi_levels` and are recorded in the registry.

### 4.3 ATR

True range is `max(high-low, abs(high-prev_close), abs(low-prev_close))`.
Wilder smoothing with period `n` (default 14), seeded with the mean of the first
`n` true ranges. Unavailable before `n+1` closed bars.

Published readings: `atr`, `atr_ratio = atr / SMA(atr, 20)`.
`atr_ratio` is null when fewer than 20 ATR values are available or the SMA is
zero. A measured ATR of zero is a value.

### 4.4 RVOL

`rvol = volume / SMA(volume, n)` with `n` default 20. Null when volume is absent
or the baseline is zero. Unavailable before `n` closed bars with volume.

Published reading: `rvol`.

### 4.5 MACD

`line = EMA(close, fast) - EMA(close, slow)`.
`signal = EMA(line, signal_period)`, seeded with the arithmetic mean of the first
`signal_period` line values.
`histogram = line - signal`.

The line is unavailable before `slow` closed bars. The signal and histogram are
unavailable before `slow + signal - 1` closed bars. The signal EMA uses the same
`alpha = 2/(n+1)` formula as every other EMA.

Published readings: `macd_line`, `macd_signal`, `macd_histogram`.
Published states:

| State | Code | Condition |
|---|---|---|
| `macd_state` | +1 | line > signal |
| `macd_state` | -1 | line < signal |
| `macd_state` | 0 | line == signal |

`macd_state` is null when line or signal is null.

MACD is a trend/momentum indicator. Its cross events are Signal events; its line
and signal are also eligible Level references when `events.level.references`
includes `"macd"`. The default reference set is EMA and pivots only.

### 4.6 Confirmed pivots

A pivot candidate at bar `i` is a bar whose high is strictly greater than the
highs of the `k` bars to its left, or whose low is strictly lower than the lows
of those `k` bars to its left. A candidate becomes a confirmed pivot when the `k`
bars to its right have closed and the strict condition still holds.

- Minor: `k = 2`. Major: `k = 5` by default.
- A bar that qualifies as both a candidate high and a candidate low at the same
  scale is discarded at that scale.
- Equal highs or lows are not strict extrema; they are discarded at that scale.
- `occurred_bar_index` is `i`; `available_bar_index` is `i + k`.
- A pivot is never used before its availability bar.

Published record: the latest confirmed minor high, minor low, major high, and
major low, each with price, occurrence bar, availability bar, and scale.

### 4.7 ADX and DI

ADX measures trend strength and is context; `+DI` and `-DI` carry direction.
The family uses Wilder smoothing with period `n`:

```
+DM_t = high_t - high_(t-1)  when positive and greater than low_(t-1) - low_t
-DM_t = low_(t-1) - low_t    when positive and greater than high_t - high_(t-1)
TR_t  = max(high-low, |high-prev_close|, |low-prev_close|)
+DI   = 100 * Wilder(+DM, n) / Wilder(TR, n)
-DI   = 100 * Wilder(-DM, n) / Wilder(TR, n)
DX    = 100 * |+DI - -DI| / (+DI + -DI)
ADX   = Wilder(DX, n)
```

The first DI and DX are at bar `n`; the first ADX is at bar `2*n - 1`.
A zero smoothed true range produces null DI. A zero DI sum produces null DX.
A null DX resets the ADX seed window; ADX is never replaced by zero.

Published readings: `adx`, `di_plus`, `di_minus`.
Published states: `adx_regime` = 1 at or above `threshold`, 0 below, null when
ADX is null; `adx_slope` = sign of `ADX(t) - ADX(t-slope_bars)`; `di_state` =
sign of `+DI - -DI`.
Published event: `signal.di_cross` (series crossover, source `+DI`, reference
`-DI`). ADX itself emits no directional event.


## 5. Events

Three event types share one record.

| `event_type_code` | Type |
|---|---|
| 1 | Level |
| 2 | Divergence |
| 3 | Signal |

### 5.1 Common record

| Field | Type | Meaning |
|---|---|---|
| `event_index` | integer | Stable sequence within the timeframe |
| `event_code` | positive integer | Stable registry event identity; unique across all events |
| `event_type_code` | 1..3 | Level, Divergence, Signal |
| `event_subtype_code` | integer | Subtype defined below |
| `source_code` | integer | Reading or comparison that produced the event |
| `indicator_code` | integer | Indicator family that owns the source |
| `role_code` | 1..3 | 1 signal, 2 context, 3 diagnostic |
| `timeframe_secs` | positive integer | Source timeframe |
| `occurred_bar_index` | integer | Bar the relationship describes |
| `available_bar_index` | integer | First closed bar on which it is knowable |
| `direction_code` | -1, 0, +1 | Bearish, neutral, or bullish implication |
| `reference_code` | integer or null | Level or series reference |
| `primary_value` | number or null | Main measured value |
| `reference_value` | number or null | Compared value |
| `data_completeness` | 0..1 or null | Fraction of required inputs available |
| `event_strength` | number or null | ATR-normalised magnitude; definition in the registry |

An event is appended once, when it becomes knowable, and is never upgraded in
place. If several references qualify on one bar, they produce separate records in
the fixed reference order. The same `(event_code, reference_code,
occurred_bar_index)` is never emitted twice. The code is the event's identity;
`event_type_code`, `event_subtype_code`, and `source_code` describe its family
and may be shared by several registered events.

### 5.2 Level events

Level references are the active EMA values and the latest confirmed swing high
and low at each active scale. The default bounded set is three EMAs plus four
pivots; the bound is recorded in the registry.

Let `zone = test_atr * ATR`.

- **Test/retest (subtype 1):** the bar range intersects `level ± zone`.
- **Cross (subtype 2):** the previous close is at or below the level and the
  current close is strictly above it, or the previous close is at or above and
  the current close is strictly below it.
- **Break (subtype 3):** a cross where the current close finishes at least
  `break_atr * ATR` beyond the level.

When more than one subtype qualifies for the same reference and bar, only the
highest-priority subtype is emitted: break, then cross, then test/retest.
`direction_code` is +1 when the close finishes above the level, -1 when below,
and 0 when equal.

A level event is evaluated only when ATR is available. A measured ATR of zero
produces a zero-width zone and remains valid. The proximity zone is closed: a
range that touches its boundary intersects it.

### 5.3 Divergence events

Divergence is a relationship between confirmed price pivots and RSI. Use
consecutive confirmed pivots of the same kind at the configured scale. Around
each pivot, use the minimum RSI in the two bars either side for a low, and the
maximum RSI in the two bars either side for a high. The window is evaluated only
after the pivot is confirmed.

| Subtype | Code | Condition | Direction |
|---|---|---|---|
| Regular bullish | 1 | P2 < P1 and R2 > R1 at lows | +1 |
| Regular bearish | 2 | P2 > P1 and R2 < R1 at highs | -1 |
| Hidden bullish | 3 | P2 > P1 and R2 < R1 at lows | +1 |
| Hidden bearish | 4 | P2 < P1 and R2 > R1 at highs | -1 |

`occurred_bar_index` is the newer pivot bar; `available_bar_index` is its
confirmation bar. `price_delta_atr = (P2-P1)/ATR` at the newer pivot occurrence;
it is null when ATR is null or zero, and the divergence is still emitted.
`reference_code` identifies the newer confirmed pivot level.

### 5.4 Signal events

| Subtype | Code | Definition |
|---|---|---|
| Threshold crossing | 1 | RSI crosses a configured level |
| Series crossover | 2 | One configured series crosses another |
| State change | 3 | A published state code changes between two code-bearing bars |
| Histogram slope flip | 4 | The MACD histogram's slope changes sign |
| Zero cross | 5 | A series crosses zero |

- **RSI threshold crossing:** for each level in `rsi_levels`, a strict cross from
  one side to the other. Equality on the previous bar counts as a side; equality
  on the current bar is not a cross.
- **EMA pair crossover:** for each configured pair, the faster EMA crosses the
  slower one. The source is the faster EMA; `reference_code` is the slower EMA.
- **MACD line/signal crossover:** the MACD line crosses the signal. Source is the
  line; `reference_code` is the signal. This is the same relationship as the
  histogram crossing zero, so only the series crossover is emitted; a separate
  histogram zero cross is not duplicated.
- **MACD histogram slope flip:** `sign(hist_t - hist_{t-1})` differs from
  `sign(hist_{t-1} - hist_{t-2})` and both differences are non-zero. Direction is
  the sign of the current difference. This is a momentum-acceleration event, not
  a duplicate of the line/signal cross.
- **MACD zero cross:** the MACD line crosses zero. Source is the line; the
  reference value is zero.
- **State change:** a state code changes between two code-bearing bars. A
  transition from null, a transition to null, and two equal codes are not
  changes. `primary_value` is the new code, `reference_value` the old code, and
  `direction_code` is the sign of their difference.

Precedence avoids double counting the same underlying occurrence:

- An RSI threshold crossing takes precedence over an RSI zone state change on the
  same bar.
- An EMA pair crossover takes precedence over an EMA stack state change on the
  same bar when the stack change is caused by that pair.
- A MACD line/signal crossover takes precedence over a MACD state change on the
  same bar.

Distinct definitions that coincide on one bar are emitted separately because they
name different relationships. The registry records each event's source, role, and
claim.

## 6. Progression

Progression describes a structural leg, not the instrument in the abstract. Each
timeframe maintains a minor and a major leg.

**Creation.** A leg is created from a pivot candidate at scale `k`. It is
lifecycle 0 (forming) until the candidate is confirmed at `i + k`, then lifecycle
1 (active). A candidate that fails to confirm is discarded unless price closes
beyond its origin against the leg by more than `invalidation_atr * ATR_at_origin`;
then the leg is invalidated.

**Transitions.** A confirmed opposite pivot completes the active leg (lifecycle
2) and starts a new leg. A confirmed same-kind pivot supersedes it (lifecycle 4).
A close beyond the origin against the leg by more than
`invalidation_atr * ATR_at_origin` invalidates it (lifecycle 3).

**Published leg.** The most recently created retained leg whose origin is
confirmed, across both scales. Before the first confirmation, the newest forming
leg is published. A newer unconfirmed candidate does not displace a confirmed
leg.

**Target.** At leg creation, choose the nearest confirmed structural level in the
leg's direction beyond the origin at the same scale whose distance is at least
`min_target_atr * ATR_at_origin`. If no such level exists, use the fallback
`origin + direction * target_atr * ATR_at_origin`. This minimum-distance rule
replaces the v1 target rule that could select a level almost coincident with the
origin and produce a null progress with no usable information.

**Target revision.** When a newly confirmed or invalidated level changes the
chosen target, update `target_price`, increment `target_version`, and set
`target_stable_code = 0`. Set it back to 1 after the target has remained
unchanged for `stability_bars` closed bars. Stage promotion is suppressed while
the target is unstable.

**Arithmetic.**

```
progress = (close - origin_price) / (target_price - origin_price)
displacement_price = close - origin_price
displacement_atr = (close - origin_price) / ATR_at_origin
```

Progress is not clamped. It is null when the target distance is below
`min_target_atr * ATR_at_origin` (reason 3, invalid denominator) or when there is
no target (reason 4, no target).

**Lifecycle codes.** 0 forming, 1 active, 2 completed, 3 invalidated,
4 superseded.

**Stage codes.** 0 progress < 0; 1 `0 <= progress < 0.25`; 2 `0.25 <= progress <
0.50`; 3 `0.50 <= progress < 0.75`; 4 `progress >= 0.75`; 5 terminal lifecycle.
A terminal lifecycle takes precedence. The stage is null while the origin is
unconfirmed, the target is unstable, or progress is null.

## 7. Output

Each timeframe publishes:

| Field | Shape | Meaning |
|---|---|---|
| `timeframe_secs` | integer | Source timeframe |
| `coverage` | record | Expected bars, present bars, missing bars, first and last open time |
| `readings` | record | Every active indicator value, with null where unavailable |
| `states` | record | Every active discrete state code |
| `levels` | list | Active level references with source, price, scale, occurrence, availability, distance |
| `events` | list | Event records, bounded to the retention window |
| `event_counts` | record | Counts by type and subtype over the last five closed bars |
| `progression` | record | Published leg plus the minor and major leg records |
| `context` | record | ATR ratio, RVOL ratio, regime code |
| `registry_version` | string | Registry version that defines the codes |

The report also carries the effective per-timeframe config and the data version.
The model publishes no axis values, no axis consensus, no cross-timeframe
agreement, no percentile, no readiness score, and no state label.

The model report's `flat` section is the v3 projection. It carries one entry per
ladder timeframe with coverage, the latest readings and states, every confirmed
level, event counts plus the latest and most recent records, and the latest
progression leg at each scale. It carries no axis value and no consensus.

Full per-bar series are written as CSV artifacts by the shell:
`readings.csv`, `states.csv`, `levels.csv`, and `events.csv`. The JSON report is
the summary and the latest snapshot; the CSV files are the exhaustive record.

The legacy `axes` section of the market report is retained only while the old
report command and the retired v3–v7 strategies are removed. It is not part of
the v3 contract and no v8 strategy may read it.

## 8. Availability and causality

- A value at bar `t` depends only on bars at or before `t`.
- A confirmed pivot is used only at or after `available_bar_index`.
- An event is published only at or after `available_bar_index`.
- A divergence is never emitted before the newer pivot is confirmed.
- A target revision is visible only at its revision bar.
- Each timeframe is computed independently; no timeframe reads another.
- The same candles and config produce the same values and records.
- Iteration over unordered collections never affects output.

## 9. Numeric contract

- Only addition, subtraction, multiplication, division, comparison, clamp, sign,
  and rounding appear in the signal path.
- No transcendental function, regression, correlation, p-value, fitted
  coefficient, or percentile enters a model value or a strategy decision.
- A denominator that is zero produces a null with reason 3, except where this
  contract states a defined limit. A null is never replaced by zero.
- The accumulation order is oldest bar forward; Wilder averages accumulate in bar
  order; EMA seeds are arithmetic means.
- `data_completeness` is `available_required_inputs / required_inputs`, computed
  from non-null inputs. It is deterministic and is not a probability.
- `event_strength` is defined per event family in the registry, using an
  ATR-normalised magnitude or distance. It is descriptive, not a probability.

## 10. Roles and registry

Every indicator and event has a registry entry with:

- a stable ID and numeric codes;
- the formula and inputs;
- parameters, defaults, ranges, and whether each is configurable or fixed;
- availability and outputs;
- the event definitions it can produce;
- a role: `signal`, `context`, or `diagnostic`;
- a falsifiable claim;
- the tests that cover it;
- its status.

Only `signal` and `context` measurements may be read by a strategy. A `diagnostic`
measurement is reporting-only. A signal event needs an A/B grade from the model
validation gate before a strategy may use it; a C grade is allowed only as a
single human-approved hypothesis; a D grade blocks.

## 11. Reporting taxonomy

The four familiar names survive only here, as a way to group indicators in
documentation and the shell:

| Taxonomy | Indicators and records |
|---|---|
| Trend | EMA, MACD line and signal |
| Momentum | RSI, MACD histogram |
| Structure | Confirmed pivots, levels, swing relations |
| Progression | Legs, targets, stages |
| Volatility and participation | ATR, RVOL, context regime |

These are labels. They are not numeric fields, not model outputs, and not
strategy inputs. Analytics may count how many timeframes share a state; the count
is a reporting view and the strategy cannot read it.

## 12. Explicit exclusions

- Axis values, axis consensus, and axis weights.
- A single composite score across indicators or timeframes.
- Trade entries, exits, stops, targets for a trade, sizing, cash, or P&L.
- Labels, confidence probabilities, and state strings in the model output.
- Statistical, machine-learning, or neural-network signal generation.
- Cross-timeframe agreement as a strategy input.

## 13. Code registries

| Registry | Codes |
|---|---|
| Indicator | 1 EMA, 2 RSI, 3 ATR, 4 RVOL, 5 MACD, 6 pivots, 9 ADX, 10 DMI |
| Event type | 1 Level, 2 Divergence, 3 Signal |
| Level subtype | 1 test/retest, 2 cross, 3 break |
| Divergence subtype | 1 regular bullish, 2 regular bearish, 3 hidden bullish, 4 hidden bearish |
| Signal subtype | 1 threshold crossing, 2 series crossover, 3 state change, 4 histogram slope flip, 5 zero cross, 6 trend continuation |
| Source | 1 EMA fast, 2 EMA mid, 3 EMA slow, 4 RSI, 5 ATR, 6 RVOL, 7 minor pivots, 8 major pivots, 9 Direction (legacy), 10 Structure (legacy), 11 Momentum (legacy), 12 progression, 13 MACD line, 14 MACD signal, 15 MACD histogram, 16 ADX, 17 +DI, 18 -DI |
| Role | 1 signal, 2 context, 3 diagnostic |
| Scale | 1 minor, 2 major |
| Target basis | 1 confirmed structural level, 2 ATR fallback |
| Lifecycle | 0 forming, 1 active, 2 completed, 3 invalidated, 4 superseded |
| Stage | 0..4 progress bands, 5 terminal |
| Unavailable reason | 1 insufficient bars, 2 insufficient confirmed pivots, 3 invalid denominator, 4 no target, 5 target unstable, 6 origin unconfirmed |

A code is stable once published. A retired code is not reused.

## 14. Validation boundary

The validation harness joins each event at `available_bar_index` and measures the
outcome its claim names:

| Claim | Primary outcome |
|---|---|
| Directional trend state | Sign of forward return |
| Momentum force or acceleration | Forward change in force or acceleration |
| Structure | Path reaction or continuation versus a matched barrier baseline |
| Progression | Target hit before invalidation |
| Level | Path reaction or break versus a matched barrier baseline |
| Divergence | Path reversal after confirmation versus a matched barrier baseline |
| Signal | Path directional move or volatility versus a matched barrier baseline |
| ADX trend strength | Forward range expansion or trend persistence; not direction |
| DI cross / trend continuation | Path directional move versus a matched barrier baseline |

The path outcome is defined in
[`path-validation.md`](path-validation.md): entry at the event close, an
ATR-scaled target and stop, a fixed bar horizon, and a stop-first tie-break. The
fixed-horizon sign is a screening metric, not the claim test for an event family
whose claim names a reaction, continuation, reversal, or target. A path outcome
that overlaps other events is evaluated with a cluster-robust bound; the event
count alone is not the effective sample size.

A hit rate is reported with its sample size, coverage, and a 95% lower bound. A
single cell is not a model baseline. Validation uses statistics; the model and
strategy never read them.
