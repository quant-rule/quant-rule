# The Trustworthy Backtest Kata

- Status: Current; approved 2026-10-31
- Owner: Human
- Part of: `docs/architecture/README.md`

## About this kata

This is a product kata in the architecture-kata tradition. Dave Thomas introduced
the code kata as deliberate practice. Ted Neward adapted the idea to architectural
katas, and Neal Ford and Mark Richards use the practice in their architecture
teaching. Here the kata is a one-page, solution-agnostic product definition used to
align the human and the agents before solutioning. It is not a small-group workshop
exercise, and it names no technology.

## Name

QuantRule — The Trustworthy Backtest Kata.

## One line

A local, deterministic shell that turns an OHLCV hypothesis into rule-based
evidence you can reproduce, audit, and accept — or reject.

## Intent

Backtests are cheap; trustworthy backtests are not. QuantRule exists so that the
same candles and the same configuration produce the same bytes, every time, and so
that a negative result is a legitimate outcome. The goal is not to predict the
market; it is to reach an honest verdict on a rule-based edge, including costs and
out-of-sample evidence.

## User

The human owner: a quantitative trader or researcher working locally, who owns the
policy (account, objective, risk, universe) and the strategy (thesis, entry, exit,
risk/reward), and who accepts the result. Validation and audit are hats the same
human wears; they consume the lab evidence and do not get a second surface. The
system is single-user and local.

## Job to be done

Given M1 OHLCV candles, a data version, a policy, and a strategy, run the whole
pipeline in one command and give me a lab I can reproduce byte-for-byte, inspect
engine by engine, and defend in a validation record — so I can accept the
hypothesis, revise it through a pre-registered experiment, or stop with "no edge."

## Requirements (thin end-to-end)

1. Ingest and validate interchangeable OHLCV candles; build the configured ladder
   from M1 with explicit gaps and partial buckets.
2. Measure each timeframe with four independent axes and three event types; publish
   numbers, nulls, and codes — never labels or a single composite score.
3. Evaluate the human-owned policy and strategy through execution and portfolio
   accounting, with explicit costs; no look-ahead, no statistics in the signal path.
4. Produce per-engine evidence bundles and a lab summary; refuse stale or
   mismatched labs; every artifact deterministic.
5. Expose every capability as a discoverable shell command; keep a local store and
   research export that can be verified.

## Constraints and non-goals

- Candles only — no ticks, order books, machine learning, or statistical signals.
- Single-user local tool — no server, GUI, embedded library, live trading, or
  multi-tenancy.
- Statistics is confined to evaluation; it never enters the signal path.
- A backtest is not a profitability claim; out-of-sample evidence is required before
  a result is called promising.
- Technology and architecture choices are out of scope here;
  `docs/architecture/characteristics.md` records them.

## Done when

- One run on a full-year M1 file produces the five evidence bundles and the root
  lab summary.
- A rerun over the same candles, data version, and configuration is byte-identical.
- A stale or mismatched lab is refused.
- A validation record can be assembled from the lab artifacts and their provenance.
- Every command, argument, and output shape is discoverable from help.
- Every axis and rule has causal, deterministic, fixed-expected-value, and
  edge-case evidence.
- A negative result can be recorded and the improvement workflow can stop.
