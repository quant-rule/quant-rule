# Experiment EXP-2026-11-01-04: Mid-Timeframe Cost/Sample Balance

- Date: 2026-11-01 (UTC)
- Status: Stopped — not green; no edge on the tested sample
- Owner: Governing / Experiment Control (sequential pass)
- Policy: [`config/shared/policy-mid.json`](config/shared/policy-mid.json)
- Strategy: [`config/shared/strategy-mid.json`](config/shared/strategy-mid.json)
- Data: `dukascopy-bid-<instrument>-m1-<run-window>`, filtered from `data/sources/<instrument>_M1_2018-01-01_2024-12-31.csv`
- Tickets: BTCUSD, ETHUSD, XAUUSD
- Timeframes: `15m` execution / `1h` context
- In-sample: evaluation years 2021 and 2022
- Out-of-sample: evaluation year 2024 only; 2023 is excluded by AUD-2026-11-01-70
- Costs: commission 0.1% of equity per operation; slippage 2 bps; funding 0.0001 per 8h; execution fee 0.0
- Iteration cap: one matrix (9 treatment cells); a cell may be rerun only for a harness or config error, and the rerun is recorded
- Stop conditions: missing data, 2024 1h coverage below 90% for BTCUSD/ETHUSD, config error, nondeterminism, look-ahead, incomplete bundles, a critical audit finding, or a change to a threshold after seeing a result
- Control: the 5m/15m cells for 2021, 2022, and 2024 are reused from EXP-2026-11-01-03 with identical configs and data versions
- Quant Validator: not invoked; no green claim and no strategy change is
  proposed. The determinism rerun and data checks are recorded under Results.

## Hypothesis

The 1h/4h treatment was too selective: 9–11 OOS trades per instrument against a
30-trade floor, with a 4h context that loses more than half its bars to the
provider gaps. The 5m/15m control is cost-dominated. The 15m/1h ladder is the
middle point: it seeks moves three times larger than the 5m ladder while keeping
three times more trade opportunities than 1h/4h, and it uses a 1h context that
is more complete than 4h. With a 2 ATR stop and a 2R target, the treatment should
produce at least 30 OOS trades per instrument and overcome the 0.1% commission.
The experiment tests that hypothesis on 2024; it does not claim an edge from
2021–2022.

## Matrix

The matrix is 3 instruments × 1 variant × 3 evaluation years = 9 treatment
cells. The authoritative list is [`matrix-mid.txt`](matrix-mid.txt); the naming
is `<instrument>-mid-<year>`.

| Variant | Ladder | Stop / target | Policy | Execution | Analytics |
|---|---|---|---|---|---|
| `mid` | 15m / 1h | 2.0 ATR / 2.0R | `policy-mid.json` | `execution-mid.json` (15m) | `analytics-mid.json` |

| Instrument | Source | Evaluation years |
|---|---|---|
| BTCUSD | `BTCUSD_M1_2018-01-01_2024-12-31.csv` | 2021, 2022, 2024 |
| ETHUSD | `ETHUSD_M1_2018-01-01_2024-12-31.csv` | 2021, 2022, 2024 |
| XAUUSD | `XAUUSD_M1_2018-01-01_2024-12-31.csv` | 2021, 2022, 2024 |

Every cell runs a 13-month window from 1 December of the prior year through 31
December of the evaluation year. December is causal warm-up: it supplies the 1h
history, and the evaluation metrics start on 1 January. The data version names
the exact run window, for example
`dukascopy-bid-btcusd-m1-2023-12-01_2024-12-31`.

The 5m/15m control is reused from EXP-2026-11-01-03:
`btcusd-control-2021`, `btcusd-control-2022`, `btcusd-control-2024`, and the
matching ETHUSD/XAUUSD cells. Those cells use the same source windows, policy,
portfolio, model, execution, and analytics as a fresh control would; their
config hashes and data versions are recorded in the prior experiment's
`results.json`.

## Metrics

- Full-run final equity, total return, net P&L, maximum drawdown, halt state
- Trades, profit factor, win rate, average trade, exposure, hold bars
- Cost drag: commission + execution fees + funding
- Evaluation-year return and maximum drawdown, rebased on the first equity point
  on or after 1 January
- In-sample 2021–2022 and out-of-sample 2024 aggregates
- OOS net expectancy and a normal-approximation 95% confidence interval
- Stability: positive instruments, contribution share, per-instrument trade count
- Determinism: a separate rerun of one treatment cell, bundle byte comparison

