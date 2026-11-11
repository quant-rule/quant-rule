# QuantRule

QuantRule is a local, single-user, deterministic application for rule-based
research on OHLCV candles. The current release is the local desktop app plus the command shell: the app
covers the service-backed workflow, and the shell remains the headless surface.
The desktop application lives in `apps/desktop` and calls the same application
service; there is no account, no deployed service, and no cloud runtime.

The current model is **M1.0**: a flat per-timeframe measurement of readings,
discrete states, confirmed levels, events, progression, and context. It publishes
numbers, nulls, and stable codes. It carries no composite score, no composite direction
vote, no composite view, and no cross-timeframe aggregate.

The current shell surface covers data inspection, event records, the flat model
report, strategy decisions, execution fills, portfolio accounting, the coupled
backtest, qualification, analytics, run validation, source fetch, and help.
Every pipeline stage is implemented. The stage contract and the command status
are in
[`docs/architecture/command-surface.md`](docs/architecture/command-surface.md).
Only the commands in the table below are available today.

The S3 command surface is implemented. No tiered improvement run has used it yet,
and no strategy or portfolio baseline exists. The current status of the model,
strategy, portfolio, data, and tooling is in
[`docs/README.md`](docs/README.md).

## Documentation

| Document | What it answers |
|---|---|
| [`docs/README.md`](docs/README.md) | Global documentation index and current status |
| [`docs/workflows/README.md`](docs/workflows/README.md) | Workflow index, selection guide, status vocabulary, and record map |
| [`docs/architecture/README.md`](docs/architecture/README.md) | Architecture baseline, pipeline, and reading order |
| [`docs/course/README.md`](docs/course/README.md) | Course deliverables and PDF build |
| [`docs/architecture/command-surface.md`](docs/architecture/command-surface.md) | Stage contract, command registry, and discovery gate |
| [`docs/model/2026-11-05/model-m1.0.md`](docs/model/2026-11-05/model-m1.0.md) | M1.0 model contract |
| [`docs/config/2026-11-11/run-v3-and-portfolio-constraints.md`](docs/config/2026-11-11/run-v3-and-portfolio-constraints.md) | Run v3 and portfolio constraints |
| [`docs/registry/README.md`](docs/registry/README.md) | Indicator, event, parameter, validation, and verification registry |
| [`docs/improvement/README.md`](docs/improvement/README.md) | Run index, baselines, invalidation tombstones, and run record template |
| [`docs/releases/README.md`](docs/releases/README.md) | Release index, tag rule, historical tags, and release-note template |
| [`CHANGELOG.md`](CHANGELOG.md) | Grouped release history for meaningful user-facing or developer-facing changes |

## Build and run

```bash
cargo build --release
./target/release/quantrule
```

The interactive prompt reads commands from stdin. A non-interactive invocation runs
one command and exits with the command's status:

```bash
./target/release/quantrule candles crates/quantrule-shell/tests/fixtures/three.csv
```

## Desktop

The desktop UI is a local Tauri 2 application with a Svelte 5 frontend. It covers
workspace, data, configs, model run, research memory, export/backup, and settings.
Backtest, strategy, execution, portfolio, and analytics remain CLI operations.

```bash
cd apps/desktop
npm ci
npm run dev          # browser fixture backend
npm run tauri dev    # native window
```

See [`apps/desktop/README.md`](apps/desktop/README.md) for the native build and
test commands. The Linux AppImage/deb bundles and the static download page are
built with `./site/build.sh`; the page source is [`site/index.html`](site/index.html).

## Commands

