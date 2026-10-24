# Execution Plan: Research Dataset Verification (AN7)

- Issue: [#82](https://github.com/quant-rule/quant-rule/issues/82) — test: verify the research dataset
- Branch: `test/82-verify`
- Pull request: pending
- Status: Verified
- Date: 2026-10-24 (UTC)
- Contract: [`docs/analytics/2026-10-24/research-output.md`](../../analytics/2026-10-24/research-output.md)
- Report: [`docs/analytics/2026-10-24/dataset-report.md`](../../analytics/2026-10-24/dataset-report.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/82-verify.md`](../../ai-transparency-log/2026-10-24/82-verify.md)

## 1. Intent

Verify the layer dataset end to end against the research contract and report the 100k cost. The issue is the last analytics sub-issue; a passing verification closes the analytics container #13.

## 2. Current state

- `main` is at `164a069`; #76–#81 are merged.
- The analytics crate builds, exports, and summarizes the layers; the shell has `research export` and `research summary`.
- The unit tests cover each module; there is no integration test across the data engine, model, layers, export, and summary.

## 3. Scope

Included:

- `crates/quantrule-analytics/tests/research_verification.rs`: counts, the Layer 2 derivation, resolution ranges, as-of blocks, byte identity, widths, the dictionary, the code table, a sample analysis from `layer2.csv`, and causality.
- An ignored 100k release benchmark that reports the build time, render time, and output size.
- `docs/analytics/2026-10-24/dataset-report.md` with the verification table and the 100k numbers.

Excluded:

- The record files (store epic #64).
- The SQLite store and its verification (#74).
- Any behavior change to the layers, export, or summary.

## 4. Assumptions and uncertainty

- The tests build their own M1 series through `TimeframeSet::full`; they do not read the shell fixtures or the network.
- The 600-bar fixture is large enough to exercise the M5/M15/H1 blocks and the warmup nulls.
- The 100k benchmark is release-only and ignored by default; the default suite stays fast.
- The sample analysis reads `layer2.csv`, whose fields contain no commas. The record files are not parsed.
- The dataset report is checked in; the PR also records the 100k numbers.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Test location | Unit module; integration test | `tests/research_verification.rs` | The checks span four crates and the public API |
| D2 | Fixture source | Synthesize higher series; derive with the data engine | `TimeframeSet::full` | The test exercises the real ladder construction and the 100k path |
| D3 | Sample analysis | Re-run the summary only; parse the CSV | Parse `layer2.csv` | It proves the exported values round-trip and the analysis is reproducible from the file |
| D4 | 100k run | Regular test; ignored release test | Ignored release test | The default suite stays fast; the result is reported |
| D5 | Report | PR only; checked-in doc | Checked-in doc | The issue asks for a dataset report; the PR links it |
| D6 | Record files | Include; state the gap | State the gap | The export's scope is the layer files; the records belong to #64 |

## 6. Acceptance criteria

- The layer row counts match the inputs.
- Every Layer 2 resolution equals the mapping of its own Layer 1 metric.
- Every continuous value is in range, every ordinal matches its bucket, and every null pair carries a reason.
- Every Layer 3 block is the latest closed higher bar; every higher close is at or before the base close; staleness matches.
- Two exports are byte-identical; the widths and the dictionary are complete.
- The sample analysis from `layer2.csv` matches the in-memory summary.
- The first 300 Layer 3 rows are unchanged when 60 later bars are appended.
- The 100k release run completes and reports the time and size.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Add the integration test file.
2. Run the default suite.
3. Run the 100k benchmark in release.
4. Write the dataset report.
5. Run the full checks, write the plan and the log, commit, push, open the PR, move #82 to Done, merge, and clean up.
6. Close #13 once #76–#82 are done.

## 8. Verification

| Check | Result |
|---|---|
| `cargo test -p quantrule-analytics --test research_verification` | 7 passed, 1 ignored |
| `cargo test --workspace` | 753 passed, 0 failed, 3 ignored |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | clean |
| 100k release run | 128,332 Layer 1/2 rows, 100,000 Layer 3 rows; build 4.122 s; render 4.105 s; `layer3.csv` 228,808,595 bytes; all files 368,567,856 bytes |

The byte figures are the rendered output size, not a process peak. The benchmark asserts the 100k counts.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| The 100k export is large | The report records the 228.8 MB Layer 3 and the 368.6 MB total; the cap is a documented dataset limit |
| The integration test duplicates module tests | It checks the composed output, not the module internals; the module tests remain the focused checks |
| The CSV sample parser is naive | Layer files contain no commas in fields; the test reads only numeric columns |
| The record files are missing | The report states the gap and names the store epic |
| A model change invalidates the fixture counts | The fixture is generated deterministically; the counts are asserted, so a change fails loudly |

## 10. Stop conditions

- A verification needs the SQLite store.
- A contract rule cannot be checked from the public API.
- A check fails and the fix is outside this issue.

## 11. Delivery

| Item | Value |
|---|---|
| Commit | pending |
| Branch | `test/82-verify` |
| Pull request | pending |
| Board | #82 moved to `Done` when the PR is open; #13 closed after the merge |

Merge, release, tag, and deployment remain with the human.
