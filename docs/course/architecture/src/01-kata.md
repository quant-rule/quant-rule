# QuantRule - Reproducible Trading Evidence

## Users

- One quantitative researcher (the operator): interactive sessions between long
  batch runs.
- One reviewer per result, who must reproduce it without an account.

## Requirements

- **R1.** Ingest local M1 OHLCV candles; identify each dataset by a content hash.
- **R2.** Build a multi-timeframe ladder; publish readings, states, events, and
  progression as numbers, never labels.
- **R3.** Evaluate one human-authored rule through strategy, execution, and
  portfolio, with explicit costs and no look-ahead.
- **R4.** Run a backtest as a job that survives the client closing and is
  readable from another machine.
- **R5.** Produce a reproducible bundle with provenance and a comparable digest.
- **R6.** Expose the same capabilities through a browser client and a command
  shell.
- **R7.** Support chart-first exploration: a human marks good and bad entries on
  a window, a deterministic oracle marks the same window, and the check runs on
  a later, unmarked window.
- **R8.** Keep candle data on the operator's machine and require no account.

## Additional Context

- Single operator, one machine or a small internal server. No cloud runtime,
  no accounts.
- The goal is a durable, cost-aware edge on OHLCV candles. "No edge" is a valid
  result.
- The course requires a distributed deployment: services over a network.
- Determinism is the trust claim: the same inputs produce the same bytes.
