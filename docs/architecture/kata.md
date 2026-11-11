# The Local Research Workstation Kata

- Status: Current; approved 2026-11-11
- Owner: Human
- Part of: `docs/architecture/README.md`

## About this kata

This is the product kata for the Tauri Platform. It is a one-page,
solution-agnostic description of the product, its users, and its measurable
requirements. The current product is for the solo researcher; a collaborative
team cloud is future work.

## Name

QuantRule — The Local Research Workstation.

## One line

A local-first desktop application that turns an OHLCV hypothesis into
reproducible, portfolio-constrained evidence — and keeps the candles, compute,
and research memory on the researcher's machine.

## Intent

Backtests are cheap; trustworthy backtests are not. QuantRule exists so that the
same candles and the same configuration produce the same evidence, every time,
and so that a negative result is a legitimate outcome. The product is for
adoption by the solo researcher. A team cloud is a future increment, not a
current requirement.

## User

The primary user is a solo systematic researcher who owns or can download their
own OHLCV data, works on one Linux machine, and runs batch backtests. A secondary
user is the course evaluator, who must install the app and reproduce a run with
sample data. The current increment is single-user, single-machine, and local.

## Job to be done

Given local M1 OHLCV candles, a data version, a model, a strategy, and portfolio
constraints, run the pipeline in one command and give me a lab I can reproduce,
inspect, and defend — so I can accept the hypothesis, revise it through a
pre-registered experiment, or stop with "no edge."

## Requirements

1. Import or download OHLCV data locally, with a data version and checksum.
2. Measure each timeframe with readings, states, confirmed levels, events,
   progression, and context; publish numbers, nulls, and codes — never labels or
   a composite score.
3. Evaluate the model, strategy, execution, and portfolio constraints through a
   deterministic backtest with explicit costs and no look-ahead.
4. Produce a deterministic evidence bundle with provenance and a local research
   memory that can be exported, imported, and backed up.
5. Expose the same capabilities through a headless CLI and a Svelte desktop UI.
6. Ship Linux installers through a static download page with checksums.
7. Require no account and send no candle data over the network.

## Constraints and non-goals

- Candles only — no ticks, order books, machine learning, or statistical signals.
- Single-user, single-machine, offline-capable. No accounts, team cloud, sync,
  subscription, cloud compute, or cloud data storage.
- No prop-firm account rules, profit targets, evaluation windows, or funded-account
  simulation. Portfolio capital, sizing, costs, drawdown, leverage, and position
  limits remain.
- A backtest is not a profitability claim; out-of-sample evidence is required
  before a result is called promising.
- The download page is distribution, not a cloud runtime.

## Done when

- One configured run over a local M1 file produces the evidence bundle.
- A rerun over the same candles, data version, and configuration is byte-identical.
- A bundle is refused when its target is not empty.
- The desktop app installs on Linux and runs the core workflow offline.
- A validation record can be assembled from the bundle and its provenance.
- Every command, argument, and output shape is discoverable from help.
- Every reading, event, and rule has causal, deterministic, fixed-expected-value,
  and edge-case evidence.
- A negative result can be recorded and the improvement workflow can stop.
