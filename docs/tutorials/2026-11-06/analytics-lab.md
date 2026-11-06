# S3.x Analytics Lab — Descriptive Metrics and Evaluation Tables

- Date: 2026-11-06 (UTC)
- Status: Current
- Scope: `analytics evaluate` over a completed backtest bundle
- Claim type: procedural stage lab; no verdict, edge, or acceptance claim.

This lab reads a completed backtest and publishes descriptive metrics and
evaluation tables. It does not produce a verdict, accept the strategy, or claim
an edge.

## 1. Build

```bash
cargo build --release
```

## 2. Produce the evidence bundle

```bash
rm -rf target/analytics-lab target/analytics-lab-a target/analytics-lab-b
./target/release/quantrule backtest run \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/analytics-lab
```

The evidence bundle is `target/analytics-lab/backtest/`.

## 3. Publish the analytics bundle

```bash
./target/release/quantrule analytics evaluate \
  --evidence target/analytics-lab/backtest \
  --out target/analytics-lab
```

The command prints the metrics, tables, and summary paths. It writes:

```text
target/analytics-lab/analytics/
  metrics.json
  tables/
    trades.csv
    monthly_returns.csv
    costs.csv
    decisions.csv
    by_timeframe.csv
    equity.csv
    funding.csv
    risk_events.csv
  summary.txt
  provenance.json
```

The verified fixture produces one closed losing trade. `metrics.json` records
`trade_count: 1`, `win_rate: 0`, `profit_factor: 0`, the drawdown, the cost
totals, the monthly return, and the decision counts `hold=51` and `enter=1`.

## 4. Inspect the metrics and tables

```bash
cat target/analytics-lab/analytics/metrics.json
cat target/analytics-lab/analytics/summary.txt
cat target/analytics-lab/analytics/tables/decisions.csv
cat target/analytics-lab/analytics/tables/by_timeframe.csv
cat target/analytics-lab/analytics/tables/costs.csv
```

`metrics.json` uses `quantrule.analytics-metrics/v1`. It carries the evidence
identity, the descriptive metrics, one reason code for every null metric, the
table manifest, and the unsupported-table list.

The `by_regime` and `by_instrument` tables are not synthesized. The backtest
bundle publishes no regime label and no instrument identity, so `metrics.json`
records them as unsupported with `regime_label_not_published` and
`instrument_identity_not_published`.

## 5. Determinism

```bash
./target/release/quantrule analytics evaluate \
  --evidence target/analytics-lab/backtest \
  --out target/analytics-lab-a
./target/release/quantrule analytics evaluate \
  --evidence target/analytics-lab/backtest \
  --out target/analytics-lab-b
diff -r target/analytics-lab-a/analytics target/analytics-lab-b/analytics
```

`diff` prints nothing when the analytics bundle is reproducible.

## 6. Boundaries

- The analytics bundle carries no verdict, no acceptance, and no human decision.
  Qualification is the stage that compares evidence with human-owned criteria.
- The analytics metrics are descriptive. They are not a profitability claim, an
  edge claim, or a trade recommendation.
- No analytics metric enters a strategy decision, a model event value, or a
  model reading.
- A metric the evidence does not support is `null` with a reason code. A table
  the evidence does not support is listed as unsupported with a reason code.
- The stage validates the manifest and every stage provenance hash before it
  reads a metric.
