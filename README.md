# QuantRule

A deterministic, rule-based trading signal engine.

QuantRule is a shell application. You start it and write commands — print a candle series, compute an indicator, run a backtest. The shell is the whole interface: there is no graphical interface, no service, and no library to embed.

## Build and run

The workspace uses the Rust version in `Cargo.toml`.

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

## Commands

| Command | Arguments | Output |
|---|---|---|
| `candles` | `<csv-path>` | Up to the last 1000 candles as NDJSON, oldest first |
| `candles-tolerant` | `<csv-path>` | Up to the last 1000 valid candles as NDJSON, plus a quality report on stderr |
| `timeframes` | `<csv-path>` | M1, M5, M15, and H1 as NDJSON, plus an aggregation report on stderr |
| `market-state` | `<csv-path>` [`--window full\|latest`] | The four-timeframe market state as one JSON object |
| `market-chart` | `<csv-path> --png <path>` [options] | A deterministic PNG of the last bars; `--window latest` supports inputs above the 100k cap |
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
| `db series` | `<dataset>` [options] | Stored candles as NDJSON |
| `db bars` | `<dataset>` [options] | Full stored bar rows as CSV |
| `db metrics` | `<dataset>` [options] | Metrics and resolutions as CSV |
| `db layer3` | `<dataset>` [options] | Materialized Layer 3 rows as CSV |
| `fetch` | `<symbol> <start> <end>` [options] | A CSV of native BID candles and an optional PNG chart |
| `help` | none | The command reference |
| `quit` | none | Exits with status 0 |

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

`research export` reads an M1 CSV, builds M1/M5/M15/H1, and writes `<root>/raw/<dataset>/m1.csv` and `<root>/processed/<dataset>/` with `layer1.csv`, `layer2.csv`, `layer3.csv`, `manifest.json`, `data_dictionary.csv`, `data_dictionary.md`, `codes.csv`, and `README.md`. The manifest records the data version, the model and contract versions, the configuration, the row counts, and the data file list. The export is deterministic: the same input and configuration produce byte-identical files. The input is capped at 100,000 M1 candles; an over-cap input fails rather than being truncated.

`research summary` prints one line per timeframe and axis with the present count, null count, minimum, maximum, mean, and the ten ordinal bucket counts. It then prints one line per timeframe and axis pair with the Pearson correlation of the two continuous values. Ordinal codes are counted, never averaged, and no line combines the axes. The output is deterministic: the same input prints the same text.

`store init` options:

| Option | Meaning |
|---|---|
| `--db <path>` | Database path (default `data/quantrule.db`) |

`store init` opens the SQLite store, applies the schema migrations in order, each in one transaction (version 1, then the version 2 event-index migration), and prints the path and the schema version. Reopening an existing store is idempotent, and a version 1 database is migrated in place. A store whose schema is newer than the binary is rejected, and a file that is not a database fails with a clear error. The store is a shell-owned module; the data, model, and analytics crates remain pure.

`store ingest` options:

| Option | Meaning |
|---|---|
| `--dataset <name>` | Required unique dataset name |
| `--data-version <id>` | Required identity of the input series: source, timeframe, and coverage |
| `--db <path>` | Database path (default `data/quantrule.db`) |

`store ingest` reads one M1 CSV, builds M1/M5/M15/H1 in memory, and writes one `dataset` row, one wide `bar` row per Layer 2 row, one `consensus` row per closed bar per timeframe, one `layer3` row per M1 bar, one `pivot` row per pivot occurrence, one `level` row per confirmed level, one `event` row per event record, and one `leg` row per retained progression leg in a single `BEGIN IMMEDIATE` transaction. It writes the candle, the six readings, the four-axis metrics, and the 12 axis-resolution columns for every ladder bar; a missing value is `NULL` with its unavailable-reason code, and a valid zero stays `0`. The resolution columns carry each axis's continuous value, ordinal bucket, and unavailable reason. The consensus row carries the three axis counts and the weakest directional axis code; a missing axis is excluded, and Progression does not vote. The Layer 3 row carries the as-of M5/M15/H1 block columns and the 82 cross-timeframe aggregate columns over the configured ladder; a missing measurement is `NULL`, a count of zero stays `0`, and a list is `;`-separated text with no spaces, with an empty list stored as an empty string rather than `NULL`. A block that has no closed higher bar yet is `NULL` in all its columns; a new dataset is marked `layer3_materialized = 1`. A #68–#72 dataset is not backfilled and stays `0`. A pivot candidate is available on its occurrence bar; a confirmed pivot carries the confirmation bar as its availability bar and is stored once, with `confirmed_code = 1`, rather than duplicated. Every level carries its `1000 + n` reference code and both its occurrence and availability bars. Every event is stored once, on or after its availability bar, and never before it. Coincident Signal threshold crossings are distinct rows because the threshold value is part of their identity; every other event keeps the original occurrence key. Every retained leg carries its origin, target, progress, displacement, and lifecycle codes; a value that is not yet knowable is `NULL`. The M1 `bar` count equals the accepted source rows; the full count covers the whole ladder. The input is capped at 100,000 M1 candles and a count mismatch or over-cap input is rejected before anything is written. Re-running the same name with the same identity is a no-op that prints `unchanged`; a different identity for an existing name is a conflict and changes nothing. Output is one deterministic line: `ingested <name> bars=<n> timeframes=<k>` or `unchanged <name> bars=<n> timeframes=<k>`.

