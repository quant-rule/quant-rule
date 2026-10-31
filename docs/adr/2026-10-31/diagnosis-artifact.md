# ADR: A Separate Cross-Engine Diagnosis Artifact

- Date: 2026-10-31
- Status: Accepted
- Work item: `diagnosis-v1`

## Context

Five per-engine bundles already record what each engine did. A reader can see the
final equity and the trade list, but not which engine caused the result. The model
bundle has no P&L by contract; the portfolio bundle has no model reading. Extending
either bundle would cross a boundary the project deliberately keeps.

## Decision

Add one read-only `diagnosis/` artifact beside the five bundles. The shell projects a
completed `BacktestRun` into a plain `DiagnosisInput`; `quantrule-analytics` computes
the evidence; the shell renders text, CSV, and PNG. The diagnosis binds to the data
version, config hash, model version, and report versions. It is never read by an
engine.

`backtest --diagnose` uses the completed run. `lab diagnose` re-runs the configuration
and compares key results before writing the same artifact.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Add cross-engine fields to the model or analytical bundle | Violates the no-P&L boundary and makes one bundle depend on downstream records |
| Add cross-engine fields to the portfolio bundle | Makes the portfolio bundle depend on model readings and strategy context |
| Compute the diagnosis in the shell | Moves statistics and aggregation out of analytics and breaks the shell's "parse, call, render" role |
| Put the diagnosis in a new crate | Adds a crate for one artifact; `quantrule-analytics` already owns cross-timeframe research and depends only on the model |
| Make the diagnosis part of the signal path | Contradicts the read-only evaluation boundary and the no-statistics-in-signals rule |

## Consequences

- The five bundles stay pure and unchanged.
- `quantrule-analytics` keeps its one-way dependency on `quantrule-model`; it receives
  plain facts instead of depending on `quantrule-backtest`.
- The artifact can be reproduced without a second pipeline run.
- A stale lab is detectable because the identity is explicit.
- The diagnosis adds a new file set to a full run. It is derived output and can be
  removed without affecting the five bundles.
- The checks are per-check statuses, not a composite score. No consumer can mistake a
  count of passes for a health score.
