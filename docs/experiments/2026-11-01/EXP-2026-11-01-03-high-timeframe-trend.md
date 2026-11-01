# Experiment EXP-2026-11-01-03: High-Timeframe Trend Continuation

- Date: 2026-11-01 (UTC)
- Status: Stopped — not green; data and sample-size blockers
- Owner: Governing / Experiment Control (sequential pass)
- Policy: [`config/shared/policy-high.json`](config/shared/policy-high.json) for the treatments; [`config/shared/policy-control.json`](config/shared/policy-control.json) for the control
- Strategy: [`config/shared/strategy-control.json`](config/shared/strategy-control.json), [`config/shared/strategy-t2.json`](config/shared/strategy-t2.json), [`config/shared/strategy-t3.json`](config/shared/strategy-t3.json)
- Data: `dukascopy-bid-<instrument>-m1-<run-window>`, filtered from `data/sources/<instrument>_M1_2018-01-01_2024-12-31.csv`
- Tickets: BTCUSD, ETHUSD, XAUUSD
- Timeframes: control `5m` execution / `15m` context; treatments `1h` execution / `4h` context
- In-sample: evaluation years 2021 and 2022
- Out-of-sample: evaluation years 2023 and 2024
- Costs: commission 0.1% of equity per operation; slippage 2 bps; funding 0.0001 per 8h; execution fee 0.0
- Iteration cap: one matrix; a cell may be rerun only for a harness or config error, and the rerun is recorded
- Stop conditions: missing data, config error, nondeterminism, look-ahead, incomplete bundles, a critical audit finding, or a change to a threshold after seeing a result
- Quant Validator: not invoked; no green claim and no strategy change is
  proposed. The determinism rerun and data checks are recorded under Results.

## Hypothesis

The 5m/15m control is dominated by turnover: in the published BTCUSD 2024
baseline the gross P&L was `+125.46` while commission was `-2085.46`, a cost drag
of `16.66×` gross. A higher-timeframe ladder should make each trade seek a larger
move, so the same percentage cost consumes a smaller share of the gross edge. The
treatment is a trend-continuation rule on `1h` execution with `4h` context,
using the same entry thresholds, exits, and risk intent as the control, but a
wider bracket: 2 ATR stop / 2R target (t2) and 3 ATR stop / 3R target (t3). The
sensitivity pair distinguishes a real time-frame effect from a single bracket
choice. The experiment tests whether either treatment meets the policy objective
out-of-sample on three instruments; it does not claim an edge from the in-sample
sample.

## Matrix

The matrix is the full cross product of three instruments, three variants, and
four evaluation years: 36 cells. Each cell is one deterministic `backtest` with
its own isolated output root. The authoritative list is
[`matrix.txt`](matrix.txt); the naming is `<instrument>-<variant>-<year>`.

| Variant | Ladder | Stop / target | Policy | Execution | Analytics |
|---|---|---|---|---|---|
| `control` | 5m / 15m | 2.0 ATR / 1.5R | `policy-control.json` | `execution-control.json` (5m) | `analytics-control.json` |
| `t2` | 1h / 4h | 2.0 ATR / 2.0R | `policy-high.json` | `execution-high.json` (1h) | `analytics-high.json` |
| `t3` | 1h / 4h | 3.0 ATR / 3.0R | `policy-high.json` | `execution-high.json` (1h) | `analytics-high.json` |

| Instrument | Source | Evaluation years |
|---|---|---|
| BTCUSD | `BTCUSD_M1_2018-01-01_2024-12-31.csv` | 2021, 2022, 2023, 2024 |
| ETHUSD | `ETHUSD_M1_2018-01-01_2024-12-31.csv` | 2021, 2022, 2023, 2024 |
| XAUUSD | `XAUUSD_M1_2018-01-01_2024-12-31.csv` | 2021, 2022, 2023, 2024 |

Every cell runs a 13-month window from 1 December of the prior year through 31
December of the evaluation year. December is causal warm-up: it supplies the 4h
history, and the evaluation metrics start on 1 January. The data version names
the exact run window, for example
`dukascopy-bid-btcusd-m1-2020-12-01_2021-12-31`.

## Metrics

- Full-run final equity, total return, net P&L, maximum drawdown, halt state
- Trades, profit factor, win rate, average trade, exposure, hold bars
- Cost drag: commission + execution fees + funding
- Evaluation-year return and maximum drawdown, rebased on the first equity point
  on or after 1 January
- In-sample (2021–2022) and out-of-sample (2023–2024) aggregates
- Two-year OOS compounded return and drawdown
- OOS net expectancy and a normal-approximation 95% confidence interval
- Stability: positive instruments, contribution share, per-instrument trade count
- Determinism: a separate rerun of one control and one treatment cell, bundle
  byte comparison

## Green criteria

A treatment variant is green only if all of the following hold for the
2023–2024 out-of-sample period:

1. At least two of the three instruments have a positive compounded OOS return.
2. The equal-weight mean compounded OOS return is at least 21% (10% annualized
   over two years).
