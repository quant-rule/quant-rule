# Data Change DATA-2026-11-08-MR7-01

- Date: 2026-11-08 (UTC)
- Status: Active
- Owner: Data Steward
- Workflow: [`data-lifecycle.md`](../../workflows/data-lifecycle.md)
- Claim type: data lifecycle; no model, strategy, or edge claim.

## Source

| Item | COPPERCMDUSD | TSLAUSUSD |
|---|---|---|
| Provider | Local M1 source workspace | Local M1 source workspace |
| Instrument | `COPPERCMDUSD` | `TSLAUSUSD` |
| Timeframe | 1m | 1m |
| Coverage | 2018-01-01..2024-12-31 | 2018-01-01..2024-12-31 |
| Cadence | 1m | 1m |
| Data version prefix | `mr7-copper-m1-<year>` | `mr7-tsla-m1-<year>` |
| Source file | `COPPERCMDUSD_M1_2018-01-01_2024-12-31.csv` | `TSLAUSUSD_M1_2018-01-01_2024-12-31.csv` |

## Quality

| Check | COPPERCMDUSD | TSLAUSUSD |
|---|---|---|
| Header | `Date,Open,High,Low,Close,Volume` | `Date,Open,High,Low,Close,Volume` |
| Parse | all rows parse | all rows parse |
| Order | strictly increasing | strictly increasing |
| Duplicates | 0 | 0 |
| Gaps | recorded, not filled | recorded, not filled |
| Zero volume | 904,640 of 979,317 rows 2018–2020 | 0 of 292,830 rows 2018–2020 |
| Coverage | 2018-01-01 23:05..2020-12-31 21:14 | 2018-01-02 14:30..2020-12-31 20:59 |
| Rows 2018 / 2019 / 2020 | 333,074 / 310,756 / 335,487 | 97,200 / 97,558 / 98,072 |
| Volume unit stability | monthly median is zero for every month | 6.0× / 2.6× / 11,975.4× (2020 unit shift) |

The full quality evidence is in
[`docs/reports/2026-11-08/mr7-data-validation.md`](../../reports/2026-11-08/mr7-data-validation.md).

## Partition

- Partitions: `model_dev`, `model_validation`, `strategy_dev`,
  `strategy_validation`, `portfolio_dev`, `portfolio_holdout`.
- Instruments: `COPPERCMDUSD`, `TSLAUSUSD`.
- Aliases: `COPPER` → `COPPERCMDUSD`; `TSLA` → `TSLAUSUSD`.
- Holdout: unchanged; these instruments inherit the declared partition windows.

## Admissibility

- Rule: a price/level event may use these instruments; a volume, participation,
  or flow event may not, because the volume column is zero or unit-unstable.
- Result: **price-only admitted**; volume/participation/flow excluded.
- Consequence: `MR7.4` (relative-volume confirmation) uses only the five
  existing instruments. `MR7.1`, `MR7.2`, `MR7.3`, and `MR7.5` may use all
  seven instruments.
- Excluded instrument-years: COPPER volume for 2018–2020; TSLA volume for
  2018–2020.

## Affected baselines

| Baseline | Action |
|---|---|
| Accepted compression baselines | Unchanged; no re-baseline required |
| MR5/MR6 model research | Unchanged; historical records |
| MR7 model research | New; uses the declared price-only instruments |

## Records

- Data contract: [`docs/config/2026-10-30/data-v3.md`](../2026-10-30/data-v3.md)
- Partition file: [`data/partitions.json`](../../../data/partitions.json)
- Quality record: [`docs/reports/2026-11-08/mr7-data-validation.md`](../../reports/2026-11-08/mr7-data-validation.md)
- Admissibility review: [`docs/improvement/2026-11-08/MR7-DATA-REVIEW.md`](../../improvement/2026-11-08/MR7-DATA-REVIEW.md)
- Baseline registry: [`docs/improvement/baselines.md`](../../improvement/baselines.md)
- Invalidation: none
- Changelog row: `CHANGELOG.md`
- AI log: [`docs/ai-transparency-log/2026-11-08/mr7-programme.md`](../../ai-transparency-log/2026-11-08/mr7-programme.md)
