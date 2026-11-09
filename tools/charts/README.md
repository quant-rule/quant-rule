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

Each chart contains OHLC candles, volume, the model EMA 20/50/200
(`ema_fast`, `ema_mid`, `ema_slow`), BUY/SELL entry and exit markers, and the
recorded stop/target levels when the execution bundle has them. A chart has
at most 1,000 candles and is centered on the trade entry. Windows with no trade
are not saved.

The manifest records the window, candle count, exact times and prices, the plot
geometry, the per-bar high/low, and the PNG SHA-256. The same inputs produce the
same bytes.

## Click annotator

```bash
python3 tools/charts/annotate.py   --charts data/improvement/2026-11-09/BINANCE-BTCUSDT/charts   --out data/improvement/2026-11-09/BINANCE-BTCUSDT/annotations-2018.csv   --port 8765
```

Open `http://127.0.0.1:8765/`. Select a chart, press a marker key, then click the
candle. The page writes `annotations.csv` directly; no hand-edited CSV and no
pixel detection.

| Key | Marker |
|---:|---|
| 1 | Good long entry: green filled circle |
| 2 | Good long exit: red filled circle |
| 3 | Bad long entry: green hollow circle |
| 4 | Bad long exit: red hollow circle |
| 5 | Good short entry: red filled square |
| 6 | Good short exit: green filled square |
| 7 | Bad short entry: red hollow square |
| 8 | Bad short exit: green hollow square |

The annotation CSV is the same schema consumed by
`tools/trades/annotations.py`; the `bar` column is optional for hand-written
files and is written by the click tool.

## Tests

```bash
python3 -m unittest discover -s tools/charts -p 'test_*.py'
```

The rendering tests are skipped when Pillow is unavailable; the window, model
EMA, and business-rule tests still run.