3. Every instrument's OOS maximum drawdown is at most 20%, and no cell is halted.
4. Every instrument has at least 30 closed OOS trades.
5. Every positive instrument has an OOS profit factor of at least 1.2.
6. The pooled OOS net-expectancy 95% confidence-interval lower bound is above
   zero.
7. No single instrument contributes more than 60% of the aggregate OOS net P&L.

The control is reported for comparison and is not required to pass. A variant
that passes the in-sample sample but not the OOS sample is not green. A variant
that passes one instrument but not the stability criteria is not green. No
criterion may be changed after the first cell runs.

## OOS method

Each evaluation year is a separate causal run with a 13-month warm-up. Trades
are attributed to the evaluation year by entry time. Equity is rebased to the
first point on or after 1 January, so warm-up P&L is excluded. The two-year OOS
return compounds 2023 and 2024; the two-year drawdown concatenates the two
rebased equity curves. This is a yearly-reset design, not one continuous
two-year run: the model and portfolio restart each January. The limitation is
recorded; it does not leak future data, and no threshold is chosen from OOS.

## Reproduction

```bash
# 1. Build the shell.
cargo build --release -p quantrule-shell

# 2. Generate the 36 deterministic configs.
python3 docs/experiments/2026-11-01/generate_configs.py

# 3. Validate one cell per variant.
QR=target/release/quantrule
$QR policy check --config docs/experiments/2026-11-01/config/btcusd-control-2024/run.json
$QR policy check --config docs/experiments/2026-11-01/config/btcusd-t2-2024/run.json
$QR policy check --config docs/experiments/2026-11-01/config/btcusd-t3-2024/run.json

# 4. Run the matrix.
QR=$QR tools/experiment/run-matrix.sh \
  docs/experiments/2026-11-01/matrix.txt \
  target/experiment-EXP-2026-11-01-03

# 5. Analyze.
python3 docs/experiments/2026-11-01/analyze.py \
  target/experiment-EXP-2026-11-01-03 \
  --json target/experiment-EXP-2026-11-01-03/results.json \
  --text target/experiment-EXP-2026-11-01-03/results.txt
```

The source files are local and not committed. The generator reads no clock,
network, or random source; its output is deterministic.

## Results

All 36 cells completed and wrote five bundles plus `backtest.json`. The tables
below use evaluation-year metrics rebased on 1 January; the full-run metrics and
the per-cell details are in
[`results.json`](results.json) and [`results.txt`](results.txt).

### In-sample 2021–2022

| Variant | Instrument | 2y return | Max DD | Trades | PF | CI lower | Halted |
|---|---|---|---|---|---|---|---|
| control | BTCUSD | -33.70% | 33.70% | 184 | 0.271 | -25.58 | yes |
| control | ETHUSD | -34.87% | 34.87% | 164 | 0.356 | -32.46 | yes |
| control | XAUUSD | -34.29% | 34.29% | 168 | 0.003 | -23.72 | yes |
| t2 | BTCUSD | -2.53% | 5.67% | 13 | 0.768 | -115.29 | no |
| t2 | ETHUSD | -10.39% | 10.39% | 8 | 0.000 | -209.47 | no |
| t2 | XAUUSD | -1.31% | 1.93% | 9 | 0.518 | -51.03 | no |
| t3 | BTCUSD | -1.35% | 4.53% | 13 | 0.866 | -100.99 | no |
| t3 | ETHUSD | -0.79% | 9.94% | 8 | 0.929 | -318.57 | no |
| t3 | XAUUSD | -0.27% | 1.86% | 9 | 0.910 | -58.36 | no |

### Out-of-sample 2023–2024

| Variant | Instrument | 2y return | Max DD | Trades | PF | CI lower | Halted |
|---|---|---|---|---|---|---|---|
| control | BTCUSD | -34.70% | 34.70% | 178 | 0.129 | -25.18 | yes |
| control | ETHUSD | -31.77% | 32.00% | 203 | 0.289 | -21.37 | yes |
| control | XAUUSD | -31.11% | 31.11% | 153 | 0.021 | -23.68 | no |
| t2 | BTCUSD | +1.39% | 1.14% | 11 | 1.340 | -56.02 | no |
| t2 | ETHUSD | -11.88% | 11.88% | 9 | 0.000 | -215.09 | no |
| t2 | XAUUSD | -0.04% | 1.73% | 11 | 0.993 | -48.56 | no |
| t3 | BTCUSD | -3.73% | 3.73% | 10 | 0.335 | -92.68 | no |
| t3 | ETHUSD | -14.11% | 14.11% | 9 | 0.000 | -266.74 | no |
| t3 | XAUUSD | -1.91% | 1.95% | 11 | 0.430 | -50.83 | no |

The only positive OOS cell is `t2` BTCUSD at `+1.39%`; it is one instrument,
11 trades, and its 95% CI straddles zero. The control is a cost-drag failure:
the 5m/15m cell loses 15–35%; BTCUSD and ETHUSD halt, while XAUUSD does not halt
but still loses 31%.

### Green check

