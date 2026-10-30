# QuantRule

A deterministic, rule-based trading signal engine.

QuantRule is a shell application. You start it and write commands — print a candle series, compute an indicator, run a backtest. The shell is the whole interface: there is no graphical interface, no service, and no library to embed.

## Set up once

These steps are per machine, not per run. They put the Rust toolchain and the
binary on `PATH`, so later commands are just `quantrule ...` from any directory.
The examples use `~/.bashrc`. If your shell is zsh, substitute `~/.zshrc`.

**1. Put the Rust toolchain on `PATH`.** The workspace pins Rust 1.99
(`rust-version` in `Cargo.toml`). Install it with [rustup](https://rustup.rs) if
`cargo` is missing. The installer usually adds `~/.cargo/bin` to the shell
profile; if `cargo --version` still fails in a new terminal, append it yourself:

```bash
# Only when rustup is absent:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Only when `cargo --version` still fails after reopening the terminal:
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
exec "$SHELL" -l
cargo --version
```

Do not install a second `cargo` from the distribution package manager. It can
shadow rustup and compile the project with a different toolchain.

**2. Build the release binary once, from the repository root.**

```bash
cargo build --release -p quantrule-shell
```

**3. Put the binary on `PATH`.** Run this from the repository root, so `$PWD`
expands to the absolute path:

```bash
echo "export PATH=\"$PWD/target/release:\$PATH\"" >> ~/.bashrc
exec "$SHELL" -l
quantrule help | head -3
```

`quantrule` now resolves from any directory. Config-internal paths stay anchored
to the config files, so `quantrule backtest --config /absolute/path/run.json`
works from anywhere. If you would rather not change your shell, call the binary
by path instead: `./target/release/quantrule`.

**4. Fetch the example data once.** The full-year example reads
`data/SOLUSD_M1_2025-01-01_2025-12-31.csv`, which is not committed:

```bash
quantrule fetch sol/usd 2025-01-01 2025-12-31 \
  --csv data/SOLUSD_M1_2025-01-01_2025-12-31.csv
```

It downloads about 31 MB and takes a few minutes.

**5. Verify with one full run.**

```bash
test -f data/SOLUSD_M1_2025-01-01_2025-12-31.csv && echo "data ready"
lab=data/lab-result-2026-10-29T12-00-00Z
quantrule backtest --config config/examples/solusd-2025/run.json --out "$lab"
quantrule lab summary "$lab" \
  --role researcher \
  --data-version dukascopy-bid-solusd-m1-2025-01-01_2025-12-31 \
  --config config/examples/solusd-2025/run.json \
  --created-utc 2026-10-29T12:00:00Z
```

The backtest prints five bundle paths under `$lab`; `lab summary` adds
`$lab/summary.txt` for the whole folder. Replace the timestamp with the UTC
time of the run: a lab result folder uses `YYYY-MM-DDTHH-MM-SSZ`, and
`--created-utc` uses `YYYY-MM-DDTHH:MM:SSZ`. The backtest takes about 20 seconds
on a release build. When it does, the machine is set up.

When the code changes, rebuild with `cargo build --release -p quantrule-shell`;
the `PATH` entry keeps pointing at the new binary.

## Learn the pipeline

The [full-pipeline tutorial](docs/tutorials/2026-10-29/full-pipeline.md) runs
every stage through shell commands: data ingestion, the four-axis model,
research export, strategy decisions, the five-bundle backtest, the SQLite store,
and the lab summary. It also contains one lab for each quant role.

## Releases

| Tag | Title | Notes |
|---|---|---|
| `v0.3.0` | Lab Summary and Help | [`docs/releases/2026-10-30/v0.3.0.md`](docs/releases/2026-10-30/v0.3.0.md) |
| `v0.2.0` | MVP v1 | [`docs/releases/2026-10-28/v0.2.0.md`](docs/releases/2026-10-28/v0.2.0.md) |

## Policy, strategy, and improvement

QuantRule has three human-visible levels:

| Level | File | What it decides |
|---|---|---|
| Policy | `policy.json` | Account size, performance target, max drawdown, position size, leverage, tickets, timeframes |
| Strategy | `strategy.json` (with the human thesis) | Thesis, entry, exit, and risk/reward in trader words |
| Engine contracts | `data.json`, `model.json`, `execution.json`, `portfolio.json`, `analytics.json` | Measurement, fills, accounting, evaluation |

Validate the envelope and render the strategy:

```bash
cargo run -p quantrule-shell -- policy check --config config/examples/solusd-2025/run.json
cargo run -p quantrule-shell -- strategy explain --config config/examples/solusd-2025/run.json
```

The improvement loop is documented in
[`docs/tutorials/2026-10-30/improvement-loop.md`](docs/tutorials/2026-10-30/improvement-loop.md),
the charter in [`docs/governance/2026-10-30/improvement-charter.md`](docs/governance/2026-10-30/improvement-charter.md),
and the finding and experiment formats in
[`docs/audits/README.md`](docs/audits/README.md) and
[`docs/experiments/README.md`](docs/experiments/README.md).

## Build and run

The workspace uses the Rust version in `Cargo.toml`. After the setup above,
`quantrule` is the same binary as `cargo run -p quantrule-shell --`, so the
commands below can drop the `cargo run` prefix.

```bash
cargo run -p quantrule-shell
```

The prompt is written to stderr. stdout is reserved for command output, so the same command can be piped or captured.

Non-interactive invocation:

```bash
cargo run -p quantrule-shell -- candles path/to/candles.csv
cargo run -p quantrule-shell -- candles-tolerant path/to/candles.csv
cargo run -p quantrule-shell -- timeframes path/to/m1.csv
cargo run -p quantrule-shell -- market-state path/to/m1.csv
cargo run -p quantrule-shell -- market-state path/to/m1.csv --window latest
cargo run -p quantrule-shell -- market-dashboard path/to/m1.csv
cargo run -p quantrule-shell -- market-dashboard path/to/m1.csv --window latest --json
cargo run -p quantrule-shell -- market-chart path/to/m1.csv --png data/market.png --timeframe 1m --candles 1000
cargo run -p quantrule-shell -- axis path/to/m1.csv direction
cargo run -p quantrule-shell -- events path/to/m1.csv
cargo run -p quantrule-shell -- legs path/to/m1.csv
cargo run -p quantrule-shell -- model-series path/to/m1.csv --timeframe 1m --bars 1000
cargo run -p quantrule-shell -- research export path/to/m1.csv demo --data-version binance:BTCUSDT:60:1704067200-1710115200
cargo run -p quantrule-shell -- research summary path/to/m1.csv
cargo run -p quantrule-shell -- store init --db data/quantrule.db
cargo run -p quantrule-shell -- store ingest path/to/m1.csv --dataset demo --data-version binance:BTCUSDT:60:1704067200-1710115200
cargo run -p quantrule-shell -- store verify path/to/m1.csv --dataset demo --data-version binance:BTCUSDT:60:1704067200-1710115200 --db data/quantrule.db
cargo run -p quantrule-shell -- store list --db data/quantrule.db
cargo run -p quantrule-shell -- store show demo bar --timeframe 1m --limit 10 --db data/quantrule.db
cargo run -p quantrule-shell -- db series demo --timeframe 1m --limit 10 --db data/quantrule.db
cargo run -p quantrule-shell -- db bars demo --limit 10 --db data/quantrule.db
cargo run -p quantrule-shell -- db metrics demo --limit 10 --db data/quantrule.db
cargo run -p quantrule-shell -- db layer3 demo --limit 10 --db data/quantrule.db
cargo run -p quantrule-shell -- fetch gold 2025-01-01 2025-12-31 --csv data/XAUUSD_M1_2025-01-01_2025-12-31.csv
cargo run -p quantrule-shell -- fetch eurusd 2026-01-01 2026-06-30 --timeframe 1h --csv data/EURUSD_H1_2026-01-01_2026-06-30.csv
cargo run -p quantrule-shell -- fetch gold 2025-01-01 2025-12-31 --png data/xauusd_2025.png
```

A full config-driven run, its five report bundles, and the root lab summary:

```bash
cargo run -p quantrule-shell -- backtest --config config/examples/solusd-2025/run.json --out data/lab-result-2026-10-29T12-00-00Z
cargo run -p quantrule-shell -- report portfolio --config config/examples/solusd-2025/run.json --out data/portfolio-report
cargo run -p quantrule-shell -- lab summary data/lab-result-2026-10-29T12-00-00Z --role researcher --data-version dukascopy-bid-solusd-m1-2025-01-01_2025-12-31 --config config/examples/solusd-2025/run.json --created-utc 2026-10-29T12:00:00Z
```

## Commands

| Command | Arguments | Output |
|---|---|---|
| `candles` | `<csv-path>` | Up to the last 1000 candles as NDJSON, oldest first |
| `candles-tolerant` | `<csv-path>` | Up to the last 1000 valid candles as NDJSON, plus a quality report on stderr |
| `timeframes` | `<csv-path>` | M1, M5, M15, and H1 as NDJSON, plus an aggregation report on stderr |
| `market-state` | `<csv-path>` [`--window full\|latest`] | The four-timeframe market state as one JSON object |
| `market-dashboard` | `<csv-path>` [`--window full\|latest`] [`--json`] | The four-axis text dashboard, or the numeric dashboard state as one JSON object |
| `market-chart` | `<csv-path> --png <path>` [options] | A deterministic PNG of the last bars; `--window latest` supports inputs above the 100k cap |
| `market-report` | `<csv-path> --data-version <id> --out <dir>` [options] | A deterministic full-period report: `summary.txt`, `report.json`, and two PNGs |
| `strategy-run` | `<csv-path> --strategy <name> --data-version <id>` [options] | Strategy entries and exits as NDJSON; no fills, positions, equity, or P&L |
| `strategy explain` | `--config <run.json>` | The validated strategy thesis, entry, exit, and risk/reward in trader words |
| `policy check` | `--config <run.json>` | Validate the policy against the strategy and engine configs |
| `report` | `<model\|strategy\|execution\|portfolio\|analytical> --config <run.json>` [options] | One bundle under `<out>/<engine>/`; with `--json`, the `report.json` bytes |
| `backtest` | `--config <run.json>` [options] | One full run and all five bundles; the model is computed once and the analytical summary streams from it; with `--json`, a summary envelope |
| `lab summary` | `<lab-dir> --role <role> --data-version <id> --config <run.json> --created-utc <ts>` [options] | A deterministic root `summary.txt` for a lab result folder; prints `wrote <path>` |
| `axis` | `<csv-path> <axis>` [`--window full\|latest`] | One NDJSON line per timeframe for `direction`, `momentum`, `structure`, or `progression` |
| `events` | `<csv-path>` [`--window full\|latest`] | Every event record in the analyzed window as NDJSON, in availability order |
| `legs` | `<csv-path>` [`--window full\|latest`] | The minor and major leg per timeframe as NDJSON |
| `model-series` | `<csv-path>` [options] | The last complete model bars as NDJSON: candle, readings, four axes, context, and consensus |
| `research export` | `<csv-path> <dataset> --data-version <id>` [options] | The raw M1 file and the three research layers, with the manifest, dictionary, and code table |
| `research summary` | `<csv-path>` | Per-axis counts, ranges, ordinal histograms, and pairwise continuous correlations |
| `store init` | [options] | Creates or opens the SQLite store and prints its schema version |
| `store ingest` | `<csv-path> --dataset <name> --data-version <id>` [options] | Writes one dataset, every ladder bar, every consensus row, every M1 aggregate row, every pivot, every confirmed level, every event, and every retained leg, and prints the stored counts |
| `store verify` | `<csv-path> --dataset <name> --data-version <id>` [options] | Rebuilds the source and compares every stored table; prints `verified ...` or the first mismatch |
| `store list` | [options] | One CSV row per stored dataset with identity, counts, and Layer 3 state |
| `store show` | `<dataset> <table>` [options] | CSV rows from one stored table |
| `store remove` | `<dataset>` [options] | Removes one dataset and its cascading rows; the database file remains |
| `db series` | `<dataset>` [options] | Stored candles as NDJSON |
| `db bars` | `<dataset>` [options] | Full stored bar rows as CSV |
| `db metrics` | `<dataset>` [options] | Metrics and resolutions as CSV |
| `db layer3` | `<dataset>` [options] | Materialized Layer 3 rows as CSV |
| `fetch` | `<symbol> <start> <end>` [options] | A CSV of native BID candles and an optional PNG chart |
| `help` | [`<command>` \| `all` \| `glossary` \| `tutorial` \| `roles`] | The short map, one command contract, or a reference topic |
| `quit` | none | Exits with status 0 |

### Help contract

`help` has three levels and three reference topics. The short map, the command
pages, and `help all` render from one command metadata table, so they cannot
drift. The output is fixed-width plain text with no terminal-width dependence
and no color.

| Command | Output |
|---|---|
| `help` | A one-screen map of the pipeline, commands, and reference topics. |
| `help <command>` | The full contract for one command. One- or two-word topics such as `store ingest`, `research export`, and `lab summary` are accepted. |
| `help all` | Every command and option. |
| `help glossary` | Precise terms such as `NDJSON`, `report bundle`, `data version`, the four axes, events, and legs. |
| `help tutorial` | The full-pipeline tutorial map and its repository path. |
| `help roles` | The Quant Researcher, Quant Developer, and Quant Validator map and handoff. |

An unknown topic is an error that points to `help all`.

`fetch` options:

| Option | Meaning |
|---|---|
| `--timeframe <tf>` | Native interval: `1m`, `5m`, `15m`, or `1h` (default `1m`; `M1`, `M5`, `M15`, and `H1` are also accepted) |
| `--csv <path>` | CSV target (default `data/<SYMBOL>_<M1\|M5\|M15\|H1>_<start>_<end>.csv`) |
| `--png <path>` | Also write a chart; the PNG is opt-in |
| `--candles <n>` | Candles in the chart, from the tail of the fetched range (default 1000, max 1000) |

`research export` options:

| Option | Meaning |
|---|---|
| `--data-version <id>` | Required identity of the input series: source, timeframe, and coverage |
| `--root <dir>` | Export root (default `data`) |

`research export` reads an M1 CSV, builds M1/M5/M15/H1, and writes `<root>/raw/<dataset>/m1.csv` and `<root>/processed/<dataset>/` with `layer1.csv`, `layer2.csv`, `layer3.csv`, `manifest.json`, `data_dictionary.csv`, `data_dictionary.md`, `codes.csv`, and `README.md`. A native higher-timeframe file is rejected before any file is written. The manifest records the data version, the model and contract versions, the configuration, the row counts, and the data file list. The export is deterministic: the same input and configuration produce byte-identical files. The input is capped at 100,000 M1 candles; an over-cap input fails rather than being truncated.

`research summary` prints one line per timeframe and axis with the present count, null count, minimum, maximum, mean, and the ten ordinal bucket counts. It then prints one line per timeframe and axis pair with the Pearson correlation of the two continuous values. Ordinal codes are counted, never averaged, and no line combines the axes. A native higher-timeframe file is rejected before any line is printed. The output is deterministic: the same input prints the same text.

`store init` options:

| Option | Meaning |
|---|---|
| `--db <path>` | Database path (default `data/quantrule.db`) |

`store init` opens the SQLite store, applies the schema migrations in order, each in one transaction (version 1, then the version 2 event-index migration), and prints the path and the schema version. Reopening an existing store is idempotent, and a version 1 database is migrated in place. A store whose schema is newer than the binary is rejected, and a file that is not a database fails with a clear error. The store is a shell-owned module; the data, model, and analytics crates remain pure.

`store remove <dataset> [--db <path>]` removes one dataset and every row that references it through the foreign-key cascade. It never deletes the database file; other datasets remain. Removing a missing dataset is an error. A same-name dataset with a different identity is a conflict until it is removed explicitly.

`store ingest` options:

| Option | Meaning |
|---|---|
| `--dataset <name>` | Required unique dataset name |
| `--data-version <id>` | Required identity of the input series: source, timeframe, and coverage |
| `--db <path>` | Database path (default `data/quantrule.db`) |

`store ingest` reads one M1 CSV, builds M1/M5/M15/H1 in memory, and writes one `dataset` row, one wide `bar` row per Layer 2 row, one `consensus` row per closed bar per timeframe, one `layer3` row per M1 bar, one `pivot` row per pivot occurrence, one `level` row per confirmed level, one `event` row per event record, and one `leg` row per retained progression leg in a single `BEGIN IMMEDIATE` transaction. It writes the candle, the six readings, the four-axis metrics, and the 12 axis-resolution columns for every ladder bar; a missing value is `NULL` with its unavailable-reason code, and a valid zero stays `0`. The resolution columns carry each axis's continuous value, ordinal bucket, and unavailable reason. The consensus row carries the three axis counts and the weakest directional axis code; a missing axis is excluded, and Progression does not vote. The Layer 3 row carries the as-of M5/M15/H1 block columns and the 82 cross-timeframe aggregate columns over the configured ladder; a missing measurement is `NULL`, a count of zero stays `0`, and a list is `;`-separated text with no spaces, with an empty list stored as an empty string rather than `NULL`. A block that has no closed higher bar yet is `NULL` in all its columns; a new dataset is marked `layer3_materialized = 1`. A #68–#72 dataset is not backfilled and stays `0`. A pivot candidate is available on its occurrence bar; a confirmed pivot carries the confirmation bar as its availability bar and is stored once, with `confirmed_code = 1`, rather than duplicated. Every level carries its `1000 + n` reference code and both its occurrence and availability bars. Every event is stored once, on or after its availability bar, and never before it. Coincident Signal threshold crossings are distinct rows because the threshold value is part of their identity; every other event keeps the original occurrence key. Every retained leg carries its origin, target, progress, displacement, and lifecycle codes; a value that is not yet knowable is `NULL`. The M1 `bar` count equals the accepted source rows; the full count covers the whole ladder. The input is capped at 100,000 M1 candles and a count mismatch or over-cap input is rejected before anything is written; a native higher-timeframe file is rejected before the database is opened. Re-running the same name with the same identity is a no-op that prints `unchanged`; a different identity for an existing name is a conflict and changes nothing. Output is one deterministic line: `ingested <name> bars=<n> timeframes=<k>` or `unchanged <name> bars=<n> timeframes=<k>`.

`store verify` reads the same source again, rebuilds the layers, opens the store read-only, and compares the dataset identity, the foreign-key graph, and every row of `bar`, `consensus`, `layer3`, `pivot`, `level`, `event`, and `leg`. A success prints `verified <name> bars=<n> tables=7`; the first value, count, or structural mismatch fails with a deterministic message. A dataset with `layer3_materialized = 0` is incomplete and must be re-ingested into a fresh database. Verification never writes, migrates, or repairs the store.

### Store query contract

The query commands open an existing database read-only. A missing file is an error and is never created; a newer schema is rejected; no migration runs. CSV uses an empty field for `NULL`, the shortest round-trip decimal for a float, and quotes a field that contains a comma, quote, or newline. `--limit` is `1..=1000` and defaults to `100`. `--timeframe` accepts `1m`, `5m`, `15m`, or `1h`; `--bar` filters `bar_index` where the table has one.

| Command | Output |
|---|---|
| `store remove <dataset>` | Removes the dataset and every row that references it; prints `removed <name> bars=<n>`; the database file is kept |
| `store list` | One CSV row per dataset, sorted by name: identity, `m1_bars`, `layer3_materialized`, and `bar`/`consensus`/`layer3`/`pivot`/`level`/`event`/`leg` counts |
| `store show <dataset> <table>` | Every column of `bar`, `consensus`, `layer3`, `pivot`, `level`, `event`, or `leg`, in DDL order and primary-key order. `event.reference_code = 0` is rendered as `NULL`. |
| `db series <dataset>` | NDJSON with `dataset`, `timeframe_secs`, `bar_index`, ISO `open_time`, OHLC, and nullable volume; oldest first |
| `db bars <dataset>` | The full `bar` table as CSV (74 columns) |
| `db metrics <dataset>` | `timeframe_secs`, `bar_index`, the 47 metric columns, and the 12 resolution columns as CSV (61 columns) |
| `db layer3 <dataset>` | The materialized `layer3` table as CSV (285 columns) |

`candles` reads the file one row at a time. A missing file, bad header, malformed row, invalid OHLC, duplicate timestamp, or out-of-order timestamp aborts the command. The shell builds the window before writing, so a late error produces no partial NDJSON.

`candles-tolerant` reads the same file but skips row-level failures: wrong field count, invalid OHLC, non-finite value, bad timestamp, duplicate timestamp, and out-of-order timestamp. A missing file or bad header still aborts. It builds the window first, then writes the retained valid candles as NDJSON to stdout and the quality report to stderr. A run with skipped rows, including a file where every data row is rejected, exits with status 0.

Interactive errors are printed to stderr and keep the session open. Non-interactive errors exit with status 1. A successful non-interactive `candles` run writes nothing to stderr; a successful `candles-tolerant` run writes its quality report to stderr, and a successful `timeframes` run writes its aggregation report to stderr.

### CSV contract

| Item | Value |
|---|---|
| Header | `Date,Open,High,Low,Close,Volume` |
| Timestamp | `YYYY-MM-DD HH:MM:SS+00:00` (UTC) |
| Columns | Exactly six; volume may be empty |
| Prices | Open, high, low, and close must be finite and positive; zero or negative prices are rejected by every command |
| Order | Strictly increasing timestamps for `candles`; out-of-order rows are skipped by `candles-tolerant` |
| Ladder source | Every ladder consumer requires one-minute spacing when two or more rows are present; a file whose rows are never closer than 5m, 15m, or 1h is rejected |

A header-only file is a valid empty series and prints nothing. In tolerant mode it reports zero counts. Ordering is checked before candle validity, so an invalid row with a duplicate timestamp is counted as `duplicate`, not `invalid`.

The ladder guard measures every parsed source row before a date bound is applied, so a native higher-timeframe file is rejected even when the requested period would retain only one row. A sparse M1 file with gaps is accepted as soon as one adjacent pair is one minute apart. A single row has no spacing to measure and is accepted; a header-only file remains a valid empty series. The error names the observed spacing and asks for an M1 source.

### Timeframe construction contract

`timeframes` reads an M1 CSV strictly and builds four first-class series in the data engine. It does not fetch a source, and it does not write a derived CSV. M5, M15, and H1 are built from M1 only; H4 and D1 are not supported.

| Item | Rule |
|---|---|
| Source | Validated M1 rows from the strict CSV reader; when two or more rows are present, the smallest spacing between consecutive rows must be 60s |
| M1 validation | Timestamps strictly increasing and aligned to whole UTC minutes; a native M5/M15/H1 cadence is rejected; any invalid row aborts the command |
| Derived series | M5, M15, H1 |
| Window | 1000 candles per timeframe |
| Complete bucket | Exactly 5, 15, or 60 M1 candles, checked on bucket transition or at end of input |
| Partial bucket | Recorded and dropped; never printed as a candle |
| OHLC | Open is the first M1 open, high the maximum, low the minimum, close the last M1 close |
| Volume | Summed only when every source volume is known; otherwise `null`; a real zero is `0`; a non-finite sum is rejected at the bucket that overflowed |
| Gap | `after`, `before`, and `missing` between consecutive candles in a series; no market-session calendar is applied |
| stdout | NDJSON grouped M1, M5, M15, H1; oldest candle first within each series |
| stderr | One aggregation summary per timeframe, then its partial-bucket and gap records |

A bucket is published only after every M1 candle it contains is known. The command builds the complete set before writing stdout, so a source error produces no partial NDJSON. A complete trailing bucket is published at end of input; an incomplete trailing bucket is dropped and counted.

The report has this shape:

```text
aggregation timeframe=5m retained=3 partial=1 gaps=1
partial timeframe=5m bucket=2025-01-01T00:00:00Z present=4 expected=5
gap timeframe=5m after=2025-01-01T00:10:00Z before=2025-01-01T00:20:00Z missing=1
```

`retained` is the number of candles in the bounded window, `partial` is the number of dropped incomplete buckets, and `gaps` is the number of gap records. The four summaries always appear in M1, M5, M15, H1 order. The partial and gap records cover the entire input, not only the retained window.

### Market-state contract

`market-state`, `axis`, `events`, and `legs` read an M1 CSV, build the M1/M5/M15/H1 ladder through the data engine, and call the model. Every ladder consumer requires one-minute source spacing; a native higher-timeframe file is rejected before the model runs. They analyze the full series up to the 100,000 M1 cap by default; `--window latest` streams the file without the full-series cap and retains the last 1,000 candles per timeframe. The model publishes numbers; the shell maps the stable numeric codes to labels and never recomputes an axis, aggregate, or event. `events` covers every event in the analyzed window, not only the retained candle tail.

`market-state` prints one JSON object with four keys:

| Key | Content |
|---|---|
| `ladder` | The timeframe codes, fastest first |
| `timeframes` | One object per timeframe with its axis values, state codes, diagnostics, context, consensus counts, and event summary |
| `aggregates` | The Direction, Momentum, Structure, Progression, and Event aggregates |
| `consensus` | One cross-axis consensus per timeframe |

`axis` prints one NDJSON line per timeframe for the selected axis. It carries the numeric fields, the compact code, the readiness and reason codes, the label fields, and the state counters.

`events` prints one NDJSON line per event, grouped by timeframe and ordered by availability. Each line carries the type, subtype, source, occurrence and availability bars and UTC timestamps, direction, reference, measured values, and the labels for the type, subtype, and source.

`legs` prints two NDJSON lines per timeframe, one for each pivot scale. Each line carries the published leg or `null`, its origin and availability bars, target basis and version, progress, displacement, lifecycle, stage, and target-reached code, with the lifecycle, stage, direction, scale, and target-basis labels.

`model-series <csv-path> [--timeframe <tf>] [--bars <n>] [--window full|latest]` prints the last complete model bars per timeframe as NDJSON, oldest first. Each line carries the timeframe, dataset bar and stable series index, UTC timestamp, window start and length, candle, six readings, the four axes with their diagnostics, context, and consensus. `--bars` is `1..=1000` and defaults to 1000; `--timeframe` defaults to the whole ladder. Events remain a separate command because a bar can have several events. The default input window is the full series up to 100,000 M1 candles; `--window latest` streams the file without the full-series cap and uses the last 1,000 candles per timeframe.

### Market-dashboard contract

`market-dashboard <csv-path> [--window full|latest] [--json]` prints one instrument's four-axis dashboard over the fixed M1/M5/M15/H1 ladder. It calls the model's `DashboardState`; the shell only formats the published numbers. The default input is the full series up to 100,000 M1 candles; `--window latest` streams the file without the full-series cap and retains the last 1,000 candles per timeframe.

The text output has:

| Part | Content |
|---|---|
| Header | Window, ladder, model and view-contract versions |
| `as-of` | The open time of each timeframe's latest closed bar, or `— no closed bar` |
| `cross-TF agreement` | The strict-majority count over Direction, Momentum, and Structure, with coverage |
| `churn` | The maximum 50-bar state-change count, its sources, threshold, history completeness, and class |
| Track header and legend | The four tracks and the glyph key |
| One row per timeframe | Four cells; a missing timeframe prints `— no closed bar` |
| `diagnostics` | Volatility and participation ratios, the numeric context code, and the per-timeframe consensus |
| `evidence` | The axis-specific details, with current Structure reference prices on a `levels` line |
| `readings` | The latest dated Event or Transition per axis, or `—` |

The four tracks are identical and never reversed. The position is the dot's slot in an eleven-slot track; the three-step radius is `·` small, `•` medium, or `●` large; readiness is `■` solid or `□` dashed. An undefined position has no dot and prints `— r<code>` with the model's unavailable reason. A missing size prints `—`. A valid zero prints `0%`; it is never shown as absence.

`--json` prints one object with `model_version`, `view_contract`, and the numeric `dashboard_state`. Enum codes are stable strings; numbers keep full precision; a missing value is `null` or `{"kind":"undefined","reason_code":n}`. The JSON carries the full per-timeframe state, readings, cross-timeframe aggregates, agreement, and churn. It contains no labels beyond the stable enum codes and no strategy vocabulary.

The text and JSON outputs are deterministic: the same input, window, and configuration print the same bytes, interactively or non-interactively. The view is descriptive; it is not a strategy, forecast, entry, exit, stop, target, confidence, probability, or recommendation.

### Market chart

`market-chart <csv-path> --png <path> [--timeframe <tf>] [--candles <n>] [--window full|latest]` renders the last bars of one timeframe as a deterministic PNG. The top panel shows the OHLCV candles and a volume strip; the four panels below show the continuous Direction, Momentum, Structure, and Progression resolutions on the shared `[-1, +1]` scale. A missing axis value is a gap, never a zero. `--timeframe` accepts `1m`, `5m`, `15m`, or `1h` and defaults to `1m`; `--candles` defaults to 1000 and is capped at 1000. The default window is the full series up to 100,000 M1 candles; `--window latest` uses the last 1,000 candles per timeframe and lets a larger fetch be charted. The command prints `png=<path> timeframe=<tf> candles=<n> bytes=<n>`.

The output is deterministic: the same input and configuration produce the same bytes, and the interactive and non-interactive invocations print the same text. The command set is the product's public interface; changing a command name or what it means is a command-contract decision.

### Market-report contract

`market-report <csv-path> --data-version <id> [--from <date>] [--to <date>] --out <dir> [--json]` runs the model over the extended M1 range and writes the model's full-period descriptive view as exactly four artifacts, in this order: `summary.txt`, `report.json`, `price-axes.png`, `events.png`. It calls the model's `MarketReport`; the shell only loads, renders, and writes.

- `--data-version <id>` is the caller-supplied identity of the input series; the report records it and never infers it from the path.
- `--from` and `--to` are inclusive UTC `YYYY-MM-DD` dates. The shell filters the M1 source before building the ladder, and no pre-range warm-up is used. Omitting both uses the file's full coverage.
- The source must be one-minute spaced. A native higher-timeframe file is rejected before the report is built, including when the date bound would leave only one row; no output directory is created.
- `--out` must be absent or an empty directory. Every byte is built before the directory is populated, each artifact is written atomically, and a failure removes the partial set. A non-empty directory or an output path that is a file is refused.
- `--json` prints the exact `report.json` bytes to stdout. Without it, the command prints the four artifact paths in contract order.
- The chart uses H1, or the slowest available timeframe, reduced to at most 240 buckets. Each bucket keeps the last published axis value; an ordinal code is never averaged.

The report publishes coverage, axis statistics and ordinal histograms, full-period event counts, the latest published state, cross-timeframe aggregates, and chart buckets. It contains no order, fill, position, trade, equity, drawdown, or P&L field, and it applies no strategy. A range with no M1 candles fails and writes no artifacts. The JSON envelope carries `model_version`, `view_contract`, `data_version`, `requested_period`, and `market_report`; object keys have a fixed order, numbers keep full precision, and a missing value is `null`.

The output is deterministic: the same input, range, data version, and configuration produce the same four files byte for byte, interactively or non-interactively. The view contract is `docs/model/2026-10-27/market-report.md`. The extended range accepts up to 600,000 M1 candles. `market-state`, `market-dashboard`, `market-chart`, `axis`, `events`, `legs`, `model-series`, the store, and research accept up to 100,000 M1 candles on the full-window path; `--window latest` streams the file without that cap and keeps the last 1,000 candles per timeframe. `market-report` and `strategy-run` accept up to 600,000; `report` and `backtest` take their cap from `data.json.max_candles`, at most 600,000.

### Report bundles and backtest

`report <engine> --config <run.json> [--out <dir>] [--json]` runs only the engines one bundle needs and writes it under `<out>/<engine>/`. `backtest --config <run.json> [--out <dir>] [--json]` loads the candles once, runs the pipeline once, and writes all five bundles from that result. Both require a one-minute-spaced source; a native higher-timeframe file is rejected before any bundle directory is created.

| Bundle | Engines | Artifacts |
|---|---|---|
| `model` | data, model | `summary.txt`, `report.json`, `price-axes.png`, `events.png`, `provenance.json` |
| `strategy` | data, model, strategy | `summary.txt`, `report.json`, `decisions.csv`, `price-decisions.png`, `provenance.json` |
| `execution` | data, model, strategy, execution, portfolio | `summary.txt`, `report.json`, `orders.csv`, `fills.csv`, `rejections.csv`, `price-fills.png`, `provenance.json` |
| `portfolio` | data, model, strategy, execution, portfolio | `summary.txt`, `report.json`, `trades.csv`, `equity.csv`, `equity.png`, `drawdown.png`, `provenance.json` |
| `analytical` | data, model | `summary.txt`, `report.json`, `axes.csv`, `relations.csv`, `provenance.json` |

The `strategy` bundle written by `backtest` uses the executed walk and marks
`execution_state` as `executed`; it also publishes `executed_entries` and
`executed_exits`. The standalone `report strategy` uses the virtual diagnostic
walk and marks `execution_state` as `virtual`; its executed counts are null.

A run is specified by eight files. The command line carries only the config path, an optional output override, and an optional JSON flag.

| File | Contents |
|---|---|
| `run.json` | Run name, policy reference, output root, and the six engine file names |
| `policy.json` | Account, objective, risk envelope, tickets, and timeframes |
| `data.json` | Instrument ticket, source CSV, data version, UTC period, the fixed `1m`/`5m`/`15m`/`1h` ladder, and `max_candles` (`1..=600000`) |
| `model.json` | Structure scale (`major` or `minor`) |
| `strategy.json` | Thesis, `axis-agreement` timeframe, long/short thresholds, stop ATR multiple, and reward-to-risk |
| `execution.json` | Fill timeframe, order type, tie break, fee rate, slippage, zero-volume and end-of-data policy |
| `portfolio.json` | Cash, currency, sizing, leverage, position limit, commission, funding, drawdown limit and action, mark policy |
| `analytics.json` | Analytical report scale, timeframe subset, and relations switch |

`data.source.path` resolves against `data.json`; `output_root` and the other file names resolve against `run.json`. The loader validates every file before a candle is read or a byte is written: an error names the file and the JSON path, never an absolute path. `analytics.json.timeframes` must be a subset of the ladder, and its structure scale must equal the model's. The policy must agree with the portfolio cash, drawdown, leverage, position size, ticket, and timeframe. The contracts are [`docs/config/2026-10-30/policy-and-strategy.md`](docs/config/2026-10-30/policy-and-strategy.md) and [`docs/reports/2026-10-28/per-engine-reports.md`](docs/reports/2026-10-28/per-engine-reports.md).

`--out` defaults to the run's `output_root`. A bundle is available if and only if its `report.json` exists; the writer refuses a non-empty target and removes a partial bundle on failure. `report --json` prints the `report.json` bytes; `backtest --json` prints a summary envelope with the data version, the requested and actual periods, the five bundle paths, and the portfolio's final equity, return, net P&L, drawdown, trade count, and halt flag. `provenance.json` records the data version, the config hash, the periods, the config file names, the resolved engine values, and the artifact list.

The model is computed once per run. The analytical bundle summarizes that model one bar at a time, so no layer set is materialized and the model is not recomputed. The model bundle's four legacy artifacts are byte-identical to `market-report` for the same data, period, and default model settings.

A fresh checkout has no source data, because `data/` is not committed. The example run needs its M1 file first:

```bash
cargo run -p quantrule-shell -- fetch sol/usd 2025-01-01 2025-12-31 --csv data/SOLUSD_M1_2025-01-01_2025-12-31.csv
cargo run -p quantrule-shell -- backtest --config config/examples/solusd-2025/run.json --out data/lab-result-2026-10-29T12-00-00Z
```

Without `--json`, the command prints the five bundle paths in contract order:

```text
data/lab-result-2026-10-29T12-00-00Z/model
data/lab-result-2026-10-29T12-00-00Z/strategy
data/lab-result-2026-10-29T12-00-00Z/execution
data/lab-result-2026-10-29T12-00-00Z/portfolio
data/lab-result-2026-10-29T12-00-00Z/analytical
```

`--json` prints one envelope instead:

```json
{"actual_period":{"first_open_time":1735740900,"last_open_time":1767191400},"bundles":{"analytical":"data/lab-result-2026-10-29T12-00-00Z/analytical","execution":"data/lab-result-2026-10-29T12-00-00Z/execution","model":"data/lab-result-2026-10-29T12-00-00Z/model","portfolio":"data/lab-result-2026-10-29T12-00-00Z/portfolio","strategy":"data/lab-result-2026-10-29T12-00-00Z/strategy"},"data_version":"dukascopy-bid-solusd-m1-2025-01-01_2025-12-31","model_version":"0.3.0","portfolio":{"final_equity":8016.143909900684,"halted":true,"max_drawdown":0.20315025809884202,"net_pnl":-1983.85609009931,"total_return":-0.19838560900993163,"trades":48},"report":"backtest","report_version":"2026-10-28","requested_period":{"from":"2025-01-01","to":"2025-12-31"}}
```

`first_open_time` and `last_open_time` are the strategy timeframe's first and last open times. The run is deterministic: the same inputs and configuration produce byte-identical bundles and the same envelope, interactively or non-interactively. A single report writes only its own directory:

```bash
cargo run -p quantrule-shell -- report portfolio --config config/examples/solusd-2025/run.json --out data/portfolio-report
```

### Lab result summary

`lab summary <lab-dir> --role <role> --data-version <id> --config <run.json> --created-utc <YYYY-MM-DDTHH:MM:SSZ> [--operator human|agent] [--notes <path>] [--force]` writes one deterministic `summary.txt` at the root of a lab result folder. A lab result folder is `data/lab-result-YYYY-MM-DDTHH-MM-SSZ` in UTC and holds the five report bundles from one `backtest` plus that root summary.

| Option | Meaning |
|---|---|
| `--role <role>` | Required: `researcher`, `developer`, or `validator` |
| `--data-version <id>` | Required; must match every bundle's `report.json` and `provenance.json` |
| `--config <path>` | Required; recorded as the run configuration |
| `--created-utc <ts>` | Required explicit UTC time, `YYYY-MM-DDTHH:MM:SSZ`; the command never reads the clock |
| `--operator <kind>` | `human` (default) or `agent` |
| `--notes <path>` | Include the file verbatim under Role notes |
| `--force` | Overwrite an existing root `summary.txt` |

The summary has seven sections: Identity, Artifacts, Observed result, Checks, Role notes, Limitations, and Next. It copies published values from the five `report.json` files; it does not recompute the model. It refuses a missing bundle, a data-version mismatch, or an existing summary without `--force`, and writes atomically. The same lab and the same arguments produce the same bytes. It prints `wrote <path>`.

### Strategy-run contract

`strategy-run <csv-path> --strategy <name> --data-version <id> [--config <run.json>] [--from <date>] [--to <date>] [--json]` runs the first model-driven strategy, `axis-agreement`, over the M15 series and prints its entries and exits. It requires a one-minute-spaced source and rejects a native higher-timeframe file before printing. With `--config`, it uses the run's strategy and model configuration. The JSON envelope marks `execution_state` as `virtual`: the diagnostic walk assumes every decision fills immediately. The standalone `report strategy` is also virtual; only the `strategy` bundle written by `backtest` reads the executed walk. It stops at the strategy decision: it creates no order, fill, position, equity, trade, or P&L record. Those belong to the backtest path.

- `axis-agreement` enters long when Direction, Momentum force, and Structure score are all above the configured long threshold (default `+0.1`), and short when all are below the configured short threshold (default `-0.1`). Equality at a threshold is neutral. It exits when the agreement is lost; it never reverses on the same bar and never exits at the end of the data.
- `--data-version <id>` is the caller-supplied identity of the input series; the envelope records it and never infers it from the path.
- `--from` and `--to` are inclusive UTC `YYYY-MM-DD` dates applied to the M1 source before the ladder is built. No pre-range warm-up is used.
- Without `--json`, the command prints one NDJSON line per entry or exit, oldest first. Each record carries `type`, `strategy`, `timeframe`, `timeframe_secs`, `bar`, `datetime`, `side`, `side_label`, `direction`, `momentum_force`, and `structure_score`; an exit also carries `reason` (`agreement_lost`). Model Signal events remain a separate event type and are never relabelled as strategy entries or exits.
- With `--json`, the command prints one envelope with `model_version`, `strategy`, `strategy_config`, `data_version`, `requested_period`, `timeframe_secs`, and `records`; `records` carries the same records as the bare NDJSON output.

`--config` uses the strategy blueprint from the run configuration; without it the built-in defaults are used. The strategy reads only the configured timeframe, uses the extended range up to 600,000 M1 candles, and fails with a message rather than printing an empty result when the requested period has no M1 candles or no M15 bars. The output is deterministic: the same input, range, data version, and configuration produce the same bytes, interactively or non-interactively.

### Fetch contract

`fetch` reads native OHLCV candles from Dukascopy's `freeserv.dukascopy.com` chart endpoint on the BID side. Each timeframe is requested with the provider's own interval and is never synthesized from 1-minute candles. These native intervals are for source inspection and comparison; the canonical M5, M15, and H1 series used for analysis come from `timeframes`, which derives them from M1. The request uses a fixed JSONP callback and fixed browser headers, and the page cursor is always the last timestamp delivered by the page, so the same data and configuration produce the same request sequence and the same output bytes.

| Timeframe | Provider interval | File code |
|---|---|---|
| `1m` | `1MIN` | `M1` |
| `5m` | `5MIN` | `M5` |
| `15m` | `15MIN` | `M15` |
| `1h` | `1HOUR` | `H1` |

| Item | Value |
|---|---|
| Price side | BID |
| Date range | Inclusive UTC dates, `YYYY-MM-DD`; the end covers its whole day |
| Default CSV | `data/<SYMBOL>_<M1\|M5\|M15\|H1>_<start>_<end>.csv`, for example `data/XAUUSD_M1_2025-01-01_2025-12-31.csv` |
| PNG | Written only with `--png <path>`; the chart shows the last `--candles` candles |
| Chart | Price and volume panels, no moving average, UTC labels |
| Duplicate cursor row | Dropped at the page boundary |
| Invalid candle | Skipped and counted; the remaining candles are kept |
| No accepted candles | The command fails and commits no CSV |

A successful fetch prints a fixed summary to stdout:

```text
symbol=XAU/USD timeframe=1m start=2025-01-01 end=2025-12-31
rows=354441 accepted=354441 rejected=0 pages=12
first=2025-01-01T23:00:00Z last=2025-12-31T21:58:00Z
csv=data/XAUUSD_M1_2025-01-01_2025-12-31.csv
png=none
```

The CSV is written through a temporary sibling and renamed only after the fetch completes, so an interrupted or failed fetch does not leave a partial file under the requested name. A fetch whose accepted row count exceeds 100,000 writes a warning to stderr: `store` and `research` reject that file, `market-chart` needs `--window latest` to chart it, and `market-report` accepts it up to 600,000 rows. The source requires network access; the test suite does not, because pagination, retries, JSONP parsing, CSV shape, and chart determinism are exercised with fixtures.

### NDJSON contract

One candle per line, oldest first:

```json
{"series_index":0,"window_index":0,"datetime":"2025-01-01T00:00:00Z","open":1,"high":2,"low":0.5,"close":1.5,"volume":10}
```

`timeframes` adds `timeframe` as the first field and groups the output by series:

```json
{"timeframe":"5m","series_index":0,"window_index":0,"datetime":"2025-01-01T00:00:00Z","open":1,"high":2,"low":0.5,"close":1.5,"volume":50}
```

| Field | Meaning |
|---|---|
| `timeframe` | Series code (`1m`, `5m`, `15m`, or `1h`); present only in `timeframes` output |
| `series_index` | Stable index assigned by the data engine; survives eviction |
| `window_index` | Position in the printed window; the oldest candle is 0 |
| `datetime` | RFC 3339 UTC timestamp |
| `open`, `high`, `low`, `close` | Candle prices |
| `volume` | Source volume, or `null` when absent; a real zero is `0` |

### Quality report contract

`candles-tolerant` writes a fixed summary line followed by one line per skipped row, in source order:

```text
quality rows_read=8 accepted=2 rejected=6 duplicate=1 out_of_order=1 invalid=4 evicted=0 retained=2
skipped line=3 reason=invalid: high is below low
skipped line=4 reason=invalid: open is not finite
skipped line=5 reason=invalid: invalid timestamp: timestamp has an invalid date
skipped line=6 reason=duplicate: duplicate timestamp at 2025-01-01T00:00:00Z
skipped line=7 reason=out_of_order: timestamp 2024-12-31T23:59:00Z is earlier than 2025-01-01T00:00:00Z
skipped line=8 reason=field_count: expected 6 fields, found 5
```

| Count | Meaning |
|---|---|
| `rows_read` | Data rows after the header, including rejected rows |
| `accepted` | Rows accepted by the window, including candles later evicted |
| `rejected` | Rows skipped; equals `duplicate + out_of_order + invalid` |
| `duplicate` | Rejected rows whose timestamp equals the last accepted timestamp |
| `out_of_order` | Rejected rows earlier than the last accepted timestamp |
| `invalid` | Rejected rows with a wrong field count or an invalid candle |
| `evicted` | Accepted candles no longer retained by the 1000-candle window |
| `retained` | Candles printed in the NDJSON output |

The counts satisfy `rows_read = accepted + rejected` and `accepted = evicted + retained`. A skipped row's `reason` is one of `invalid: ...`, `duplicate: ...`, `out_of_order: ...`, or `field_count: ...`. The report contains no clock value and no unordered iteration, so the same input produces the same bytes.

## Scope

MVP v1 covers local M1 CSV ingestion, both strict and tolerant, the candle window, M5/M15/H1 construction from M1, the four-axis market state with its events and legs, a native Dukascopy fetch for `1m`, `5m`, `15m`, and `1h`, and a full config-driven backtest: the axis-agreement strategy, next-open execution with an ATR bracket, portfolio accounting with commission, funding, and a drawdown halt, and the five report bundles. The fetched higher intervals are source references; canonical multi-timeframe series are derived from M1 by the data engine. A run is deterministic: the same data and configuration produce the same bundles and the same printed output.

## Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

## Developing QuantRule

The agent roles, review gates, and verification rules are in [`AGENTS.md`](AGENTS.md). The role overview and handoff chain are in [`docs/roles/2026-10-29/quant-roles.md`](docs/roles/2026-10-29/quant-roles.md). The operating manuals are [Quant Researcher](docs/roles/2026-10-29/quant-researcher-manual.md), [Quant Developer](docs/roles/2026-10-29/quant-developer-manual.md), and [Quant Validator](docs/roles/2026-10-29/quant-validator-manual.md). The rest of the documentation is indexed in [`docs/README.md`](docs/README.md).