## Green criteria

The `mid` treatment is green only if all of the following hold for the 2024
out-of-sample period:

1. At least two of the three instruments have a positive net return.
2. The equal-weight mean OOS return is at least 10% (the policy's 365-day
   performance target).
3. Every instrument's OOS maximum drawdown is at most 20%, and no cell is halted.
4. Every instrument has at least 30 closed OOS trades.
5. Every positive instrument has an OOS profit factor of at least 1.2.
6. The pooled OOS net-expectancy 95% confidence-interval lower bound is above
   zero.
7. No single instrument contributes more than 60% of the aggregate OOS net P&L.

The reused control is reported for comparison and is not required to pass. A
result that passes 2021–2022 but not 2024 is not green. No criterion may be
changed after the first treatment cell runs.

## OOS method

Each evaluation year is a separate causal run with a 13-month warm-up. Trades
are attributed to the evaluation year by entry time. Equity is rebased to the
first point on or after 1 January, so warm-up P&L is excluded. The OOS period is
2024 only; 2023 is excluded by the data audit. This is a yearly-reset design, not
one continuous run: the model and portfolio restart each January. The limitation
is recorded; it does not leak future data, and no threshold is chosen from OOS.

## Reproduction

```bash
cargo build --release -p quantrule-shell
python3 docs/experiments/2026-11-01/generate_mid_configs.py
QR=target/release/quantrule
$QR policy check --config docs/experiments/2026-11-01/config/btcusd-mid-2024/run.json
QR=$QR tools/experiment/run-matrix.sh \
  docs/experiments/2026-11-01/matrix-mid.txt \
  target/experiment-EXP-2026-11-01-04
python3 docs/experiments/2026-11-01/analyze_mid.py \
  target/experiment-EXP-2026-11-01-04 \
  --json target/experiment-EXP-2026-11-01-04/results-mid.json \
  --text target/experiment-EXP-2026-11-01-04/results-mid.txt
```

The source files are local and not committed. The generator reads no clock,
network, or random source; its output is deterministic.

## Results

All 9 treatment cells completed and wrote five bundles plus `backtest.json`. The
tables use evaluation-year metrics rebased on 1 January; the full-run metrics and
per-cell details are in [`results-mid.json`](results-mid.json) and
[`results-mid.txt`](results-mid.txt).

### In-sample 2021–2022

| Variant | Instrument | 2y return | Max DD | Trades | PF | CI lower | Halted |
|---|---|---|---|---|---|---|---|
| mid | BTCUSD | -20.45% | 23.11% | 81 | 0.474 | -46.67 | no |
| mid | ETHUSD | -16.60% | 20.58% | 83 | 0.641 | -46.75 | no |
| mid | XAUUSD | -11.78% | 11.78% | 54 | 0.087 | -27.85 | no |
| control | BTCUSD | -33.70% | 33.70% | 184 | 0.271 | -25.58 | yes |
| control | ETHUSD | -34.87% | 34.87% | 164 | 0.356 | -32.46 | yes |
| control | XAUUSD | -34.29% | 34.29% | 168 | 0.003 | -23.72 | yes |

### Out-of-sample 2024

| Variant | Instrument | 1y return | Max DD | Trades | PF | CI lower | Halted |
|---|---|---|---|---|---|---|---|
| mid | BTCUSD | -9.07% | 9.31% | 46 | 0.478 | -38.77 | no |
| mid | ETHUSD | -14.88% | 16.55% | 56 | 0.392 | -47.67 | no |
| mid | XAUUSD | -3.86% | 3.86% | 20 | 0.166 | -29.08 | no |
| control | BTCUSD | -19.24% | 19.55% | 96 | 0.187 | -26.08 | yes |
| control | ETHUSD | -18.56% | 18.56% | 98 | 0.259 | -25.04 | yes |
| control | XAUUSD | -15.39% | 15.39% | 70 | 0.031 | -24.71 | no |

The mid ladder is less bad than the control but not profitable: every OOS
instrument is negative, the mean OOS return is `-9.27%`, and only BTCUSD and
ETHUSD reach 30 trades. XAUUSD has 20 trades. The mid treatment improves the
trade count over 1h/4h (46/56 vs 11/9 on BTC/ETH) but does not create a positive
gross edge.

### Green check

| 2/3 positive | Mean >= 10% | DD <= 20% | Trades >= 30 | PF >= 1.2 | CI low > 0 | Contribution <= 60% | Green |
|---|---|---|---|---|---|---|---|
| no | no | yes | no | yes | no | no | no |