| Variant | 2/3 positive | Mean >= 21% | DD <= 20% | Trades >= 30 | PF >= 1.2 | CI low > 0 | Contribution <= 60% | Green |
|---|---|---|---|---|---|---|---|---|
| control | no | no | no | yes | yes | no | no | no |
| t2 | no | no | yes | no | yes | no | no | no |
| t3 | no | no | yes | no | yes | no | no | no |

No variant is green. Both treatments fail the trade-count floor by a wide
margin: 9–11 OOS trades per instrument against the pre-registered 30.

### Data quality

The diagnosis was run on the six treatment OOS cells. Coverage is the
diagnosis's expected-bar denominator, so the XAUUSD rows include normal weekend
sessions and are not comparable to the 24/7 crypto rows.

| Cell | 1m coverage | 1h coverage | 4h coverage | Note |
|---|---|---|---|---|
| BTCUSD 2023 | 92.36% | 61.64% | 45.98% | 43,506 missing 1m bars; longest gap 1,261 min; zero trades |
| ETHUSD 2023 | 90.01% | 45.71% | 27.68% | 56,893 missing 1m bars; longest gap 1,261 min; 2 trades |
| XAUUSD 2023 | 67.32% | 65.73% | 57.55% | weekend sessions; longest gap 4,381 min |
| BTCUSD 2024 | 98.55% | 91.76% | 84.76% | 8,283 missing 1m bars; 10 trades |
| ETHUSD 2024 | 98.80% | 91.76% | 83.25% | 6,876 missing 1m bars; 7 trades |
| XAUUSD 2024 | 67.07% | 66.51% | 57.75% | weekend sessions; 6 trades |

The BTCUSD and ETHUSD 2023 4h coverage is a data blocker: the 4h context is
missing for more than half of the year, and the BTCUSD 2023 treatment produced
zero trades. The 2023 treatment cells cannot support a clean `no edge` claim.
The 2024 cells have better 1m coverage but 4h coverage is still 83–85%; the
sample remains small. The read-only audit is
[`docs/audits/2026-11-01/crypto-2023-data-audit.md`](../../audits/2026-11-01/crypto-2023-data-audit.md);
it shows that a fresh provider fetch reproduces the same gap and that the 4h
loss is amplified by the documented complete-bucket rule.

### Determinism

The control and `t2` BTCUSD 2024 cells were rerun into separate roots. All five
bundles (`model`, `strategy`, `execution`, `portfolio`, `analytical`) were
byte-identical for both cells.

## Triage

| Area | Finding | Route |
|---|---|---|
| Data | BTCUSD/ETHUSD 2023 4h coverage is 46%/28%; the 2023 treatment cells are not a clean test. The source has 43,506/56,893 missing 1m bars and a 1,261-minute longest gap. | Data audit/fix before any re-run. |
| Strategy | The 1h/4h rule is too selective: 9–11 OOS trades per instrument against a 30-trade floor. The strict `conflict_count_max = 0` and `late_or_terminal_count_max = 0` context filters are the binding constraint. | New pre-registration, not a threshold change in this one. |
| Strategy | The 5m/15m control loses 15–35%; BTCUSD and ETHUSD halt, XAUUSD does not halt but loses 31%. Commission and turnover still dominate. | The lower-timeframe baseline is not viable. |
| Program | No program defect found. The zero-trade BTCUSD 2023 cell is explained by the configured context filters and the data gaps; the rerun is deterministic. | No audit finding from this matrix. |

The result is **not green** and is **not a clean `no edge`**: the sample is too
small, and the 2023 data is incomplete. The 2024 evidence is negative for both
treatments except the single `t2` BTCUSD cell.

## Decision

Stop at the pre-registered cap. Do not change a threshold, cost, or split after
seeing the result. No edge or profitability claim is made.

The next cycle, if the human approves it, must start with a data audit/fix for
the BTCUSD/ETHUSD 2023 source and a new pre-registration. A candidate hypothesis
is that the strict context filters, not the higher timeframe itself, are the
binding constraint; relaxing one context limit is a new experiment, not a
change to this one. A `15m`/`1h` ladder is the other candidate. Neither was run
here.

## Limitations

- Three instruments and four years; this is a sample, not a cross-sectional
  proof.
- Yearly runs reset the model and portfolio; December is warm-up and the first
  January days may lack 4h context. A position opened during warm-up and still
  open on 1 January is a boundary effect.
- The 0.1% commission is conservative for liquid venues; a green result under it
  would be stronger, a failure does not prove the rule is unprofitable at lower
  costs. It also means the control's failure is partly a cost-assumption result.
- The source is Dukascopy BID M1 with provider gaps and no spread model; slippage
  is a fixed 2 bps.
- BTCUSD/ETHUSD 2023 4h coverage is a data blocker; the 2023 treatment cells are
  not a clean out-of-sample test.
- No R-multiple field, no partial fills, no market impact, and no capacity
  estimate.
- No independent Quant Validator was invoked because no green claim or strategy
  change is proposed. The rerun is a same-agent determinism check, not an
  independent review.
