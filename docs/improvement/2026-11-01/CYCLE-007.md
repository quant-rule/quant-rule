# Improvement Cycle 007 — Level-Fade Rule

- Date: 2026-11-01 (UTC)
- Status: Stopped at the strategy gate; no strategy or portfolio baseline
- Strategy comparison: [`STRATEGY-003`](STRATEGY-003-comparison.md)
- Prior cycle: [`CYCLE-006`](CYCLE-006.md)
- Claim type: strategy evidence; no edge claim

## Question

Does the inverse rule family — fading a failed test of a confirmed level — have a
cost-inclusive 2024 edge?

## Result

No. BTCUSD −15.23%, ETHUSD −18.97%, XAUUSD −4.73%. BTCUSD and ETHUSD hit the
drawdown halt; win rates are 5.9–11.9%. The 2025 OOS was not run because the
pre-registered validation gate failed.

## Review

The new rule family fails on the validation sample. It is not a threshold
problem; the failed-level thesis itself is not supported on 2024.

## Decision

Stop at the strategy gate. No strategy baseline is accepted. Do not run the 2025
OOS or a portfolio baseline from this rule.

## New Hypothesis

None of the rule families tested — trend agreement, level retest, three-timeframe
reversal, breakout, failed-level fade — produces a green strategy. The next step
is not another variant. The options are a genuinely different market thesis, a
different dataset/regime, or stopping the loop.

No option is implemented in this cycle.
