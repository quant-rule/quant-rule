# Compression-Range 5m Next-Bar Resolution Protocol

- Date: 2026-11-04 (UTC)
- Status: **Executed; development gate no edge (back-lag); validation not opened**
- Issue: [#379](https://github.com/quant-rule/quant-rule/issues/379)
- Plan: [`docs/execution-plans/2026-11-04/379-compression-bracket-5m.md`](../../execution-plans/2026-11-04/379-compression-bracket-5m.md)
- Prior 1m range gate: [`MODEL-2026-11-04-06..07`](../../improvement/2026-11-04/COMPRESSION-RANGE-CANDIDATE.md)
- Claim type: model validation evidence; no strategy, cost, or edge claim.

## 1. Question

Does a completed 5-bar coiling window on 5m predict a next-bar touch of at least
one of its published range boundaries, more often than a matched non-coiling
window completion?

This is the same timing/resolution claim as the 1m range gate, evaluated on 5m
because the bracket's cost-to-ATR ratio is lower there.

## 2. Data and partitions

| Partition | Window | Instruments | Use |
|---|---|---|---|
| `model_dev` | 2018-01-01..2020-12-31 | BTCUSD, ETHUSD, XAUUSD | Development |
| `model_validation` | 2021-01-01..2022-12-31 | BTCUSD, ETHUSD, XAUUSD | One-shot confirmation |

Timeframe: 5m, derived from the same 1m source. 2023 is excluded.

**Third-use disclosure.** `model_validation` 2021–2022 has already been used for
the accepted 1m 4 ATR compression-breakout baseline and the accepted 1m
compression-range baseline. This protocol uses the same partition for a 5m event
and outcome. The third use raises residual data-snooping risk; it is disclosed
in the run record and the decision.

## 3. Event definition

The event is `signal.compression_range` (type 3, subtype 8, source ATR 5). For
bar `b` on 5m:

1. The window `b-K+1..=b` has `K = 5` bars.
2. Every bar in the window has `context_code == 2` (ATR ratio < 1 and RVOL < 1).
3. `range_high` is the highest high; `range_low` is the lowest low.
4. The event occurs and becomes knowable at bar `b`, after the window closes.
5. `direction_code = 0`; `primary_value = range_high`; `reference_value =
   range_low`.

The event is appended once per completed window. Overlapping windows produce one
event each. The detector never reads bar `b+1` or later.

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

The 5m bracket strategy reads the published range values and the 5m bar
identity. It does not recompute the range or read a future bar. The strategy
protocol is
The strategy contract was removed in #463; the historical decision is [STRATEGY-BRACKET-GATE.md](../../improvement/2026-11-04/STRATEGY-BRACKET-GATE.md).

## 10. Boundaries and limitations

- The outcome is a boundary touch, not a return or a cost-aware payoff.
- The 5m series is derived from the same 1m source; boundary alignment and gaps
  follow the data engine.
- The 2021–2022 validation partition is reused for a third time; see section 2.
- The harness, review, and decision are same-session passes, not an independent
  Quant Validator record.

## 11. Result

The 5m cell fails the pre-registered back-lag control.

| n | J | Hit | Matched base | Lift | Lift LCB | Rand p | Back lag | Event lift − back lag | Rep | Pass |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 84,627 | 11,234 | 0.5545 | 0.4732 | +0.0813 | +0.0775 | 0.0050 | +0.0333 | +0.0480 | 3 | no |

The sample, lift, lower bound, random control, and replication criteria pass.
The one-bar-lagged rule already captures +3.33pp of the +8.13pp lift, leaving
+4.80pp, 0.20pp below the required +5pp. The gate is not relaxed after the
result.

The validation partition is not opened; the 5m strategy is not run; no 5m
baseline is accepted.

- Gate: [`COMPRESSION-RANGE-5M-GATE`](../../improvement/2026-11-04/COMPRESSION-RANGE-5M-GATE.md)
- Development: [`MODEL-2026-11-04-08`](../../improvement/2026-11-04/MODEL-2026-11-04-08-compression-range-5m-dev.md)
