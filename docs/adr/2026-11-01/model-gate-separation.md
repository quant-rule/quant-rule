# ADR: Separate the Model-Correctness Baseline from the Strategy-Edge Gate

- Date: 2026-11-01 (UTC)
- Status: Accepted
- Work item: `model-gate-separation`

## Context

Four model-evidence cycles tested Direction, Momentum, Structure, Progression,
regimes, and events. None produced a stable cross-instrument predictive edge.
The old gate treated that as a model failure and blocked the strategy tier. That
conflated two questions: whether the model measures correctly and whether a rule
using the measurement has an edge.

## Decision

- The model run promotes a **correctness baseline**: formulas, causality,
  determinism, availability, coverage, and descriptive review. A D grade still
  blocks.
- Signal-usability grades remain A/B/C. A/B may be standalone signals. C may be
  used only as an explicit strategy hypothesis with the grade recorded.
- The strategy run is the only place a cost-inclusive OOS edge is judged. The
  green criteria are unchanged.
- The model baseline accepted on 2026-11-01 is
  `MODEL-2026-11-01-01..04`, with the C-axis limitation recorded.

## Alternatives

1. Require every axis to predict before any strategy run: rejected; it rejects a
   correct measurement layer and blocks the actual edge test.
2. Change the model formula to make the baseline pass: rejected; it fits the
   evidence sample.
3. Declare the axes descriptive only and change the rule family: deferred; it is
   a strategy hypothesis, not a model-gate change.

## Consequences

- The first strategy comparison can run on the accepted model correctness
  baseline.
- A strategy using C axes must record the limitation and cannot be called green
  without the full OOS criteria.
- A D axis still blocks; a model formula change still requires a new model run.
