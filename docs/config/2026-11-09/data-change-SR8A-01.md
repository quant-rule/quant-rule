# Data Change DATA-2026-11-09-SR8A-01

- Date: 2026-11-09 (UTC)
- Status: **Rejected — not admitted**
- Owner: Data Steward
- Workflow: [`data-lifecycle.md`](../../workflows/data-lifecycle.md)
- Work item: [`sr8a-h3-archive.md`](../../execution-plans/2026-11-09/sr8a-h3-archive.md)
- Claim type: data lifecycle; no model, strategy, or edge claim

## Source

| Item | Value |
|---|---|
| Provider | Dukascopy historical archive |
| Instrument | `BTCUSD` |
| Timeframe | 1m BID candles |
| Coverage | 2023-01-01..2023-12-31 |
| Cadence | 1m |
| Candidate data version | `dukascopy-bid-btcusd-m1-2023-01-01_2023-12-31-archive-v1` |
| Source file | `BTCUSD_M1_2023-01-01_2023-12-31.csv` |
| SHA-256 | `dc31c23be0c9fabef24c7c92ba24f1f6b87568908703ced8d3b33ff0b18758c2` |

## Quality

| Check | Result |
|---|---|
| Header | `Date,Open,High,Low,Close,Volume` |
| Parse | all rows parse |
| Order | strictly increasing |
| Duplicates | 0 |
| Rows | 525,600 |
| Row-based 1h full hours | 8,760 / 8,760 = 100% |
| Flat zero-volume bars | 38,948 (7.41%) |
| Longest flat run | 1,260 minutes |
| Archive-only minutes | 38,121 |
| Archive-only flat placeholders | 38,093 (99.93%) |
| Full hours completed only by the archive | 3,406 |
| Full hours among those with a flat bar | 3,406 |
| Merged genuine-minute 1h coverage | 61.4% |
| OHLC mismatches vs chart source | 615 / 487,479 |
| Maximum absolute OHLC difference | $105.40 |
| Archive tick probe (2023-07-29 12h) | 0 bytes |

The full evidence is in
[`docs/reports/2026-11-09/sr8a-data-validation.md`](../../reports/2026-11-09/sr8a-data-validation.md).

## Decision

**Rejected.** The archive pads missing ticks with flat zero-volume candles. Its
100% row coverage is not market coverage: 3,406 hours are complete only because
the archive repeats the last price with no volume. The series is not admissible
for the H3 buffer or any other strategy run.

- No data version is admitted.
- No partition change; 2023 remains outside every formal partition.
- No H3 backtest was run under this change.
- 2024 remains closed.
- The source file is retained locally as evidence only.

## Affected baselines

| Baseline | Action |
|---|---|
| Accepted model baselines | Unchanged |
| SR1–SR8 strategy records | Unchanged; historical |
| Strategy baseline | None accepted |

## Records

- Data contract: [`docs/config/2026-10-30/data-v3.md`](../2026-10-30/data-v3.md)
- Partition file: [`data/partitions.json`](../../../data/partitions.json)
- Quality record: [`docs/reports/2026-11-09/sr8a-data-validation.md`](../../reports/2026-11-09/sr8a-data-validation.md)
- Programme record: [`docs/improvement/2026-11-09/SR8-A-H3-ARCHIVE.md`](../../improvement/2026-11-09/SR8-A-H3-ARCHIVE.md)
- Baseline registry: [`docs/improvement/baselines.md`](../../improvement/baselines.md)
- Invalidation: none
- Changelog row: `CHANGELOG.md`
- AI log: [`docs/ai-transparency-log/2026-11-09/sr8a-h3-archive.md`](../../ai-transparency-log/2026-11-09/sr8a-h3-archive.md)
