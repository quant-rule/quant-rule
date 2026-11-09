# SR9 Session Coverage — Declared Trading Calendar

- Date: 2026-11-09 (UTC)
- Status: **Implemented**
- Programme: `SR9-15M-STRUCTURE`
- Claim type: data/report measurement; no model or strategy claim

## 1. Intent

The model report counts weekends and holidays as missing bars because its
coverage is span-based. That is correct for BTCUSD but wrong for XAUUSD and
EURUSD, whose markets close. This work item adds an optional declared trading
calendar and a session-aware coverage figure while keeping the existing span
coverage for compatibility.

## 2. Current state

- `FlatTimeframe` reports `expected_bars`, `missing_bars`, and `coverage_share`
  over the closed span.
- A 15m EURUSD 2022 Q1 smoke run shows 69.9% span coverage but 96.9% session
  coverage; the gap is the weekend.
- `data.json` has no session field.

## 3. Scope

Included:

- Optional `session` in `data.json` (`"24x7"` or a weekly object).
- A pure `TradingCalendar` value in the model crate.
- `session_expected_bars`, `session_missing_bars`, and
  `session_coverage_share` in the flat report.
- JSON and text rendering.
- Calendar tests, data-config tests, and a real-data smoke record.
- A data-change record and a quality record.

Excluded:

- Partial holidays; a holiday closes the whole UTC day.
- A change to the existing span coverage.
- A change to any model reading, state, or event.
- A strategy run.

## 4. Calendar contract

- Weekdays: `Sunday`..`Saturday`, 0 = Sunday.
- Times: UTC `HH:MM`; the open minute is expected and the close minute is not.
- Holidays: UTC dates that close the whole day.
- BTCUSD: `"24x7"`.
- XAUUSD/EURUSD: Sunday 22:00 to Friday 21:00 UTC, with declared holidays.

## 5. Acceptance criteria

- An absent or `"24x7"` session reproduces the span coverage exactly.
- A weekly session excludes weekends and holidays.
- The report carries both span and session fields.
- The data parser rejects a bad weekday, time, date, or empty window.
- Existing reports and model values are unchanged otherwise.
- All Rust, Python, and repository gates pass.

## 6. Verification

- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `cargo doc` with warnings denied
- `cargo fmt --check`
- A real EURUSD Q1 2022 15m run: span 69.9%, session 96.9%.
- Repository checks and `git diff --check`.

## 7. Risks and stop conditions

Risks: a wrong calendar misstates coverage; holidays are a maintenance list.
Mitigation: the calendar is declared in the data version and tested; the span
coverage remains visible.

Stop if the calendar cannot be expressed deterministically or if a 24×7 source
changes its existing coverage.
