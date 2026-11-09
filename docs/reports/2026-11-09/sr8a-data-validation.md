# SR8-A Data Validation — Dukascopy 2023 Archive

- Date: 2026-11-09 (UTC)
- Status: **Rejected — flat zero-volume placeholders; not admitted**
- Owner: Data Steward
- Data change: [`DATA-2026-11-09-SR8A-01`](../../config/2026-11-09/data-change-SR8A-01.md)
- Work item: [`sr8a-h3-archive.md`](../../execution-plans/2026-11-09/sr8a-h3-archive.md)
- Claim type: data quality evidence; no model or strategy result

## 1. Source

| Item | Value |
|---|---|
| Provider | Dukascopy historical archive |
| Instrument | `BTCUSD` |
| Timeframe | 1m BID candles |
| Coverage | 2023-01-01..2023-12-31 |
| Files | 365 daily `BID_candles_min_1.bi5` |
| Decoder | `tools/improvement/dukascopy_archive.py` |
| Output | `data/sources/BTCUSD_M1_2023-01-01_2023-12-31.csv` |
| SHA-256 | `dc31c23be0c9fabef24c7c92ba24f1f6b87568908703ced8d3b33ff0b18758c2` |
| Rows | 525,600 |

## 2. Row integrity

| Check | Result |
|---|---|
| Header | `Date,Open,High,Low,Close,Volume` |
| Parse | all rows parse |
| Order | strictly increasing |
| Duplicates | 0 |
| First / last | 2023-01-01 00:00 / 2023-12-31 23:59 UTC |
| Row-based 1h coverage | 8,760 / 8,760 = 100% |

Row integrity passes. The row-based coverage is not market coverage.

## 3. Market-activity audit

The archive emits a flat, zero-volume candle for a minute with no ticks. Those
candles are indistinguishable from real candles to the 1h bucket builder.

| Check | Value |
|---|---:|
| Flat zero-volume bars | 38,948 (7.41%) |
| Longest flat run | 1,260 minutes, from 2023-12-25 00:00 UTC |
| Archive-only minutes vs the chart source | 38,121 |
| Archive-only flat placeholders | 38,093 (99.93%) |
| Archive-only non-flat minutes | 28 |
| Full hours containing any flat bar | 3,422 / 8,760 |
| Full hours that are entirely flat | 62 |
| Hours full only in the archive | 3,406 |
| Of those, containing a flat bar | 3,406 |
| Of those, entirely flat | 61 |

The 1h coverage rises from the chart source's 5,354 full hours (61.4%) to the
archive's 8,760 full hours only because 3,406 hours are completed by flat
placeholders. Merging only the 28 genuine archive-only minutes leaves coverage
at 5,354 / 8,714 = 61.4%.

## 4. Cross-source comparison

| Check | Value |
|---|---:|
| Chart 2023 rows | 487,479 |
| Overlapping rows | 487,479 |
| OHLC mismatches | 615 (0.126%) |
| Maximum absolute OHLC difference | $105.40 |
| Volume differences | 102,146 |
| Maximum absolute volume difference | 0.000592 |

The mismatches are not ordinary tick rounding: long archive stretches repeat a
single price while the chart source shows a moving market. Example: 2023-07-29
12:30–12:41, the archive holds 29232.2 with zero volume while the chart source
moves between 29257 and 29273.

The archive tick file for 2023-07-29 12h is 0 bytes, so the flat bars reflect a
missing tick feed, not a genuine no-trade period. The two Dukascopy endpoints are
not the same series.

## 5. Coverage context

Full-hour coverage in the existing chart source, by year:

| Year | Rows | Full 1h hours | Coverage |
|---|---:|---:|---:|
| 2018 | 365,060 | 5,320 | 85.8% |
| 2019 | 490,743 | 6,863 | 82.6% |
| 2020 | 495,163 | 7,802 | 94.3% |
| 2021 | 523,580 | 8,642 | 99.0% |
| 2022 | 509,527 | 7,381 | 84.5% |
| 2023 | 487,479 | 5,354 | 61.4% |

2023 is the weakest year in the chart source and is below the SR8 95% gate. The
archive does not repair it; it only hides the gap with flat bars.

## 6. Decision

- **Rejected.** The archive is not admitted as a 2023 BTCUSD source.
- No H3 backtest is run. The SR8-A buffer stops as data-ineligible.
- No data version is admitted; `dukascopy-bid-btcusd-m1-2023-01-01_2023-12-31-archive-v1`
  is the rejected candidate identity.
- 2023 remains outside every formal partition. 2024 remains closed.
- The local CSV is retained as evidence, not as an admitted source.

## 7. Reproduction

```bash
python3 tools/improvement/dukascopy_archive.py \
  --symbol BTCUSD --year 2023 \
  --out data/sources/BTCUSD_M1_2023-01-01_2023-12-31.csv

python3 - <<'PY'
# Stream both 2023 series, count flat zero-volume bars, full hours, archive-only
# minutes, and OHLC differences. The observed values are in the tables above.
PY
```

The archive tool reports `rows=525600 full_hours=8760/8760 flat_zero=38948
longest_flat=1260`. The `flat_zero` count is the gate that matters.
