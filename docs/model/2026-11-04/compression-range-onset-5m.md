# Fresh-Onset Compression-Range 5m Protocol

- Date: 2026-11-04 (UTC)
- Status: **Executed; both partitions pass; accepted model baseline (human, 2026-11-04)**
- Issue: [#381](https://github.com/quant-rule/quant-rule/issues/381)
- Plan: [`docs/execution-plans/2026-11-04/381-compression-onset.md`](../../execution-plans/2026-11-04/381-compression-onset.md)
- Prior 5m gate: [`COMPRESSION-RANGE-5M-GATE`](../../improvement/2026-11-04/COMPRESSION-RANGE-5M-GATE.md) — back-lag fail
- Claim type: model validation evidence; no strategy, cost, or edge claim.

## 1. Question

Does the **first** completed 5-bar coiling window after a known non-coiling bar
on 5m predict a next-bar touch of at least one published range boundary, more
often than a matched non-coiling window completion?

The rolling 5m range event had a +8.13pp lift but failed the back-lag control by
0.20pp because adjacent coiling windows overlap. This protocol removes the
overlap by selecting the onset of each coiling run.

## 2. Data and partitions

| Partition | Window | Instruments | Use |
|---|---|---|---|
| `model_dev` | 2018-01-01..2020-12-31 | BTCUSD, ETHUSD, XAUUSD | Development |
| `model_validation` | 2021-01-01..2022-12-31 | BTCUSD, ETHUSD, XAUUSD | One-shot confirmation |

Timeframe: 5m, derived from the same 1m source. 2023 is excluded.

**Third-use disclosure.** `model_validation` 2021–2022 was already used for the
accepted 1m 4 ATR compression-breakout baseline and the accepted 1m
compression-range baseline. This protocol uses the same partition for a 5m
fresh-onset event and outcome. The third use raises residual data-snooping risk;
it is disclosed in the run records and decision.

## 3. Event definition

The event is `signal.compression_range_onset` (type 3, subtype 9, source ATR 5).
For bar `b` on 5m:

1. The window `b-K+1..=b` has `K = 5` bars and is all-coiling
   (`context_code == 2` on every bar).
2. The bar immediately before the window, `b-K`, carries a known non-coiling
   context code (`Some(code)` with `code != 2`). An unknown predecessor is not
   an onset.
3. `range_high` is the highest high; `range_low` is the lowest low.
4. The event occurs and becomes knowable at bar `b`, after the window closes.
5. `direction_code = 0`; `primary_value = range_high`; `reference_value =
   range_low`.

Overlapping windows later in the same coiling run are suppressed. The detector
never reads bar `b+1` or later.

## 4. Outcome

For an event at bar `b` with a full next 5m bar:

```text
touch = high[b+1] >= range_high or low[b+1] <= range_low
```

The next bar's open and close are not used. A gap counts. The horizon is one 5m
bar, fixed before the run.

## 5. Matched baseline

The baseline is every 5m bar `b` with a full `K`-bar window and a full next bar
where the window is **not** all-coiling. Both the event and the baseline are
window completions; the only difference is the coiling context.

## 6. Controls

| Control | Definition | Pass condition |
|---|---|---|
| Random completion | 200 draws of `n` non-event 5m window completions per bundle | event rate > 95th percentile; p ≤ 0.05 |
| Back-lag | The same structural rule one 5m bar earlier: window `b-K..b-1`, touch at `b` | back-lag lift ≤ +0.01, or event lift exceeds it by ≥ +0.05 |

The onset predecessor is non-coiling, so the lagged window is a non-coiling
completion; the control isolates the onset timing.

## 7. Estimator and correction

- `p = hits / n`; `lift = p - baseline_rate`.
- Clusters form inside each one-year bundle; a new cluster starts when the event
  bar is at least 20 bars after the previous event bar. Clusters never span
  bundles.
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

A development pass freezes the event definition and the exact 5m config. The
validation partition is opened once with the same rule. A validation pass is a
**candidate model baseline pending human acceptance**, not a strategy go. A
validation failure is `development_only` and stops the program.

## 9. Strategy boundary

The pre-registered 5m onset bracket reads the published range values and the 5m
bar identity. It does not recompute the range or read a future bar. The strategy
protocol is
[`docs/config/2026-11-04/strategy-v10-compression-bracket-onset-5m.md`](../../config/2026-11-04/strategy-v10-compression-bracket-onset-5m.md).

## 10. Boundaries and limitations

- The outcome is a boundary touch, not a return or a cost-aware payoff.
- The 5m series is derived from the same 1m source.
- The 2021–2022 validation partition is reused for a third time; see section 2.
- The harness, review, and decision are same-session passes, not an independent
  Quant Validator record.

## 11. Result

Both partitions pass every pre-registered criterion.

| Partition | n | J | Hit | Matched base | Coiling base | Lift | Lift LCB | Rand p | Back lag | Rep |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `model_dev` | 18,324 | 12,521 | 0.5706 | 0.4760 | 0.5628 | +0.0946 | +0.0874 | 0.0050 | +0.0365 | 3 |
| `model_validation` | 14,303 | 9,708 | 0.5373 | 0.4651 | 0.5319 | +0.0722 | +0.0640 | 0.0050 | −0.0065 | 3 |

The onset selection removes the rolling-window overlap: the development
back-lag difference is +5.81pp, up from +4.80pp for the 5m rolling event, and
validation has a −0.65pp back-lag. The event definition was frozen before
validation.

Verdict: accepted model baseline (human, 2026-11-04). The strategy tier proceeds
under the approved onset bracket protocol.

- Candidate: [`COMPRESSION-ONSET-5M-CANDIDATE`](../../improvement/2026-11-04/COMPRESSION-ONSET-5M-CANDIDATE.md)
- Development: [`MODEL-2026-11-04-10`](../../improvement/2026-11-04/MODEL-2026-11-04-10-compression-onset-5m-dev.md)
- Validation: [`MODEL-2026-11-04-11`](../../improvement/2026-11-04/MODEL-2026-11-04-11-compression-onset-5m-validation.md)
