# ADR: Abstraction Levels and Engine Boundaries

- Date: 2026-10-30
- Status: Accepted
- Work item: `audit-improvement-workflow`

## Context

QuantRule needs a human-owned policy and strategy layer above the engine
contracts. Without it, account limits, strategy intent, model measurement,
execution mechanics, portfolio accounting, and analytics measurement blur
together, and a change to one is easily mistaken for a change to another.

## Decision

The system uses five explicit levels.

| Level | Owner | Artifact | Responsibility |
|---|---|---|---|
| Policy | Human | `policy.json` | Account, objective, risk envelope, tickets, timeframes |
| Strategy | Human | `strategy.json` (with the human thesis) | Thesis, entry, exit, risk/reward |
| Engine contracts | Engineering/Quant | `model.json`, `execution.json`, `portfolio.json`, `analytics.json` | Measurement, fill mechanics, accounting, evaluation |
| Implementation and runs | Engineering | Rust code, labs, reports | Deterministic execution and evidence |
| Improvement | Governing/CI | Audits, experiments, plans | Decide what to change, verify closure |

The boundaries are one-way:

```text
policy → strategy → model → execution → portfolio → analytics
```

- The policy constrains; it never emits a signal.
- The strategy decides; it never computes an indicator, a fill, or P&L.
- The model measures; it never reads strategy, execution, portfolio, or policy.
- Execution fills; it never decides direction.
- Portfolio accounts; it never decides a signal.
- Analytics evaluates; it never feeds a rule.
- Audits, experiments, and plans decide what to change; they never implement.

## Alternatives considered

1. **Keep only the seven run files.** Rejected: no human-owned account/objective
   envelope, and strategy risk/reward stays mixed with execution mechanics.
2. **Put policy inside `portfolio.json`.** Rejected: portfolio is an accounting
   contract, not a human goal; it would make the human edit engine internals.
3. **Put policy and strategy in code.** Rejected: the product’s public interface
   is the shell and its config files; a human must be able to inspect and change
   the blueprint without editing Rust.

## Consequences

- `run.json` becomes `quantrule.run/v2` and names `policy.json`.
- `strategy.json` becomes `quantrule.strategy/v2` and owns risk/reward.
- `execution.json` becomes `quantrule.execution/v2` and owns mechanics/costs.
- The policy is validated against the strategy and engine configs before a
  candle is read.
- A human can change the account/objective envelope without touching engine
  internals, and can read the strategy in trader words.
