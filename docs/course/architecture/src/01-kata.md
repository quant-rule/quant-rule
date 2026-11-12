# QuantRule

A trader who researches alone wants a workstation that turns an OHLCV hypothesis
into reproducible evidence, so that a trading edge can be found and defended.

**Users:** one quantitative researcher, working alone on their own candle data.

**Requirements:**

- ingest local M1 OHLCV candles; identify each dataset by content hash
- measure each timeframe; publish readings, states, events, and progression as
  numbers
- evaluate one human-authored rule through strategy, execution, and portfolio,
  with costs and no look-ahead, and report expectancy with its inputs
- run a backtest as a job that survives the client closing
- produce a reproducible evidence bundle with provenance and a comparable digest
- expose the same capabilities through a browser client and a command shell
- mark good and bad entries on a chart, compare them with a deterministic oracle,
  and check the implied rule on a later, unmarked window
- keep candle data local and require no account

**Additional Context:**

- An edge is a measurable advantage that produces positive expected value over
  many trades: a 45% win rate with +3R winners and -1R losers is +0.8R per trade
  before costs; a high win rate with a poor payoff is not.
- Most hypotheses fail. A negative result must be as trustworthy as a positive
  one, so the evidence is the product, not the verdict.
- A backtest is a filter, not proof: out-of-sample data, costs, and sample size
  decide.
- Runs are batch and CPU-bound: minutes to hours on one machine or one internal
  server.
