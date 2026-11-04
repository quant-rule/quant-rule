# Compression-Range Next-Bar Resolution Protocol

- Date: 2026-11-04 (UTC)
- Status: **Executed; both partitions pass; accepted model baseline (human, 2026-11-04)**
- Issue: [#377](https://github.com/quant-rule/quant-rule/issues/377)
- Plan: [`docs/execution-plans/2026-11-04/377-compression-bracket.md`](../../execution-plans/2026-11-04/377-compression-bracket.md)
- Prior baseline: `MODEL-2026-11-04-04..05` (1m, 4 ATR, 20 bars)
- Claim type: model validation evidence; no strategy, cost, or edge claim.

## 1. Question

Does a completed coiling window predict a next-bar touch of at least one of its
published range boundaries, more often than a matched non-coiling window
completion?

This is a **timing/resolution** claim. It does not claim direction, a target, or
a payoff. It validates the pre-breakout levels that the bracket strategy uses.

## 2. Data and partitions

| Partition | Window | Instruments | Use |
|---|---|---|---|
| `model_dev` | 2018-01-01..2020-12-31 | BTCUSD, ETHUSD, XAUUSD | Development |
| `model_validation` | 2021-01-01..2022-12-31 | BTCUSD, ETHUSD, XAUUSD | One-shot confirmation |

Timeframe: 1m. 2023 is excluded.

**Second-use disclosure.** `model_validation` 2021–2022 was already used to
confirm the accepted 1m 4 ATR compression-breakout baseline. This protocol uses
the same partition for a different event (`signal.compression_range`) and a
different outcome (next-bar boundary touch). The second use raises residual
data-snooping risk; it is disclosed in the run record and decision.

## 3. Event definition

The event is a **signal** event, `signal.compression_range` (type 3, subtype 8,
source ATR 5). For bar `b`:

1. The window `b-K+1..=b` has `K = 5` bars.
2. Every bar in the window has `context_code == 2` (ATR ratio < 1 and RVOL < 1).
3. `range_high` is the highest high of the window; `range_low` is the lowest low.
4. The event occurs and becomes knowable at bar `b`, after the window closes.
5. `direction_code = 0`; `primary_value = range_high`; `reference_value =
   range_low`.

The event is appended once per completed window. Overlapping windows produce one
event each. The detector never reads bar `b+1` or later.

## 4. Outcome

For an event at bar `b` with a full next bar:

```text
touch = high[b+1] >= range_high or low[b+1] <= range_low
```

The next bar's open and close are not used. A gap counts: if the next bar opens
beyond a boundary, it is a touch. The horizon is one bar, fixed before the run.

## 5. Matched baseline

The baseline is every bar `b` with a full `K`-bar window and a full next bar
where the window is **not** all-coiling. Both the event and the baseline are
window completions; the only difference is the coiling context. The all-bars and
coiling-bar rates are reported descriptively.

## 6. Controls

| Control | Definition | Pass condition |
|---|---|---|
| Random completion | 200 draws of `n` non-event window completions per bundle | event rate > 95th percentile; p ≤ 0.05 |
| Back-lag | The same structural rule one bar earlier: window `b-K..b-1`, touch at `b` | back-lag lift ≤ +0.01, or event lift exceeds it by ≥ +0.05 |

The back-lag is a placebo: it cannot see the event window's completion. The
random control is direction-free because the outcome is direction-free.

## 7. Estimator and correction

- `p = hits / n`; `lift = p - baseline_rate`.
- Clusters form inside each one-year bundle; a new cluster starts when the event
  bar is at least 20 bars after the previous event bar. Clusters never span
  bundles.
- The pooled cluster-robust variance uses the pooled `p` and the bundle-local
  cluster counts.
- `lift_lcb = lift - z * se`; the single cell uses `m = 1`, `z = 1.96`.
- If `J <= 1`, the cell is inconclusive.

## 8. Pass rule

The cell passes only when all hold:

1. `n >= 200` and `J >= 20`.
2. `lift >= +0.05` and `lift_lcb > 0`.
3. Random-completion control p ≤ 0.05.
4. Back-lag control passes.
5. At least 2 of 3 instruments have `n >= 30` and a touch rate above the
   matched baseline.

A development pass freezes the event definition and the exact config. The
validation partition is then opened once with the same rule. A validation pass
is a **candidate model baseline pending human acceptance**, not a strategy go. A
validation failure is `development_only` and stops the program.

## 9. Strategy boundary

The model publishes numbers only. The bracket strategy reads the published range
values and the bar identity; it does not recompute the range, read a statistic,
or read a future bar. The strategy protocol is
[`docs/config/2026-11-04/strategy-v10-compression-bracket.md`](../../config/2026-11-04/strategy-v10-compression-bracket.md).

## 10. Boundaries and limitations

- The outcome is a boundary touch, not a return, target, or cost-aware payoff.
- A touch can be a false breakout; the strategy's stop handles it.
- The event uses the published context code; it adds no new indicator.
- The 2021–2022 validation partition is reused; see section 2.
- The harness, review, and decision are same-session passes, not an independent
  Quant Validator record.

## 11. Result

Both partitions pass every pre-registered criterion.

| Partition | n | J | Hit | Matched base | Lift | Lift LCB | Rand p | Back lag | Rep |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `model_dev` | 346,120 | 60,605 | 0.6474 | 0.5442 | +0.1032 | +0.1011 | 0.0050 | +0.0531 | 3 |
| `model_validation` | 247,989 | 43,937 | 0.5802 | 0.4984 | +0.0818 | +0.0795 | 0.0050 | +0.0291 | 3 |

The event definition and configuration were frozen before validation: 1m,
5 coiling bars, type 3 subtype 8 source 5, direction 0. No threshold was changed
after the development result.

`model_validation` 2021–2022 was reused from the accepted 4 ATR breakout
baseline; the second use is disclosed in the run record and the candidate.

Verdict: accepted model baseline (human, 2026-11-04). The strategy tier may
proceed under the approved bracket protocol.

- Candidate: [`COMPRESSION-RANGE-CANDIDATE`](../../improvement/2026-11-04/COMPRESSION-RANGE-CANDIDATE.md)
- Development: [`MODEL-2026-11-04-06`](../../improvement/2026-11-04/MODEL-2026-11-04-06-compression-range-dev.md)
- Validation: [`MODEL-2026-11-04-07`](../../improvement/2026-11-04/MODEL-2026-11-04-07-compression-range-validation.md)
