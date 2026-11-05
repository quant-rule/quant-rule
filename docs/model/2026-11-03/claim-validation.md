# Claim Validation Protocol (Model Gate)

- Date: 2026-11-03 (UTC)
- Status: Pre-registered and executed for `MODEL-2026-11-03-01..02`; result **no edge**
- Issue: [#348](https://github.com/quant-rule/quant-rule/issues/348)
- Harness: `tools/improvement/claim_validation.py`
- Contract: [`docs/model/2026-11-03/model-v3.md`](../../registry/history.md)

## 1. Question

Does any signal-role model state or event have a forward outcome that differs
from the unconditional base rate, after a multiple-testing correction, with the
same sign replicated across instruments and across the development and
validation partitions?

This is a predictive-validity gate. It does not include trading costs; a passing
family is a candidate for a strategy run, not an edge claim.

## 2. Data

| Partition | Window | Instruments |
|---|---|---|
| `model_dev` | 2018-01-01..2020-12-31 | BTCUSD, ETHUSD, XAUUSD |
| `model_validation` | 2021-01-01..2022-12-31 | BTCUSD, ETHUSD, XAUUSD |

Each year is a separate one-year model run because the M1 window is bounded at
600,000 candles. The harness pools the years per instrument. Timeframes are 5m,
15m, and 1h. The model config is `config/defaults/model.json` with MACD active.

## 3. Primary cells

One primary horizon per family:

| Family | Primary horizon | Outcome |
|---|---:|---|
| EMA stack state | 5 bars | sign of forward return equals the stack state |
| RSI zone | 5 bars | sign of forward return equals the zone |
| MACD state | 5 bars | sign of forward return equals the state |
| MACD histogram sign | 5 bars | sign of forward return equals the sign |
| Progression leg direction | 10 bars | sign of forward return equals the leg direction |
| Level test/cross/break | 5 bars | sign of forward return equals the event direction |
| RSI divergence (4 subtypes) | 5 bars | sign of forward return equals the event direction |
| RSI threshold crossing | 5 bars | sign of forward return equals the event direction |
| EMA series crossover | 5 bars | sign of forward return equals the event direction |
| MACD line/signal crossover | 5 bars | sign of forward return equals the event direction |
| MACD histogram slope flip | 5 bars | sign of forward return equals the event direction |
| MACD zero cross | 5 bars | sign of forward return equals the event direction |

Horizons 1, 3, 10, and 20 are secondary and are reported but not graded.

## 4. Statistics

- `hit_rate = hits / n`, where a hit means the forward-return sign equals the
  state or event direction.
- `base_rate = positive forward returns / all forward returns` for the same
  timeframe, instrument, and horizon, independent of the event.
- `lift = hit_rate - base_rate`.
- The lower bound is the one-sided normal approximation
  `p - z * sqrt(p(1-p)/n)`, clamped at zero.
- The primary family count is `m`; the grading bound uses
  `z = NormalDist().inv_cdf(1 - 0.05/(2m))` (Bonferroni).
- A raw 95% bound is reported for reference; grading uses the corrected bound.
- Coverage is `n / observations with a computable forward outcome`.
- Replication counts instruments with `n >= 30` and `hit_rate > 0.50`.

## 5. Grades

| Grade | Rule |
|---|---|
| A | coverage >= 0.95, corrected LCB > 0.55, lift >= 0.02, replication >= 3 |
| B | coverage >= 0.90, corrected LCB > 0.50, lift >= 0.01, replication >= 2 |
| C | anything else, or the metric is not computed |
| D | coverage < 0.80 |

A and B are the only grades that can gate a strategy. C is diagnostic. D blocks.

## 6. Gate

`MODEL-2026-11-03-01` runs `model_dev`. `MODEL-2026-11-03-02` runs the one-shot
`model_validation` partition. The gate passes only when the same signal-role
family is A or B on both partitions. A family that passes on development but
fails on validation is rejected.

If no family passes, the model is descriptive. The next step is a new thesis or
a stop, not a threshold change.

## 7. Result (2026-11-03)

- Development: 48 primary cells, all grade C.
- Validation: 47 grade C and one B, `divergence.regular_bullish` 5m.
- The B cell did not replicate on development, which was C for the same cell.
- Combined gate: **no edge**. No signal-role family passed both partitions.

## 8. Limitations

- The model bundle's `readings.csv` includes the readings and states; the event
  batch is in `events.csv`.
- The harness measures sign outcomes, not magnitude or cost.
- Yearly folds are pooled; regime changes inside a partition are not separated.
- The same-session harness and review are sequential passes, not an independent
  Quant Validator record.
