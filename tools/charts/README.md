# Trade chart tooling

`trade_charts.py` renders deterministic 15m trade charts from a recorded
backtest bundle. It reads the model readings and portfolio trades, so it does not
recompute a model, strategy, or decision. Charts are evaluation artifacts and
never enter the signal path.

Requirements: Python 3 and Pillow (already used by the historical SR2 chart
tool).

## Example

```bash
python3 tools/charts/trade_charts.py \
  --base data/improvement/2026-11-09/SR9-15M-STRUCTURE/artifacts/BTCUSD \
  --run-glob '*/breakonly-9bps/long' \
  --out data/improvement/2026-11-09/SR9-15M-STRUCTURE \
  --theme light \
  --title 'BTCUSD 15m' \
  --strategy 'BTC break-only long control · 9 bps'
```

Output:

```text
<out>/
  charts/<symbol>-15m-<year>-trade-<id>.png
  charts/manifest.json
  charts/index.csv
```

Each chart contains OHLC candles, volume, EMA 10/50/200, BUY/SELL entry and
exit markers, and the recorded stop/target levels when the execution bundle has them. A chart has
at most 1,000 candles and is centered on the trade entry. Windows with no trade
are not saved.

The manifest records the window, candle count, exact times and prices, and the
PNG SHA-256. The same inputs produce the same PNG bytes.

## Tests

```bash
python3 -m unittest discover -s tools/charts -p 'test_*.py'
```

The rendering tests are skipped when Pillow is unavailable; the window, EMA, and
business-rule tests still run.
