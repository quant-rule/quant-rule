# AI Transparency Log: Summary and Relation Analysis (AN6)

- Work item: [#81](https://github.com/quant-rule/quant-rule/issues/81) — feat: add summary and relation analysis
- Plan: [`docs/execution-plans/2026-10-24/81-summary.md`](../../execution-plans/2026-10-24/81-summary.md)
- Date: 2026-10-24 (UTC)

## Entry: the summary command

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Add a deterministic per-timeframe summary of the continuous values and ordinal buckets, plus pairwise continuous correlations, and expose it as `research summary`. No composite score, no ordinal averaging, no new dependency.
- **Verification and evidence:**
  - Added `quantrule-analytics::summary` with `AxisSummary`, `RelationSummary`, `DatasetSummary`, and `summarize`.
  - Added the shell command, the shared `load_layers` path, the rendering, both dispatchers, the help text, and the README entry.
  - 7 summary unit tests and 1 CLI test pass, including perfect positive and negative relations, a constant series, null handling, the ordinal histogram, an empty set, and repeated runs.
  - `cargo test --workspace`: 746 passed, 0 failed, 2 ignored.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
  - The 260-bar fixture prints 41 lines: 1 summary, 16 axis lines, and 24 relation lines.
- **Learning:**
  - A perfect correlation computed from the raw sums is `0.9999999999999991`, not exactly `1.0`, because the products and sums are not exact. The test uses a `1e-12` tolerance, and the reported value is clamped to `[-1, 1]` without rounding. The output keeps the shortest round-trip form, so a summary line can be long; rounding is a presentation choice that the issue does not require.
  - The first design would have read the exported `layer2.csv` back. That needs a second CSV parser and would test the parser rather than the analysis. The summary now builds the same layers as the export; the export test already proves the files are byte-identical, so the analysis is reproducible from either path.
  - The pair count is not the axis count: a null on either side removes the row from the pair. The fixture's H1 direction is entirely null, so its relations are `r=null` with `n=0`; the output is explicit rather than a fabricated zero.
  - The ordinal histogram is per axis and per timeframe, and its sum plus `ordinal_null` equals the row count. The mean is computed only from the continuous values, so an ordinal code is never averaged.
  - The summary uses the resolution's continuous value, which clamps Progression. That is the contract's comparable value; the raw `progress` remains in Layer 1 for analyses that need the unclamped value.
  - `load_layers` was extracted so the export and the summary share one build path. This also keeps the command output and the written dataset derived from the same layers.
  - A manual `research summary ... | head` run printed a broken-pipe error after the consumer closed; that is the shell's normal behavior for a closed pipe, not a defect in the summary.

## Final verification

- `cargo test -p quantrule-analytics summary`: 7 passed.
- `cargo test -p quantrule-shell --test cli research_summary`: 1 passed.
- `cargo test --workspace`: 746 passed, 0 failed, 2 ignored.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Status: `Verified`. Commit, push, pull request, and board transition follow the delivery step.
