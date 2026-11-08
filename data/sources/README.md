# Sources

Raw provider candles. This is the only data that cannot be rebuilt from the
repository.

- Naming: `<SYMBOL>_<TF>_<start>_<end>.csv`, for example
  `BTCUSD_M1_2024-01-01_2024-12-31.csv`.
- Produced by: `quantrule fetch <symbol> <start> <end>`.
- Format: `Date,Open,High,Low,Close,Volume` with UTC timestamps; see the CSV
  contract in the repository README.
- Ladder commands (`timeframes`, `market-state`, `backtest`) require a
  one-minute source. Native `5m`, `15m`, and `1h` files are inspection
  references and are rejected as ladder inputs.
- Never edit, trim, or rename a source in place. Fetch a new range instead. A
  new range may be sliced from an existing source with the same provider data
  when the network is unavailable; keep the new file and its data version
  explicit.
- The current BTCUSD 2024 example is a slice of
  `BTCUSD_M1_2018-01-01_2024-12-31.csv`.
- `COPPERCMDUSD` and `TSLAUSUSD` are admitted for price-only model research;
  COPPER volume is zero on most rows and TSLA volume is unit-unstable in 2020,
  so volume, participation, and flow events must not use them. See
  [`DATA-2026-11-08-MR7-01`](../../docs/config/2026-11-08/data-change-MR7-01.md).
