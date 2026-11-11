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

- The current shell run is deterministic; `cargo test` passes 1,443 tests.
- `v0.8.0` freezes the pre-Tauri surface.
- Phase 3 will provide the installer and the end-to-end demo.

## Limit

The evidence is single-user and historical. It does not prove adoption, team
collaboration, or future profitability.

## Decision

Proceed to the local Tauri increment; do not build the team cloud until the solo
workflow is validated.
