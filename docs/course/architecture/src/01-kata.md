# QuantRule - Finding a Trading Edge

## Users

**One quantitative researcher - the only user.** A systematic trader working
alone, on their own OHLCV candles, on one machine or a small internal server.

- **What they are looking for:** an *edge* - a measurable advantage that makes a
  strategy produce positive expected value over many trades. A 45% win rate with
  +3R winners and -1R losers is an edge: `(0.45 x 3) - (0.55 x 1) = +0.8R` per
  trade before costs. A high win rate with a poor payoff is not.
- **How they work:** short interactive sessions - marking a chart, inspecting a
  run - between long unattended runs that continue after the client closes. Many
  hypotheses, few survivors.
- **What they need:** to kill a hypothesis cheaply and defend a survivor months
  later; a negative result must be as trustworthy as a positive one.
- **Why one user:** accounts, sharing, and tenancy would add cost without
  improving the research.

**Reader of the evidence - not a user.** A reviewer receives a bundle and a
digest, and reproduces the run offline, without an account.

Pattern: many hypotheses, few survivors.

## Requirements

- **R1.** Ingest local M1 OHLCV candles; identify each dataset by a content hash.
- **R2.** Build a multi-timeframe ladder; publish readings, states, events, and
  progression as numbers, never labels.
- **R3.** Evaluate one human-authored rule through strategy, execution, and
  portfolio, with explicit costs and no look-ahead; report expectancy and the
  inputs that produced it.
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

- Single operator, small deployment: one machine or one internal server.
- The goal is an edge. "No edge" is the most common outcome and a valid result.
  A backtest is a filter, not proof: out-of-sample data, costs, and a large
  enough sample decide.
- Determinism is the trust claim: the same inputs produce the same bytes. Runs
  are batch and CPU-bound: minutes to hours, not milliseconds.
