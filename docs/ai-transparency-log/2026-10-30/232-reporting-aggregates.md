# AI Transparency Log — Reporting Aggregates outside the Model Run

- Date: 2026-10-30 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#232](https://github.com/quant-rule/quant-rule/issues/232)
- Parent plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Status: Implemented; PR pending

## Intent and boundary

Phase 4 removes automatic cross-timeframe aggregation from the model run.
`ModelRun` and `MarketState` publish per-timeframe measurements only. Reports,
the dashboard, and research compute the descriptive view explicitly through
`quantrule_analytics::cross_timeframe`.

This is a reporting-boundary change. It does not change an axis formula, a
strategy decision, execution, or portfolio accounting. Report, dashboard,
research, and store outputs must remain byte-identical.

## Implemented

- Removed the `aggregates` field from `ModelRun` and `MarketState`.
- Added `ModelRun::snapshots()` for explicit reporting input.
- Added `quantrule_analytics::cross_timeframe(&ModelRun)`.
- Changed `MarketReport::compute` to take the reporting view explicitly.
- Changed `DashboardState::from_run` to take the reporting view explicitly.
- Updated `market-state`, `market-report`, `market-dashboard`, and config-driven
  model reports to compute the view through analytics.
- Updated model, shell, dashboard, report, and research tests.
- Updated `AGENTS.md`, the model contract, the docs index, and the plan.

## Verification and evidence

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | Passed; 45 suites, 0 failures |
| `cargo doc --no-deps` | Passed |
| Report and dashboard goldens | Passed; rendered output unchanged |
| Store ingest/verify | Passed |
| Research export/summary | Passed |
| `ModelRun`/`MarketState` aggregate field | Removed |
| Strategy aggregate input | None; strategy input unchanged |

No model formula, strategy decision, execution fill, or portfolio accounting
changed. The reporting view is computed from the same snapshots as before.

## Deviations and decisions

- The pure view records and `CrossTimeframeAggregates::compute` remain in
  `quantrule_model::aggregate` as a shared reporting helper. Moving that module
  to a new crate would require either a new shared crate or moving the report
  and dashboard views, which is a larger architectural change. The model run no
  longer computes or publishes the view; all reporting call sites go through
  `quantrule_analytics::cross_timeframe`.
- This keeps the dependency direction one-way: shell -> analytics -> model.
- The `market-state` JSON still contains `aggregates`; the shell computes it
  explicitly. The user-facing output is unchanged.
- Path commands still use the default four-timeframe ladder; the configured
  available ladder remains the Phase 2 behavior.

## Learning and rejected suggestions

- Rejected adding a new shared crate in this phase. The model/analytics
  dependency would otherwise need a cycle or a large view move.
- Rejected keeping `run.aggregates()` as a convenience. It would leave the model
  publishing the view and defeat the boundary.
- Rejected changing report field names or ordering. The reporting view is a
  refactor; its output must stay byte-identical.

## Residual risk

- The aggregate implementation still lives in the model crate as a view helper.
  A later refactor can move it to a shared reporting crate if the duplication
  or coupling becomes material.
- The model run now builds snapshots twice in some reporting paths (once for
  consensus and once for `snapshots()`). The cost is linear and measured by the
  existing tests; a future optimization can share the snapshot vector.
- No independent verifier was available; this is a same-agent implementation and
  test pass, not an independent review.
