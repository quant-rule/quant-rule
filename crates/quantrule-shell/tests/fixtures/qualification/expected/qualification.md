# Qualification — Q-FIXTURE-PASS

- Verdict: pass
- Criteria: 4 total, 3 pass, 1 fail, 0 insufficient_evidence
- Human decision: pending; the engine never accepts or rejects.

## Evidence

- Data version: test:v1
- Config identity: 9d7a8bf7d8644fe6
- Code revision: unknown
- Period: 2025-01-01..2025-01-01
- Partitions: portfolio_holdout
- Costs: fee_rate=0 slippage_bps=2 commission_rate=0.001 funding_rate=0.0001 initial_cash=10000 leverage=1
- Seed: null
- Trades: 1
- Multiple testing: 2 primary cells, 2 trials, holm correction, family alpha 0.05, adjusted alpha 0.025

## Criteria

| ID | Dimension | Mandatory | Status | Observed | Operator | Threshold | Evidence artifact | Evidence field | Reason |
|---|---|---|---|---|---|---|---|---|---|
| trade-count | trade_count | true | pass | 1 | >= | 1 | portfolio | metrics.trades | operator_satisfied |
| drawdown | drawdown | true | pass | 0.02696674483544992 | <= | 0.2 | portfolio | metrics.max_drawdown | operator_satisfied |
| zero-win-rate | consistency | true | pass | 0 | == | 0 | portfolio | metrics.win_rate | operator_satisfied |
| return-sign | out_of_sample | false | fail | -0.02461275898231363 | > | 0 | portfolio | metrics.total_return | operator_not_satisfied |

## Reasons

- `return-sign`: `operator_not_satisfied`

## Not applicable

- `walk_forward`: the fixture declares no sequential folds

## Human acceptance

The engine writes `human_decision: null`. Review this record, the evidence bundle, and the residual risks, then record the decision in a separate `decision.md`. A `pass` may still be rejected for residual risk, and a `fail` is a valid research result.
