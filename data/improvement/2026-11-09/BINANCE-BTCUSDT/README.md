# BINANCE-BTCUSDT — Cross-Venue Control

- Date: 2026-11-09 (UTC)
- Status: Complete
- Data version: `binance-spot-btcusdt-m1-2018-01-01_2025-12-31-v1`
- Source: `data/sources/BTCUSDT_M1_2018-01-01_2025-12-31.csv`
- Source SHA-256: `46af1a54a82a809cc4eb8e14ddda9dfbe04c0670ad143faaaad1c2bf7fb7c1b1`
- Committed record: [`docs/improvement/2026-11-09/BINANCE-BTCUSDT.md`](../../../docs/improvement/2026-11-09/BINANCE-BTCUSDT.md)
- Validation: [`docs/reports/2026-11-09/binance-btcusdt-data-validation.md`](../../../docs/reports/2026-11-09/binance-btcusdt-data-validation.md)

## Contents

| Path | Holds |
|---|---|
| `coverage.json` | Coverage, gaps, flat bars, and 2023 cross-venue basis |
| `artifacts/<year>/9bps/` | Backtest and analytics bundles for 2018–2023 |
| `charts/` | 73 fifteen-minute trade charts |
| `manifest.json`, `index.csv` | Chart manifest and index |
| `results.json` | Aggregated result record |

## Result

| Feed | Period | Trades | Net | PF | Max DD | Halt |
|---|---|---:|---:|---:|---:|---|
| Dukascopy BTCUSD | 2018–2022 | 30 | +$795.69 | 1.747 | 2.87% | no |
| Binance BTCUSDT | 2018–2022 | 65 | −$389.64 | 0.871 | 5.40% | 2018, 2021 |
| Binance BTCUSDT | 2023 | 8 | −$202.50 | 0.229 | 2.03% | no |

2023 benchmarks: strategy −2.03%, buy-and-hold +155.54%, random median −0.51%,
strategy percentile 3.2%, probability random beats strategy 96.8%.

Decision: no promotion; stop the BTC 15m break-only family; 2023 remains outside
the formal partitions; 2024 remains closed.
