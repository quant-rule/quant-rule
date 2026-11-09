# Session Coverage Quality — 2026-11-09

- Date: 2026-11-09 (UTC)
- Status: Validated for session-aware coverage
- Owner: Data Steward
- Data change: [`DATA-2026-11-09-SESSION-COVERAGE-01`](../../config/2026-11-09/data-change-SESSION-COVERAGE-01.md)
- Claim type: data quality evidence; no strategy result

## 1. Method

The model report computes two coverage figures over the same bars:

- **Span coverage**: expected bars from the first to the last closed bar,
  including weekends and holidays.
- **Session coverage**: expected bars inside the declared trading calendar.

The session calendar is a pure value: weekdays 0 = Sunday through 6 = Saturday,
UTC minutes from midnight, half-open window, and whole-day holidays.

## 2. Real-data smoke

EURUSD, 2022-01-01..2022-03-31, `model_timeframes = ["5m", "15m"]`, weekly
session Sunday 22:00 to Friday 21:00 UTC, no declared holidays:

| Timeframe | Bars | Span expected | Span coverage | Session expected | Session coverage |
|---|---:|---:|---:|---:|---:|
| 5m | 18,172 | 25,367 | 71.64% | 18,311 | 99.24% |
| 15m | 5,913 | 8,455 | 69.93% | 6,103 | 96.89% |

The span figure counts the weekend as missing. The session figure does not. The
15m session coverage is above the 95% gate for this slice.

## 3. Contract checks

- An absent or `"24x7"` session reproduces the span coverage exactly.
- A weekly session excludes Saturday and the Friday close minute.
- A holiday closes the whole UTC day.
- An open equal to the close is rejected; use `"24x7"` instead.
- A bad weekday, time, or calendar date is rejected by the config parser.

## 4. Limitations

- Holidays are a declared list; an omitted holiday is still counted as missing.
- Partial holidays and early closes are not represented.
- The calendar explains the expected market window; it does not repair a
  missing candle inside an open session.
- The smoke run uses an existing source; it does not validate a new data version.

## 5. Reproduction

```bash
# A data.json with the weekly session and a run.json whose model_timeframes
# include the execution fill timeframe:
quantrule report model --config run.json --out target/session-smoke
# Read the [coverage] table in target/session-smoke/model/summary.txt.
```

The observed values are in the table above.
