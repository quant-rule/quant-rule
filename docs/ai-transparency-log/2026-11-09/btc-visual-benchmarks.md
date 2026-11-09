# AI Transparency Log — BTC Visual Review, Cross-Venue Data, and Benchmarks

- Date: 2026-11-09 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash
- Work item: [`btc-visual-benchmarks`](../../execution-plans/2026-11-09/btc-visual-benchmarks.md)
- Intent: add a dense BTC spot source, render trade charts, wire evaluation
  benchmarks, and clean the local workspace.
- Claim type: data, tooling, and evaluation; no edge claim.

## Proposed and implemented

- Implemented a Binance BTCUSDT monthly-archive importer with checksum
  verification, timestamp normalization, atomic output, and tests.
- Imported 2018–2025: 4,199,615 rows, 96 monthly files, SHA-256
  `46af1a54a82a809cc4eb8e14ddda9dfbe04c0670ad143faaaad1c2bf7fb7c1b1`.
- Implemented a coverage and cross-venue audit; observed 99.2–100% yearly
  15m coverage and a 2023 median basis of +0.115% against Dukascopy.
- Implemented the deterministic 15m trade-chart tool and generated 30 Dukascopy
  control charts plus 73 Binance cross-venue charts.
- Wired the existing buy-and-hold and matched random-timing benchmark engine into
  `analytics evaluate`; added `benchmarks.csv`, JSON fields, help, README, and a
  benchmark contract.
- Ran the frozen break-only control on Binance 2018–2023. The result does not
  reproduce the Dukascopy lead.
- Archived large historical local evidence outside the working tree and left
  symlinks; removed build cache, scratch, and bytecode.

## Verification and evidence

- `python3 -m unittest discover -s tools/data -p 'test_*.py'` — pass.
- `python3 -m unittest discover -s tools/charts -p 'test_*.py'` — pass.
- `cargo test -p quantrule-shell --test analytics_command` — 9 pass, including
  byte determinism and the golden fixture.
- `cargo test` — see the plan verification.
- 2023 Binance: 8 trades, −$202.50, PF 0.229, buy-and-hold +155.54%, strategy
  percentile 3.2%.
- 2018–2022 Binance: 65 trades, −$389.64, PF 0.871, max drawdown 5.40%.
- Dukascopy 2018–2022: 30 trades, +$795.69, PF 1.747, max drawdown 2.87%.

## Learning and corrections

- Dukascopy's BTCUSD source is sparse: full 15m coverage is 64.5% in 2018 and
  74.0% in 2023. The dense Binance spot series changes the trade set and flips the
  control's sign. The earlier positive result is not venue-robust.
- The positive Dukascopy control was not promoted. No 2024 data was opened.
- A literal random-walk price generator was rejected for the first benchmark
  pass; the matched random-timing null is the appropriate timing test. Its
  limitations (no execution fees, funding, or halt) are recorded.
- The cleanup was done by archive-and-symlink, not deletion. Tracked historical
  configs and tools were retained because they are cited evidence and CI runs
  them.
- The chart labels were enlarged and the right-edge EMA labels were removed after
  the first render because the values overlapped. The final set is readable at
  2400×1600.

## Not claimed

- No strategy or portfolio baseline is accepted.
- No edge is claimed from the Dukascopy control or the Binance check.
- The Binance series is USDT-quoted and is not spliced into BTCUSD.
- The benchmark percentile is descriptive, not a p-value.
