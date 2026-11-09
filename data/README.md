# Local data workspace

Everything under `data/` is local. Git tracks only this file, the README in each
subdirectory, and `partitions.json`; candles, charts, reports, databases, and run
artifacts never leave the machine.

## Current workspace

| Path | Holds |
|---|---|
| [`sources/`](sources/README.md) | Provider candles, the only input data |
| [`improvement/`](improvement/README.md) | The local improvement run store |
| [`partitions.json`](partitions.json) | The partition windows, instruments, and aliases |
| `README.md` | This guide |

## Historical local directories

The following directories are retained locally for the reproducibility of
historical records. They are not current tool defaults, and no current command
writes to them:

- `charts/` — retired chart artifacts.
- `db/` — retired SQLite stores.
- `labs/` — retired lab bundles.
- `notes/` — human-authored notes.
- `reports/` — retired report bundles.
- `research/` — retired research exports and programme artifacts.
- `scratch/` — disposable intermediate output.
- `macd-pilot/`, `model-gate/`, `model-v4/`, `path-pilot/`,
  `prediction-pilot/`, `session-spike/`, `structure-pilot/` — historical
  programme artifacts.

Their content is evidence, not a current surface. A cleanup is a separate,
explicitly authorized change.

## Archived local evidence

On 2026-11-09, large historical directories were moved to
`/home/zantgo/quant-rule-archive/2026-11-09/` and left in place as symlinks. The
original paths still resolve; the repository listing and `du` no longer carry
the full payload. The cleanup record is
[`docs/reports/2026-11-09/local-data-cleanup.md`](../docs/reports/2026-11-09/local-data-cleanup.md).

| Path | Archive |
|---|---|
| `model-v4/` | `data/model-v4/` |
| `model-gate/` | `data/model-gate/` |
| `session-spike/` | `data/session-spike/` |
| `prediction-pilot/` | `data/prediction-pilot/` |
| `macd-pilot/` | `data/macd-pilot/` |
| `path-pilot/` | `data/path-pilot/` |
| `structure-pilot/` | `data/structure-pilot/` |
| `improvement/2026-11-08/STRATEGY-2026-11-08-01/` | the same relative path |
| `improvement/2026-11-08/STRATEGY-2026-11-08-02/` | the same relative path |
| `improvement/2026-11-08/SR3-FEATURES/` | the same relative path |
| `improvement/2026-11-09/SR4-STACK-FADE/` | the same relative path |
| `improvement/2026-11-09/SR6-VWAP/` | the same relative path |

## Rules

1. Nothing sits at the root except this file and `partitions.json`.
2. A source is `<SYMBOL>_<TF>_<start>_<end>.csv`.
3. A source candle file is immutable. Everything regenerable is disposable;
   `scratch/` may be emptied at any time.
4. `fetch` writes to `sources/`. `run_current.py` writes to
   `data/improvement/<date>/<run-id>/`.
5. Keep a run only while it is cited or still reproducible. Delete it otherwise.
6. A run may cite only a partition and instrument declared in `partitions.json`.
7. A data defect is fixed through the Audit Workflow and gets a new data version;
   see [`docs/workflows/data-lifecycle.md`](../docs/workflows/data-lifecycle.md).
