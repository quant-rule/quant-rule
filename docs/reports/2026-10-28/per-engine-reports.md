# Per-Engine Reports — MVP v1

- Date: 2026-10-28 (UTC)
- Issue: [#168](https://github.com/quant-rule/quant-rule/issues/168)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Status: Approved; implemented by #173, #174, #175, and #176
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md)
- Report contract version: `2026-10-28`

## 1. Purpose

A run produces five independent report bundles, one per engine. Each bundle is a
directory under the run root. A bundle is available if and only if its
`report.json` exists. There is no placeholder report, no empty report, and no
top-level manifest that can go stale.

`backtest` writes all five bundles. `report <engine>` writes exactly one. The
presence of a directory is therefore the answer to "was this report generated?".

## 2. Bundle layout

```text
<out>/
  model/
  strategy/
  execution/
  portfolio/
  analytical/
```

| Bundle | Artifacts, in fixed order |
|---|---|
| `model/` | `summary.txt`, `report.json`, `price-axes.png`, `events.png`, `provenance.json` |
| `strategy/` | `summary.txt`, `report.json`, `decisions.csv`, `price-decisions.png`, `provenance.json` |
| `execution/` | `summary.txt`, `report.json`, `orders.csv`, `fills.csv`, `rejections.csv`, `price-fills.png`, `provenance.json` |
| `portfolio/` | `summary.txt`, `report.json`, `trades.csv`, `equity.csv`, `equity.png`, `drawdown.png`, `provenance.json` |
| `analytical/` | `summary.txt`, `report.json`, `axes.csv`, `relations.csv`, `provenance.json` |

The artifact list is also recorded in `provenance.json`. A test asserts the
directory contents against this table, so an extra file fails the build.

## 3. Availability and failure

- `backtest` requires the run root to be absent or empty. It refuses to mix runs.
- `report <engine>` allows the run root to exist, but `<out>/<engine>` must be
  absent or empty.
- `report.json` is written last. A partial bundle therefore has no `report.json`
  and is not available.
- On failure, the command removes the partial bundle. `backtest` also removes the
  run root if it created it, or the bundles it wrote into an existing empty root.
- An existing non-empty target is refused before any byte is written.
- The command prints the bundle paths, or the artifact paths, in contract order.

## 4. Provenance

Every bundle contains `provenance.json`:

```json
{
  "schema": "quantrule.provenance/v1",
  "report": "portfolio",
  "report_version": "2026-10-28",
  "model_version": "0.2.0",
  "data_version": "dukascopy-bid-solusd-m1-2025-01-01_2025-12-31",
  "requested_period": { "from": "2025-01-01", "to": "2025-12-31" },
  "actual_period": { "first_open_time": 1735740300, "last_open_time": 1767193080 },
  "config_files": ["run.json", "data.json", "model.json", "strategy.json", "execution.json", "portfolio.json", "analytics.json"],
  "config": {
    "run": { "name": "solusd-2025" },
    "data": { "data_version": "...", "from": "2025-01-01", "to": "2025-12-31", "ladder": ["1m", "5m", "15m", "1h"], "max_candles": 600000 },
    "model": { "structure_scale": "major" },
    "strategy": { "name": "axis-agreement", "timeframe": "15m", "long_threshold": 0.1, "short_threshold": -0.1 },
    "execution": { "fill_timeframe": "15m", "order_type": "market_next_open", "stop_atr_multiple": 2.0, "target_atr_multiple": 3.0, "tie_break": "stop_first", "fee_rate": 0.0, "slippage_bps": 2.0 },
    "portfolio": { "initial_cash": 10000.0, "sizing": { "mode": "fixed_notional", "notional": 10000.0, "quantity_step": 0.0001 }, "leverage": 1.0, "max_open_positions": 1, "allow_short": true, "commission": { "rate": 0.001, "base": "equity", "per": "operation" }, "funding": { "rate": 0.0001, "interval_hours": 8 }, "max_drawdown": { "limit": 0.2, "action": "halt" } },
    "analytics": { "structure_scale": "major", "timeframes": ["1m", "5m", "15m", "1h"], "relations": true }
  },
  "artifacts": ["summary.txt", "report.json", "trades.csv", "equity.csv", "equity.png", "drawdown.png", "provenance.json"]
}
```

- `config_files` are file names, not absolute paths, so the bundle is portable.
- `config` holds resolved values, not raw bytes.
- `actual_period` is the first and last bar of the report's primary timeframe. For
  the model report it spans the earliest and latest bar across the ladder; for
  the portfolio report it spans the first and last strategy bar with a state.
- `model_version` is the shell's compiled package version.

## 5. Model report

The model report is the existing `MarketReport` view. It answers descriptive
questions about coverage, axis statistics, ordinal histograms, event counts, the
latest state, cross-timeframe aggregates, and chart buckets.

Artifacts:

