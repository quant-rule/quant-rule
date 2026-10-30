# Retirement of the Axis-Agreement Strategy

- Date: 2026-10-30 (UTC)
- Status: Accepted; superseded by the unified `ladder-context` rule
- Issues: [#228](https://github.com/quant-rule/quant-rule/issues/228), [#235](https://github.com/quant-rule/quant-rule/issues/235)
- Plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Current rule: [`docs/config/2026-10-30/strategy-v3.md`](../../config/2026-10-30/strategy-v3.md)
- Archived fixture: [`axis-agreement-golden.csv`](axis-agreement-golden.csv)

## Context

The first strategy was `axis-agreement`. It read one timeframe's Direction,
Momentum force, and Structure score and entered long when all three were
strictly above a threshold, short when all three were strictly below it, and
neutral otherwise. It exited when the agreement was lost. It was stateless with
respect to position and emitted the same `TradeIntent` boundary introduced in
Phase 6.

The unified ladder rule replaces it. The agreement condition remains, but it is
integrated into a rule that also reads the active ladder, the as-of
higher-timeframe context, the Structure state shift, and the configured exit
reasons.

## Decision

Remove `axis-agreement` from the live config and command surface.

- `quantrule.strategy/v2` is rejected with a migration message.
- `strategy-run` accepts `ladder-context` only.
- The old strategy source, its tests, and its command name are removed.
- The old golden fixture is archived in this directory; it is evidence, not a
  live test.
- The old rule's behavior is documented here and in the archived fixture.

## Archived fixture

`axis-agreement-golden.csv` has columns:

```text
bar,timestamp,timeframe_secs,direction,momentum_force,structure_score,expected_decision,expected_side,expected_reason,expected_position
```

An empty reading is unavailable; a measured zero is written as `0.0`. The
fixture records the decisions and the virtual position after each bar. It is
kept as historical evidence and is not executed by the test suite.

## Alternatives considered

1. **Keep `axis-agreement` as a second public rule.** Rejected: one unified
   system is the goal, and two public rules would duplicate the decision path.
2. **Keep the old fixture as a live test against a compatibility shim.**
   Rejected: a shim would keep the removed rule alive and add a second strategy
   path.
3. **Delete the old fixture entirely.** Rejected: the behavior change should
   remain reproducible as evidence.
4. **Migrate v2 configs automatically.** Rejected: the old flat fields do not
   express an active ladder, context weights, or the new exit reasons. A silent
   migration would invent a strategy.

## Consequences

- The old command name and config schema are gone. Users see a migration error.
- The old golden fixture no longer runs, so it cannot catch a regression in a
  rule that no longer exists. The unified rule has its own golden, causality,
  determinism, and edge-case tests.
- Historical reports and releases still describe the old rule; they are dated
  records, not current contracts.
