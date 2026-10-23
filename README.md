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
cargo run -p quantrule-shell -- fetch gold 2025-01-01 2025-12-31 --csv data/XAUUSD_M1_2025-01-01_2025-12-31.csv
cargo run -p quantrule-shell -- fetch eurusd 2026-01-01 2026-06-30 --timeframe 1h --csv data/EURUSD_H1_2026-01-01_2026-06-30.csv
cargo run -p quantrule-shell -- fetch gold 2025-01-01 2025-12-31 --png data/xauusd_2025.png
```

## Commands

| Command | Arguments | Output |
|---|---|---|
| `candles` | `<csv-path>` | Up to the last 1000 candles as NDJSON, oldest first |
| `candles-tolerant` | `<csv-path>` | Up to the last 1000 valid candles as NDJSON, plus a quality report on stderr |
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

`candles` reads the file one row at a time. A missing file, bad header, malformed row, invalid OHLC, duplicate timestamp, or out-of-order timestamp aborts the command. The shell builds the window before writing, so a late error produces no partial NDJSON.

`candles-tolerant` reads the same file but skips row-level failures: wrong field count, invalid OHLC, non-finite value, bad timestamp, duplicate timestamp, and out-of-order timestamp. A missing file or bad header still aborts. It builds the window first, then writes the retained valid candles as NDJSON to stdout and the quality report to stderr. A run with skipped rows, including a file where every data row is rejected, exits with status 0.

Interactive errors are printed to stderr and keep the session open. Non-interactive errors exit with status 1. A successful non-interactive `candles` run writes nothing to stderr; a successful `candles-tolerant` run writes its quality report to stderr.

### CSV contract

| Item | Value |
|---|---|
| Header | `Date,Open,High,Low,Close,Volume` |
| Timestamp | `YYYY-MM-DD HH:MM:SS+00:00` (UTC) |
| Columns | Exactly six; volume may be empty |
| Order | Strictly increasing timestamps for `candles`; out-of-order rows are skipped by `candles-tolerant` |

A header-only file is a valid empty series and prints nothing. In tolerant mode it reports zero counts.

### Fetch contract

`fetch` reads native OHLCV candles from Dukascopy's `freeserv.dukascopy.com` chart endpoint on the BID side. Each timeframe is requested with the provider's own interval and is never synthesized from 1-minute candles. The request uses a fixed JSONP callback and fixed browser headers, and the page cursor is always the last timestamp delivered by the page, so the same data and configuration produce the same request sequence and the same output bytes.

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

| Field | Meaning |
|---|---|
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

This slice covers local M1 CSV ingestion, both strict and tolerant, the candle window, and a native Dukascopy fetch for `1m`, `5m`, `15m`, and `1h` with an optional chart. The intervals are downloaded as published by the provider, not aggregated by QuantRule. Indicators, timeframes derived from M1 by the model engine, and backtests are not implemented yet.

## Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```
