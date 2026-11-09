# Binance BTCUSDT Data Validation and Cross-Venue Control

- Date: 2026-11-09 (UTC)
- Status: **Complete — cross-venue control does not reproduce the Dukascopy lead**
- Owner: Data Steward / Quant Validator (sequential pass)
- Data change: [`DATA-2026-11-09-BINANCE-BTCUSDT-01`](../../config/2026-11-09/data-change-BINANCE-BTCUSDT-01.md)
- Plan: [`btc-visual-benchmarks.md`](../../execution-plans/2026-11-09/btc-visual-benchmarks.md)
- Claim type: data quality and cross-venue robustness; no edge claim.

## 1. Source

| Item | Value |
|---|---|
| Provider | Binance public spot monthly klines |
| Instrument | `BTCUSDT` |
| Timeframe | 1m source; 15m derived |
| Coverage | 2018-01-01..2025-12-31 |
| Rows | 4,199,615 |
| SHA-256 | `46af1a54a82a809cc4eb8e14ddda9dfbe04c0670ad143faaaad1c2bf7fb7c1b1` |
| Checksums | 96 monthly files verified |
| Data version | `binance-spot-btcusdt-m1-2018-01-01_2025-12-31-v1` |

## 2. Integrity and coverage

| Check | Result |
|---|---|
| Header | `Date,Open,High,Low,Close,Volume` |
| Parse | all rows parse |
| Order | strictly increasing; 0 out-of-order |
| Duplicates | 0 |
| Normalized unaligned rows | 1,201 in 2018-02, floored by 14.789 seconds |
| Flat zero-volume bars | 307 |
| Gaps | 31; 8,065 missing minutes; longest 2,010 minutes |
| Longest gap | 2018-02-08 00:28 → 2018-02-09 09:59 UTC |

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

The dense source clears the 95% gate every year. Dukascopy BTCUSD full 15m
coverage, by comparison, is 64.5% (2018), 85.8% (2019), 92.3% (2020), 99.4%
(2021), 89.4% (2022), 74.0% (2023), and 96.9% (2024).

## 3. Cross-venue basis, 2023

| Measure | Value |
|---|---:|
| Overlap rows | 487,399 |
| Close difference, median | $33.57 |
| Close difference, p95 | $61.87 |
| Close difference, max | $620.10 |
| Basis, median | +0.115% |
| Basis, p05 / p95 | +0.013% / +0.230% |
| Basis, min / max | −1.429% / +2.258% |

The median basis is consistent with a USDT quote. The series is not spliced or
rescaled.

## 4. Cross-venue control, 9 bps

The frozen SR9 break-only BTC long strategy, same model, execution, portfolio,
and costs, run on both feeds:

| Feed | Period | Trades | Net | PF | Max DD | Halt | Positive years |
|---|---|---:|---:|---:|---:|---|---:|
| Dukascopy BTCUSD | 2018–2022 | 30 | +$795.69 | 1.747 | 2.87% | no | 4 / 5 |
| Binance BTCUSDT | 2018–2022 | 65 | −$389.64 | 0.871 | 5.40% | 2018, 2021 | 1 / 5 |
| Binance BTCUSDT | 2023 | 8 | −$202.50 | 0.229 | 2.03% | no | — |

Binance per-year:

| Year | Trades | Net | PF | Max DD | Halted |
|---|---:|---:|---:|---:|---|
| 2018 | 11 | −$102.63 | 0.853 | 5.40% | yes |
| 2019 | 23 | +$93.12 | 1.102 | 3.65% | no |
| 2020 | 23 | −$189.25 | 0.808 | 4.41% | no |
| 2021 | 1 | −$89.82 | 0.000 | 5.00% | yes |
| 2022 | 7 | −$101.07 | 0.686 | 4.64% | no |
| 2023 | 8 | −$202.50 | 0.229 | 2.03% | no |

## 5. Benchmarks

The new `analytics evaluate` output on the 2023 Binance run:

| Baseline | Total return | Max drawdown |
|---|---:|---:|
| Strategy | −2.03% | 2.03% |
| Buy-and-hold | +155.54% | 20.00% |
| Random timing, median | −0.51% | 0.81% |

- Alpha vs buy-and-hold: **−157.57%**.
- Strategy percentile vs random timing: **3.2%**.
- Probability a random path beats the strategy: **96.8%**.

The strategy did not merely fail to beat the market; it was worse than almost
every matched random-timing path in 2023.

## 6. Conclusion

The dense 24/7 spot series changes the trade set and the sign of the result. The
positive Dukascopy control is **not venue-robust**. It does not survive a clean
Binance BTCUSDT series, and the 2023 cell is negative and far behind
buy-and-hold.

This does not prove the rule has no edge in every context. It does prove that the
earlier positive control is not evidence of a durable edge and must not be
promoted. The practical conclusion is:

- Stop the BTC 15m break-only family; do not build a candidate on the Dukascopy
  control.
- Do not open 2024; it would not repair a sign flip on a cleaner feed.
- Keep 2023 outside the formal partitions.
- A new hypothesis needs a new premise and a new pre-registered programme, not
  another filter cut on this one.

## 7. Limitations

- BTCUSDT is quoted in USDT, not USD. The basis is recorded, not converted.
- The two feeds have different venue microstructure and different data gaps;
  different entries are expected. The point is whether the rule survives both.
- The Binance 2021 cell has one trade; the aggregate 2018–2022 result is 65
  trades, not a large sample.
- The run uses 9 bps commission and 1.5 bps slippage; execution fees are zero.
- The random-timing null replays commission and sizing, not execution fees,
  funding, or a drawdown halt.

## 8. Reproduction

```bash
python3 tools/data/binance_klines.py \
  --symbol BTCUSDT --start 2018-01 --end 2025-12 \
  --out data/sources/BTCUSDT_M1_2018-01-01_2025-12-31.csv \
  --cache data/scratch/binance \
  --manifest data/sources/BTCUSDT_M1_2018-01-01_2025-12-31.manifest.json \
  --data-version binance-spot-btcusdt-m1-2018-01-01_2025-12-31-v1

python3 tools/data/audit_coverage.py \
  --source data/sources/BTCUSDT_M1_2018-01-01_2025-12-31.csv \
  --instrument BTCUSDT \
  --out data/improvement/2026-11-09/BINANCE-BTCUSDT/coverage.json \
  --compare data/sources/BTCUSD_M1_2018-01-01_2024-12-31.csv \
  --compare-from 2023-01-01 --compare-to 2023-12-31

quantrule policy check --config config/sr9/runs/BTCUSDT/2023-9bps.json
quantrule backtest run \
  --config config/sr9/runs/BTCUSDT/2023-9bps.json \
  --strategy config/sr9/strategy/sr9-long-breakonly.json \
  --out data/improvement/2026-11-09/BINANCE-BTCUSDT/artifacts/2023/9bps
quantrule analytics evaluate \
  --evidence data/improvement/2026-11-09/BINANCE-BTCUSDT/artifacts/2023/9bps/backtest \
  --out data/improvement/2026-11-09/BINANCE-BTCUSDT/artifacts/2023/9bps
```
