# Fresh-Onset 5m Breakout — Measurement Protocol

- Date: 2026-11-10 (UTC)
- Status: **Executed; development gate failed; premise rejected; 2024 closed**
- Programme: [`SR13-CHARTER`](../../governance/2026-11-10/SR13-CHARTER.md)
- Tool: [`breakout_edge.py`](../../../tools/research/breakout_edge.py)
- Claim type: strategy research protocol; no edge claim.

## 1. Question

Does the first confirmed close outside the accepted fresh-onset compression
range predict continuation in the direction of that close over the next 1, 3, 6,
and 12 bars, after realistic costs?

This is not a test of whether compression predicts volatility expansion. The
onset range is an accepted neutral measurement. The new claim is directional.

## 2. Event and rule

The event is `signal.compression_range_onset` (type 3, subtype 9, source 5) on
the 5m ladder. It publishes `range_high` and `range_low` at the last coiling bar
`b`.

The rule is a deterministic state machine:

1. The onset range is valid only when `range_high > range_low` and both are
   finite.
2. Candidate break bars are `b+1 .. b+W`.
3. The first bar whose **close** is strictly above `range_high` is a long
   break; the first whose close is strictly below `range_low` is a short break.
   A close exactly at a boundary is inside.
4. The first close wins. A later close on the other side does not change the
   direction.
5. If a new onset occurs before the break, the unfinished onset is dropped; the
   new onset is processed independently.
6. If no close is outside within `W` bars, the event is dropped.
7. Entry is the next bar open after the break close. A missing entry bar drops
   the event.
8. No stop, target, halt, filter, or position state enters the measurement.

## 3. Parameters

| Parameter | Value | Kind |
|---|---:|---|
| Timeframe | 5m | fixed |
| Coiling window | 5 bars | inherited from the accepted onset event |
| Breakout window `W` | 3 bars | primary; fixed before the run |
| Descriptive windows | 1, 6 bars | robustness only; cannot promote |
| Horizons | 1, 3, 6, 12 bars | fixed |
| Primary horizon | 6 bars | fixed |
| Sides | pooled primary; long/short descriptive | fixed |
| Costs | 6 bps base; 9 bps stress | fixed |
| Cluster gap | 20 bars | fixed |
| Random seed | 0 | fixed |
| Random draws | 200 | fixed |

## 4. Estimand

For a break at bar `t`, direction `d ∈ {+1, −1}`, entry `E = open[t+1]`, and
horizon `h`:

```text
r = d * (close[t+h] - E) / E * 10_000   (basis points)
```

The primary estimand is the mean of `r` at `h = 6`, pooled across long and short
breaks, on the non-overlapping sample. The 95% interval uses the sample standard
error and the normal approximation.

## 5. Sample

- Primary inference uses a non-overlapping sample: process break bars in order
  and keep one only when its break bar is at least 20 bars after the last kept
  break bar.
- The full sample is reported descriptively.
- The count gate is at least 200 non-overlapping signals and 20 clusters.
- The per-year means use the non-overlapping sample.

## 6. Controls

| Control | Definition | Gate |
|---|---|---|
| Unconditional | All bars, same side sign, same entry convention | Signal mean and LCB > baseline mean |
| Fade | Same breaks, opposite direction | Signal mean and LCB > fade mean |
| Random direction | 200 seeded sign flips on the same breaks | Signal mean > 95th percentile |
| Back-lag | Same rule with the onset range shifted one bar earlier | Back-lag mean ≤ 0, or signal − back-lag ≥ 5 bps with LCB > 0 |
| Placebo range | Every non-coiling 5-bar window completion, same close rule | Signal mean and LCB > placebo mean |
| Compression-only | Entry at the onset bar, before the break, long-only and short-only | Descriptive only; no gate |

The placebo range is the matched baseline for the compression condition: the
only difference from the signal is that the window is not all-coiling. The
compression-only control is descriptive because conditioning it on the eventual
break direction would use the future.

## 7. Gates

The primary must pass all of:

1. **G1 baseline:** signal mean > baseline mean and signal LCB > baseline mean.
2. **G2 fade:** signal mean > fade mean and signal LCB > fade mean.
3. **G3 random:** signal mean > 95th percentile of the random-direction means.
4. **G4 back-lag:** back-lag mean ≤ 0, or signal mean − back-lag mean ≥ 5 bps
   and signal LCB − back-lag mean ≥ 5 bps.
5. **G5 placebo:** signal mean > placebo mean and signal LCB > placebo mean.
6. **G6 gross:** signal mean > 0 and signal LCB > 0.
7. **G7 cost:** signal LCB > 9 bps.
8. **G8 stability:** at least 3 of 5 development years positive.
9. **G9 sample:** at least 200 signals and 20 clusters.

Only the pooled 6-bar primary can promote. The 1/3/12-bar horizons, the two
sides, and the W=1/W=6 windows are descriptive. A positive secondary result
does not rescue a failed primary.

## 8. Partitions

| Partition | Window | Use |
|---|---|---|
| Development | `strategy_dev` 2018-01-01..2022-12-31 | Screen and gate |
| Validation | `strategy_validation` 2024-01-01..2024-12-31 | One-shot; closed because the development gate failed |

2021-2022 is not validation for this premise: the accepted onset baseline used
it as a third use, and the model gates have used it repeatedly. It may be
reported as a descriptive replication only.

## 9. Production path

The strategy boundary forbids the strategy from comparing the candle close to
the onset range: that comparison is a model measurement. The current
`event_composite` engine reads events and states, not the candle close, and the
`compression_bracket` engine enters on a touch at the boundary, not on a close.
The existing `signal.compression_breakout` is a different rule (0.2 ATR buffer,
RVOL ≥ 1.2, pre-trigger window).

If the development gate and the 2024 validation pass, the production path is a
new model event:

- ID `signal.compression_range_onset_break`;
- type 3 Signal, subtype 10, source 5;
- direction ±1, `primary_value` = break close, `reference_value` = broken
  boundary;
- available at the break bar;
- the same state machine and `W=3` as this protocol;
- role `signal`, with a causal/deterministic/golden/edge-case test set and a
  Model Change record.

The model event is not built because the gate failed.

## 10. Verification

- Fixture test: `python3 -m unittest tools/research/test_breakout_edge.py -v`
  (9 tests: long, short, boundary-equal, window expiry, next-onset
  supersession, gap-open entry, end-of-series, non-overlap, stats).
- Ladder check: the 5m-only rows are byte-identical to the full
  1m/5m/15m/1h ladder on the 2018 Q1 common sample (18,267 rows, 0
  differences; 201,431 5m events identical).
- Determinism: sorted inputs, `math.fsum`, explicit seed; repeated runs produce
  the same JSON.

## 11. Result

The primary failed. Non-overlapping pooled 6-bar mean −2.1790 bps, 95% interval
[−3.5608, −0.7972], hit rate 44.5%, 2/5 years positive. The 1-bar mean is
+0.2801 bps (below the 6 bps cost); the 3- and 12-bar means are negative. The
W=1 and W=6 windows are also negative. The full record is
[`SR13`](../../improvement/2026-11-10/SR13.md).
