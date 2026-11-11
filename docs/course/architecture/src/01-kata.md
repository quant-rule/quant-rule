# QuantRule — The Local Research Workstation

## Description

A local-first desktop application that turns an OHLCV hypothesis into
reproducible, portfolio-constrained evidence. The researcher's data, configs,
runs, analytics, and research memory stay on the machine. A static download page
distributes Linux installers. The current solo program is the adoption product;
a collaborative team cloud is future work.

## Users

The primary user is a solo systematic researcher who owns or can download their
own OHLCV data, works on one Linux machine, and runs batch backtests. A secondary
user is the course evaluator, who must install the app from the download page and
reproduce a run with sample data. The current increment is single-user.

## Requirements

1. Import or download OHLCV data locally, with a data version and checksum.
2. Configure model, strategy, execution, and portfolio constraints.
3. Run each pipeline stage or one coupled backtest.
4. Produce deterministic evidence and a local research memory with analytics.
5. Export/import the research memory and back up the workspace.
6. Ship Linux installers through a static download page with checksums.
7. Expose the same capabilities through a headless CLI.
8. Require no account and send no candle data over the network.

## Additional context

- Single-user, single-machine, offline-capable.
- No team collaboration, cloud sync, cloud backtesting, live trading, prop-firm
  rules, or signal marketplace.
- The download page is a distribution channel, not a runtime service.
- The team cloud is a possible future increment, not a requirement of this kata.
- A negative result is a valid outcome.
