# ADR: The Four-Axis Analytical Model

- Date: 2026-10-21 (UTC)
- Status: Proposed — accepted for implementation once the human confirms it
- Issue: [#14](https://github.com/quant-rule/quant-rule/issues/14) — feat: build the model engine
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Plan: [`docs/execution-plans/2026-10-21/14-model-engine.md`](../../execution-plans/2026-10-21/14-model-engine.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-21/14-model-engine.md`](../../ai-transparency-log/2026-10-21/14-model-engine.md)

## Context

The model engine had to be designed before any of it exists. `AGENTS.md` section 2 described it as a stage that "computes indicators, which are categorized on four axes", and issue #14 asked for an indicator abstraction carrying a name, an axis, required history, and the metrics it emits. That framing makes an indicator the owner of an axis, and it leaves the model's output as "the combined time series" of candles and indicator metrics — one series, one consumer story, no cross-timeframe view.

The specification that drove this decision replaces that framing. It describes market state along four axes, a shared three-type event model, and cross-timeframe aggregation in which each axis is aggregated by a method that suits it.

## Decision

The model engine is a four-axis analytical model with three internal stages and one shared event model.

1. **Indicator readings** are the model's calculation inputs: EMA 20/50/200, RSI 14, ATR 14, relative volume 20, and confirmed swing pivots at a minor and a major scale. A reading is not owned by an axis. It may feed several axes, and each use states its own definition.
2. **Four axes and event detection** produce the published numbers: Direction (continuous alignment score), Momentum (continuous force, acceleration, intensification), Structure (swing score and code, plus EMA dynamic-level features), and Progression (a named leg's unclamped progress and ordinal stage). Events are Level, Divergence, and Signal, encoded numerically, appended when they become knowable, and never upgraded in place.
3. **Cross-timeframe aggregation** combines the per-timeframe results over an ordered fastest-to-slowest ladder, with a method per axis: means and shares for Direction and Momentum, histograms and spans for Structure and Progression, and counts for events.

The model layer publishes numbers only. Words are a presentation mapping in the shell.

## Alternatives considered

| Alternative | Why it was rejected |
|---|---|
| **A single composite market score.** One number summarising the market, with a threshold on it. | The axes describe different things, and an average of them is not any of them. It also hides disagreement, which is information: a strong direction with fading momentum and a broken structure is a different state from a strong direction with strengthening momentum and an intact structure. The model publishes axis agreement as counts instead. |
| **Indicators own axes.** An indicator declares one axis, and axes are the union of their indicators' outputs. | It makes a shared reading impossible to use twice without duplicating it, which is exactly the double-counting the specification forbids. EMAs serve Direction as a spread and Structure as a dynamic level; the roles have different definitions, and both are legitimate. |
| **An event taxonomy per indicator.** Each indicator owns its own event kinds, so a dozen indicators produce a dozen event namespaces. | The event count grows with the indicator count, and a consumer cannot ask "how many divergences are there" without knowing which indicators define one. Three shared types — Level, Divergence, Signal — describe what happened, and a numeric `source_code` says which reading produced it. |
| **An event lifecycle.** Events move through pending, active, and expired states, with their own scoring pipeline. | The lifecycle duplicates state the model already holds: a divergence is knowable only once its pivot confirms, and a level event describes an interaction that already happened. The availability bar records the same fact with one integer. |
| **Events as extra votes.** Events adjust axis scores, or count alongside them as independent evidence. | A divergence and the RSI values that produced it are the same evidence twice. Events are observations that explain the numbers; the numbers stay authoritative. |
| **One aggregated time series.** Combine the timeframes into a single series, as the previous output description implied. | Timeframes are not interchangeable samples of one thing. Averaging an ordinal structure code across timeframes destroys the code, and averaging Direction is only valid because Direction happens to use one signed scale on every timeframe. The ladder is aggregated axis by axis, and each aggregate names its sources. |
| **Volume as a directional input.** | Participation says how much trade there was, not which way. Relative volume is context, and it never votes. |

## Consequences

- The model engine's output is a market state, not a series of indicator values, and it is a nested structure: readings, then axes, then per-timeframe results, then cross-timeframe aggregates and events.
- `AGENTS.md` section 2 now describes the analytical model and its three stages, and section 2's invariants gained axis independence, availability-not-occurrence, and no double counting. Sections 3, 4, 5, and 12 gained the rules that make the model implementable and testable: axis roles, numeric-only output, ordinal codes that are counted rather than averaged, event record shape, model-layer and event and progression and aggregation test rules, the model contract's documentation kind, and the fixed numeric contract and bounded retention.
- The indicator abstraction issue #14 asked for does not exist as described. Issue #14 is rewritten as the four-axis model and split into fourteen tracked sub-issues, so the work is sequenced and independently visible.
- The specification left eleven points open. They are decided in the execution plan, in its Decisions table as A1 to A11, and each is applied where that table says: nine in the contract document, one in this record under "Crate layout", and one in M13 (issue #29). They are agent decisions, taken under the human's instruction to apply the recommended default and flag it. Each is isolated enough to override without a redesign, and the plan names the section it would change.
- A model this shape is a larger surface than a set of indicator functions. The cost is paid in test obligations: per-axis causality, determinism, golden fixtures, edge cases, event availability, progression lifecycle, and aggregation method tests.

## Crate layout

The workspace holds one crate per pipeline stage — a data crate, a model crate, and the shell binary — with dependencies pointing from the shell to the model to the data. The concrete crate names and paths are recorded in `AGENTS.md` section 2 in the change that creates the workspace, because section 2 states that no crate name is assumed before the workspace exists. This decision fixes the shape of the layout, not the names.

Inside the model crate, the modules follow the contract's own sections: a kernel for the shared numeric helpers and the code registries, a readings module for the indicator families, an axes module with one module per axis, an events module with one module per event type, a context module, an aggregate module for the cross-timeframe layer, and a state module that assembles the published output.

## Open decisions

None of the eleven refinements blocks the work. They are recorded in the execution plan, flagged in the pull request that introduces the contract, and may be overridden individually before the affected sub-issue starts. The human's confirmation of this record is what moves the status from Proposed to Accepted.
