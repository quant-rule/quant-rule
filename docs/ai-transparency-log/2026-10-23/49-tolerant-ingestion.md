# AI Transparency Log — Add Candle Quality Reporting and Tolerant Ingestion

- **Issue:** [#49](https://github.com/quant-rule/quant-rule/issues/49)
- **Date:** 2026-10-23 (UTC)
- **Pull request:** [#54](https://github.com/quant-rule/quant-rule/pull/54)
- **Execution plan:** [`docs/execution-plans/2026-10-23/49-tolerant-ingestion.md`](../../execution-plans/2026-10-23/49-tolerant-ingestion.md)

## Entry: planning and preflight

- **Date and tool:** 2026-10-23 (UTC). opencode (DeepSeek V4.1 Flash Free), running in a local shell with `git`, `gh`, and filesystem tools.
- **Intent and boundary:** Start #49 after #48 merged. The boundary is an additive tolerant command and a deterministic quality report. The default `candles` command must remain unchanged. No indicators, higher timeframes, or model behavior.
- **Verification and evidence:**
  - `main` was at `b2876fd`, the project root was clean, and no other worktrees were present.
  - Issue #49 was moved to `In Progress` with start date 2026-10-23.
  - The worktree `quant-rule--49-tolerant-ingestion` was created on `feat/49-tolerant-ingestion` from `origin/main`.
  - The strict shell loader, command parser, entrypoints, fixtures, and README were inspected.
  - `quantrule-data` already rejects invalid candles, duplicates, and out-of-order timestamps, and preserves zero volume.
  - No build, test, lint, or doc result is claimed in this entry; implementation had not been written.
- **Learning:**
  - A new command is required: changing `candles` would alter an existing public contract.
  - The report belongs on stderr so stdout remains the NDJSON stream.
  - Header and read failures are file-level and must remain fatal; row-level failures are what tolerant mode skips.
  - A real zero volume is already represented as `Some(0.0)` in the data engine; the shell must not normalize it to `null`.
  - Every skipped row needs its source line and reason to satisfy attribution.

## Entry: implementation and verification

- **Date and tool:** 2026-10-23 (UTC). opencode (DeepSeek V4.1 Flash Free), running in the #49 worktree.
- **Intent and boundary:** Implement the approved additive tolerant command and verify it. No default command behavior, indicators, higher timeframes, or model behavior were added.
- **Implemented:**
  - `crates/quantrule-shell/src/csv.rs`: shared strict source helpers; a tolerant loader that skips row-level failures; an `IngestionReport` with the fixed summary and source-ordered skipped rows; row reasons for field count, invalid candle, duplicate timestamp, and out-of-order timestamp.
  - `crates/quantrule-shell/src/command.rs`: `candles-tolerant <csv-path>` parsing, help text, and execution with separate NDJSON and report writers.
  - `crates/quantrule-shell/src/main.rs`: interactive and non-interactive dispatch for the tolerant command.
  - `crates/quantrule-shell/tests/`: mixed, all-invalid, header-only, eviction, and file-level error fixtures plus 12 new integration tests and 3 new parser tests.
  - `README.md` and `crates/quantrule-shell/tests/fixtures/help.txt`: command and quality-report contracts.
  - `docs/execution-plans/2026-10-23/49-tolerant-ingestion.md`: plan and observed evidence.
- **Verification and evidence (observed in this worktree):**
  - `cargo fmt --all -- --check` exited 0.
  - `cargo clippy --all-targets -- -D warnings` exited 0 with no warnings.
  - `cargo test` exited 0: 50 `quantrule-data` unit tests, 10 shell parser tests, 32 shell integration tests, and 2 doctests passed.
  - `cargo doc --no-deps` exited 0.
  - Manual tolerant run over `/home/zantgo/Proyectos/quant-rule/data/BTCUSD_M1_2018-01-01_2024-12-31.csv`: exit 0, 1000 NDJSON lines, report `quality rows_read=3393175 accepted=3393175 rejected=0 duplicate=0 out_of_order=0 invalid=0 evicted=3392175 retained=1000`. The first and last output records matched the retained window boundaries.
  - Manual strict run over the same file: exit 0, 1000 lines, empty stderr, stdout byte-identical to the tolerant run.
  - The tests read only checked-in fixtures; the manual real-file check is not a test and does not use `data/` from test code.
- **Learning:**
  - **Rejected:** adding `--tolerant` to `candles`. It would change the existing command's contract; the additive `candles-tolerant` command keeps the default strict behavior and its tests intact.
  - **Rejected:** writing the report to stdout. It would mix a human-readable report into the NDJSON stream; stderr keeps stdout machine-readable.
  - **Rejected:** rendering NDJSON while reading. A late read or header failure would leave partial stdout; the window is built before either output.
  - **Rejected:** swallowing header or read errors in tolerant mode. Those are file-level failures, not attributable rows, and remain fatal.
  - **Rejected:** capping the skipped-row details. The issue requires every skipped row to be attributable; the plan records the resulting memory trade-off for a pathological file.
  - **Corrected:** the first help and README report samples showed a summary with more rejects than the sample skipped lines. Both samples were made internally consistent before the final gates.
  - The human authorized the Git handoff on 2026-10-23. The change was committed as `4208c99`, pushed, and opened as PR #54. The human approved the PR for merge on 2026-10-23.
