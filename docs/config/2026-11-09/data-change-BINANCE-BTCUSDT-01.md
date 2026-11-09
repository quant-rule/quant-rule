# Data Change DATA-2026-11-09-BINANCE-BTCUSDT-01

- Date: 2026-11-09 (UTC)
- Status: **Active** — admitted for cross-venue robustness, not a formal partition
- Owner: Data Steward
- Workflow: [`data-lifecycle.md`](../../workflows/data-lifecycle.md)
- Plan: [`btc-visual-benchmarks.md`](../../execution-plans/2026-11-09/btc-visual-benchmarks.md)
- Audit: [`binance-btcusdt-data-validation.md`](../../reports/2026-11-09/binance-btcusdt-data-validation.md)
- Claim type: data lifecycle; no strategy or edge claim.

## Source

| Item | Value |
|---|---|
| Provider | Binance public spot archive (`data.binance.vision`) |
| Instrument | `BTCUSDT` (BTC spot vs USDT) |
| Timeframe | 1m source; 15m derived by the data engine |
| Coverage | 2018-01-01..2025-12-31, 96 monthly files |
| Data version | `binance-spot-btcusdt-m1-2018-01-01_2025-12-31-v1` |
| Local file | `data/sources/BTCUSDT_M1_2018-01-01_2025-12-31.csv` |
| SHA-256 | `46af1a54a82a809cc4eb8e14ddda9dfbe04c0670ad143faaaad1c2bf7fb7c1b1` |
| Rows | 4,199,615 |
| Import manifest | `data/sources/BTCUSDT_M1_2018-01-01_2025-12-31.manifest.json` |
| Importer | [`tools/data/binance_klines.py`](../../../tools/data/binance_klines.py) |

Every monthly zip is verified against its published `.CHECKSUM` before parsing.

## Normalization

- 1,201 rows in 2018-02 have an open time offset by 14.789 seconds. The importer
  floors them to the containing minute and records the count in the manifest.
  There are no timestamp collisions after flooring.
- Duplicates: 0.
- Out-of-order rows: 0.
- Flat zero-volume bars: 307.
- Gaps: 31, 8,065 missing minutes; the longest is 2,010 minutes from
  2018-02-08 00:28 to 2018-02-09 09:59. The other large gaps are exchange
  maintenance windows.
- The source is BTCUSDT, not BTCUSD. The economic instrument is labelled
  `BTCUSD` for the existing policy ticket; the data version and this record carry
  the venue and quote. The 2023 cross-venue basis against Dukascopy is recorded
  below.

## Coverage

| Year | 1m | Full 15m | Full 1h | Full 4h |
|---|---:|---:|---:|---:|
| 2018 | 99.244% | 99.235% | 99.201% | 98.904% |
| 2019 | 99.664% | 99.658% | 99.635% | 99.406% |
| 2020 | 99.762% | 99.758% | 99.727% | 99.408% |
| 2021 | 99.811% | 99.809% | 99.783% | 99.498% |
| 2022 | 100.000% | 100.000% | 100.000% | 100.000% |
| 2023 | 99.985% | 99.983% | 99.977% | 99.954% |
| 2024 | 100.000% | 100.000% | 100.000% | 100.000% |
| 2025 | 100.000% | 100.000% | 100.000% | 100.000% |

The 2023 full 15m/1h/4h coverage is above the 95% gate. The source is admitted
for the cross-venue robustness check.

## Cross-venue basis, 2023

Timestamp-aligned comparison against the Dukascopy BTCUSD chart source:

| Measure | Value |
|---|---:|
| Overlap rows | 487,399 |
| Close difference, median | $33.57 |
| Close difference, p95 | $61.87 |
| Close difference, max | $620.10 |
| Basis, median | +0.115% |
| Basis, p05 / p95 | +0.013% / +0.230% |
| Basis, min / max | −1.429% / +2.258% |

The USDT quote and venue explain a positive basis of about 0.1%. The tails are
larger; the series is not spliced, rescaled, or treated as USD.

## Decision

- **Admitted** as a separate data version for cross-venue robustness and
  evaluation.
- **Not** added to `data/partitions.json`; 2023 remains outside every formal
  partition and 2024 remains closed.
- **Not** spliced into the Dukascopy BTCUSD series.
- The same frozen SR9 break-only strategy is run on the Binance series for
  2018–2023. The result is recorded in the validation report.
- No accepted baseline is changed; no strategy is promoted.

## Affected baselines

| Baseline | Action |
|---|---|
| Accepted model baselines | Unchanged |
| Strategy or portfolio baselines | None accepted |
| Dukascopy BTCUSD control lead | Not promoted; the cross-venue result weakens it |
| 2023 partition | Unchanged; excluded |

## Records

- Data change: `DATA-2026-11-09-BINANCE-BTCUSDT-01`
- Audit finding: `AUD-2026-11-01-70` (dense-source option; remains open pending
  independent verification)
- Run record: [`BINANCE-BTCUSDT`](../../improvement/2026-11-09/BINANCE-BTCUSDT.md)
- Validation: [`binance-btcusdt-data-validation.md`](../../reports/2026-11-09/binance-btcusdt-data-validation.md)
