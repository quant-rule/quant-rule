# ADR: Flat Indicator/Event Model — Axes Removed from Decisions

- Date: 2026-11-03 (UTC)
- Status: Accepted
- Work item: [#343](https://github.com/quant-rule/quant-rule/issues/343)
- Epic: [#342](https://github.com/quant-rule/quant-rule/issues/342)
- Plan: [`docs/execution-plans/2026-11-03/342-model-v3.md`](../../execution-plans/2026-11-03/342-model-v3.md)
- Contract: [`docs/model/2026-11-03/model-v3.md`](../../model/2026-11-03/model-v3.md)
- Supersedes for axes: [`docs/adr/2026-10-21/four-axis-analytical-model.md`](../2026-10-21/four-axis-analytical-model.md)
- Supersedes for strategy input: [`docs/adr/2026-10-30/rule-purity-and-statistics-boundary.md`](../2026-10-30/rule-purity-and-statistics-boundary.md) (the statistics boundary is retained)

## Context

The four-axis model was the first analytical contract. It published Direction,
Momentum, Structure, and Progression as numeric axis values, with events beside
them. The strategy then used axis thresholds, context weights, and cross-axis
agreement to decide entries.

The evidence from the 2026-11-01 model and strategy runs does not support that
design:

- Direction hit rates were `0.470–0.514` across 30 cells; only one cell had a
  lower bound above `0.50`, and it did not replicate.
- Momentum was below `0.50` as a direction predictor. That test did not match
  momentum's actual claim, which is acceleration.
- Structure monotonicity ranged from `−1.0` to `+0.6` across instruments, and it
  was tested as a direction score rather than as swing behaviour.
- Progression coverage was about 19% on the 1h sample, and target selection could
  pick a level almost coincident with the origin.
- Four of 300 event buckets had a lower bound above `0.50`; all four were BTCUSD
  and none replicated. Level events were extremely dense.

The axes therefore added an aggregation layer that hid which indicator produced a
decision, forced weights that were not a trading thesis, and made attribution
hard. The strategy failures were downstream of a descriptive model.

## Decision

1. **The model publishes indicators and events, not axes.** Model v3 publishes
   per-timeframe readings, discrete states, confirmed levels, event records,
   progression legs, context, and coverage. It publishes no Direction, Momentum,
   Structure, or Progression numeric value, no axis consensus, and no axis
   weights.

2. **The four names survive only as a reporting taxonomy.** Trend, Momentum,
   Structure, and Progression are documentation and presentation labels. A
   reporting view may group indicators under them and count how many timeframes
   share a state. The strategy cannot read those views.

3. **The strategy composes events.** Strategy v8 names the event or state, its
   source, and its timeframe. It uses a bounded predicate list, not a general
   expression language. It has no axis threshold, no weight, and no cross-axis
   agreement. It supports one to many timeframes with explicit roles
   (`execution`, `context`, `regime`).

4. **Every indicator and event has a role and a claim.** The registry records
   whether a measurement is `signal`, `context`, or `diagnostic`, and states the
   falsifiable claim the validation harness must test. Only `signal` and
   `context` may be read by a strategy. A `signal` event needs an A/B grade
   before a strategy uses it; a C grade is allowed only as one human-approved
   hypothesis; D blocks.

5. **The model gate is hard.** No strategy v8 work starts until at least one
   signal-role event passes A/B on `model_dev` and `model_validation` with
   replication across instruments and timeframes. If nothing passes, the model
   is descriptive and the loop stops or changes thesis. It is not tuned.

6. **MACD is added.** The previous contract excluded MACD. Model v3 includes
   MACD 12/26/9 with line/signal cross, histogram slope flip, and zero-cross
   events. Its absence was a contract choice, not a defect; its addition is a
   deliberate contract change.

## Alternatives considered

1. **Keep the axes as model output but not as strategy input.** Rejected: the
   aggregation layer would remain, its values would still be tested and reported,
   and a later strategy could start reading them again. Attribution would stay
   ambiguous.
2. **Keep the axes as reporting only, with no numeric value.** Chosen: the
   familiar names remain useful for human reading, but no number and no
   strategy path depends on them.
3. **Keep the axes and add a meta-model over the events.** Rejected: it adds
   another opaque layer without evidence, and it makes the validation question
   harder to state.
4. **Keep the strategy's axis weights but let it also read events.** Rejected:
   that is double counting. The same feature would arrive as an axis and as an
   event.
5. **Remove all indicator groupings, including the reporting taxonomy.**
   Rejected: the names are a useful documentation index, and keeping them out of
   the model output is sufficient to prevent the old path from returning.

## Consequences

- The model config becomes `quantrule.model/v3`; the model output loses axes,
  percentiles, readiness, consensus, and age counters.
- Strategy v3–v7 are retired. Their configs are not accepted by v8, and their
  runs are invalidated by the contract change.
- The shell model report and charts are rewritten around readings, states,
  levels, events, and progression. The structure panel becomes a swing map
  rather than a `-1..+1` score line.
- The registry becomes mandatory: a behaviour-affecting number that is not in the
  registry cannot enter the model or the strategy.
- The validation harness grades claims, not axes. A no-edge result is a valid
  outcome and stops the strategy tier.
- The old axis contract and the per-timeframe delta are retained as history with
  a superseded status. The statistics boundary in the rule-purity ADR remains:
  no statistical value enters the signal path.

## Links

- Model v3 contract: [`docs/model/2026-11-03/model-v3.md`](../../model/2026-11-03/model-v3.md)
- Registry: [`docs/registry/README.md`](../../registry/README.md)
- Execution plan: [`docs/execution-plans/2026-11-03/342-model-v3.md`](../../execution-plans/2026-11-03/342-model-v3.md)
- Prior evidence: [`docs/improvement/2026-11-01/CYCLE-007.md`](../../improvement/2026-11-01/CYCLE-007.md)
