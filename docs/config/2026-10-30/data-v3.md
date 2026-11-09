# Data Configuration v3 — Available Timeframes

- Date: 2026-10-30 (UTC)
- Updated: 2026-11-09 for the optional session calendar and session-aware coverage
- Status: Implemented (#230, PR #239)
- Issues: [#228](https://github.com/quant-rule/quant-rule/issues/228), [#230](https://github.com/quant-rule/quant-rule/issues/230)
- Plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Previous contract: [`docs/config/2026-10-30/policy-and-strategy.md`](policy-and-strategy.md)

## 1. Purpose

`data.json` owns the source, the **available** timeframes the data engine can
build, and the ladder the model runs. The current shell has no strategy file; the
model runs every available timeframe, subject to the model ceiling.

## 2. Shape

```json
{
  "schema": "quantrule.data/v3",
  "source": {
    "kind": "csv",
    "path": "../../../data/EURUSD_M1_2025-01-01_2025-12-31.csv"
  },
  "instrument": "EURUSD",
  "data_version": "dukascopy-bid-eurusd-m1-2025",
  "source_timeframe": "1m",
  "available_timeframes": ["1m", "5m", "15m", "1h", "4h"],
  "session": "24x7",
  "from": "2025-01-01",
  "to": "2025-12-31",
  "max_candles": 600000
}
```

## 3. Fields

| Field | Rule |
|---|---|
| `schema` | Exactly `quantrule.data/v3` |
| `source.kind` | Exactly `csv` |
| `source.path` | Non-empty; resolves against the file's directory |
| `instrument` | Non-empty; must be in the policy universe |
| `data_version` | Non-empty; caller-supplied identity |
| `source_timeframe` | Exactly `1m` |
| `available_timeframes` | Ascending, unique, includes `1m`; members of `1m`, `5m`, `15m`, `1h`, `4h` |
| `from` | Inclusive UTC date or `null` |
| `to` | Inclusive UTC date or `null` |
| `max_candles` | `1..=600000` |
| `session` | Optional; `"24x7"` or a weekly object. Absent means 24×7. |

## 4. Validation

- The list is ascending by duration.
- The list has no duplicates.
- `1m` is present because the source is M1.
- Every policy universe timeframe is in the available list.
- Every model override names an available timeframe.
- The execution fill timeframe is in the available list and the policy universe.
- The source cadence must be one minute. Native M5/M15/H1/H4 files are rejected.
- A sparse M1 file is accepted while at least one adjacent pair is one minute
  apart. A single row is accepted because its cadence cannot be measured.
- A candle timestamp is a UTC Unix second in 1970-01-01T00:00:00Z through
  9999-12-31T23:59:59Z. The data engine rejects a value outside that range, so
  downstream bucket and context arithmetic is bounded.

## 5. 4h construction

- 4h means 14,400 seconds.
- Buckets align to UTC 00:00, 04:00, 08:00, 12:00, 16:00, 20:00.
- The first and last buckets may be partial when the date range cuts them.
- 4h is derived from M1; the source is never re-read per timeframe.

## 6. Session-aware coverage

The model report keeps the span coverage and adds a session-aware coverage:

| Field | Meaning |
|---|---|
| `session_expected_bars` | Bars expected inside the declared session |
| `session_missing_bars` | `session_expected_bars - bars`, never negative |
| `session_coverage_share` | `bars / session_expected_bars`, clamped to 0..=1 |

A weekly session is half-open: the open minute is expected and the close minute
is not. Weekdays are `Sunday`..`Saturday`; times are UTC `HH:MM`; holidays are
UTC dates that close the whole day.

```json
{
  "session": {
    "kind": "weekly",
    "open_weekday": "Sunday",
    "open_time": "22:00",
    "close_weekday": "Friday",
    "close_time": "21:00",
    "holidays": ["2025-01-01"]
  }
}
```

For BTCUSD the session is `"24x7"`. For XAUUSD and EURUSD the weekly session is
the default market window; a data version that uses a different provider
calendar must declare it. Holidays are part of the data version identity.

## 7. Ownership

- `data.json` says what can be built and which ladder the model runs.
- `model.json` says how each available timeframe is measured.
- `execution.json` is validated but is not run by the current shell.
- No strategy file or strategy ladder is part of the current run.
