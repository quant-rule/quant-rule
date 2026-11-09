# BINANCE-BTCUSDT — Cross-Venue Control Check

- Date: 2026-11-09 (UTC)
- Status: **Complete — the Dukascopy control does not reproduce**
- Type: cross-venue data robustness check; not a tiered improvement run
- Data change: [`DATA-2026-11-09-BINANCE-BTCUSDT-01`](../../config/2026-11-09/data-change-BINANCE-BTCUSDT-01.md)
- Validation: [`binance-btcusdt-data-validation.md`](../../reports/2026-11-09/binance-btcusdt-data-validation.md)
- Local artifacts: `data/improvement/2026-11-09/BINANCE-BTCUSDT/`
- Claim type: data and cross-venue robustness; no edge claim.

## Inputs

| Item | Value |
|---|---|
| Data version | `binance-spot-btcusdt-m1-2018-01-01_2025-12-31-v1` |
| Source | Binance spot BTCUSDT 1m, 2018-01-01..2025-12-31 |
| Strategy | `config/sr9/strategy/sr9-long-breakonly.json` |
| Model | `config/sr9/model-compression.json` |
| Execution | `config/sr9/execution-15m-9bps.json` |
| Portfolio | `config/sr9/portfolio-5pct-9bps.json` |
| Policy | `config/sr9/policies/15m/policy-multi.json` |
| Costs | 9 bps total; 1.5 bps slippage; commission 0.03%/operation |
| Periods | 2018–2022 and 2023, per year |
| Partition | None; 2023 remains outside every formal partition |

## Results

| Feed | Period | Trades | Net | PF | Max DD | Halt |
|---|---|---:|---:|---:|---:|---|
| Dukascopy BTCUSD | 2018–2022 | 30 | +$795.69 | 1.747 | 2.87% | no |
| Binance BTCUSDT | 2018–2022 | 65 | −$389.64 | 0.871 | 5.40% | 2018, 2021 |
| Binance BTCUSDT | 2023 | 8 | −$202.50 | 0.229 | 2.03% | no |

The 2023 Binance cell also trails buy-and-hold by 157.57 percentage points and
sits at the 3.2nd percentile of 1,000 matched random-timing paths.

## Decision

- Do not promote the break-only BTC long control.
- Stop the BTC 15m break-only family; the positive Dukascopy result is not
  venue-robust.
- Do not open 2024.
- Keep 2023 outside the formal partitions.
- A new hypothesis requires a new premise and a new pre-registered programme.

## Limitations

- BTCUSDT is USDT-quoted; the 2023 basis against Dukascopy is recorded, not
  converted.
- The two feeds have different gaps and venue microstructure.
- Binance 2021 has one trade; the aggregate is 65 trades.
- The benchmark null replays commission and sizing, not execution fees, funding,
  or the drawdown halt.

## Records

- Data change: `DATA-2026-11-09-BINANCE-BTCUSDT-01`
- Audit finding: `AUD-2026-11-01-70` remains open pending independent verification
- Charts: `data/improvement/2026-11-09/BINANCE-BTCUSDT/charts/` (73 trade windows)
- Dukascopy charts: `data/improvement/2026-11-09/SR9-15M-STRUCTURE/charts/` (30)