The `PF >= 1.2` row is vacuously true because no instrument is positive; it does
not rescue the result. The treatment is not green.

A side diagnostic of the existing trades shows the loss is not one-directional:
long and short are both negative for every instrument in 2021, 2022, and 2024.
The only positive side is ETHUSD short in 2021 (`+1188.99`, 5 wins), which is
one instrument-year and not a stable edge. A long-only or short-only variant is
not a rescue on this sample.

### Diagnosis and data quality

| Instrument | 1m coverage | 15m coverage | 1h coverage | OOS trades |
|---|---|---|---|---|
| BTCUSD | 98.55% | 95.44% | 91.76% | 46 |
| ETHUSD | 98.80% | 95.99% | 91.76% | 56 |
| XAUUSD | 67.07% | 66.93% | 66.51% | 20 |

The XAUUSD percentages are calendar coverage including weekends; the weekday
session is present, and the 20 OOS trades are a frequency limit, not a data
blocker. The 2024 1h coverage is `91.76%` for BTCUSD/ETHUSD, above the
pre-registered `90%` threshold.

`btcusd-mid-2024` diagnosis:

| Check | Observed |
|---|---|
| Entry attempts / fills | 48 / 48 |
| Gross expectancy | -1.70 (95% CI [-20.27, 16.88]) |
| Net expectancy | -20.77 (95% CI [-39.34, -2.20]) |
| Exit reasons | strategy 28, stop 15, target 5 |
| Gross profit factor | 0.932 |
| Net profit factor | 0.452 |

### Determinism

`btcusd-mid-2024` was rerun into a separate root. All five bundles (`model`,
`strategy`, `execution`, `portfolio`, `analytical`) were byte-identical.

## Triage

| Area | Finding | Route |
|---|---|---|
| Strategy | The 15m/1h ladder produces enough trades for BTCUSD/ETHUSD (46/56) but every OOS instrument is negative; the mean OOS return is -9.27%. The gross edge is not positive and costs amplify it. | No edge for this rule on the tested sample. Stop; a new rule/hypothesis is a new work item. |
| Strategy | XAUUSD produces only 20 OOS trades, below the 30-trade floor. | The green criteria fail independently of the negative returns. |
| Data | 2024 BTCUSD/ETHUSD 1h coverage is 91.76%; the 2023 data blocker is excluded. | No data blocker for this 2024 test; 2023 remains open in AUD-2026-11-01-70. |
| Program | No program defect found; the rerun is deterministic and the diagnosis is consistent. | No audit finding from this matrix. |

This is a clean `not green` and a **no-edge** result for the `ladder-context` rule
on the 2021–2022/2024 sample: the in-sample years are also negative, and the
out-of-sample year is negative on all three instruments. The 2023 data gap
remains an open data finding, but it is not needed to reject this hypothesis.

## Decision

Stop at the pre-registered cap. Do not change a threshold, cost, split, or
sample after seeing the result. No edge or profitability claim is made.

The ladder family has now been tested at 5m/15m, 15m/1h, and 1h/4h over
2021–2024 (2023 crypto excluded by the data audit). None is green. A further
attempt on this rule would be threshold tuning; a genuinely different hypothesis
would need a new rule, a new pre-registration, and human approval. The data
audit `AUD-2026-11-01-70` remains `Open` for any future 2023 work.

## Limitations

- One OOS year (2024), a bull regime for crypto and gold; there is no
  bear-market OOS. The 2021–2022 in-sample years are negative, so the result is
  not a single-regime artifact.
- 2023 is excluded by AUD-2026-11-01-70; the source is provider-limited.
- 2024 BTCUSD/ETHUSD 1h coverage is 91.76%; XAUUSD's 66.51% is calendar
  coverage including weekends, not missing weekday sessions.
- Yearly runs reset the model and portfolio; December is warm-up and the first
  January days may lack 1h context.
- The 0.1% commission is conservative for liquid venues; a failure does not
  prove the rule is unprofitable at lower costs. It does not explain a negative
  gross edge: the BTCUSD gross expectancy CI straddles zero and the gross
  profit factor is below 1.
- No R-multiple field, no partial fills, no market impact, and no capacity
  estimate.
- No independent Quant Validator was invoked because no green claim or strategy
  change is proposed. The rerun is a same-agent determinism check, not an
  independent review.

