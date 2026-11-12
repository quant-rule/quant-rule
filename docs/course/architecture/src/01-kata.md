# QuantRule — The Distributed Research Platform Kata

## About this kata

A one-page, solution-agnostic description of the product, its operator, and its
measurable requirements. It is the course submission copy of
`docs/architecture/kata.md`; where the two differ, the architecture copy governs.

## Name

QuantRule — the reproducible evidence platform.

## One line

A distributed research platform that turns an OHLCV hypothesis into reproducible,
portfolio-constrained evidence, and keeps that evidence auditable after the
client that submitted it has closed.

## Intent

Backtests are cheap; trustworthy backtests are not. QuantRule exists so that the
same candles and the same configuration produce the same evidence, every time,
and so that a negative result is a legitimate outcome. The product is for one
operator on an internal network. It is not a public service and it is not a
prediction engine.

## Operator

A systematic researcher who owns their candles, submits batch runs, and needs the
result to survive their laptop closing. A secondary reader is a reviewer who must
reproduce a result from an image digest and a canonical digest, without an
account.

The product is single-principal. It is not multi-user and does not pretend to be.

## Job to be done

Given candle data, a data version, a model, a strategy, and portfolio
constraints, submit a run and walk away. Come back to a bundle I can reproduce,
inspect, and defend: an audit trail, a canonical digest a reviewer can compare,
and a recorded decision to accept the hypothesis, revise it, or stop at "no edge".

## Requirements

1. Accept candles as an upload, identify them by content, and keep the bytes.
2. Measure each timeframe with readings, states, confirmed levels, events,
   progression, and context; publish numbers, nulls, and codes — never labels.
3. Evaluate model, strategy, execution, and portfolio constraints in one
   deterministic run with explicit costs and no look-ahead.
4. Return `202` on submission and continue after the client disconnects.
5. Produce a bundle with provenance naming the data version, config hash, model
   version, and image digest, plus a canonical digest over the data artifacts.
6. Expose the same capabilities through a browser client, a command shell, and a
   desktop shell that reads no local data.
7. Start with one command on a machine with Docker; expose one port.
8. Require no account, and keep candle data on the operator's network.

## Constraints and non-goals

- Candles only — no ticks, order books, machine learning, or statistical signals.
- One principal. No accounts, no tenancy, no billing, no cloud provider. Profiles
  organise research; they do not isolate it.
- No high availability, no failover, no uptime target.
- No public distribution page. Deployment is an image and one command; release
  artifacts live on GitHub Releases.
- A backtest is not a profitability claim. Out-of-sample evidence is required.

## Done when

- One command starts the stack and prints a URL.
- A submitted run survives the client closing and completes.
- A rerun over the same inputs produces the same canonical digest.
- A bundle is refused when its artifact set is incomplete.
- Provenance names the image digest that produced the run.
- Every command, argument, and output shape is discoverable from help.
- A second machine following the verification tutorial prints the same digest.
- A simulation run completes the research loop, premise to decision, in the
  browser.
- A negative result can be recorded and the workflow can stop.
