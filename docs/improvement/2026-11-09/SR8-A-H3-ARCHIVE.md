# SR8-A-H3-ARCHIVE — Dukascopy 2023 Archive Buffer

- Date: 2026-11-09 (UTC)
- Status: **Archive rejected — flat zero-volume placeholders; no H3 run; 2024 not opened**
- Charter: [`SR8-A-CHARTER.md`](../../governance/2026-11-09/SR8-A-CHARTER.md)
- Plan: [`sr8a-h3-archive.md`](../../execution-plans/2026-11-09/sr8a-h3-archive.md)
- Data change: [`DATA-2026-11-09-SR8A-01`](../../config/2026-11-09/data-change-SR8A-01.md)
- Quality record: [`sr8a-data-validation.md`](../../reports/2026-11-09/sr8a-data-validation.md)
- Predecessor: [`SR8-H3-BUFFER.md`](SR8-H3-BUFFER.md)
- Claim type: strategy robustness buffer; no candidate or edge claim

## 1. Intent

SR8's 2023 buffer was data-ineligible because the chart-endpoint source had
61.3% 1h coverage. The approved follow-up was to obtain a complete BTC 2023 1m
source and re-run the frozen H3 long/short rule once. The Dukascopy historical
archive supplied 365 daily files and 525,600 one-minute rows. This record tests
whether that archive is admissible before any strategy run.

## 2. Data quality

| Check | Value |
|---|---:|
| Archive days | 365 / 365 |
| Rows | 525,600 |
| First / last | 2023-01-01 00:00 / 2023-12-31 23:59 UTC |
| Duplicates / order | 0 / strictly increasing |
| Row-based 1h coverage | 8,760 / 8,760 = 100% |
| Flat zero-volume bars | 38,948 (7.41%) |
| Longest flat run | 1,260 minutes, from 2023-12-25 00:00 UTC |
| Archive-only minutes | 38,121 |
| Archive-only flat placeholders | 38,093 (99.93%) |
| Archive-only non-flat minutes | 28 |
| Full hours containing a flat bar | 3,422 / 8,760 |
| Full hours that are entirely flat | 62 |
| Hours full only in the archive | 3,406 |
| Of those, containing a flat bar | 3,406 |
| Merged genuine-minute 1h coverage | 5,354 / 8,714 = 61.4% |
| OHLC mismatches vs chart source | 615 / 487,479; max $105.40 |
| Archive tick probe (2023-07-29 12h) | 0 bytes |

## 3. Why the archive fails

The archive emits a flat, zero-volume candle for every minute with no ticks.
Those candles look like valid candles to the 1h bucket builder, so the row-based
coverage reaches 100% even though the market data is missing. The 3,406 hours
that are full only in the archive are exactly the hours the chart source lacks,
and every one of them contains a flat placeholder.

Example: 2023-07-29 12:30–12:41. The archive holds 29232.2 with zero volume for
twelve minutes while the chart source moves between 29257 and 29273. The archive
tick file for that hour is 0 bytes, so the flat bars are a missing feed, not a
no-trade market. The two Dukascopy endpoints are not the same series.

Merging only the 28 genuine archive-only minutes leaves 1h coverage at 61.4%,
unchanged from the chart source. The archive does not repair the 2023 gap; it
hides it.

## 4. Coverage context

Full-hour coverage in the existing chart source:

| Year | Rows | Full 1h hours | Coverage |
|---|---:|---:|---:|
| 2018 | 365,060 | 5,320 | 85.8% |
| 2019 | 490,743 | 6,863 | 82.6% |
| 2020 | 495,163 | 7,802 | 94.3% |
| 2021 | 523,580 | 8,642 | 99.0% |
| 2022 | 509,527 | 7,381 | 84.5% |
| 2023 | 487,479 | 5,354 | 61.4% |

2023 is the weakest year and is below the SR8 95% gate. A complete source would
still be the right way to test it; the archive is not that source.

## 5. Result

- **No H3 backtest was run.** The data failed the quality gate before the
  strategy was executed.
- **No H3 outcome was produced.** The SR8 chart-source result (0 long / 4 short)
  remains data-ineligible and is not evidence for or against H3.
- **2023 was not consumed by a strategy run.** It was read only for the data
  audit. The one-shot buffer remains unused, but the archive cannot fill it.
- **2024 remains closed.**
- The H3 BTC family remains underpowered: 39 trades over 2018–2022, about
  0.078%/month, far below the 10%/month target.

## 6. Decision

Reject the archive. Do not admit
`dukascopy-bid-btcusd-m1-2023-01-01_2023-12-31-archive-v1`. Do not run H3 on it.
Do not open 2024.

The next step is a human decision:

| Option | Consequence | Assessment |
|---|---|---|
| A. Onboard a genuine 24/7 BTC 2023 1m source (spot exchange export) | A cross-venue buffer with real 1m activity; one more robustness test | Best remaining evidence path, but a different venue and a new source approval |
| B. Revise the 95% gate and accept the 61.4% chart year | No new data; the buffer runs on the weakest year | Not recommended; the gate was pre-registered and 2023 is below every development year |
| C. Stop the stack-fade family | Move to a new premise | Rational if the 39-trade lead is not worth another data programme |

Recommendation: choose A only if the H3 lead is still worth one more test; the
expected return remains far below the target. Otherwise choose C and stop the
family. Do not choose B.

## 7. Limitations

- The archive is a different Dukascopy feed from the chart source; it is not a
  repair of the same series.
- The archive tick probe covers one gap hour; the flat-bar pattern is measured
  across the full year, not every individual hour.
- No strategy result exists, so this record says nothing about H3 performance.
- The 2023 chart source's 61.4% coverage is the best available Dukascopy series;
  it is still below the gate.

## 8. Links

- [Charter](../../governance/2026-11-09/SR8-A-CHARTER.md)
- [Plan](../../execution-plans/2026-11-09/sr8a-h3-archive.md)
- [Data change](../../config/2026-11-09/data-change-SR8A-01.md)
- [Quality record](../../reports/2026-11-09/sr8a-data-validation.md)
- [Predecessor](SR8-H3-BUFFER.md)
- [AI transparency log](../../ai-transparency-log/2026-11-09/sr8a-h3-archive.md)
- Acquisition tool: `tools/improvement/dukascopy_archive.py`
- Local evidence: `data/sources/BTCUSD_M1_2023-01-01_2023-12-31.csv`
