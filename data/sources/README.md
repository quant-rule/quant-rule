# Sources

Raw provider candles. This is the only data that cannot be rebuilt from the
repository.

- Naming: `<SYMBOL>_<TF>_<start>_<end>.csv`, for example
  `SOLUSD_M1_2025-01-01_2025-12-31.csv`.
- Produced by: `quantrule fetch <symbol> <start> <end>`.
- Format: `Date,Open,High,Low,Close,Volume` with UTC timestamps; see the CSV
  contract in the repository README.
- Ladder commands (`timeframes`, `market-state`, `backtest`) require a
  one-minute source. Native `5m`, `15m`, and `1h` files are inspection
  references and are rejected as ladder inputs.
- Never edit, trim, or rename a source in place. Fetch a new range instead.
