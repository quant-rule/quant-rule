# Model Prediction Evaluation

- Date: 2026-11-05 (UTC)
- Status: **Implemented** (P2); no edge claim
- Research contract: [`docs/analytics/2026-10-24/research-output.md`](../../registry/history.md)
- Plan: [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](../../execution-plans/2026-11-05/410-m2-model-programme.md)
- Implementation: `tools/improvement/model_prediction.py`,
  `tools/research/adapters.py`
- Claim type: evaluation contract; the evaluator joins a forward outcome after
  the model run and does not feed a rule

## 1. Purpose and boundary

The evaluator answers one question: does a model version's causal prediction
carry more information about the forward path than a direction-matched
unconditional base, after costs, and does it survive the controls?

It is not a strategy. It does not open a position, size a position, compute a
fill, or account for cash. It reads `prediction.csv` and `readings.csv`, joins
the forward path, and writes `model_prediction.json` and
`model_prediction.md`. The forward outcome is never read by the model.

## 2. Inputs

| Input | Rule |
|---|---|
| `prediction.csv` | `timeframe_secs,bar,timestamp,prediction,long_votes,short_votes`; prediction is `-1`, `0`, or `1` |
| `readings.csv` | The model bundle's per-bar readings; `atr` is required for the outcome |
| `provenance.json` | Optional; supplies `spec.id`, `spec.family`, and `spec.instrument` |

A bundle directory contains `prediction.csv` and `readings.csv`; the evaluator
accepts one bundle or several. A relative path is resolved by the shell that
invokes the evaluator.

## 3. Outcome modes

### Forward ATR-normalized return

For a prediction at bar `t` with direction `d`:

```text
gross_r = d * (close[t + H] - close[t]) / atr[t]
cost_r  = cost_bps / 10000 * close[t] / atr[t]
net_r   = gross_r - cost_r
hit     = net_r > 0
```

### Barrier target/stop

For a prediction at bar `t` with direction `d`:

```text
target = close[t] + d * target_atr * atr[t]
stop   = close[t] - d * stop_atr * atr[t]
```

Scan `t+1 .. t+H`. The stop wins when both are touched in the same bar. If
neither is touched, the timeout uses the close at `t+H`. The gross R is
`+target_atr`, `-stop_atr`, or the timeout return in ATR units; the cost hurdle
is subtracted before the hit test.

A prediction is valid only when the ATR is finite and positive and the full
horizon exists. Invalid or missing bars are counted in the raw prediction count
but not in `n`.

## 4. Split discipline

The evaluator accepts the declared partition and optional UTC bounds:

```text
--partition model_dev --split-start 2018-01-01 --split-end 2020-12-31
```

A prediction outside the bounds is purged. A prediction whose outcome exit bar
is after `split_end` is purged as well, so a development outcome cannot cross
into validation. The evaluator does not open a partition; the caller must feed
the readings for the declared partition. The `purged` count and the split bounds
are recorded in the output.

Overlapping outcomes are clustered by entry bar with a horizon gap. A
prediction whose full horizon is absent from the readings is not evaluated.

## 5. Base rate and lift

The base rate is direction-matched and unconditional within the evaluated
partition. For each direction, the evaluator computes the outcome on every valid
bar. The prediction base is the prediction-weighted mean of the two direction
base rates.

```text
hit_rate = hits / n
base_rate = weighted direction base
lift = hit_rate - base_rate
```

The prediction outcome is compared with the same forward rule that the base
uses, so the lift is not an artefact of a different outcome definition.

## 6. Controls and uncertainty

| Control | Method |
|---|---|
| Random entries | Sample the same long/short counts from the valid bars of each direction, 200 draws by default |
| Direction shuffle | Shuffle the prediction directions among the evaluated bars, 200 draws by default |
| Back-lag | Use the outcome at `t - back_lag` with the same direction |

`random_p` and `shuffle_p` use the add-one empirical tail:
`(1 + draws >= observed) / (draws + 1)`. The default seed is `0`, and the
shuffle uses `seed + 1`.

`clusters` are formed by entry bar within each bundle; two predictions closer
than the horizon share a cluster. The cluster-robust standard error gives the
lift lower bound. When every prediction is its own cluster, the cluster variance
is undefined; the evaluator falls back to the Wilson lower bound and records
`lcb_method: "wilson"`.

`corrected_alpha = alpha / primary_cells`; the default alpha is `0.05` and the
default primary-cell count is `5`. The lower bound uses the corresponding
two-sided z quantile.

