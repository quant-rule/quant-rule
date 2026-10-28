# AI Transparency Log — Execution and Portfolio Reports

- Date: 2026-10-28 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#174](https://github.com/quant-rule/quant-rule/issues/174)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md)
- Branch/worktree: `feat/174-execution-portfolio-reports` in `quant-rule--174-execution-portfolio-reports`
- PR: [#186](https://github.com/quant-rule/quant-rule/pull/186)
- Status: PR open; merge after checks pass.

## Intent and boundary

R2 wires `report execution` and `report portfolio`. Both run the full pipeline
once, so the execution quantities and the portfolio equity come from the same
fills. Execution publishes orders, fills, rejections, slippage, and fees and no
capital; portfolio publishes cash, equity, trades, costs, funding, and drawdown
and no model or strategy label. The analytical report remains not implemented
until R3.

## Work performed

1. Added `command/report_run/bundles.rs` with the execution and portfolio bundle
   writers and their CSV, text, JSON, and chart renderers.
2. Added a full-pipeline helper in `report_run.rs` and wired the two report
   names through the existing dispatcher.
3. Added a single-series line chart for the daily equity and drawdown curves and
   exposed the chart's entry and exit colors.
4. Added integration tests for the artifact sets, the fixed CSV headers, the
   boundary scans, and determinism, plus unit tests for the summary and CSV
   helpers.

## Decisions and corrections

- **Both bundles run the pipeline.** `report execution` does not simulate a
  separate sizing path; it uses the same `run_backtest` result as `report
  portfolio`. The execution quantities therefore match a full backtest.
- **Fill ids come from the order record.** A fill has no id of its own, so the
  bundle pairs each batch's filled order records with its fills in order. The
  order record carries the deterministic id that the report writes.
- **A rejection's `decision` comes from the order.** The rejection record itself
  has no side; the order with the same id carries the entry or exit reason.
- **The boundary is tested, not described.** The execution JSON is scanned for
  cash, equity, P&L, commission, funding, drawdown, and trade; the portfolio JSON
  is scanned for direction, momentum, structure, agreement, and axis.
- **The line chart plots the point order, not a resampled calendar.** A missing
  date is absent from the equity points, so the line has a gap; the renderer
  never inserts a zero. The daily points are already the contract's daily
  aggregation.
- **The fixture has no decisions.** The 260-row M1 fixture produces 17 M15 bars,
  too few for the model's agreement. The integration test therefore checks the
  empty record shapes, and the populated serialization is covered by unit tests
  with synthetic orders, fills, trades, and equity points. The runner tests cover
  the populated pipeline itself.
- The first R2 work was written on the R1 branch after its PR merged. It was
  moved to the #174 worktree from `origin/main` as a patch before any commit, so
  the merged R1 branch stayed clean and #174 has its own branch.

## Verification and evidence

Commands were run in `quant-rule--174-execution-portfolio-reports` on 2026-10-28.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` | 1,338 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo test -p quantrule-shell --test report_command` | 7 passed, 0 failed |
| `cargo doc --no-deps` | Passed; exit 0; no warning |

The full-suite count rose from 1,335 (R1 baseline) to 1,338: one expanded report
integration test and two bundle unit tests. No existing test was changed or
removed.

## Learning

- Running the same pipeline for two bundles is what keeps their numbers in step;
  a separate execution-only run would be a second, divergent simulation.
- A boundary is easiest to trust when a test reads the rendered JSON and fails on
  a forbidden word. The execution and portfolio bundles are separate directory
  trees, so the scan is cheap.
- A fill's id belongs to its order record. Pairing the batch's filled orders with
  its fills keeps the report deterministic without adding a field to `Fill`.
