# Compression-Breakout Range-Expansion Protocol

- Date: 2026-11-04 (UTC)
- Status: **Pre-registered and executed; inconclusive — outcome saturated; validation not opened**
- Issue: [#371](https://github.com/quant-rule/quant-rule/issues/371)
- Plan: [`docs/execution-plans/2026-11-04/371-compression-breakout.md`](../../execution-plans/2026-11-04/371-compression-breakout.md)
- Model contract: [`docs/model/2026-11-05/model-m1.0.md`](../2026-11-05/model-m1.0.md)
- Claim type: model validation evidence; no strategy, cost, or edge claim.

## 1. Question

Does a coiling volatility regime followed by a confirmed compression breakout
predict a larger forward range than coiling alone, after matched baselines,
negative controls, and a multiple-testing correction?

This is a **magnitude/timing** claim. It does not claim direction. The direction
of the breakout is published for the strategy layer, but the model gate ignores
it.

## 2. Data and partitions

| Partition | Window | Instruments | Use |
|---|---|---|---|
| `model_dev` | 2018-01-01..2020-12-31 | BTCUSD, ETHUSD, XAUUSD | Development and freeze |
| `model_validation` | 2021-01-01..2022-12-31 | BTCUSD, ETHUSD, XAUUSD | One-shot confirmation |

Each year is a separate one-year model run. Timeframes: 1h and 15m. 2023 is
excluded. The validation partition is opened once, after the candidate is
frozen, and only if the development gate passes.

## 3. Event definition

The event is a **signal** event, `signal.compression_breakout` (type 3, subtype
7, source ATR 5). For bar `b`:

1. The previous `K = 5` bars are coiling:
   `context_code == 2` (ATR ratio < 1 and RVOL < 1) on each bar.
2. The compression range is the highest high and the lowest low of those five
   bars.
3. ATR 14 at `b` is present and greater than zero.
4. The close at `b` is at least `0.2 * ATR` above the range high (long) or at
   least `0.2 * ATR` below the range low (short).
5. RVOL 20 at `b` is at least `1.2`.
6. If both the long and the short conditions hold on the same bar, the bar is
   ambiguous and produces no event.

The event occurs and becomes knowable at bar `b`. Direction is the breakout
side. `primary_value` is the close; `reference_value` is the broken range
boundary. The event is appended once and is never upgraded.

The event is available only after the five coiling bars and the trigger bar are
closed. The detector never reads a later bar.

## 4. Outcome

For an event at bar `b` with `atr = ATR14[b] > 0` and a valid forward window:

```text
forward_high = max(high[b + 1 .. b + 20])
forward_low  = min(low[b + 1 .. b + 20])
forward_range = forward_high - forward_low
hit = forward_range >= 2.0 * atr
```

A timeout or a smaller range is a miss. The event bar's own high and low are
not used. Gaps are part of the forward window. The horizon is a bar count, not a
clock duration.

## 5. Baselines

The event claim is incremental over **coiling**. Three baselines are fixed:

1. **All bars.** Every bar with ATR > 0 and a full forward window.
2. **Coiling bars.** Every bar whose `context_code` is 2, with a full forward
   window.
3. **Placebo transition.** Every bar whose previous `context_code` is 2 and
   whose current `context_code` is 1 (ATR ratio ≥ 1 and RVOL ≥ 1), but which is
   not a compression-breakout event. This isolates the breakout from the
   volatility transition.

The event base is the event-weighted coiling rate across instruments and years.
The event must beat the coiling rate and the placebo-transition rate.

## 6. Controls

| Control | Definition | Pass condition |
|---|---|---|
| Random coiling | 200 draws of `n` coiling bars with a full window, per bundle | event rate > 95th percentile; p ≤ 0.05 |
| Back-lag | Entry moved 20 bars before the event; the forward window ends at the event bar | back-lag lift ≤ +0.01, or event lift exceeds it by ≥ +0.05 |

The back-lag is a placebo: it cannot see the event. The random control is
direction-free because the claim is magnitude. A direction-flip diagnostic is
reported but is not a pass condition.

## 7. Estimator and correction

- `p = hits / n`; `lift = p - coiling_base`.
- Clusters form inside each one-year bundle; a new cluster starts when the
  event bar is at least 20 bars after the previous event bar. Clusters never
  span bundles.
- The pooled cluster-robust variance uses the pooled `p` and the bundle-local
  cluster counts.
- `lift_lcb = lift - z * se`.
- The primary timeframe choice is one of two (1h or 15m), so `m = 2` and
  `z = NormalDist().inv_cdf(1 - 0.05 / (2 * 2))`.
- The secondary timeframe is descriptive and is not graded.
- If `J <= 1`, the cell is inconclusive.

## 8. Primary timeframe rule

Run the model on the development cells and count events at 1h and 15m. The
counts are computed **before any outcome** and only the counts are inspected.

- If 1h has at least 200 valid events across the three instruments, 1h is
  primary.
- Otherwise 15m is primary.
- The other timeframe is secondary and descriptive.

The rule is fixed. No outcome, lift, or control may influence the choice.

## 9. Pass rule

A development cell passes only when all hold:

1. `n >= 200` valid events and `J >= 20` clusters on the primary timeframe.
2. `lift >= +0.05` over the coiling base and `lift_lcb > 0`.
3. The event also beats the placebo-transition base by at least +0.02, or the
   placebo-transition base is not higher than the coiling base.
4. The random-coiling control passes.
5. The back-lag control passes.
6. At least 2 of 3 instruments have `n >= 30` and a hit rate above the coiling
   base.

A pass freezes the primary timeframe and the exact config. The validation
partition is then opened once with `m = 1` and the same rule. A validation pass
is a **candidate model baseline pending human acceptance**, not an accepted
baseline and not a strategy go. A validation failure is `development_only` and
stops the program.

## 10. Strategy boundary

No strategy is designed, run, or tuned before the model baseline is accepted by
the human. The model event publishes direction for a later strategy, but the
model claim is expansion, not direction. A strategy run is a separate tier and a
separate issue.

## 11. Boundaries and limitations

- The outcome is a forward range, not a return, cost, or tradable payoff.
- The event uses the published context, ATR, and RVOL readings; it adds no new
  indicator.
- The same instruments were used for directional tests; the validation partition
  is unopened, but residual data-snooping risk remains.
- The threshold set is fixed before the run and is not tuned.
- The harness, review, and decision are same-session passes, not an independent
  Quant Validator record.

## 12. Result

Executed on `model_dev` (BTCUSD, ETHUSD, XAUUSD; 2018–2020; 1h and 15m). The
count-only switch selected 1h as primary (302 events). The 2 ATR / 20-bar
outcome saturates: the coiling base is 98.88% at 1h and 95.83% at 15m, so the
maximum possible lift is 1.12pp and 4.17pp, below the 5pp threshold. The 1h
event lift is +0.79pp (corrected LCB +0.04pp; random p 0.144). The 15m secondary
cell is +2.03pp (corrected LCB +1.12pp; random p 0.005) but is not a primary
cell and is not promoted.

The validation partition is not opened; no baseline is accepted. The result is
`inconclusive`, not `no edge`.

- Gate: [`COMPRESSION-GATE`](../../improvement/2026-11-04/COMPRESSION-GATE.md)
- Run record: [`MODEL-2026-11-04-02`](../../improvement/2026-11-04/MODEL-2026-11-04-02-compression-breakout.md)
- Local artifacts: `data/improvement/2026-11-04/MODEL-2026-11-04-02/`
