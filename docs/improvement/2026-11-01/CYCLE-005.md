# Improvement Cycle 005 — First Strategy Comparison

- Date: 2026-11-01 (UTC)
- Status: Stopped at the strategy gate; no strategy or portfolio baseline
- Model baseline: `MODEL-2026-11-01-01..04` (correctness baseline, C-axis limitation)
- Strategy comparison: [`STRATEGY-001`](STRATEGY-001-comparison.md)
- Prior cycle: [`CYCLE-004`](CYCLE-004.md)
- Claim type: strategy evidence; no edge claim

## Question

With the model correctness baseline accepted, does any candidate produce a
cost-inclusive 2024 out-of-sample edge?

## Result

No. v3 BTCUSD −9.06%, v4 BTCUSD −11.11%, v5 BTCUSD −4.50%, v6 BTCUSD +2.62%
with only 5 trades, v6 ETHUSD −2.53% with 3 trades, v6 XAUUSD −1.19% with 1
trade. None meets the 30-trade floor or the return/stability criteria.

## Review

The only positive cell is BTCUSD v6, one instrument with five trades. Selecting
it would be post-hoc. The strategy gate is red.

## Decision

No strategy baseline is accepted. The next hypothesis is a new rule or a new
sample, not a threshold change.

## New Hypothesis

The v6 breakout is too selective on 15m/1h. A lower-timeframe or higher-frequency
v6 trigger is a candidate, but it is a new pre-registration. Alternatively, the
rule family changes to a level/retest or event-driven rule on a denser sample.

No option is implemented in this cycle.
