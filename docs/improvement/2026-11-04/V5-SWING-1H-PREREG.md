# V5 1h Swing Model Gate — Pre-registration

- Date: 2026-11-04 (UTC)
- Status: **Pre-registered; frozen before the development gate**
- Program: `PROGRAM-2026-11-04-04`
- Run: `MODEL-2026-11-04-18` (`model_dev`)
- Charter: [`docs/governance/2026-11-04/v5-program-charter.md`](../../governance/2026-11-04/v5-program-charter.md)
- Partition: `model_dev` 2018–2020
- Instruments: BTCUSD, ETHUSD, XAUUSD
- Claim type: model research evidence; no strategy, cost, or edge claim

## 1. Question

Does a 1h v4 event family reach a **4 ATR target before a 2 ATR stop within 100
1h bars** more often than a direction-matched 1h barrier base, after a
multiplicity correction, negative controls, and instrument replication?

This tests the horizon explanation for the v4 no-edge result. It is not another
15m event variant: the event set, timeframe, horizon, and payoff are different,
and the test is pre-registered before the run.

## 2. Cells

| Cell | Family ID | Event | State | Primary |
|---|---|---|---|---|
| `CELL-01` | `h1_trend_continuation` | type 3, subtype 6, source 16 | `trend` | yes |
| `CELL-02` | `h1_major_pivot_break` | type 1, subtype 3, source 8 | `none` | yes |
| `CELL-03` | `h1_ema50_test` | type 1, subtype 1, source 2 | `ema_side` | yes |

Family definitions: `config/v5-swing/1h/families.json`.
Matrix: `config/model-v4/dev/matrix.json` (`adx25` primary, `adx20`
sensitivity). The families are not changed after the result is seen.

The family IDs are `h1_*` because the event definitions are the v4 definitions
unchanged; only the evaluation horizon and payoff are new. The harness keys its
`adx20` sensitivity rule on `h1_trend_continuation`, so the ID is reused rather
than renamed. It is the same event, not a new hypothesis with a new event.

`CELL-02` makes no state claim: a major-pivot break is tested on the
unconditional base only. `CELL-01` and `CELL-03` also clear the
state-conditioned incremental test, because they claim that the trigger adds
information beyond the state itself.

## 3. Sample pre-flight

Raw 1h event counts on `model_dev` before the gate:

| Cell | `adx25` | `adx20` |
|---|---:|---:|
| `CELL-01` trend continuation | 919 | 1124 |
| `CELL-02` major pivot break | 3126 | 3126 |
| `CELL-03` EMA50 test | 6722 | 6722 |

All three cells clear `n >= 200` by a wide margin. The valid count at the gate is
lower by at most the events in the final 100 bars of each one-year bundle, so the
pre-flight passes. Had any cell been below 200, the gate would not run.

## 4. Barrier

For an event at availability bar `b`, direction `d`, entry `close[b]`, and
`atr = atr[b]`:

```text
target = close[b] + d * 4.0 * atr
stop   = close[b] - d * 2.0 * atr
```

Walk bars `b + 1` through `b + 100`:

- a stop touch is a miss; a target touch is a hit;
- if both are touched on the same bar, the stop wins (`stop_first`);
- neither touched within 100 bars is a timeout, counted as a miss;
- a missing or zero ATR makes the bar invalid.

The event bar's own high and low are not used. The entry is the event close, as
in the v4 protocol; the strategy tier will enter at the next open and will
therefore face a worse entry and the full costs.

## 5. Baselines and controls

| Item | Rule |
|---|---|
| Unconditional base | the same barrier on every valid 1h bar, direction-matched, event-weighted across bundles |
| State base | `trend`: the published trend conjunction; `ema_side`: `(close - ema50) * d > 0`; `none`: not applicable |
| Random | 200 draws of the event count from the valid 1h bars, direction mix from the events |
| Back-lag | entry moved 100 bars before the event; pass when back-lag lift `<= +0.01` or event lift exceeds it by `>= +0.01` |
| Shuffle | 200 permutations of the event directions at the same bars |
| Seed | 0 |
| Clusters | cluster break at 100 bars (the horizon) inside each bundle; clusters never span bundles |

The 100-bar cluster gap is deliberate: paths overlap for 100 bars, so events
inside one path window are not independent evidence. This is stricter than the
20-bar default.

## 6. Correction and pass rule

Bonferroni over the three primary cells: `z = NormalDist().inv_cdf(1 - 0.05/6)`
= `2.394`.

A cell passes only when all hold:

1. `n >= 200` and `J >= 20` clusters.
2. `lift >= +0.05` over the direction-matched 1h base.
3. `lift_lcb > 0` at `z = 2.394`.
4. For state-bearing cells, `incremental_lift >= +0.01` and
   `incremental_lcb > 0` (the harness grade requirement).
5. Random control `p <= 0.05`.
6. Back-lag control passes.
7. Shuffle control `p <= 0.05`.
8. At least two instruments have `n >= 30` and `hit_rate > base`.
9. `adx20` sensitivity: `lift > 0` for `CELL-01`; identical event set for
   `CELL-02` and `CELL-03`.

The harness grade `A`/`B` is necessary. The `+0.05` threshold is the
pre-registered promotion rule; a grade A/B cell below `+0.05` is
`development_only` and the program stops.

## 7. Decision

| Outcome | Action |
|---|---|
| No cell passes | Record **no edge**; stop Phase 1; do not tune; Phase 2 needs human approval |
| A cell passes | Freeze it and open `model_validation` once with `--config adx25 --skip-sensitivity --primary-cells 1` |
| Validation passes | Candidate model baseline pending human acceptance; no strategy run without acceptance |
| Validation fails | `development_only`; stop the program |
| Sample below 200 | `inconclusive — underpowered`; stop before the gate |

## 8. Command

Development gate:

```bash
python3 tools/improvement/path_validation.py \
  --matrix config/model-v4/dev/matrix.json \
  --families config/v5-swing/1h/families.json \
  --out data/improvement/2026-11-04/MODEL-2026-11-04-18/artifacts \
  --timeframes 1h --horizon 100 --target-atr 4 --stop-atr 2 \
  --back-lag 100 --primary-cells 3 --seed 0
```

Validation (only if a dev cell passes):

```bash
python3 tools/improvement/path_validation.py \
  --matrix config/model-v4/validation/matrix.json \
  --config adx25 --skip-sensitivity --primary-cells 1 \
  --families config/v5-swing/1h/families.json \
  --out data/improvement/2026-11-04/V5-SWING-1H-VALIDATION/artifacts \
  --timeframes 1h --horizon 100 --target-atr 4 --stop-atr 2 \
  --back-lag 100 --seed 0
```

## 9. Boundaries and limitations

- Entry is the event close; no costs, slippage, funding, or execution. The model
  gate is pre-cost by construction.
- Yearly folds are pooled per instrument; regime changes inside a partition are
  not separated.
- XAUUSD carries market-hours gaps; the path includes them.
- The 1h sample has fewer independent clusters than events because the horizon
  is long; this is reflected in the cluster-robust bound.
- No independent Quant Validator record for a no-edge result. A candidate
  baseline requires one before a strategy is called promising.
- No model or strategy code changes. No validation access without a passing dev
  gate and human approval.
