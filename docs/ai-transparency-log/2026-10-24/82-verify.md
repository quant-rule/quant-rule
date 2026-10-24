# AI Transparency Log: Research Dataset Verification (AN7)

- Work item: [#82](https://github.com/quant-rule/quant-rule/issues/82) — test: verify the research dataset
- Plan: [`docs/execution-plans/2026-10-24/82-verify.md`](../../execution-plans/2026-10-24/82-verify.md)
- Report: [`docs/analytics/2026-10-24/dataset-report.md`](../../analytics/2026-10-24/dataset-report.md)
- Date: 2026-10-24 (UTC)

## Entry: end-to-end verification

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Verify the layer dataset across the data engine, the model, the layers, the export, and the summary, and report the 100k cost. No behavior change.
- **Verification and evidence:**
  - Added `crates/quantrule-analytics/tests/research_verification.rs`: 7 passing tests and 1 ignored 100k benchmark.
  - The checks cover the layer counts, the Layer 2 derivation, every resolution's range and bucket, every Layer 3 block's as-of bar and staleness, byte-identical export, the 61/73/356 widths, the 496-row dictionary, the code table, a sample analysis from `layer2.csv`, and causality.
  - `cargo test --workspace`: 753 passed, 0 failed, 3 ignored.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
  - The 100k release run: 128,332 Layer 1/2 rows and 100,000 Layer 3 rows; build 4.122 s; render 4.105 s; `layer3.csv` 228,808,595 bytes; all files 368,567,856 bytes.
  - The dataset report is checked in at `docs/analytics/2026-10-24/dataset-report.md`.
- **Learning:**
  - The 100k Layer 3 file is 228.8 MB because each row carries three full higher-timeframe blocks. The total export is 368.6 MB. This is the cost of the self-contained block design; the report states it rather than hiding it behind the row count.
  - The test builds its M1 series through `TimeframeSet::full`, so it exercises the full-series path from #66 as well as the ladder. The 600-bar fixture gives 770 Layer 1/2 rows and 600 Layer 3 rows.
  - The sample analysis reads `layer2.csv` and recomputes the direction count, mean, ordinal histogram, and direction/momentum correlation. It matches the in-memory summary to `1e-12`, which also proves the shortest-round-trip floats survive the CSV round trip.
  - The causality check builds 300 and 360 bars from the same prefix and compares the first 300 Layer 3 rows; the later bars do not change an earlier row.
  - The record files are still out of scope. The report names the gap and the store epic, so a reader does not mistake the layer export for the complete dataset contract.
  - The 100k benchmark is ignored by default; the default suite stays under a few seconds, and the release run is the evidence for the cap.
  - No test reads the shell fixtures, the network, the wall clock, or a file outside the repository. The report's reproduction commands are the only filesystem-facing examples, and they are not run by the suite.

## Final verification

- `cargo test -p quantrule-analytics --test research_verification`: 7 passed, 1 ignored.
- `cargo test --workspace`: 753 passed, 0 failed, 3 ignored.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- 100k release: 4.122 s build, 4.105 s render, 368,567,856 bytes of output.
- Status: `Verified`. Commit `d80b9e2` is pushed on `test/82-verify`, pull request [#98](https://github.com/quant-rule/quant-rule/pull/98) is open, and #82 is `Done` on the board.