| File | Contents |
|---|---|
| `summary.txt` | The human-readable full-period summary |
| `report.json` | The legacy envelope with the `market_report` object |
| `price-axes.png` | Price with the four axis panels |
| `events.png` | Event distribution |
| `provenance.json` | The run provenance |

The four legacy artifacts are byte-identical to `market-report` for the same
data, period, and default model settings. `report model` adds only
`provenance.json`.

The model bundle publishes no order, fill, position, trade, equity, drawdown, P&L,
or strategy decision.

## 6. Strategy report

The strategy report runs the data engine, the model, and the strategy. It does
not run execution or portfolio accounting.

Artifacts:

| File | Contents |
|---|---|
| `summary.txt` | Decision counts and agreement counts |
| `report.json` | Summary and decision records |
| `decisions.csv` | One row per entry or exit |
| `price-decisions.png` | Price with entry and exit markers |
| `provenance.json` | The run provenance |

Summary fields:

```text
bars, decisions, entries, exits, long_entries, short_entries,
exit_reasons { agreement_lost },
agreement_counts { long, short, neutral, unavailable },
open_position_at_end
```

Each record carries `type`, `side` (`1` or `-1`), `bar`, `datetime`, `direction`,
`momentum_force`, `structure_score`, and `reason` on an exit. It carries no fill,
price paid, quantity, cash, equity, or P&L field.

## 7. Execution report

The execution report runs the full pipeline internally so its quantities match a
full backtest, then publishes execution fields only. It contains no cash, equity,
trade, P&L, commission, funding, or drawdown field.

Artifacts:

| File | Contents |
|---|---|
| `summary.txt` | Order, fill, rejection, slippage, and fee counts |
| `report.json` | Summary and the three record arrays |
| `orders.csv` | Order lifecycle |
| `fills.csv` | Fills |
| `rejections.csv` | Rejections |
| `price-fills.png` | Price with fill markers |
| `provenance.json` | The run provenance |

Summary fields:

```text
orders, filled, rejected, pending_at_end, fills, entries, exits,
fill_reasons { entry, exit, stop, target },
rejections { <reason_code> },
slippage { total, average }, fees { total }
```

`pending_at_end` is zero after `finish()`: every order resolves to filled or
rejected. A `pending` row appears only in a per-bar batch, never in the final
report.

## 8. Portfolio report

The portfolio report runs the full pipeline and publishes the accounting result.
It contains no model label, axis word, or strategy recommendation.

Artifacts:

| File | Contents |
|---|---|
| `summary.txt` | The key metrics |
| `report.json` | Summary and the four record arrays |
| `trades.csv` | Closed trades |
| `equity.csv` | Daily equity |
| `equity.png` | Daily equity curve |
| `drawdown.png` | Daily drawdown curve |
| `provenance.json` | The run provenance |

Summary fields:

```text
initial_cash, final_equity, total_return, realized_pnl, unrealized_pnl,
gross_pnl, net_pnl, execution_fees, commission, funding_paid, funding_received,
max_drawdown, max_drawdown_duration_bars, trades, wins, losses,
win_rate, profit_factor, average_trade, average_hold_bars, exposure,
halted, open_position, monthly_returns [{ month, return }]
```

`win_rate`, `profit_factor`, `average_trade`, `average_hold_bars`, `exposure`, and
`open_position` are `null` when undefined. They are never zero. `monthly_returns`
has one row per month in the actual period; `return` is null when the month has no
equity point or the prior equity is zero.

Records:

- `trades`: one row per closed trade, with entry and exit prices, gross P&L,
  execution fees, commission, funding, net P&L, return, hold bars, and exit reason.
- `equity_curve`: one row per UTC date with at least one strategy bar, at the last
  bar close of that date.
- `funding`: one row per charged boundary.
- `risk_events`: one row per halt, with the drawdown and equity at the breach.

## 9. Analytical report

The analytical report runs the model and builds the research layers from that
model run. It does not run the strategy, execution, or portfolio.

Artifacts:

| File | Contents |
|---|---|
| `summary.txt` | Layer counts and per-axis summaries |
| `report.json` | Summary and the relations list |
| `axes.csv` | Per-timeframe, per-axis statistics |
| `relations.csv` | Pairwise correlations |
| `provenance.json` | The run provenance |

Summary fields:

```text
layer1_rows, layer2_rows, layer3_rows,
timeframes [{ timeframe_secs, axes [{ axis, count, nulls, min, max, mean, ordinal_counts, ordinal_nulls }] }],
relations [{ timeframe_secs, left, right, count, correlation }]
```

`relations` is empty when `analytics.json` sets `relations: false`; the CSV has a
header and no rows. A missing correlation is null, never zero.

## 10. CSV schemas

### 10.1 `strategy/decisions.csv`

```text
type,side,timeframe,timeframe_secs,bar,datetime,direction,momentum_force,structure_score,reason
```

