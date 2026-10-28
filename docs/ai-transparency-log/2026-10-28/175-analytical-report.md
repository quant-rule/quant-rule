# AI Transparency Log — Analytical Report

- Date: 2026-10-28 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#175](https://github.com/quant-rule/quant-rule/issues/175)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md)
- Branch/worktree: `feat/175-analytical-report` in `quant-rule--175-analytical-report`
- PR: [#187](https://github.com/quant-rule/quant-rule/pull/187)
- Status: PR open; merge after checks pass.

## Intent and boundary

R3 wires `report analytical`. The report builds the research layers for the
timeframes named by `analytics.json`, summarizes the four axes and their
pairwise correlations, and writes `summary.txt`, `report.json`, `axes.csv`,
`relations.csv`, and `provenance.json` under `<out>/analytical/`. It runs no
strategy, execution, or portfolio step and publishes no order, fill, cash,
equity, or P&L field.

## Work performed

1. Added the analytical bundle to `command/report_run/bundles.rs`.
2. Built the layers with `build_layers` for the configured analytical
   timeframes and summarized them with `summary::summarize`.
3. Wrote the fixed `axes.csv` and `relations.csv` headers and rows, the summary
   text, and the JSON summary grouped by timeframe.
4. Removed the not-implemented report error and the now-unused report-name
   accessor; all five report names are wired.
5. Added a binary-level test for the artifact set, the row counts, the JSON
   summary, the boundary, and the provenance.

## Decisions and corrections

- **The analytics timeframe set is the layer input.** `analytics.json` names a
  subset of the data ladder; the bundle filters the four candle series to those
  timeframes before building the layers. A subset without M1 has no Layer 3
  rows, which is the model's rule rather than a report defect.
- **`relations: false` is an empty list, not a zero.** The CSV keeps its header
  and has no rows, and the JSON `relations` array is empty. A missing
  correlation is null; the summary module never substitutes zero.
- **Ordinal codes are counted.** The `ordinal_0..9` columns are the summary
  module's buckets; the report writes the counts and the null count and never
  averages a code.
- **The bucket's row counts are the model's.** `layer1_rows`, `layer2_rows`, and
  `layer3_rows` come from the layer set; the report does not recompute a layer.
- The test module was initially left in the middle of `bundles.rs` after the
  analytical functions were appended. Clippy's `items_after_test_module` caught
  it; the module moved to the end.
- `ReportEngine::code` and `ReportError::NotImplemented` were removed once the
  last report was wired, so no dead path remains.

## Verification and evidence

Commands were run in `quant-rule--175-analytical-report` on 2026-10-28.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` | 1,338 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo test -p quantrule-shell --test report_command` | 7 passed, 0 failed |
| `cargo doc --no-deps` | Passed; exit 0; no warning |

The full-suite count stayed at 1,338: the analytical test replaced the
not-implemented test, and no other test changed.

## Learning

- The analytics crate already owned the summary and relation math. The report
  layer only had to choose the timeframe inputs and render the numbers, which
  kept the axis independence rule in the engine rather than in the shell.
- An empty list and a zero are different answers for a disabled relation. The
  bundle keeps them separate in both the CSV and the JSON.
