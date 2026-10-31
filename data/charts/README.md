# Charts

PNG renders derived from a candle source.

- Produced by: `market-chart` and `fetch --png`.
- Naming: the source stem plus `_candles.png`, for example
  `XAUUSD_M1_2025-01-01_2025-12-31_candles.png`.
- Content is a presentation of candles and axis tracks; it is never evidence.
- Fully regenerable: delete any file here and render it again.
- Charts are deterministic for the same input and options, so two renders of
  the same source produce the same bytes.
