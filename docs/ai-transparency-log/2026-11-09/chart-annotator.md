# AI Transparency Log — Chart Annotator

- Date: 2026-11-09 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash
- Work item: [`chart-annotator`](../../execution-plans/2026-11-09/chart-annotator.md)
- Claim type: evaluation tooling; no edge claim.

## Intent and boundary

Remove the manual annotation CSV step. The tool shows the existing chart, maps a
click to a bar using the manifest geometry, and writes the annotation CSV. It
does not change the chart image, a trade, a model, or a strategy.

## Implemented

- Added plot geometry and per-bar high/low/timestamp to each chart manifest.
- Added `tools/charts/annotate.py`, a local HTTP page with eight marker classes.
- Added CSV upsert/delete, timestamp derivation, local-only file serving, and
  atomic writes.
- Added tests for bar mapping, annotation validation, upsert/delete, and CSV
  round-trip.
- Added the README section and tutorial.
- Regenerated the 73 Binance and 30 Dukascopy chart manifests.

## Verification

- `python3 -m unittest discover -s tools/charts -p 'test_*.py'` — 17 pass.
- Local server smoke: manifest read, POST annotation, GET annotation, CSV written
  with `chart,bar,marked_time_utc,action,quality,side,note`.
- `python3 tools/check_changelog.py` and `python3 tools/check_links.py` — pass.
- Chart manifest check: 73 + 30 charts, each with 1,000 bars and geometry.

## Learning

- A click UI is more reliable than image recognition or hand-entered timestamps.
- The chart manifest is the correct source of plot geometry; the image itself
  should not be parsed for coordinates.
- The annotation CSV keeps the existing `annotations.py` schema; `bar` is an
  optional addition.
- The trailing stop is a separate hypothesis. It was not implemented here.

## Follow-up — undo

Requested after the first annotation pass: `Ctrl+Z` should erase the last choice.

- Added an in-memory undo history to the annotator server, bounded to 100
  entries.
- A save records the row it replaced; a delete records the removed row.
- Undo removes a newly added row, restores the row a save replaced, or re-adds a
  deleted row.
- Added `/api/annotations/undo`, a visible **Undo** button, and `Ctrl+Z`/`Cmd+Z`
  handling.
- Added unit tests for add, overwrite, delete, multi-step, empty, and bounded
  history.
- Smoke-tested the live endpoint through save, undo, overwrite, delete, and
  restore.

The history is session-local; a server restart clears it. The change is tooling
only and does not touch model or strategy behavior.

## Verification (undo follow-up)

- `python3 -m unittest discover -s tools/charts -p 'test_*.py'` — 23 pass.
- Live endpoint smoke: save, undo, overwrite, delete, restore, empty undo — pass.
- Repository checks — pass (recorded in the handoff).

## Not claimed

- No model or strategy behavior change.
- No trailing-stop result.
- No 2024 access.
- No edge or candidate claim.
