# Data tooling

These scripts build and audit source CSVs. They are offline evaluation tools:
they never compute a model reading, event, strategy decision, or portfolio
result, and they never enter the signal path.

## Binance importer

```bash
python3 tools/data/binance_klines.py \
  --symbol BTCUSDT --start 2018-01 --end 2025-12 \
  --out data/sources/BTCUSDT_M1_2018-01-01_2025-12-31.csv \
  --cache data/scratch/binance \
  --manifest data/sources/BTCUSDT_M1_2018-01-01_2025-12-31.manifest.json \
  --data-version binance-spot-btcusdt-m1-2018-01-01_2025-12-31-v1
```

The importer verifies each monthly `.CHECKSUM`, validates every OHLCV row, floors
the 1,201 known 2018-02 timestamps that are offset by 14.789 seconds, and records
the count in the manifest. It writes no partial output.

## Coverage audit

```bash
python3 tools/data/audit_coverage.py \
  --source data/sources/BTCUSDT_M1_2018-01-01_2025-12-31.csv \
  --instrument BTCUSDT \
  --out data/improvement/2026-11-09/BINANCE-BTCUSDT/coverage.json \
  --compare data/sources/BTCUSD_M1_2018-01-01_2024-12-31.csv \
  --compare-from 2023-01-01 --compare-to 2023-12-31
```

The audit reports per-year 1m and full 15m/1h/4h coverage, gaps, flat zero-volume
bars, duplicates, and a timestamp-aligned cross-venue close/basis comparison.

## Tests

```bash
python3 -m unittest discover -s tools/data -p 'test_*.py'
```

The tools use only the Python standard library.
