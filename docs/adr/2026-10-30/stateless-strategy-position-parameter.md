# Stateless strategy and the position parameter

- Date: 2026-10-30 (UTC)
- Status: Accepted
- Context: AUD-2026-10-30-02, architecture-hardening Phase 3c
- Related: `docs/execution-plans/2026-10-30/architecture-hardening.md`

## Decision

The strategy is stateless. The runner supplies the position execution currently
holds as an explicit `Option<Side>` argument to `Strategy::decide`. The position
is the strategy's own state, not a market reading: it carries no cash, equity,
P&L, indicator, statistic, or future bar.

`StrategyInput` remains limited to the three primary axis values and bar identity.
The strategy may use the position argument to decide whether an entry or exit is
possible; it may not read execution or portfolio internals.

## Alternatives considered

1. Keep the strategy stateful and reconcile fills/rejections into it. Rejected:
   the reconciliation path is larger and leaves two position owners.
2. Make the strategy stateless but pass the whole execution/portfolio record.
   Rejected: it would expose cash, P&L, and accounting fields to the signal path.
3. Keep the strategy decision-only with no position argument. Rejected: the rule
   cannot avoid pyramiding or emit a valid exit without knowing its position.

## Consequences

- The executed walk's `positions` are execution positions; a rejected entry never
  becomes a strategy position.
- The diagnostic `strategy-run` and standalone `report strategy` use a virtual
  fill walk; the `backtest` strategy bundle uses the executed walk and marks
  `execution_state`.
- The strategy boundary test covers `StrategyInput`; the position argument is a
  separate explicit parameter with no accounting or market fields.
- A future strategy that needs more execution state requires a new decision, not
  an expansion of `StrategyInput`.
