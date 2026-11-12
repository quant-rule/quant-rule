# QuantRule - Trading Edge Research

### Description

Build a research system that discovers and evaluates trading edges in historical
OHLCV market data. It separates a repeatable statistical advantage from
randomness and overfitting.

### Users

- **Quantitative researcher** - defines hypotheses, explores data, marks
  candidate entries and exits, and runs experiments.
- **Research reviewer** - independently reproduces a result and challenges it
  before the researcher accepts it.

### Requirements

- Define and record a market hypothesis before testing it.
- Explore historical candles and mark candidate entries and exits on a chart.
- Evaluate a hypothesis as a deterministic rule, with explicit costs and no
  look-ahead.
- Measure expected return, hit rate, drawdown, and trade count over a large
  enough sample.
- Validate on out-of-sample data and reject overfitted results.
- Compare experiments and reproduce any previous result from its recorded
  inputs.

### Additional context

- A research tool, not an execution platform: no live trading, no order routing.
- The goal is to establish whether an apparent edge is statistically meaningful,
  robust, and worth further investigation.
- Results are evidence to be defended, not a profit claim.
