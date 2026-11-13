# SR8-A-H3-ARCHIVE — 2023 Dukascopy Archive Check

- Date: 2026-11-09 (UTC)
- Status: **Archive rejected — flat zero-volume placeholders; no H3 run**
- Claim type: data quality evidence; no strategy result

## Evidence

- `data-quality.json` — rows, flat bars, full hours, cross-source differences.
- `data/sources/BTCUSD_M1_2023-01-01_2023-12-31.csv` — local source, SHA-256
  `dc31c23be0c9fabef24c7c92ba24f1f6b87568908703ced8d3b33ff0b18758c2`.
- Committed records: `docs/improvement/2026-11-09/SR8-A-H3-ARCHIVE.md`,
  `docs/reports/2026-11-09/sr8a-data-validation.md`,
  `docs/config/2026-11-09/data-change-SR8A-01.md`.

## Verdict

The archive has 525,600 rows and 100% row-based 1h coverage, but 38,948 bars are
flat zero-volume placeholders. 3,406 hours are complete only because of them;
only 61.4% of hours are genuine. The source is rejected, no H3 backtest was run,
and 2024 remains closed.