| Command | Arguments | Output |
|---|---|---|
| `candles` | `<csv-path>` | One NDJSON object per retained candle |
| `candles-tolerant` | `<csv-path>` | NDJSON candles on stdout; the quality report on stderr |
| `timeframes` | `<csv-path>` | NDJSON candles per timeframe on stdout; the aggregation report on stderr |
| `events` | `<csv-path> [--window full\|latest]` | One JSON object per registered and extended event |
| `report model` | `--config <run.json> [--spec <spec.json>] [--out <dir>] [--json]` | The flat model bundle, or its artifact paths |
| `strategy explain` | `--model <model-bundle> --strategy <strategy.json> --bar <index> [--position flat\|long\|short]` | One JSON decision record and its as-of evidence |
| `strategy run` | `--model <model-bundle> --strategy <strategy.json> [--out <dir>] [--json]` | The decisions bundle, or its artifact paths |
| `execution run` | `--decisions <strategy-bundle> --config <run.json> [--out <dir>] [--json]` | The execution bundle, or its artifact paths |
| `portfolio replay` | `--fills <execution-bundle> --config <run.json> [--out <dir>] [--json]` | The portfolio bundle, or its artifact paths |
| `backtest run` | `--config <run.json> --strategy <strategy.json> [--out <dir>] [--json]` | The manifest and stage bundle paths |
| `qualify` | `--evidence <backtest-bundle> --criteria <qualification.json> [--out <dir>] [--json]` | The verdict, criterion counts, and the qualification bundle |
| `analytics evaluate` | `--evidence <backtest-bundle> [--out <dir>] [--json]` | The metrics, benchmarks, tables, and summary paths |
| `run check` | `--config <run.json>` | One `run ok ...` line after every config check |
| `fetch` | `<symbol> <start> <end> [--timeframe <1m\|5m\|15m\|1h>] [--csv <path>]` | The CSV path and the accepted/rejected counts |
| `help` | `[<command>\|all\|glossary\|workflows]` | The command map, one contract, the glossary, or the workflow selection guide |

`events` uses the data engine's default ladder M1, M5, M15, H1 and the default
model configuration. `report model` uses the configured run's ladder and model
configuration.

### Examples

```bash
quantrule candles crates/quantrule-shell/tests/fixtures/three.csv
quantrule events crates/quantrule-shell/tests/fixtures/state_m1.csv --window latest
quantrule run check --config crates/quantrule-shell/tests/fixtures/config-run/run.json
quantrule report model --config crates/quantrule-shell/tests/fixtures/config-run/run.json --out target/report
quantrule report model --config crates/quantrule-shell/tests/fixtures/config-run/run.json --spec crates/quantrule-shell/tests/fixtures/config-run/spec.json --out target/report-spec
quantrule strategy explain --model target/report/model --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json --bar 21
quantrule strategy run --model target/report/model --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json --out target/report
quantrule execution run --decisions target/report/strategy --config crates/quantrule-shell/tests/fixtures/config-run/run.json --out target/report
quantrule portfolio replay --fills target/report/execution --config crates/quantrule-shell/tests/fixtures/config-run/run.json --out target/report
quantrule backtest run --config crates/quantrule-shell/tests/fixtures/config-run/run.json --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json --out target/backtest
quantrule qualify --evidence target/backtest/backtest --criteria crates/quantrule-shell/tests/fixtures/qualification/pass.json --out target/backtest
quantrule analytics evaluate --evidence target/backtest/backtest --out target/backtest
quantrule fetch gold 2025-01-01 2025-12-31 --timeframe 1h --csv data/sources/XAUUSD_H1.csv
quantrule help workflows
```

The command table is the complete current surface. A planned command is not
listed here until it is implemented, documented in help, and demonstrated in a
tutorial.

## Pipeline stages

The target is a stage-isolated surface: every stage can be run, stopped, and
inspected on its own. The command names, inputs, outputs, and implemented/planned
status are in [`docs/architecture/command-surface.md`](docs/architecture/command-surface.md).
The stage table below is the target, not a promise that a command exists today.

| Stage | Engine | Input | Output | Status |
|---:|---|---|---|---|
| 1 | Data Engine | Raw OHLCV | Normalized candles, timeframes, gaps, quality | Implemented |
| 2 | Model Engine | Candles/timeframes and model config | Readings, states, levels, events, progression, context | Implemented |
| 3 | Strategy Engine | Model measurements, strategy config, position state | Decisions/signals and as-of rationale | Implemented |
| 4 | Execution Engine | Decisions, execution config, portfolio sizing config, candles | Orders, fills, rejections | Implemented |
| 5 | Portfolio Engine | Fills, portfolio config, marks | Positions, equity, PnL, exposure, drawdown | Implemented |
| 6 | Qualification Engine | Evidence bundle and human-owned criteria | Qualification record and verdict | Implemented |
| 7 | Analytics Engine | Result artifacts | Descriptive metrics, buy-and-hold/random-timing benchmarks, and evaluation tables | Implemented |

The Qualification Engine evaluates the complete evidence set, not profitability
alone. Its verdict is `pass`, `fail`, or `insufficient evidence`; the human makes
the final acceptance decision. The criteria contract is in
[`docs/config/2026-11-06/qualification-criteria.md`](docs/config/2026-11-06/qualification-criteria.md).

