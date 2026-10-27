# ADR: Market Analysis and Full-Execution Backtest Boundaries

- Date: 2026-10-27 (UTC)
- Status: Accepted. The program plan and this decision are approved; implementation follows the plan.
- Related: [program plan](../../execution-plans/2026-10-27/141-market-analysis-and-backtest.md), [model contract](../../model/2026-10-21/four-axis-analytical-model.md), [history and retention policy](../2026-10-25/history-retention-and-display-policy.md), issues [#10](https://github.com/quant-rule/quant-rule/issues/10), [#11](https://github.com/quant-rule/quant-rule/issues/11), [#12](https://github.com/quant-rule/quant-rule/issues/12)

## 1. Context

QuantRule has a complete descriptive model and a shell that renders it. It has no strategy, execution, portfolio, or backtest path. The two capabilities are easy to confuse because both start from the same candles and both produce visible output:

- **Market analysis** describes the market. It runs the model and shows axes, events, legs, aggregates, dashboards, and charts. It never decides a trade.
- **Backtest** simulates a strategy. It applies rules to the model output, creates orders, simulates fills, tracks a position, and produces trades, equity, and P&L.

The shell is the only product surface. A capability that is not reachable by a command does not exist for the user. The model contract forbids strategy vocabulary, entries, exits, stops, targets, confidence, and recommendations in model output. The backtest therefore cannot be a flag on an existing model command, and the model cannot learn to trade.

The data engine also caps a full series at 100,000 M1 candles. A full-year 2025 SOLUSD series is roughly 520k M1 rows. Both analysis and backtest need a shared, explicit way to load it without changing the existing command contracts or the store/research cap.

## 2. Decision

### 2.1 Two separate paths

| Path | Input | Computation | Output | Never does |
|---|---|---|---|---|
| Market analysis | Candles + model configuration | Model run and descriptive views | Dashboard, report, charts, events, legs, aggregates | Create an order, fill, position, trade, or P&L |
| Backtest | Candles + model configuration + strategy configuration + execution and portfolio configuration | Model run, strategy, execution, portfolio, report | Trades, fills, positions, equity, drawdown, P&L, report, charts | Publish model labels or descriptive axis words as if they were a strategy |

The paths are not modes of one command. A user who wants to see the market runs a `market-*` view or `market-report`. A user who wants to know whether a strategy made money runs `backtest`.

### 2.2 Names

| Term | Meaning |
|---|---|
| Model run | The pure computation of readings, four axes, events, legs, and aggregates over the candle ladder. |
| Market analysis | Descriptive use of a model run. |
| Strategy run | Rules applied to model output, producing entries and exits. No fills or P&L. |
| Backtest | Strategy run + execution + portfolio + report. The only path that simulates fills and P&L. |
| Research | The existing dataset export and summary path. It is not market analysis and not a backtest. |

The model already publishes **Signal events** (threshold crossing, series crossover, compact state change). Those are descriptive events. Strategy output is called an **entry** or **exit**, never a model signal. The strategy-only command is `strategy-run`, not `signals`.

### 2.3 Commands

| Command | Path | Contract |
|---|---|---|
| `market-report` | Market analysis | Runs the model over a range and writes `summary.txt`, `report.json`, `price-axes.png`, and `events.png`. No trades or P&L. |
| `strategy-run` | Strategy run | Prints deterministic NDJSON entries and exits. No fills, positions, equity, or P&L. |
| `backtest` | Backtest | Runs the full execution and writes `summary.txt`, `report.json`, `trades.csv`, `price-trades.png`, `equity.png`, and `drawdown.png`. |

Signatures:

```text
market-report <csv> --data-version <id> [--from <date>] [--to <date>] --out <dir> [--json]
strategy-run <csv> --strategy <name> --data-version <id> [--from <date>] [--to <date>] [--json]
backtest <csv> --strategy <name> --data-version <id> [--from <date>] [--to <date>] --out <dir> [--json]
```

- Dates are inclusive UTC dates. Omitting both uses the file's full coverage.
- `--out` must be a new or empty directory; a run never mixes artifacts from another run.
- `--data-version` is the caller-supplied identity of the input series. The report records it.
- The existing `market-dashboard`, `market-chart`, `model-series`, `events`, `legs`, and `axis` commands keep their contracts and remain market-analysis views.
- There is no standalone execution or portfolio command. The `backtest` report is the shell surface for fills, positions, equity, and P&L, satisfying the shell exposure required by #10 and #12 through the full run.

### 2.4 Crate boundaries

One new pure crate, `crates/quantrule-backtest`, with cohesive modules:

```text
quantrule-backtest
  strategy    rules -> entries/exits
  execution   entries/exits + candles -> orders/fills/position changes
  portfolio   fills -> position, cash, equity, P&L, drawdown
  runner      orchestrates model -> strategy -> execution -> portfolio
  report      summary, trades, metrics, chart series
```

Dependency direction:

```text
quantrule-data <- quantrule-model <- quantrule-backtest <- quantrule-shell
```

- `quantrule-backtest` depends on `quantrule-model` and `quantrule-data`.
- `quantrule-shell` depends on `quantrule-backtest`.
- `quantrule-data` and `quantrule-model` never depend on the backtest crate.
- The market-report view lives in `quantrule-model` as a descriptive view, like `DashboardState`. The shell renders it; it does not compute it.
- The mapping module in `quantrule-shell` renders numbers to words. The backtest crate publishes numbers and records, not presentation labels.

The backtest crate is not a pipeline stage. It is a consumer of the model output, and it is the only crate that decides a trade.

### 2.5 Full-year data window

`market-report` and `backtest` use a shared extended-range M1 loader. The loader is the subject of the next work item, D2 (#144). The decision is:

- Existing `market-*` commands, store, and research keep the 100,000 M1 cap.
- Only the two new commands use the extended range, with an explicit documented cap or a streaming path chosen by the D2 benchmark.
- The input is never silently truncated. An over-cap input fails with a deterministic message.
- The report records the actual first and last bar and the data version, not just the requested range.

### 2.6 Determinism and causality

- Strategy rules may read only the current closed bar and earlier bars.
- Events are consumed by their availability bar, never their occurrence bar.
- A fill for a bar never changes when later bars are appended, changed, or removed.
- A backtest over fixed input and configuration produces byte-identical artifacts.
- No engine reads the clock or uses randomness. The report records the configuration and data version.

## 3. Alternatives considered

| Alternative | Why rejected |
|---|---|
| One `backtest` command that also prints the market dashboard or charts | It conflates a descriptive view with a trade simulation. The user could not tell whether a number is a market measurement or a P&L result, and the model's no-trade contract would be blurred. |
| Add strategy and execution to `quantrule-model` | The model must publish numbers about the market and must never decide a trade. Adding entries, exits, stops, or P&L would violate the model contract and the axis independence boundary. |
| Put strategy, execution, and portfolio in `quantrule-shell` | The shell holds no domain logic and is the only layer allowed to do I/O. A backtest is pure computation and belongs in an engine. |
| Three new crates: `quantrule-strategy`, `quantrule-execution`, `quantrule-portfolio` | Cleaner separation, but no independent consumer exists yet. It adds three crate manifests, three dependency edges, and three release surfaces for one product path. The modules can be split later if reuse appears. |
| Call the descriptive path `analytics` or `research` | `quantrule-analytics` already means the research dataset builder, and `research` already means export/summary. Reusing either would make the command vocabulary ambiguous. |
| Call strategy output `signals` | The model already publishes Signal events. Reusing the word for a strategy entry/exit would hide the difference between a descriptive event and a decision. |
| Extend the existing `market-chart` or `market-dashboard` commands with a backtest flag | It would change an existing command contract. A new capability arrives as a new command. |
| Raise `MAX_SERIES` for every command | It would change the existing 100k model, store, and research contracts and their tests. The new commands get a separate, explicit path. |

## 4. Consequences

- The shell has two clearly named paths; neither can be mistaken for the other.
- `market-report` and `backtest` share the extended-range loader but not their outputs.
- The backtest crate becomes the only place that owns strategy rules, fills, positions, and P&L.
- The model remains descriptive and its existing contracts remain unchanged.
- The new commands require `--data-version` and `--out`, so every artifact carries provenance.
- The first strategy and execution defaults are product decisions recorded in the program plan. They are not encoded in the model.
- A future split into separate strategy/execution/portfolio crates remains possible without changing the shell command contracts.
- The 100k store/research cap remains; a full-year backtest is not persisted to the store in this program.
- Multi-instrument portfolios, live execution, optimization, funding, and leverage remain out of scope.

## 5. Verification

This ADR is verified by review of the command contracts and the dependency direction. The implementation work items verify behavior:

- `market-report` produces no order, fill, position, trade, equity, or P&L field.
- `backtest` produces trades, equity, and P&L and does not publish descriptive axis labels as decisions.
- `strategy-run` produces entries/exits but no fills or P&L.
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo doc --no-deps`, and the determinism/causality tests in the program plan pass.
- The full-year SOLUSD 2025 run produces byte-identical artifacts across repeated runs.

## 6. Non-goals

- Multi-instrument portfolios, pair maps, or cross-pair state.
- Paper trading, live execution, broker connectivity, or order routing.
- Walk-forward optimization, parameter search, or strategy tuning.
- Funding, borrow, margin, leverage, or liquidation modeling.
- Intraday M1 fills in the first backtest; v1 fills at the next M15 open.
- Changes to existing `market-*`, `research`, `store`, or `fetch` command contracts.
