# S3.x Portfolio Lab — Fills to Equity and Performance

> **Historical.** This lab records the v0.3.0 local surface. The local stage
> commands were removed from the shell in v1.0.0; set
> `BIN=./target/release/quantrule` before running a block, and see
> [the shell tutorial](../2026-11-14/shell-http.md) for the current surface.

- Date: 2026-11-06 (UTC)
- Status: Current
- Scope: `report model`, `strategy run`, `execution run`, and `portfolio replay`
- Claim type: procedural stage lab; no edge or profitability claim.

This lab runs the model, strategy, and execution stages, then replays the fills
through portfolio accounting. The portfolio stage does not decide a signal and
does not alter a fill.

## 1. Build

```bash
cargo build --release
```

## 2. Produce the model, strategy, and execution bundles

```bash
rm -rf target/portfolio-lab target/portfolio-lab-a target/portfolio-lab-b
"$BIN" report model \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/portfolio-lab
"$BIN" strategy run \
  --model target/portfolio-lab/model \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/portfolio-lab
"$BIN" execution run \
  --decisions target/portfolio-lab/strategy \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/portfolio-lab
```

The fills bundle is `target/portfolio-lab/execution/`.

## 3. Replay the portfolio

```bash
"$BIN" portfolio replay \
  --fills target/portfolio-lab/execution \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/portfolio-lab
```

It prints the seven artifact paths. The bundle is
`target/portfolio-lab/portfolio/`:

```bash
cat target/portfolio-lab/portfolio/summary.txt
cat target/portfolio-lab/portfolio/trades.csv
cat target/portfolio-lab/portfolio/equity.csv
cat target/portfolio-lab/portfolio/funding.csv
cat target/portfolio-lab/portfolio/risk_events.csv
cat target/portfolio-lab/portfolio/metrics.json
cat target/portfolio-lab/portfolio/provenance.json
```

Expected shape:

| File | Contents |
|---|---|
| `trades.csv` | One row per closed trade, with net P&L and exit reason. |
| `equity.csv` | One row per UTC date with a mark: cash, position value, equity, peak, and drawdown. |
| `funding.csv` | One row per aligned funding charge. |
| `risk_events.csv` | One row per drawdown-limit breach. |
| `metrics.json` | `quantrule.portfolio-metrics/v1` with the run's accounting metrics. |
| `summary.txt` | A fixed-order rendering of the metrics and the data version. |
| `provenance.json` | `quantrule.stage-provenance/v1` with the execution source artifact. |

The accounting invariant is `final_equity = initial_cash + net_pnl`. A metric
that is undefined is `null`: win rate with no trades, profit factor with no
losses, and average trade with no trades are not zero. A valid zero is a value.

## 4. Determinism

```bash
"$BIN" portfolio replay \
  --fills target/portfolio-lab/execution \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/portfolio-lab-a
"$BIN" portfolio replay \
  --fills target/portfolio-lab/execution \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/portfolio-lab-b
diff -r target/portfolio-lab-a/portfolio target/portfolio-lab-b/portfolio
```

`diff` prints nothing when the run is reproducible.

## 5. Boundaries

- The portfolio stage reads the fills bundle and the run config. It never reads a
  strategy decision, a model bundle, or a statistic.
- It applies funding before the bar's fills and marks the open position at the
  execution-timeframe close.
- The execution bundle's fee is carried into the trade; portfolio commission and
  funding are separate accounting costs.
- The portfolio bundle carries account cash, equity, P&L, exposure, and drawdown.
  It never changes a signal, a decision, or a fill.
- The metrics are accounting measurements, not a profitability claim or an edge
  verdict.
