# Chart Annotator

- Date: 2026-11-09 (UTC)
- Status: **Verified**
- Work item: `chart-annotator`
- Workflows: Requirements & Design, Strategy Research (evaluation input)
- Claim type: evaluation tooling; no strategy or edge claim.

## Intent

Replace hand-edited annotation timestamps with a local click-to-annotate page. The
page maps a candle click to a bar through the chart manifest geometry and writes
the annotation CSV directly.

## Scope

Included:

- chart manifest geometry and per-bar high/low;
- `tools/charts/annotate.py` local HTTP page;
- annotation CSV write/update/delete;
- tests, README, tutorial, changelog, AI log.

Excluded:

- model or strategy behavior changes;
- trailing-stop implementation;
- 2024 access;
- a new shell command.

## Design

- The chart manifest records `plot_left`, `plot_width`, `price_top`,
  `price_bottom`, `price_low`, `price_high`, `window_start_bar`,
  `window_end_bar`, `candles`, `bars`, and `timeframe_secs`.
- The page maps image x to the candle index and bar number.
- Marker keys 1–8 encode action, quality, and side.
- The server binds to `127.0.0.1`, serves only files under the chart directory,
  and writes the CSV atomically.
- The CSV keeps the `annotations.py` schema; the `bar` column is optional for
  hand-written files.

## Acceptance criteria

- A click inside the plot maps to the correct bar.
- A click outside the plot does nothing.
- A save writes one row with the derived UTC timestamp.
- An update replaces the same chart/bar/action/side row.
- A delete removes that row.
- The page serves 8 marker classes and the existing PNG.
- The server exposes only local files.
- Chart manifest geometry is present for every chart.
- Chart tests pass; repository checks pass.

## Verification

| Command | Result |
|---|---|
| `python3 -m unittest discover -s tools/charts -p 'test_*.py'` | 17 pass |
| Local server smoke: `/api/manifest`, POST `/api/annotations`, GET `/api/annotations` | pass |
| Smoke CSV row | chart, bar, timestamp, action, quality, side, note |
| Chart manifest geometry | 73 Binance and 30 Dukascopy charts |
| `python3 tools/check_changelog.py` | pass |
| `python3 tools/check_links.py` | pass |

## Risks

- The server is local-only and has no authentication; do not expose it on a
  public interface.
- The click is mapped by x only; the user can click anywhere in the candle's
  vertical column.
- The annotation quality is explicit; it is not inferred from the marker.

## Outcome

The click page is implemented and smoke-tested. The trailing-stop hypothesis is
not implemented; it remains a separate Strategy Research question.
