# AI Transparency Log — Model and Strategy Reports

- Date: 2026-10-28 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#173](https://github.com/quant-rule/quant-rule/issues/173)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md)
- Branch/worktree: `feat/173-model-strategy-reports` in `quant-rule--173-model-strategy-reports`
- PR: [#185](https://github.com/quant-rule/quant-rule/pull/185)
- Status: PR open; merge after checks pass.

## Intent and boundary

R1 adds the config-driven `report` command and wires its first two bundles:
`model` and `strategy`. The command loads one `run.json`, runs only the engines
the selected report needs, and writes a bundle under `<out>/<engine>/`. The
shared writer refuses a non-empty target, writes `report.json` last, and removes
a partial bundle on failure; the shared provenance record names the data
version, the periods, the config file names, the resolved values, and the
artifact list, never a resolved absolute path.

The boundary is the shell. The model bundle reuses the existing `MarketReport`
view and renderers; the strategy bundle stops at the decision and publishes no
fill, price paid, quantity, cash, equity, or P&L. The execution, portfolio, and
analytical reports answer with a not-implemented error until R2 and R3.

## Work performed

1. Added `render_report::bundle` with the availability check, the fixed write
   order, and the cleanup on failure.
2. Added `render_report::provenance` with the `quantrule.provenance/v1` record.
3. Added `command/report_run.rs` with the `report` parser and the model and
   strategy bundles.
4. Added entry/exit markers to the price chart and used them for
   `price-decisions.png`.
5. Wired the command through the interactive parser, the argv parser, the
   dispatcher, the help text, and the README.
6. Added a checked-in config fixture and binary-level tests for the model
   compatibility, the strategy shape, determinism, the not-implemented answers,
   the non-empty target, and the parser errors.

## Decisions and corrections

- **The model bundle is the legacy view.** It calls `MarketReport::compute` and
  the same summary, JSON, and chart renderers, so its four legacy artifacts are
  byte-identical to `market-report` for the same data and period. The test
  compares the bytes rather than asserting the claim in prose.
- **`decisions.csv` is the fixed contract shape.** The header and row order come
  from the report document; the rows carry the readings and the decision, never
  a price paid or a quantity. The strategy JSON mirrors those records.
- **The strategy JSON shape is an R1 implementation choice.** The report
  contract fixes the CSV header and the summary names but not every JSON field.
  The value carries `report`, `report_version`, `model_version`, the periods,
  the resolved config, `summary`, and `records.decisions`; it adds no field that
  belongs to another engine.
- **The chart markers are drawn from the decisions.** A marker is a
  `(bar, is_entry)` pair at the bar's close; the shell does not recompute a
  decision. The existing price chart renderer gained an optional marker slice,
  and `render_png` keeps its old behavior with an empty slice.
- **The unwired reports fail explicitly.** `report execution`, `portfolio`, and
  `analytical` return `report <engine> is not implemented in this work item`,
  so a user cannot mistake an empty directory for a completed bundle.
- The first strategy integration fixture is the 260-row M1 file used by the
  market-report tests. It produces 17 M15 bars and no decisions because the
  model history is too short; the test therefore checks the header, the summary,
  and the boundary, and the decision-row serialization is covered by a unit test
  with synthetic inputs.
- The first help fixture did not include the new command; the CLI help test
  caught it, and the checked-in text now matches the binary.

## Verification and evidence

Commands were run in `quant-rule--173-model-strategy-reports` on 2026-10-28.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` | 1,335 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo test -p quantrule-shell --test report_command` | 6 passed, 0 failed |
| `cargo doc --no-deps` | Passed; exit 0; no warning |

The full-suite count rose from 1,325 (B2 baseline) to 1,335: six binary-level
report tests, two command unit tests, and two bundle unit tests. No existing test
was changed or removed.

## Learning

- A bundle is a directory boundary, and `report.json` last is what makes the
  boundary observable. Writing the provenance and the other artifacts first
  means a reader never sees a complete report over a missing input record.
- Byte identity is the cheapest compatibility test: the model bundle's four
  legacy artifacts are compared to the legacy command rather than described as
  equivalent.
- A report that cannot exist yet should say so. A not-implemented error is
  better than a bundle with a summary and no records.
