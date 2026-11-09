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

## Not claimed

- No model or strategy behavior change.
- No trailing-stop result.
- No 2024 access.
- No edge or candidate claim.