Rows are ordered by bar ascending. `type` is `entry` or `exit`. `side` is `1` or
`-1`. `reason` is empty for an entry and `agreement_lost` for an exit.

### 10.2 `execution/orders.csv`

```text
order_id,submitted_bar,submitted_time,side,quantity,trigger,state,filled_bar,filled_time,fill_id,rejection_reason
```

Rows are ordered by `order_id`. `trigger` is `market`, `stop`, or `target`.
`state` is `pending`, `filled`, or `rejected`. Empty fields are empty, not zero.

### 10.3 `execution/fills.csv`

```text
fill_id,order_id,side,quantity,reference_price,fill_price,slippage,fee,reason,bar,time,position_side,entry
```

Rows are ordered by `fill_id`. `reason` is `entry`, `exit`, `stop`, or `target`.
`entry` is `true` or `false`.

### 10.4 `execution/rejections.csv`

```text
bar,time,reason,decision
```

Rows are ordered by bar, then reason. `decision` is `entry` or `exit`.

### 10.5 `portfolio/trades.csv`

```text
trade_id,side,quantity,entry_bar,entry_time,entry_price,exit_bar,exit_time,exit_price,gross_pnl,execution_fees,commission,funding,net_pnl,return,hold_bars,exit_reason
```

Rows are ordered by `trade_id`. `return` is net P&L divided by equity at entry;
empty when equity at entry is zero. `exit_reason` is `agreement_lost`, `stop`, or
`target`.

### 10.6 `portfolio/equity.csv`

```text
date,cash,position_value,equity,peak_equity,drawdown
```

One row per UTC date with at least one strategy bar, at the last bar close of that
date. A missing date is absent, not zero. `drawdown` is `1 - equity / peak_equity`.

### 10.7 `analytical/axes.csv`

```text
timeframe_secs,timeframe,axis,count,nulls,min,max,mean,ordinal_0,ordinal_1,ordinal_2,ordinal_3,ordinal_4,ordinal_5,ordinal_6,ordinal_7,ordinal_8,ordinal_9,ordinal_nulls
```

Rows are ordered by timeframe, then axis. Empty values are empty, not zero.

### 10.8 `analytical/relations.csv`

```text
timeframe_secs,timeframe,left,right,count,correlation
```

Rows are ordered by timeframe, then the fixed axis-pair order. An empty
correlation is undefined, not zero.

## 11. Boundaries

| Bundle | May publish | Must not publish |
|---|---|---|
| `model` | Coverage, axis statistics, ordinal histograms, event counts, latest state, aggregates, chart buckets | Orders, fills, positions, trades, equity, drawdown, P&L, strategy decisions |
| `strategy` | Entries, exits, agreement counts, thresholds, open-position state at end | Fills, prices paid, quantities, cash, equity, P&L |
| `execution` | Orders, fills, rejections, fill reasons, slippage, execution fees, bracket levels | Cash, equity, trades, P&L, commission, funding, drawdown |
| `portfolio` | Cash, equity, trades, gross and net P&L, commission, funding, drawdown, exposure, monthly returns, risk events | Model labels, axis words, strategy thresholds as recommendations |
| `analytical` | Layer row counts, per-axis counts, ranges, histograms, correlations | Strategy, execution, portfolio, orders, fills, P&L |

A boundary test scans the JSON keys and CSV headers of each bundle and fails on a
forbidden name.

## 12. `report.json`

The four new reports share this shape:

```json
{
  "report": "strategy",
  "report_version": "2026-10-28",
  "model_version": "0.2.0",
  "data_version": "...",
  "requested_period": { "from": "2025-01-01", "to": "2025-12-31" },
  "actual_period": { "first_open_time": 0, "last_open_time": 0 },
  "config": { "...": "resolved engine config" },
  "summary": { "...": "engine-specific" },
  "records": { "...": "engine-specific" }
}
```

The model report keeps the legacy envelope: `model_version`, `view_contract`,
`data_version`, `requested_period`, `market_report`.

## 13. Determinism

- The same command, data version, and configuration produce byte-identical
  artifacts.
- Charts use deterministic downsampling and fixed dimensions.
- A missing value is a gap or a null, never zero.
- CSV rows have a fixed order.
- JSON keys are written in a fixed order by the renderer, not by a hash map.
- PNG bytes are compared in tests; a rendering change is a contract change.

## 14. Compatibility

`market-report` remains available with exactly four artifacts and no
`provenance.json`. `report model` produces the same four artifacts plus
provenance. The four are byte-identical for the same data, period, and default
model settings.

`strategy-run` remains the decision-only command. It prints NDJSON entries and
exits and creates no fill, position, equity, or P&L record.

## 15. Change control

The artifact names, artifact order, CSV headers, JSON field names, and report
names are frozen for MVP v1. A change requires a new report contract version and a
human decision. An implementation may not silently add, rename, or drop an
artifact or a field.
