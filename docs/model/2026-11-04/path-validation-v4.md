# Model v4 Path-Validation Protocol

- Date: 2026-11-04 (UTC)
- Status: **Pre-registered and executed; no edge; validation not opened**
- Epic: [#369](https://github.com/quant-rule/quant-rule/issues/369)
- Plan: [`docs/execution-plans/2026-11-04/369-model-v4.md`](../../execution-plans/2026-11-04/369-model-v4.md)
- Contract: [`docs/model/2026-11-05/model-m1.0.md`](../2026-11-05/model-m1.0.md)
- Prior protocol: [`docs/model/2026-11-03/path-validation.md`](../2026-11-03/path-validation.md)
- Harness: `tools/improvement/path_validation.py`
- Claim type: model validation evidence; no strategy, cost, or edge claim.

## 1. Question

Does any v4 event family reach a pre-registered ATR target before an ATR stop
more often than (a) a direction-matched unconditional barrier baseline and
(b) the state the event claims to mark, after a multiple-testing correction,
negative controls, and instrument replication?

The model output and signal path do not change. This protocol is evaluation only.

## 2. Data and partitions

| Partition | Window | Instruments | Use |
|---|---|---|---|
| `model_dev` | 2018-01-01..2020-12-31 | BTCUSD, ETHUSD, XAUUSD | Development and selection |
| `model_validation` | 2021-01-01..2022-12-31 | BTCUSD, ETHUSD, XAUUSD | One-shot confirmation |

Each year is a separate one-year model run because the M1 window is bounded at
600,000 candles. Timeframes are 5m, 15m, and 1h. The 1m frame is retained for
charting only and is not a hypothesis cell. 2023 is excluded. The validation
partition is opened once, after the candidate is frozen.

## 3. Hypotheses and config matrix

Four event families are pre-registered. Each is tested under two model settings
that differ only in the ADX threshold (`20` and `25`); the threshold changes the
trend-continuation conjunction and the published `adx_regime` state, and leaves
the other event families unchanged. The eight config-hypothesis pairs are:

| ID | Hypothesis | Event family | Primary config | Sensitivity config |
|---|---|---|---|---|
| H1 | A trend-continuation trigger predicts continuation | type 3, subtype 6, source 16 | `adx25` | `adx20` |
| H2 | A MACD line/signal cross predicts a move in the cross direction | type 3, subtype 2, source 13 | `adx25` | `adx20` |
| H3 | An RSI 50 cross predicts continuation in the cross direction | type 3, subtype 1, source 4, reference 50 | `adx25` | `adx20` |
| H4 | A close beyond an EMA by 0.2 ATR predicts continuation | type 1, subtype 3, sources 1/2/3 | `adx25` | `adx20` |

`adx25` is the primary setting. `adx20` is a pre-registered sensitivity check,
not a second hypothesis: the multiplicity correction covers the 12 primary
family-timeframe cells (4 families × 3 timeframes), not the duplicate
sensitivity cells. For H2, H3, and H4 the two settings must produce identical
event sets; a difference is a config-isolation defect, not a result.

### Economic rationale

- **H1 — trend continuation.** When ADX is above the threshold and rising, the
  DI direction is non-neutral, and the EMA stack and MACD state agree, the
  market is in an established trend. The event marks the bar the conjunction
  becomes true; the claim is that the established trend persists rather than
  mean-reverting at that moment.
- **H2 — MACD line/signal cross.** A MACD cross is a momentum-shift
  confirmation. The claim is that the cross marks a directional move in the
  cross direction, not merely a change in the indicator's slope.
- **H3 — RSI 50 cross.** The RSI midline separates the average gain from the
  average loss. A cross of 50 is a shift in that balance; the claim is that the
  shift continues in the cross direction.
- **H4 — EMA level break.** A close that crosses an EMA and finishes at least
  0.2 ATR beyond it is a confirmed break, not a touch. The claim is that the
  confirmation adds continuation beyond simply being on one side of the EMA.

## 4. Event preparation

1. Read the per-bar `readings.csv` and `events.csv` of each one-year bundle.
2. Keep events with `direction_code` in `{-1, +1}`. Drop neutral events.
3. One vote per bar: group by `available_bar_index`. If the bar has conflicting
   directions, drop it as ambiguous; otherwise keep the event with the lowest
   `event_index`.
4. The entry bar is the availability bar. The path starts on the next bar.
5. Valid path: ATR 14 is non-null and greater than zero, and the bundle has at
   least 20 future bars. Report raw, prepared, and valid counts.

## 5. Path outcome

For an event at bar `b`, direction `d`, entry `close[b]`, and `atr = atr[b]`:

```text
target = close[b] + d * 1.0 * atr
stop   = close[b] - d * 1.0 * atr
```

Walk bars `b + 1` through `b + 20`:

- Long (`d = +1`): if `low <= stop` and `high >= target` on the same bar, count
  a stop first. Otherwise the first stop touch is a miss and the first target
  touch is a hit.
- Short (`d = -1`): if `high >= stop` and `low <= target` on the same bar, count
  a stop first. Otherwise the first stop touch is a miss and the first target
  touch is a hit.
- Neither touched within the horizon is a timeout, counted as a miss.

The event bar's own high and low are not used. Gaps are part of the path. The
horizon is a bar count, not a clock duration.

## 6. Baselines

### 6.1 Unconditional barrier baseline

For every bar with a valid ATR and at least 20 future bars, compute the same
symmetric barrier for a long and a short entry at that bar's close. The long and
short base rates are computed per one-year bundle. The event base is the
direction-weighted average across the selected events' bundles.

### 6.2 State-conditioned baseline

The event claims a state, so the incremental test is whether the trigger adds
information beyond that state. The state baseline is the same barrier computed
on all bars where the claimed state holds, in the event direction:

| Family | State |
|---|---|
| H1 | `adx_regime = 1`, `adx_slope = 1`, `di_state = d`, `ema_stack = d`, `macd_state = d` |
| H2 | `macd_state = d` |
| H3 | `(rsi - 50) * d > 0` |
| H4 | `(close - ema_source) * d > 0`, using the event's source EMA |

The state base is event-weighted across bundles and sources. H1 is the published
conjunction itself; the event marks the bar on which it becomes true. H2, H3,
and H4 claim a persistent state, and the event marks its onset.

## 7. Estimator

- `p = hits / n` and `lift = p - base`.
- Within each one-year bundle, sort valid events by bar. Start a new cluster
  when the bar is at least 20 bars after the previous event bar. Clusters never
  span bundles.
- With `J` clusters, `n_j` events and `h_j` hits in cluster `j`, the pooled
  cluster-robust variance is

```text
var = sum_bundles (J_b / (J_b - 1)) * sum_j (h_j - p * n_j)^2 / n^2
se  = sqrt(var)
```

- `lift_lcb = lift - z * se`; `incremental_lift = p - state_base` and
  `incremental_lcb = incremental_lift - z * se`.
- `z = NormalDist().inv_cdf(1 - 0.05 / (2 * 12))` on development (Bonferroni over
  12 primary cells) and `z = 1.96` on validation (one cell).
- If `J <= 1`, the cluster-robust bound is undefined and the cell is
  inconclusive. The naive `sqrt(p*(1-p)/n)` standard error is reported for
  reference only.

## 8. Negative controls

Each control is computed per family, timeframe, and instrument bundle, then
pooled with the event base.

| Control | Definition | Pass condition |
|---|---|---|
| Random | 200 draws of `n` distinct valid bars, direction mix drawn from the event directions | event `p` > 95th percentile; `p_value <= 0.05` |
| Back-lag | entry moved 20 bars before the event; the path ends at the event bar | back-lag lift `<= +0.01`, or event lift exceeds it by `>= +0.01` |
| Shuffled | 200 permutations of the event directions at the same bars | event `p` > 95th percentile; `p_value <= 0.05` |

The back-lag is a placebo: it cannot see the event. A forward lag of 20 bars is
reported as a persistence diagnostic and is not a pass condition. All controls
use seed `0`, are deterministic, and are reported in full.

## 9. Pass rule and grades

A cell is a family × timeframe under `adx25`. It passes only when all of the
following hold on `model_dev`:

1. `n >= 200` valid events and `J >= 20` clusters.
2. `lift >= +0.01` and `lift_lcb > 0`.
3. `incremental_lift >= +0.01` and `incremental_lcb > 0`.
4. Random control passes.
5. Back-lag control passes.
6. Shuffled control passes.
7. At least 2 of 3 instruments have `n >= 30` and `hit_rate > base`.
8. The `adx20` sensitivity cell has `lift > 0` (H1) or an identical event set
   and hit rate (H2, H3, H4).

| Grade | Rule |
|---|---|
| A | All conditions; `lift >= 0.02`; replication 3 |
| B | All conditions; `lift >= 0.01`; replication 2 |
| C | Anything else, or a control is inconclusive |

Only A and B can gate the validation partition. C is diagnostic.

## 10. Selection and one-shot validation

If no cell is A or B, the program stops with **no edge**; the validation
partition is not opened and no threshold, timeframe, horizon, or family is
selected after the result.

If one or more cells are A or B, freeze the cell with the highest
`incremental_lcb`; break ties by lower timeframe (5m, 15m, 1h) and then by
family ID. Copy its exact model config into the validation run. Open
`model_validation` once and apply the same rule with `m = 1`. A validation pass
is a **candidate model baseline pending human acceptance**, not an accepted
baseline and not a strategy go. A validation failure is `development_only` and
also stops the program.

## 11. Boundaries

- Entry is the event close; no costs, slippage, funding, or execution.
- The baseline is a barrier baseline, not a matched price level.
- A path pass is model evidence, not a profitability claim.
- Statistics is used only to evaluate a rule-based event; it never enters the
  model or a strategy.
- The harness, review, and decision are same-session passes, not an independent
  Quant Validator record.

## 12. Result

Executed on `model_dev` (BTCUSD, ETHUSD, XAUUSD; 2018–2020; 5m/15m/1h). All 12
primary family-timeframe cells are grade C. No cell passes the matched-base
lift, the state-conditioned incremental lift, the three controls, and
two-instrument replication together. The validation partition was not opened.

- Gate: [`MODEL-V4-GATE`](../../improvement/2026-11-04/MODEL-V4-GATE.md)
- Run record: [`MODEL-2026-11-04-01`](../../improvement/2026-11-04/MODEL-2026-11-04-01-model-v4-dev.md)
- Local artifacts: `data/improvement/2026-11-04/MODEL-2026-11-04-01/`

## 13. Harness extension — `state: none`

Added after the v4 gate for the five 15m event-family briefs. A family that
makes no state claim declares `"state": "none"`. The state-conditioned
incremental test is not applicable: `state_rate`, `incremental_lift`, and
`incremental_lcb` are null, and the grade does not require them. The cell is
still graded on the unconditional lift, the cluster-robust lower bound, the
random control, the back-lag control, the shuffle control, and instrument
replication. The historical v4 gate above used state-bearing families and is
unchanged.
