# Trade Context Tooling

`tools/trades/` exports the recorded state around every trade entry and exit, and
describes exit quality. It is evaluation-only: it never changes a model,
strategy, decision, or portfolio result.

## Tools

| Tool | Output |
|---|---|
| `trade_context.py` | Per-trade entry/exit context, post-fill window, events, decision, orders, fills, summary |
| `annotations.py` | The same context for human-marked timestamps |
| `exit_quality.py` | One row per trade with MFE/MAE, exit efficiency, and post-exit continuation |

## Trade context

```bash
python3 tools/trades/trade_context.py \
  --bundle data/improvement/2026-11-09/BINANCE-BTCUSDT/artifacts/2018/9bps/backtest \
  --out data/improvement/2026-11-09/BINANCE-BTCUSDT/context/2018 \
  --window 250 --post 250
```

Output:

```text
<out>/<year>-trade-<id>/
  trade.json
  entry/
    context.csv   # 250 bars ending at the entry decision bar
    post.csv      # 250 bars from the entry fill bar
    events.csv
    decision.json
    orders.json
    fills.json
    summary.json
  exit/
    context.csv   # 250 bars ending at the exit decision bar
    post.csv      # 250 bars after the exit fill
    events.csv
    decision.json
    orders.json
    fills.json
    summary.json
  index.json
```

`context.csv` copies every column of `model/readings.csv`. The last row is the
decision bar; no later row is present. `post.csv` is separate and is never used
in `summary.json`.

## Annotations

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

The tool maps the timestamp to the nearest bar, rejects a mark more than half a
bar away, and writes the same context/summary artifacts.

## Exit quality

```bash
python3 tools/trades/exit_quality.py \
  --bundle data/improvement/2026-11-09/BINANCE-BTCUSDT/artifacts/2018/9bps/backtest \
  --out data/improvement/2026-11-09/BINANCE-BTCUSDT/exit_quality/2018.csv
```

The table covers captured return, trade MFE/MAE, exit efficiency, the state at the
exit decision bar, and post-exit MFE/MAE over 5/10/20/50 bars. Post-exit values are
descriptive and never enter a strategy.

## Tests

```bash
python3 -m unittest discover -s tools/trades -p 'test_*.py'
```

The tools use only the Python standard library.
