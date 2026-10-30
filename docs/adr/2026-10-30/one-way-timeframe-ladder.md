# One-Way Timeframe Ladder and Trade Intent Boundary

- Date: 2026-10-30 (UTC)
- Status: Accepted
- Issues: [#228](https://github.com/quant-rule/quant-rule/issues/228), sub-issues [#229](https://github.com/quant-rule/quant-rule/issues/229)–[#237](https://github.com/quant-rule/quant-rule/issues/237)
- Plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Related: [`docs/model/2026-10-30/per-timeframe-model.md`](../../model/2026-10-30/per-timeframe-model.md), [`docs/config/2026-10-30/strategy-v3.md`](../../config/2026-10-30/strategy-v3.md), [`docs/config/2026-10-30/trade-intent.md`](../../config/2026-10-30/trade-intent.md), [`docs/adr/2026-10-30/stateless-strategy-position-parameter.md`](stateless-strategy-position-parameter.md)

## Context

The current system has a fixed `1m/5m/15m/1h` data ladder, a model that
publishes per-timeframe axes and `CrossTimeframeAggregates`, and a strategy that
reads one timeframe's Direction, Momentum, and Structure values. Execution owns
the ATR stop/target multiples, and the strategy decision is a thin side-and-bar
record.

That arrangement cannot express the intended trader hierarchy:

- Higher timeframe regime.
- Lower timeframe setup and trigger.
- Execution always on the fastest active timeframe.
- Per-axis, per-timeframe context weights.
- A complete trade intent from the strategy.
- Fill mechanics isolated in execution.

The audit closure on 2026-10-30 verified AUD-01 through AUD-11, including the
stateless strategy fix for AUD-02. This decision builds on that foundation; it
does not reopen the audit findings.

## Decision

QuantRule adopts a one-way timeframe ladder and a trade-intent boundary.

### 1. Available vs active ladder

- `data.json` owns the **available** timeframes the data engine can build.
- `strategy.json` owns the **active** ladder.
- The active ladder is ascending.
- `ladder[0]` is the execution timeframe.
- `ladder[1..]` are context timeframes.
- The active ladder must be a subset of the available ladder and the policy
  universe.
- `execution.fill_timeframe` must equal `ladder[0]`.

### 2. Model boundary

The model computes each timeframe independently and publishes per-timeframe
measurements only:

- Readings.
- Direction, Momentum, Structure, and Progression.
- Events.
- Per-timeframe context diagnostics.
- Per-timeframe cross-axis consensus counts.

The model does **not** compute the active-ladder decision context and does not
publish decision-oriented cross-timeframe aggregates. Reporting and analytics may
compute descriptive aggregates over the available ladder, but those aggregates
are not strategy inputs.

### 3. Strategy context

The strategy layer builds an as-of `LadderContext` at each execution bar:

- Only higher-timeframe bars closed at or before the execution bar's close.
- Per-axis, per-context-timeframe weights.
- Continuous axes use weighted means.
- Structure and Progression use weighted shares and histograms; ordinal codes
  are never averaged.
- Missing context is explicit and rejects by default.
- Context bar indices, open times, and close times are recorded for audit.

The weights are strategy-owned, fixed before the run, and default equal. They
express importance, not calibration.

### 4. Trade intent boundary

The strategy emits a `TradeIntent`:

- Side.
- Entry style.
- Stop specification.
- Target specification.
- Validity.
- Optional maximum slippage.

Execution resolves the specifications, applies fill mechanics, and returns
fills/rejections. Execution never decides direction or risk distance.

The strategy remains stateless with respect to position. The runner supplies the
actual `PositionState`; a rejected entry cannot create a virtual position. This
preserves the AUD-02 guarantee.

### 5. Legacy removal

The separate `axis-agreement` strategy is removed from the public config and
command surface. Its agreement condition is integrated into the unified
`ladder-context` rule. A historical golden fixture and this ADR preserve the old
behavior as evidence. Old `quantrule.strategy/v2` configs are rejected with a
migration message.

## Alternatives considered

1. **Keep cross-timeframe aggregation in the model.**
   Rejected: the model would need to know the active ladder, coupling it to the
   strategy. The context would remain flat and not as-of a lower-timeframe bar.

2. **Put the weights in the model.**
   Rejected: every strategy would be forced to use the same weights. Weights are
   a decision preference, not a measurement.

3. **Let the strategy read the entire model output.**
   Rejected: percentiles, readiness, events, and diagnostics would leak into the
   signal path; double counting and overfitting would follow.

4. **Keep `axis-agreement` as a second public strategy.**
   Rejected: one unified system is the goal. The agreement condition remains, but
   it is integrated into the ladder rule.

5. **Convert Structure and Progression to `0..9`.**
   Rejected: the native scales carry different meanings. Ordinal codes from
   different scales are counted, never combined or averaged.

6. **Keep stop/target multiples in execution config.**
   Rejected: risk intent belongs to the strategy. Execution should resolve a
   specification, not own the risk policy.

7. **Add a new shared context crate now.**
   Rejected for the first delivery: the strategy context builder lives in
   `quantrule-backtest`, and reporting/analytics compute descriptive aggregates
   independently. If duplication becomes material, a shared crate is a separate
   decision.

## Consequences

- `data.json`, `model.json`, `strategy.json`, and `execution.json` receive new
  schemas.
- The model contract loses decision-oriented cross-timeframe aggregation.
- Reports, the dashboard, and research move descriptive aggregation to
  analytics/reporting.
- The strategy input expands from one timeframe to an execution snapshot plus
  `LadderContext`.
- The execution engine loses stop/target multiples and resolves `TradeIntent`.
- The old strategy is removed, with an archived fixture and migration error.
- The change is breaking. There are no legacy users.
- The context builder adds per-execution-bar work; it must be benchmarked.
- Per-timeframe indicator settings increase the configuration surface and must
  be validated and pre-registered.

## Follow-up

The implementation is split into sub-issues #229–#237. This ADR is complete at
Phase 1; code behavior changes begin in Phase 2.
