# AI Transparency Log — Annotation Zone Analysis

- Date: 2026-11-10 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash
- Work item: annotation zone analysis
- Report: [`annotation-zone-analysis.md`](../../reports/2026-11-10/annotation-zone-analysis.md)
- Claim type: descriptive evaluation; no strategy or edge claim.

## Intent and boundary

Use the 2018–2019 human marks as zones rather than single candles and compare
the model state around entries and exits using the new `M1.2` readings. This is
evaluation only. No model event, strategy rule, execution mechanic, or portfolio
result was changed.

## Implemented

- `tools/trades/annotation_zones.py`: builds a symmetric zone around each mark
  from the 250-bar context and post window, and publishes `zones.csv`,
  `summary.json`, and `summary.md`.
- Zone features: EMA taps (fast/mid/slow), Fibonacci tests, band tests,
  pivot-sequence counts and 2+ rates, RSI, ADX, VWAP distance, POC/value-area
  distance, EMA distances, and 4/16/50-bar forward returns plus 50-bar MFE/MAE.
- Radius 3 primary run and radius 5 sensitivity run.
- `tools/trades/test_annotation_zones.py`: zone window, EMA tap, Fibonacci/band
  tests, side-adjusted forward returns, annotation row, and summary.
- Report `docs/reports/2026-11-10/annotation-zone-analysis.md`.

## Evidence

- 2018 contexts: 595; 2019 contexts: 1,025; 1,620 zone rows.
- Radius 3 EMA tap: long entry 91.2%, short entry 87.9%; long exit 71.9%,
  short exit 68.0%.
- Radius 3 Fibonacci test: long entry 85.7%, short entry 85.7%; long exit
  59.8%, short exit 56.5%.
- Radius 3 band test: 12.9–16.8% across groups.
- Pivot 2+ lower-high: 27.7% long entry, 28.0% short entry; radius 5 raises it
  to 30.1% and 34.2%.
- `python3 -m unittest discover -s tools/trades -p 'test_*.py'` — pass.
- Repository checks — pass.

## Learning

- The user's EMA-tap observation is strongly supported, but taps are also common
  at exits; a tap is a zone property, not a sufficient trigger.
- Fibonacci proximity is more entry-specific than EMA taps in this set.
- Horizontal bands are rarely tested at the marks; they look more like an
  extension/target reference than an entry trigger.
- Multi-pivot sequences are a meaningful minority, not the default. The rate
  depends on zone width.
- 2019 is weaker than 2018 but directionally consistent.

## Not claimed

- No causal signal, no strategy change, no profitability or edge claim.
- No validation partition, no 2024 access.
- The marks are hindsight labels; the zone includes bars after the mark.

## Correction — 2026-11-10

The close-based zone features originally labelled "Fibonacci test" and "Band
test" are renamed `fib_close_proximity` and `band_close_inside`. They are not
the model events `level.fib_test` / `level.band_test`: the first is
`fib_nearest_atr <= fib_atr`, and the second is a close inside the selected
band. The reported rates are unchanged; the report now states the definitions.
