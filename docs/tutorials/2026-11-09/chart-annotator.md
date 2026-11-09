# Chart Annotator Lab

- Date: 2026-11-09 (UTC)
- Status: Current
- Tool: `tools/charts/annotate.py`
- Annotation contract: [`trade-context.md`](../../config/2026-11-09/trade-context.md)
- Claim type: evaluation procedure; no strategy or edge claim.

## 1. Start the page

```bash
python3 tools/charts/annotate.py \
  --charts data/improvement/2026-11-09/BINANCE-BTCUSDT/charts \
  --out data/improvement/2026-11-09/BINANCE-BTCUSDT/annotations-2018.csv \
  --port 8765
```

Open `http://127.0.0.1:8765/`.

The page serves only local files and writes only the requested CSV.

## 2. Mark a chart

1. Choose a chart from the selector, or use the arrow buttons.
2. Press a marker key:
   - `1` good long entry
   - `2` good long exit
   - `3` bad long entry
   - `4` bad long exit
   - `5` good short entry
   - `6` good short exit
   - `7` bad short entry
   - `8` bad short exit
3. Click the candle. The page maps the click to the bar through the chart
   manifest and writes the annotation immediately.

The existing trade entry/exit markers stay on the PNG. The annotation markers
are drawn on top and stored separately.

## 3. Review and remove

The table below the chart lists the annotations for the current chart. The `×`
button removes one. The CSV is rewritten atomically after every change.

## 4. Export the state

Run the existing context exporter against the same bundle:

```bash
python3 tools/trades/annotations.py \
  --bundle data/improvement/2026-11-09/BINANCE-BTCUSDT/artifacts/2018/9bps/backtest \
  --annotations data/improvement/2026-11-09/BINANCE-BTCUSDT/annotations-2018.csv \
  --out data/improvement/2026-11-09/BINANCE-BTCUSDT/annotations/2018
```

The click tool writes the optional `bar` column; `annotations.py` accepts it and
maps the timestamp to the same bar.

## 5. One file per bundle

Use one annotation file per year/feed because the export command receives one
backtest bundle. For example:

```text
BINANCE-BTCUSDT/annotations-2018.csv
BINANCE-BTCUSDT/annotations-2019.csv
SR9-15M-STRUCTURE/annotations-2019.csv
```

## 6. Determinism

The click tool writes rows sorted by chart, bar, action, and side. Re-running
the export is deterministic. The chart images are not modified.
