# Local data workspace

Everything under `data/` is local. Git tracks only this file and the README in
each subdirectory; candles, charts, reports, and databases never leave the
machine.

| Directory | Holds |
|---|---|
| [`sources/`](sources/README.md) | Provider candles, the only input data |
| [`charts/`](charts/README.md) | PNG charts derived from candles |
| [`reports/`](reports/README.md) | `market-report` and `report <engine>` artifacts |
| [`labs/`](labs/README.md) | One full `backtest` run with its summary |
| [`research/`](research/README.md) | `research export` layers and manifest |
| [`db/`](db/README.md) | SQLite stores |
| [`notes/`](notes/README.md) | Human-authored protocols and notes |
| [`scratch/`](scratch/README.md) | Disposable intermediate output |

## Rules

1. Nothing sits at the root except this file.
2. A source is `<SYMBOL>_<TF>_<start>_<end>.csv`; a chart is the same stem plus
   `_candles.png`.
3. A source candle file is immutable. Everything regenerable is disposable;
   `scratch/` may be emptied at any time.
4. Tool defaults write here: `fetch` to `sources/`, `research export` to
   `research/`, and `store init` to `db/quantrule.db`.
5. Keep a run only while it is cited or still reproducible. Delete it otherwise.
