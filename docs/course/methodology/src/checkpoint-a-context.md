# Checkpoint A — Context and First Behaviour

## Claim

For a solo systematic researcher with local OHLCV data, QuantRule will turn one
rule-based hypothesis into a reproducible, portfolio-constrained evidence bundle,
measured by a byte-identical rerun and a complete provenance manifest, with the
limitation that the evidence is historical and does not establish future profit.

## User and context

- User: a solo researcher on Linux who owns the candles.
- Current friction: backtests are easy to run but hard to reproduce, compare, and
  defend.
- First increment: import data, configure a model/strategy/portfolio, run one
  backtest, and inspect the evidence.

## Scope fence

Included: local app, local data, local research memory, Linux distribution.

Excluded: accounts, team cloud, cloud compute, live trading, prop-firm rules,
Windows and macOS.

## Evidence

- The current run is deterministic; `cargo test` passes 1,476 tests.
- `v0.8.0` freezes the pre-Tauri surface; Phase 3 is verified at `a81ec0a`.
- The Linux `.deb` installs in Ubuntu 24.04 and launches under Xvfb; the AppImage
  launches with the network namespace removed.
- The Phase 3 report records the end-to-end evidence.

## Limit

The evidence is single-user and historical. The native UI flow is not
WebDriver-driven, and the clean install is containerized. It does not prove
adoption, team collaboration, or future profitability.

## Decision

Proceed to the local Tauri increment; do not build the team cloud until the solo
workflow is validated.