The Analytics Engine publishes descriptive metrics, buy-and-hold and matched
random-timing benchmarks, and evaluation tables from the result artifacts. It
writes no verdict, no acceptance, and no strategy decision. A metric or table the
evidence does not support is `null` or unsupported with a reason code. The
benchmark contract is
[`docs/config/2026-11-09/analytics-benchmarks.md`](docs/config/2026-11-09/analytics-benchmarks.md).

## The M1.0 model

The model runs once per configured ladder. For every timeframe it publishes:

| Field | Meaning |
|---|---|
| `readings` | EMA 20/50/200, RSI 14, ATR 14, RVOL 20, MACD, ADX/DI |
| `states` | EMA stack, RSI zone, MACD state and histogram sign, ADX regime and slope, DI state |
| `levels` | Confirmed swing highs and lows at the minor and major scales, with occurrence and availability bars |
| `events` | Level, Divergence, and Signal records; each carries type, subtype, source, direction, occurrence bar, availability bar, and measured values |
| `progression` | The structural leg at each scale: origin, target, progress, displacement, lifecycle, and stage |
| `context` | ATR ratio, RVOL ratio, and the context code |

A missing measurement is `null`, never zero. A valid zero is a value. Ordinal codes
are counted and compared, never averaged. The model never reads a later bar through
an earlier output.

The full contract is
[`docs/model/2026-11-05/model-m1.0.md`](docs/model/2026-11-05/model-m1.0.md).
The indicator and event registry is [`docs/registry/`](docs/registry/).

## Configuration

A run is described by `run.json`:

```json
{
  "schema": "quantrule.run/v3",
  "name": "btcusd-2024",
  "model_id": "M1.0",
  "execution_id": "E1.0",
  "portfolio_id": "P1.0",
  "data": "data.json",
  "model": "model.json",
  "execution": "execution.json",
  "portfolio": "portfolio.json",
  "output_root": "../../../data/labs/backtest-btcusd-2024"
}
```

| File | Contract | Current role |
|---|---|---|
| `data.json` | `quantrule.data/v3` | Source path, data version, period, and available ladder |
| `model.json` | `quantrule.model/v4` | M1.0 reading, event, and progression settings |
| `execution.json` | `quantrule.execution/v2` | Fill mechanics and costs |
| `portfolio.json` | `quantrule.portfolio/v2` | Capital, sizing, costs, and risk limits; both directions are always available |

`strategy.json` and `analytics.json` are not part of the current run. The loader
rejects them with an unknown-field error. The strategy file and the qualification
criteria are explicit command arguments; the run schema stays
`quantrule.run/v3`. The old model schema versions 1–3 are retired and rejected
with an unsupported-schema error.

`config/defaults/` and `config/examples/` are the current config tree.
`config/compression*`, `config/model-v4/`, and `config/research/` are accepted
model or historical research evidence, not current run configs. See
[`config/README.md`](config/README.md).

`run check` loads every file, resolves every path, and validates the cross-config
rules before any candle is read: the data ladder, model ceiling, model ladder,
instrument, execution fill timeframe, and portfolio configuration must agree. An
error names the file and the JSON path, never an absolute path.

## `report model` bundle

`report model` writes `<out>/model/` and refuses a non-empty target. The bundle
contains:

| Artifact | Contents |
|---|---|
| `summary.txt` | Coverage, events, latest readings/states/context, and progression |
| `report.json` | The flat report envelope, carrying numbers and codes only |
| `result.json` | The `quantrule.model-result/v4` result with the effective config |
| `readings.csv` | Per-bar readings, states, progression, context, and extended readings |
| `events.csv` | The registered and extended event batch |
| `levels.csv` | Every confirmed level |
| `legs.csv` | Every retained progression leg |
| `provenance.json` | Data version, config hash, period, config file names, and artifact list |
| `panel.csv` | Panel readings when a v2 spec declares a panel |
| `session.csv` | Session readings, including `calendar_bucket`, when a v2 spec declares a session |

`readings.csv` keeps the context ratios as `atr_ratio` and `rvol_ratio`. The
extended ratio columns are named `extended_atr_ratio` and
`extended_rvol_ratio`, so a CSV consumer cannot confuse the two.

A v1 `--spec` run adds `conditions.csv`. A `quantrule.model-spec/v2` run adds
`prediction.csv`; it adds `conditions.csv` when conditions are declared, and
`panel.csv`/`session.csv` when those are declared. A bundle is available if and
only if its `report.json` exists;
the writer writes every other artifact first and `report.json` last, and removes a
partial bundle on failure.

