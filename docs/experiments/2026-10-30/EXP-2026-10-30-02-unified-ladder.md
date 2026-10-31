# Experiment EXP-2026-10-30-02: Unified Ladder Verification Matrix

- Date: 2026-10-30 (UTC)
- Status: Complete
- Owner: Governing / Experiment Control (sequential pass)
- Policy: `docs/experiments/2026-10-30/config/base/policy.json`
- Strategy: the four cell configs under `docs/experiments/2026-10-30/config/`
- Data: `dukascopy-bid-solusd-m1-2025-01-01_2025-12-31`
- Tickets: SOLUSD
- In-sample: 2025-01-01..2025-08-31
- Out-of-sample: 2025-09-01..2025-12-31
- Costs: commission 0.1% of equity per operation; slippage 2 bps; funding 0.0001 per 8h; execution fee 0.0
- Quant Validator: [`docs/reports/2026-10-30/unified-ladder-validation.md`](../../reports/2026-10-30/unified-ladder-validation.md)
- Green criteria: each cell completes and writes five bundles plus `backtest.json`;
  the control is byte-identical on a rerun; the data version and costs are the
  same across cells; OOS metrics are reported; no cell is called profitable
- Iteration cap: one matrix; a cell may be rerun only for a harness or config
  error, and the rerun is recorded
- Stop conditions: missing data, config error, nondeterminism, look-ahead,
  incomplete bundles, or a change to a threshold after seeing a result

## Hypothesis

The unified `ladder-context` rule can be configured in the four pre-registered
cells, runs deterministically over the SOLUSD 2025 M1 source with the same costs,
and makes causal decisions. The experiment verifies the implementation and
records descriptive evidence. It does not claim an edge.

## Matrix

| Cell | Execution | Context | Model settings | Weights | Config |
|---|---|---|---|---|---|
| Control | 5m | 15m, 1h, 4h | current defaults | equal | `config/control/run.json` |
| A | 1m | 5m, 15m, 1h, 4h | current defaults | equal | `config/a/run.json` |
| B | 5m | 15m, 1h, 4h | RSI 7 on 1m and 5m | equal | `config/b/run.json` |
| C | 5m | 15m, 1h, 4h | current defaults | 0.5 / 1.0 / 2.0 by 15m / 1h / 4h | `config/c/run.json` |

All cells use:

- the same policy envelope and portfolio costs;
- the same data version;
- the strict control context limits (`conflict_count_max = 0`,
  `late_or_terminal_count_max = 0`);
- ATR stop 2.0 and reward-to-risk 1.5;
- both `context_lost` and `structure_lost` exits.

## Metrics

- Total return
- Max drawdown and duration
- Profit factor
- Win rate
- Average trade
- Exposure
- Cost drag
- Trade count
- OOS stability
- R-multiple distribution when the report field exists (it does not in this
  delivery)

## OOS method

The full-year run is one continuous causal run. OOS metrics use trades whose
entry time is on or after 2025-09-01, and equity points from the same date onward
rebased to the first OOS point. The full-year warm-up supplies the model and
context history. This is a filtered OOS read of a continuous run, not a separate
restarted backtest; this limitation is recorded.

## Result fields to record

For each cell:

| Field | Source |
|---|---|
| Final equity, total return, net P&L, max drawdown, trades, halted | `portfolio/report.json` or `backtest.json` |
| Profit factor, win rate, average trade, exposure | `portfolio/report.json` |
| Cost drag | `portfolio/report.json` commission + funding + execution fee |
| OOS return, OOS max drawdown, OOS trades, OOS win rate | filtered `portfolio/trades.csv` and `equity.csv` |
| Determinism | rerun the control and compare bundle bytes |

## Reproduction

```bash
cargo build -p quantrule-shell
QR=target/debug/quantrule tools/experiment/run-matrix.sh \
  docs/experiments/2026-10-30/matrix.txt \
  target/experiment-EXP-2026-10-30-02
```

The data file is fetched once with:

```bash
cargo run -q -p quantrule-shell -- fetch sol/usd 2025-01-01 2025-12-31 \
  --csv data/SOLUSD_M1_2025-01-01_2025-12-31.csv
```

## Results

Full-year run:

| Cell | Return | Max DD | Trades | Profit factor | Win rate | Average trade | Exposure | Cost drag |
|---|---|---|---|---|---|---|---|---|
| Control | +0.594% | 1.966% | 12 | 1.233 | 41.67% | +4.95 | 0.000134 | 243.07 |
| A | -9.098% | 9.219% | 40 | 0.162 | 22.50% | -22.74 | 0.000088 | 755.74 |
| B | +1.160% | 1.525% | 12 | 1.473 | 50.00% | +9.67 | 0.000087 | 243.20 |
| C | -1.372% | 2.730% | 7 | 0.523 | 42.86% | -19.59 | 0.000078 | 140.34 |

Out-of-sample filter (entries on or after 2025-09-01):

| Cell | OOS return | OOS max DD | OOS trades | OOS profit factor | OOS win rate | OOS average trade | OOS cost drag |
|---|---|---|---|---|---|---|---|
| Control | -1.549% | 1.745% | 5 | 0.114 | 20.00% | -31.65 | 101.90 |
| A | -1.429% | 1.560% | 13 | 0.353 | 38.46% | -10.13 | 237.44 |
| B | -0.716% | 1.364% | 4 | 0.478 | 50.00% | -18.24 | 81.85 |
| C | -1.557% | 1.760% | 3 | 0.117 | 33.33% | -51.99 | 59.98 |

Determinism: the control and variation A were rerun into separate roots. All
five bundles were byte-identical for both cells.

Benchmark: the context builder processed 600,000 execution bars in 0.270 s in a
release build.

## Triage

`no edge` for this sample. The implementation is complete and reproducible; the
OOS result is negative in every cell. The control and B are positive over the
full year, but neither survives the OOS filter as a stability claim. The 1m
variation A is materially worse after costs.

## Decision

Keep the control rule. Stop the iteration loop at the pre-registered cap; do not
change a threshold after seeing the result. No profitability claim is made.
A future experiment may change one engine only, with a new pre-registration and
a larger or different sample.

## Limitations

- One instrument, one year, one source.
- The OOS read filters a continuous run; it does not restart the model or
  strategy at the split.
- No R-multiple field exists in this delivery.
- No profitability or edge claim is made.
