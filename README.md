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
cargo run -p quantrule-shell -- axis path/to/m1.csv direction
cargo run -p quantrule-shell -- events path/to/m1.csv
cargo run -p quantrule-shell -- legs path/to/m1.csv
cargo run -p quantrule-shell -- research export path/to/m1.csv demo --data-version binance:BTCUSDT:60:1704067200-1710115200
cargo run -p quantrule-shell -- research summary path/to/m1.csv
cargo run -p quantrule-shell -- store init --db data/quantrule.db
cargo run -p quantrule-shell -- store ingest path/to/m1.csv --dataset demo --data-version binance:BTCUSDT:60:1704067200-1710115200
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
| `market-state` | `<csv-path>` | The four-timeframe market state as one JSON object |
| `axis` | `<csv-path> <axis>` | One NDJSON line per timeframe for `direction`, `momentum`, `structure`, or `progression` |
| `events` | `<csv-path>` | Every event record as NDJSON, in availability order |
| `legs` | `<csv-path>` | The minor and major leg per timeframe as NDJSON |
| `research export` | `<csv-path> <dataset> --data-version <id>` [options] | The raw M1 file and the three research layers, with the manifest, dictionary, and code table |
| `research summary` | `<csv-path>` | Per-axis counts, ranges, ordinal histograms, and pairwise continuous correlations |
| `store init` | [options] | Creates or opens the SQLite store and prints its schema version |
| `store ingest` | `<csv-path> --dataset <name> --data-version <id>` [options] | Writes one dataset, every ladder bar, every pivot, and every confirmed level, and prints the stored counts |
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

`store init` opens the SQLite store, applies the version 1 schema in one transaction, and prints the path and the schema version. Reopening an existing store is idempotent. A store whose schema is newer than the binary is rejected, and a file that is not a database fails with a clear error. The store is a shell-owned module; the data, model, and analytics crates remain pure.

`store ingest` options:

| Option | Meaning |
|---|---|
| `--dataset <name>` | Required unique dataset name |
| `--data-version <id>` | Required identity of the input series: source, timeframe, and coverage |
| `--db <path>` | Database path (default `data/quantrule.db`) |

`store ingest` reads one M1 CSV, builds M1/M5/M15/H1 in memory, and writes one `dataset` row, one wide `bar` row per Layer 2 row, one `pivot` row per pivot occurrence, and one `level` row per confirmed level in a single `BEGIN IMMEDIATE` transaction. It writes the candle, the six readings, the four-axis metrics, and the 12 axis-resolution columns for every ladder bar; a missing value is `NULL` with its unavailable-reason code, and a valid zero stays `0`. The resolution columns carry each axis's continuous value, ordinal bucket, and unavailable reason. A pivot candidate is available on its occurrence bar; a confirmed pivot carries the confirmation bar as its availability bar and is stored once, with `confirmed_code = 1`, rather than duplicated. Every level carries its `1000 + n` reference code and both its occurrence and availability bars. `layer3_materialized` stays `0` until Layer 3 is materialized. The M1 `bar` count equals the accepted source rows; the full count covers the whole ladder. The input is capped at 100,000 M1 candles and a count mismatch or over-cap input is rejected before anything is written. Re-running the same name with the same identity is a no-op that prints `unchanged`; a different identity for an existing name is a conflict and changes nothing. Output is one deterministic line: `ingested <name> bars=<n> timeframes=<k>` or `unchanged <name> bars=<n> timeframes=<k>`.

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

`market-state`, `axis`, `events`, and `legs` read an M1 CSV, build the M1/M5/M15/H1 ladder through the data engine, and call the model. The model publishes numbers; the shell maps the stable numeric codes to labels and never recomputes an axis, aggregate, or event.

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

The output is deterministic: the same input and configuration produce the same bytes, and the interactive and non-interactive invocations print the same text. The command names `market-state`, `axis`, `events`, and `legs` are the implemented contract; changing them is a command-contract decision.

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

The CSV is written through a temporary sibling and renamed only after the fetch completes, so an interrupted or failed fetch does not leave a partial file under the requested name. The source requires network access; the test suite does not, because pagination, retries, JSONP parsing, CSV shape, and chart determinism are exercised with fixtures.

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
