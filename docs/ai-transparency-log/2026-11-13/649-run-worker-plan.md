# AI transparency log — DIST-033 run worker plan

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: produce the execution plan for the backtest worker before changing the
  run store, queue adapter, evidence boundary, or execution path.
- Boundary: this work item is read-only analysis and a plan. No worker code,
  queue consumer, schema, or strategy change is implemented here.
- Verification: repository inspection of the run submission/store, model route,
  app backtest API, queue adapter, and artifact contracts. No test was run for a
  behavior change because none was made; the plan's verification table is
  Pending by design.
- Learning: the coupled `run_backtest` takes paths, so the worker must
  materialize the six-file config and rewrite `data.json` to the catalogued
  dataset. The queue adapter publishes only; consumption needs a trait and a
  small futures dependency. Evidence has no service yet, so finalization needs a
  single evidence-shaped module rather than being scattered through the worker.
- Rejected: starting the worker implementation without the plan. It crosses the
  evidence boundary, adds a dependency, and requires a data-source rewrite; the
  decisions belong at a human gate.
