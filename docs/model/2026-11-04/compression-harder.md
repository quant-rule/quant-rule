# Harder Compression-Expansion Protocol

- Date: 2026-11-04 (UTC)
- Status: **Pre-registered and executed; 1m 4 ATR passes both partitions; accepted model baseline (human, 2026-11-04)**
- Issue: [#373](https://github.com/quant-rule/quant-rule/issues/373)
- Plan: [`docs/execution-plans/2026-11-04/373-compression-harder.md`](../../execution-plans/2026-11-04/373-compression-harder.md)
- Prior gate: [`MODEL-2026-11-04-02`](../../improvement/2026-11-04/MODEL-2026-11-04-02-compression-breakout.md)
- Claim type: model validation evidence; no strategy, cost, or edge claim.

## 1. Question

Does a compression breakout predict a **large** forward expansion — a 20-bar
range of at least 3 ATR — on the unseen 5m and 1m cells, after matched
baselines, controls, and a correction?

The prior 2 ATR gate saturated; this protocol raises the outcome threshold to
3 ATR. It does not change the event or its thresholds.

## 2. Data and partitions

| Partition | Window | Instruments | Use |
|---|---|---|---|
| `model_dev` | 2018-01-01..2020-12-31 | BTCUSD, ETHUSD, XAUUSD | Development and selection |
| `model_validation` | 2021-01-01..2022-12-31 | BTCUSD, ETHUSD, XAUUSD | One-shot confirmation |

The development cells use the nine one-year compression bundles already
computed for `MODEL-2026-11-04-02`. Their 5m and 1m events and outcomes have not
been evaluated. The 1h/15m cells from the prior run are excluded.

## 3. Cells and event

Two pre-registered cells:

| Cell | Timeframe | Outcome |
|---|---|---|
| C1 | 5m | forward 20-bar range ≥ 3 ATR |
| C2 | 1m | forward 20-bar range ≥ 3 ATR |

The event is the merged `signal.compression_breakout` (type 3, subtype 7, ATR
source): five prior bars with context code 2, a close beyond the five-bar range
by 0.2 ATR, and RVOL ≥ 1.2 on the trigger bar. The event is unchanged.

## 4. Outcome

For an event at bar `b` with `atr = ATR14[b] > 0` and a full window:

```text
forward_range = max(high[b+1..b+20]) - min(low[b+1..b+20])
hit = forward_range >= 3.0 * atr
```

The event bar's own high and low are not used. Gaps are part of the window.

## 5. Baselines and controls

- **All bars:** every bar with ATR > 0 and a full window.
- **Coiling bars:** every bar with context code 2 and a full window.
- **Placebo transition:** previous context code 2 and current context code 1,
  without a compression-breakout event.
- **Random coiling:** 200 draws of `n` coiling bars per bundle.
- **Back-lag:** entry moved 20 bars before the event; the window ends at the
  event bar.

The event must beat the coiling base and the placebo base; the random and
back-lag controls must pass.

## 6. Estimator and correction

- Cluster-robust variance with clusters formed inside each one-year bundle.
- `lift = hit_rate - coiling_base`; `lift_lcb = lift - z * se`.
- Development uses `m = 2` (the two cells); `z = NormalDist().inv_cdf(1 -
  0.05 / (2 * 2))`.
- Validation uses `m = 1` for the frozen cell; `z = 1.96`.
- A cell with `J <= 1` is inconclusive.

## 7. Pass rule

A development cell passes only when all hold:

1. `n >= 200` and `J >= 20`.
2. `lift >= +0.05` and `lift_lcb > 0`.
3. The event beats the placebo base by at least +0.02, or the placebo base is
   not higher than the coiling base.
4. Random-coiling p ≤ 0.05.
5. Back-lag lift ≤ +0.01, or the event lift exceeds it by ≥ +0.05.
6. At least 2 of 3 instruments have `n >= 30` and a hit rate above coiling.

If the coiling base is above 95%, the cell is **inconclusive (outcome
saturated)** rather than a failure.

## 8. Selection and validation

If neither cell passes, the family records no edge (or inconclusive when a cell
is saturated) and the validation partition is not opened.

If one or both pass, freeze the cell with the higher `lift_lcb`; break ties by
5m (lower cost exposure), then by lower timeframe. Open `model_validation` once
with the same event, outcome, baselines, and controls; apply the same rule with
`m = 1`. A validation pass is a **candidate model baseline pending human
acceptance**, not a strategy go. A validation failure is `development_only` and
stops the program.

## 9. Boundaries

- The outcome is a forward range, not a return, cost, or tradable payoff.
- The 5m/1m cells share the underlying M1 data with the prior 1h/15m result;
  the outcomes are unseen, but residual snooping risk remains.
- No strategy or portfolio work starts from this protocol.
- The harness, review, and decision are same-session passes, not an independent
  Quant Validator record.

## 10. Result

Executed on `model_dev`. Neither cell passes the family rule.

- 1m: 23,257 events, 92.43% hit vs 88.12% coiling, lift +4.31pp, corrected
  lower bound +3.92pp, random p 0.005, placebo +2.30pp, back-lag −4.88pp,
  replication 3. It fails only the +5pp lift threshold, by 0.69pp.
- 5m: 3,798 events, 87.70% hit vs 80.28% coiling, lift +7.43pp, corrected
  lower bound +6.23pp, random p 0.005, back-lag −4.76pp, replication 3. It
  fails the placebo: the transition reaches 86.96%, so the breakout adds
  +0.74pp.

The 1m cell is a positive predictive lead, not a baseline. The validation
partition is not opened.

- Gate: [`HARDER-COMPRESSION-GATE`](../../improvement/2026-11-04/HARDER-COMPRESSION-GATE.md)
- Run record: [`MODEL-2026-11-04-03`](../../improvement/2026-11-04/MODEL-2026-11-04-03-compression-harder.md)
- Local artifacts: `data/improvement/2026-11-04/MODEL-2026-11-04-03/`

## 11. Phase 2 — 4 ATR cells

The 3 ATR family did not pass. The 1m cell is a positive lead: +4.31pp lift, a
+3.92pp corrected lower bound, random p 0.005, placebo pass, and replication 3,
but it misses the 5pp gate by 0.69pp. The 5m cell fails the placebo because the
transition dominates.

A harder target is pre-registered before its outcome is computed:

- **Cells:** 1m and 5m, forward 20-bar range ≥ 4 ATR.
- **Event, baselines, controls, and pass rule:** unchanged.
- **Correction:** m = 2 for the two cells; validation m = 1.
- **Selection:** highest corrected lift LCB among passing cells; 5m tie-break.
- **Disclosure:** the 3 ATR results are known; the 4 ATR outcomes are not. The
  validation partition is the clean one-shot test.
- **Stop:** if neither 4 ATR cell passes, the family records no edge and the
  3 ATR 1m lead remains a descriptive positive result.

The 4 ATR run is a new outcome on the same event, not a change to the event or
its thresholds.

## 12. Final result

The 4 ATR phase selected the 1m cell. It passed development and validation.

| Partition | n | J | Hit | Coiling base | Placebo base | Lift | Lift LCB | Rand p | Back lag | Rep |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `model_dev` | 23,257 | 21,451 | 0.7943 | 0.7260 | 0.7455 | +0.0683 | +0.0622 | 0.0050 | −0.1331 | 3 |
| `model_validation` | 12,397 | 11,686 | 0.7582 | 0.6950 | 0.7294 | +0.0632 | +0.0556 | 0.0050 | −0.1354 | 3 |

Verdict: candidate model baseline pending human acceptance. No strategy or
portfolio work before acceptance.

- Candidate: [`COMPRESSION-BASELINE-CANDIDATE`](../../improvement/2026-11-04/COMPRESSION-BASELINE-CANDIDATE.md)
- Development: [`MODEL-2026-11-04-04`](../../improvement/2026-11-04/MODEL-2026-11-04-04-compression-4atr-dev.md)
- Validation: [`MODEL-2026-11-04-05`](../../improvement/2026-11-04/MODEL-2026-11-04-05-compression-4atr-validation.md)
