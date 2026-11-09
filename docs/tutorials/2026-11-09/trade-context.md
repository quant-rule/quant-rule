# Trade Context Lab

- Date: 2026-11-09 (UTC)
- Status: Current
- Tools: `tools/trades/`
- Contract: [`docs/config/2026-11-09/trade-context.md`](../../config/2026-11-09/trade-context.md)
- Claim type: evaluation procedure; no strategy or edge claim.

## 1. Export one year

```bash
python3 tools/trades/trade_context.py \
  --bundle data/improvement/2026-11-09/BINANCE-BTCUSDT/artifacts/2018/9bps/backtest \
  --out data/improvement/2026-11-09/BINANCE-BTCUSDT/context/2018 \
  --window 250 --post 250
```

Inspect one entry:

```bash
head -1 data/improvement/2026-11-09/BINANCE-BTCUSDT/context/2018/2018-trade-1/entry/context.csv
python3 -m json.tool \
  data/improvement/2026-11-09/BINANCE-BTCUSDT/context/2018/2018-trade-1/entry/summary.json
```

The last `context.csv` row is the entry decision bar. `post.csv` begins at the
entry fill bar.

## 2. Mark a chart point

```csv
chart,marked_time_utc,action,quality,side,note
btcusdt-15m-2018-trade-1.png,2018-01-27T20:15:00Z,entry,good,long,marked low
btcusdt-15m-2018-trade-1.png,2018-01-27T20:45:00Z,exit,good,long,marked high
```

```bash
python3 tools/trades/annotations.py \
  --bundle data/improvement/2026-11-09/BINANCE-BTCUSDT/artifacts/2018/9bps/backtest \
  --annotations annotations.csv \
  --out data/improvement/2026-11-09/BINANCE-BTCUSDT/annotations/2018
```

## 3. Exit-quality table

```bash
python3 tools/trades/exit_quality.py \
  --bundle data/improvement/2026-11-09/BINANCE-BTCUSDT/artifacts/2018/9bps/backtest \
  --out data/improvement/2026-11-09/BINANCE-BTCUSDT/exit_quality/2018.csv
```

Read `captured_return`, `mfe_trade`, `exit_efficiency`, and the `post_50_*`
columns. A low efficiency with a positive post-exit MFE is an early-exit
candidate; a negative captured return with a favorable post-exit move is a
stop-placement candidate. This is a descriptive read, not a rule.

## 4. Determinism

Run the same command twice to different directories and compare the files. The
JSON uses sorted keys and the CSV uses the reading header order.
