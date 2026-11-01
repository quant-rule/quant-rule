# Improvement Cycle 002 — Multi-Horizon Model Evidence

- Date: 2026-11-01 (UTC)
- Status: Stopped at the model gate; no strategy or portfolio baseline
- Model run: [`MODEL-2026-11-01-02`](MODEL-2026-11-01-02-multi-horizon.md)
- Prior cycle: [`CYCLE-001`](CYCLE-001.md)
- Protocol: [`docs/model/2026-11-01/model-evidence-protocol.md`](../../model/2026-11-01/model-evidence-protocol.md)
- Claim type: model evidence; no edge claim

## Question

The first model run found Direction and Momentum below 0.50 at a single horizon.
This cycle asked whether the axes work at a different pre-registered horizon.

## Result

The horizon hypothesis is rejected. Direction is below 0.50 at every horizon in
five of six cells; Momentum is below 0.50 in every cell and horizon. The only
cell with a lower bound above 0.50 is ETHUSD 1h Direction at 20 bars (0.5139,
LCB 0.5031), one of thirty Direction cells. Structure and Progression
monotonicity are not stable across instruments.

## Review

The failure is broad, not a single bad horizon. Selecting the ETHUSD cell would
be post-hoc and would not survive the multiple-testing surface. The model gate
remains closed; a strategy run would still read an unreliable signal.

## Decision

Stop at the model gate. No model, strategy, or portfolio baseline is accepted.

## New Hypothesis

`MODEL-2026-11-01-03`: one model change that can plausibly create a forward
edge while staying deterministic and causal:

1. a first-class volatility/regime reading that conditions the axes;
2. a documented Structure interpretation per scale or instrument class;
3. a documented decision to treat Direction/Momentum as descriptive only.

The change must be pre-registered, graded on the same multi-horizon table, and
keep strategy and portfolio frozen.
