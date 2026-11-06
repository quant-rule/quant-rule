# QuantRule

QuantRule is a local, single-user, deterministic shell application for rule-based
research on OHLCV candles. You start it and type commands; it prints or writes what
the data and model engines produce. There is no graphical interface, no service, and
no library to embed.

The current model is **M1.0**: a flat per-timeframe measurement of readings,
discrete states, confirmed levels, events, progression, and context. It publishes
numbers, nulls, and stable codes. It carries no axis score, no composite direction
vote, no dashboard, and no cross-timeframe aggregate.

The current shell surface is deliberately small: data inspection, event records,
the flat model report, policy validation, source fetch, and help. Strategy,
execution, portfolio, and backtest runs are future work; the execution and
portfolio libraries and the generic bracket/event engines remain in the workspace
for S3.x.

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

## Commands

| Command | Arguments | Output |
|---|---|---|
| `candles` | `<csv-path>` | One NDJSON object per retained candle |
| `candles-tolerant` | `<csv-path>` | NDJSON candles on stdout; the quality report on stderr |
| `timeframes` | `<csv-path>` | NDJSON candles per timeframe on stdout; the aggregation report on stderr |
| `events` | `<csv-path> [--window full\|latest]` | One JSON object per registered and extended event |
| `report model` | `--config <run.json> [--spec <spec.json>] [--out <dir>] [--json]` | The flat model bundle, or its artifact paths |
| `policy check` | `--config <run.json>` | One `policy ok ...` line after every config check |
| `fetch` | `<symbol> <start> <end> [--timeframe <1m\|5m\|15m\|1h>] [--csv <path>]` | The CSV path and the accepted/rejected counts |
| `help` | `[<command>\|all\|glossary]` | The command map, one contract, or the glossary |

`events` and `report model` use the data engine's default ladder M1, M5, M15, H1.
`events` uses the default model configuration; `report model` uses the configured
run.

### Examples

```bash
quantrule candles crates/quantrule-shell/tests/fixtures/three.csv
quantrule events crates/quantrule-shell/tests/fixtures/state_m1.csv --window latest
quantrule policy check --config crates/quantrule-shell/tests/fixtures/config-run/run.json
quantrule report model --config crates/quantrule-shell/tests/fixtures/config-run/run.json --out target/report
quantrule report model --config crates/quantrule-shell/tests/fixtures/config-run/run.json --spec crates/quantrule-shell/tests/fixtures/config-run/spec.json --out target/report-spec
quantrule fetch gold 2025-01-01 2025-12-31 --timeframe 1h --csv data/sources/XAUUSD_H1.csv
```

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
  "schema": "quantrule.run/v2",
  "name": "btcusd-2024",
  "model_id": "M1.0",
  "policy_id": "POL1.0",
  "execution_id": "E1.0",
  "portfolio_id": "P1.0",
  "policy": "policy.json",
  "data": "data.json",
  "model": "model.json",
  "execution": "execution.json",
  "portfolio": "portfolio.json",
  "output_root": "../../../data/labs/backtest-btcusd-2024"
}
```

| File | Contract | Current role |
|---|---|---|
| `policy.json` | `quantrule.policy/v1` | Human-owned account, objective, risk, and universe envelope |
| `data.json` | `quantrule.data/v3` | Source path, data version, period, and available ladder |
| `model.json` | `quantrule.model/v4` | M1.0 reading, event, and progression settings |
| `execution.json` | `quantrule.execution/v2` | Fill mechanics and costs for S3.x |
| `portfolio.json` | `quantrule.portfolio/v1` | Capital, sizing, costs, and risk limits for S3.x |

`strategy.json` and `analytics.json` are not part of the current run. The loader
rejects them with an unknown-field error. The old model schema versions 1–3 are
retired and rejected with an unsupported-schema error.

`config/defaults/` and `config/examples/` are the current config tree.
`config/compression*`, `config/model-v4/`, and `config/research/` are accepted
model or historical research evidence, not current run configs. See
[`config/README.md`](config/README.md).

`policy check` loads every file, resolves every path, and validates the cross-config
rules before any candle is read: the data ladder, model ceiling, policy universe,
instrument, execution fill timeframe, and portfolio envelope must agree. An error
names the file and the JSON path, never an absolute path.

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

A `--spec` run adds `conditions.csv`; a `quantrule.model-spec/v2` run also adds
`prediction.csv`, `panel.csv`, and `session.csv` when declared. A bundle is
available if and only if its `report.json` exists;
the writer writes every other artifact first and `report.json` last, and removes a
partial bundle on failure.

The JSON envelope carries `model_version`, `view_contract` (`2026-11-05`),
`data_version`, `requested_period`, and `flat_report`. The result carries no axes,
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
the four-axis chart surface.

## Repository layout

| Path | Role |
|---|---|
| `crates/quantrule-data` | Data engine: normalization, ladder construction, ordering, gaps |
| `crates/quantrule-model` | Model engine: readings, states, levels, events, progression, context, flat report |
| `crates/quantrule-backtest` | Execution, portfolio, and generic bracket/event engines for S3.x |
| `crates/quantrule-shell` | The binary: parse, source access, model orchestration, and rendering |
| `docs/model/2026-11-05/` | Current model contract and model-family registry |
| `docs/registry/` | Indicator/event registry, validation claims, and history boundary |
| `docs/architecture/` | Living architecture baseline |
| `docs/execution-plans/` | Plans of record for substantial work |
| `docs/ai-transparency-log/` | Material AI contributions and verification |

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
python3 tools/check_file_size.py
cargo test
python3 tools/check_legacy.py
python3 tools/check_registry.py
python3 tools/check_validation.py
python3 tools/check_links.py
python3 tools/check_terms.py
```

The registry checks cover 46 indicators, 143 active events, 189 validation
claims, and 54 construct cases. The legacy scan
rejects retired model and strategy labels in the current surface; history
directories are exempt and remain in Git.

## History

The four-axis model, the dashboard, the market state, the consensus view, the
cross-timeframe aggregate, the analytics crate, the SQLite store, the research
export, the lab tooling, and the old shell commands were retired in P3 of
[`docs/execution-plans/2026-11-05/467-clean-current-architecture.md`](docs/execution-plans/2026-11-05/467-clean-current-architecture.md).
Their evidence remains in ADRs, audits, improvement records, execution plans,
release notes, reports, and Git history. The boundary is indexed in
[`docs/registry/history.md`](docs/registry/history.md).

## Non-goals

- Tick data, order books, machine learning, and statistical signal generation.
- A server, GUI, embedded library, live trading, or multi-tenancy.
- A profitability claim from a single backtest.
- Rebuilding the strategy/portfolio surface before S3.x.