The JSON envelope carries `model_version`, `view_contract` (`2026-11-05`),
`data_version`, `requested_period`, and `flat_report`. The result carries no composite views,
no cross-timeframe aggregate, no charts, and no confidence field.

## Determinism

The same candles, data version, and configuration produce the same output, every
time, interactively or non-interactively. There is no wall-clock time, unseeded
randomness, or unordered iteration in the signal path. Shell commands sort their
input explicitly and print in a fixed order. `cargo test` includes causality,
determinism, golden-fixture, and byte-identical rerun tests.

A result that is meant to be reproducible records its data version and config hash
in `provenance.json`.

## Fetch

`fetch` reads a Dukascopy range through the shell's explicit network transport and
writes the accepted candles as CSV. The default path is
`data/sources/<SYMBOL>_<TF>_<start>_<end>.csv`. The CSV is written through a
sibling temporary file and renamed only after every row is accepted, so a failed
fetch leaves no partial file under the requested name. A result above the batch cap
prints a warning on stderr.

The current fetch surface writes CSV only. The old PNG chart option was removed with
the retired chart surface. A dense Binance BTCUSDT cross-venue source is built
with `tools/data/binance_klines.py`; it is a research input, not a replacement
for the shell fetch surface. Trade charts are rendered from recorded bundles with
`tools/charts/trade_charts.py`.

## Repository layout

| Path | Role |
|---|---|
| `crates/quantrule-data` | Data engine: normalization, ladder construction, ordering, gaps |
| `crates/quantrule-model` | Model engine: readings, states, levels, events, progression, context, flat report |
| `crates/quantrule-backtest` | Strategy, execution, portfolio, and benchmark engines |
| `crates/quantrule-app` | Application service: workspace, dataset, configuration, model run, export, and local research memory |
| `crates/quantrule-shell` | The command shell: parse, call the application service, and render |
| `apps/desktop` | Tauri 2 + Svelte 5 desktop UI over the application service |
| `site/` | Static Linux download page, packaging script, and checksum checker |
| `tools/data/` | Binance archive importer and source coverage audit |
| `tools/charts/` | Deterministic trade-chart renderer and click annotator |
| `tools/trades/` | Per-trade 250-bar context, annotations, and exit-quality evaluation |
| `docs/model/2026-11-05/` | Current model contract and model-family registry |
| `docs/registry/` | Indicator/event registry, validation claims, and history boundary |
| `docs/architecture/` | Living architecture baseline |
| `docs/workflows/` | Living workflow contracts, indexed by `docs/workflows/README.md` |
| `docs/refactors/` | Read-only structural assessments and refactor findings |
| `docs/execution-plans/` | Plans of record for substantial work |
| `docs/ai-transparency-log/` | Material AI contributions and verification |
| `docs/releases/` | Release notes, the release index, and the tag rule |
| `CHANGELOG.md` | Grouped release history for meaningful changes |

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
python3 tools/check_file_size.py
python3 tools/check_structure.py
python3 tools/check_workflows.py
python3 tools/check_execution_plans.py
python3 tools/check_claims.py
python3 tools/check_changelog.py
python3 tools/check_releases.py
python3 tools/check_partitions.py
cargo test
python3 tools/check_command_surface.py
python3 tools/check_legacy.py
python3 tools/check_registry.py
python3 tools/check_validation.py
python3 tools/check_links.py
python3 tools/check_terms.py
```

The command-surface check compares `help.rs`, the README command table, the
command registry, and the tutorial invocations. A planned command must not appear
in the available surface. The registry checks cover 49 indicators, 150 active
events, 199 validation claims, and 54 construct cases. The legacy scan rejects
retired model and strategy labels in the current surface; history directories are
exempt and remain in Git.

## History

The retired composite model, dashboard, market-state, consensus, aggregate,
analytics, database-store, and research-export surfaces, the lab tooling, and
the old shell commands were retired in P3 of
[`docs/execution-plans/2026-11-05/467-clean-current-architecture.md`](docs/execution-plans/2026-11-05/467-clean-current-architecture.md).
Their evidence remains in ADRs, audits, improvement records, execution plans,
release notes, reports, and Git history. The boundary is indexed in
[`docs/registry/history.md`](docs/registry/history.md).

## Non-goals

- Tick data, order books, machine learning, and statistical signal generation.
- A server, GUI, embedded library, live trading, or multi-tenancy.
- A profitability claim from a single backtest.
- Treating a model measurement, a qualification verdict, or a green backtest as
  an accepted edge.