## 7. Grade

| Grade | Rule |
|---|---|
| `A` | Sample, lift, controls, back-lag, and replication pass |
| `B` | Sample, lift, controls, and back-lag pass; replication does not |
| `C` | Sample and positive lift pass; at least one of lift, controls, or back-lag fails |
| `D` | Sample fails, lift is missing, or lift is not positive |

`verdict` is `pass` for `A` or `B`, and `fail` for `C` or `D`.

| Criterion | Default |
|---|---|
| `min_events` | 200 |
| `min_clusters` | 20 |
| `min_lift` | 0.02 |
| `lift_lcb` | > 0 |
| `random_p` | <= corrected alpha |
| `shuffle_p` | <= corrected alpha |
| Back-lag | `back_lag_lift <= 0.01` or `lift - back_lag_lift >= 0.01` |
| Replication | at least 2 instruments with `n >= 30` and positive lift |

A single instrument cannot satisfy replication; it can grade `B`, not `A`.

## 8. Output

`model_prediction.json`:

| Field | Content |
|---|---|
| `schema` | `quantrule.model-prediction/v1` |
| `spec_id`, `family`, `instrument` | Model identity; `pooled` when several bundles are evaluated |
| `timeframe_secs`, `partition`, `split_start`, `split_end`, `horizon` | Evaluation identity |
| `outcome`, `target_atr`, `stop_atr`, `cost_bps` | Outcome contract |
| `primary_cells`, `alpha`, `corrected_alpha`, `z` | Correction |
| `seed`, `random_draws`, `shuffle_draws`, `back_lag` | Controls |
| `criteria`, `criteria_status` | Declared and observed thresholds |
| `metrics` | n, raw predictions, coverage, purged, clusters, hits, hit rate, base, lift, LCB, controls, back-lag, replication, instruments, folds |
| `grade`, `verdict` | The decision |
| `limitations` | The residual limits of the evaluation |
| `bundles` | The input bundle names and paths |

`model_prediction.md` is the human summary with the headline metrics and the
per-instrument/per-year tables.

The output is deterministic: the same inputs and arguments produce byte-identical
JSON and Markdown. All maps are written in sorted key order; the control seeds
are explicit.

## 9. Command

```bash
python3 tools/improvement/model_prediction.py \
  --bundle XAU=data/research/M2/dev/CELL-M2.1 \
  --bundle BTC=data/research/M2/dev/CELL-M2.1-btc \
  --spec-id M2.1 --family M2 \
  --partition model_dev \
  --horizon 20 --target-atr 2.0 --stop-atr 1.0 --cost-bps 6 \
  --primary-cells 5 --seed 0 \
  --out data/research/M2/dev/CELL-M2.1-eval
```

The same evaluator accepts a single pair:

```bash
python3 tools/improvement/model_prediction.py \
  --predictions path/to/prediction.csv \
  --readings path/to/readings.csv \
  --instrument XAU \
  --partition model_dev \
  --out path/to/eval
```

The research adapter registers the output as `model_prediction`; a cell with
`"result_parser": "model_prediction"` reads `metrics`, `grade`, and `verdict`
from `model_prediction.json`.

## 10. Verification

| Test | File |
|---|---|
| Outcome, cost, stop-first tie, short history | `tools/improvement/test_model_prediction.py` |
| Known edge passes; lagged prediction fails; empty prediction fails safely | `tools/improvement/test_model_prediction.py` |
| Determinism, per-instrument, per-year | `tools/improvement/test_model_prediction.py` |
| Adapter parsing | `tools/research/test_adapters.py` |

Observed on 2026-11-05 in the `feat/455-prediction-evaluator` worktree:

```text
python3 -m unittest discover -s tools/improvement -p 'test_*.py' -> 135 passed
python3 -m unittest discover -s tools/research -p 'test_*.py'     -> 33 passed
```

## 11. Limitations

- The evaluator is a filter, not proof of a live edge. It does not model fills,
  position sizing, funding, borrow, or portfolio constraints.
- The base rate is unconditional within the partition; a state-conditioned base
  remains a later model-gate refinement.
- The Wilson fallback is used only when every prediction is its own cluster; the
  `lcb_method` field makes that visible.
- A single instrument can grade `B`; the M2 programme requires replication before
  a candidate is called promising.
- The evaluator does not open the validation partition. The caller records the
  partition and the human approves the one-shot access.