`store verify` reads the same source again, rebuilds the layers, opens the store read-only, and compares the dataset identity, the foreign-key graph, and every row of `bar`, `consensus`, `layer3`, `pivot`, `level`, `event`, and `leg`. A success prints `verified <name> bars=<n> tables=7`; the first value, count, or structural mismatch fails with a deterministic message. A dataset with `layer3_materialized = 0` is incomplete and must be re-ingested into a fresh database. Verification never writes, migrates, or repairs the store.

### Store query contract

The query commands open an existing database read-only. A missing file is an error and is never created; a newer schema is rejected; no migration runs. CSV uses an empty field for `NULL`, the shortest round-trip decimal for a float, and quotes a field that contains a comma, quote, or newline. `--limit` is `1..=1000` and defaults to `100`. `--timeframe` accepts `1m`, `5m`, `15m`, or `1h`; `--bar` filters `bar_index` where the table has one.

| Command | Output |
|---|---|
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
| Order | Strictly increasing timestamps for `candles`; out-of-order rows are skipped by `candles-tolerant` |

A header-only file is a valid empty series and prints nothing. In tolerant mode it reports zero counts.

### Timeframe construction contract

`timeframes` reads an M1 CSV strictly and builds four first-class series in the data engine. It does not fetch a source, and it does not write a derived CSV. M5, M15, and H1 are built from M1 only; H4 and D1 are not supported.

| Item | Rule |
|---|---|
| Source | Validated M1 rows from the strict CSV reader |
| M1 validation | Timestamps strictly increasing and aligned to whole UTC minutes; any invalid row aborts the command |
| Derived series | M5, M15, H1 |
| Window | 1000 candles per timeframe |
| Complete bucket | Exactly 5, 15, or 60 M1 candles, checked on bucket transition or at end of input |
| Partial bucket | Recorded and dropped; never printed as a candle |
| OHLC | Open is the first M1 open, high the maximum, low the minimum, close the last M1 close |
| Volume | Summed only when every source volume is known; otherwise `null`; a real zero is `0` |
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

`market-state`, `axis`, `events`, and `legs` read an M1 CSV, build the M1/M5/M15/H1 ladder through the data engine, and call the model. They analyze the full series up to the 100,000 M1 cap by default; `--window latest` retains the last 1,000 candles per timeframe. The model publishes numbers; the shell maps the stable numeric codes to labels and never recomputes an axis, aggregate, or event. `events` covers every event in the analyzed window, not only the retained candle tail.

`market-state` prints one JSON object with four keys:

| Key | Content |
|---|---|
| `ladder` | The timeframe codes, fastest first |
| `timeframes` | One object per timeframe with its axis values, state codes, diagnostics, context, consensus counts, and event summary |
| `aggregates` | The Direction, Momentum, Structure, Progression, and Event aggregates |
| `consensus` | One cross-axis consensus per timeframe |

`axis` prints one NDJSON line per timeframe for the selected axis. It carries the numeric fields, the compact code, the readiness and reason codes, the label fields, and the state counters.

`events` prints one NDJSON line per event, grouped by timeframe and ordered by availability. Each line carries the type, subtype, source, occurrence and availability bars, direction, reference, measured values, and the labels for the type, subtype, and source.

`legs` prints two NDJSON lines per timeframe, one for each pivot scale. Each line carries the published leg or `null`, its origin and availability bars, target basis and version, progress, displacement, lifecycle, stage, and target-reached code, with the lifecycle, stage, direction, scale, and target-basis labels.

`model-series <csv-path> [--timeframe <tf>] [--bars <n>] [--window full|latest]` prints the last complete model bars per timeframe as NDJSON, oldest first. Each line carries the timeframe, dataset bar and stable series index, UTC timestamp, candle, six readings, the four axes with their diagnostics, context, and consensus. `--bars` is `1..=1000` and defaults to 1000; `--timeframe` defaults to the whole ladder. Events remain a separate command because a bar can have several events. The default input window is the full series up to 100,000 M1 candles; `--window latest` uses the last 1,000 candles per timeframe.

### Market chart

`market-chart <csv-path> --png <path> [--timeframe <tf>] [--candles <n>] [--window full|latest]` renders the last bars of one timeframe as a deterministic PNG. The top panel shows the OHLCV candles and a volume strip; the four panels below show the continuous Direction, Momentum, Structure, and Progression resolutions on the shared `[-1, +1]` scale. A missing axis value is a gap, never a zero. `--timeframe` accepts `1m`, `5m`, `15m`, or `1h` and defaults to `1m`; `--candles` defaults to 1000 and is capped at 1000. The default window is the full series up to 100,000 M1 candles; `--window latest` uses the last 1,000 candles per timeframe and lets a larger fetch be charted. The command prints `png=<path> timeframe=<tf> candles=<n> bytes=<n>`.

The output is deterministic: the same input and configuration produce the same bytes, and the interactive and non-interactive invocations print the same text. The command names `market-state`, `market-chart`, `axis`, `events`, and `legs` are the implemented contract; changing them is a command-contract decision.

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

The CSV is written through a temporary sibling and renamed only after the fetch completes, so an interrupted or failed fetch does not leave a partial file under the requested name. A fetch whose accepted row count exceeds 100,000 writes a warning to stderr: `store` and `research` reject that file, and `market-chart` needs `--window latest` to chart it. The source requires network access; the test suite does not, because pagination, retries, JSONP parsing, CSV shape, and chart determinism are exercised with fixtures.

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

This slice covers local M1 CSV ingestion, both strict and tolerant, the candle window, M5/M15/H1 construction from M1, the four-axis market state and its event and leg records, and a native Dukascopy fetch for `1m`, `5m`, `15m`, and `1h` with an optional chart. The fetched higher intervals are source references; canonical multi-timeframe series are derived from M1 by the data engine. Backtests, strategies, and portfolio execution are not implemented yet.

## Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```
