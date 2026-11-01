# Improvement Cycle 006 — 5m/15m Sample-Size Test

- Date: 2026-11-01 (UTC)
- Status: Stopped at the strategy gate; no strategy or portfolio baseline
- Strategy comparison: [`STRATEGY-002`](STRATEGY-002-comparison.md)
- Prior cycle: [`CYCLE-005`](CYCLE-005.md)
- Claim type: strategy evidence; no edge claim

## Question

Does a 5m/15m v6 breakout produce a larger sample than the 15m/1h v6 and a
cost-inclusive 2024 edge?

## Result

No. v6 5m/15m produced 5 BTCUSD trades, 3 ETHUSD, and 0 XAUUSD. v3 controls lost
about 20% and halted on every instrument. The only positive cell is BTCUSD v6
+1.76% with five trades.

## Review

The execution timeframe is not the binding filter; the 15m level context and
breakout confirmation are. Faster execution does not create a larger sample.

## Decision

No strategy baseline is accepted. The sample-size hypothesis is rejected.

## New Hypothesis

A lower-timeframe level context (5m levels with 5m/15m execution) or a less
strict breakout confirmation is a new pre-registration. It is not implemented
here.

## Limitations

- One OOS year; 2023 excluded.
- Same-session review, not an independent Quant Validator record.
