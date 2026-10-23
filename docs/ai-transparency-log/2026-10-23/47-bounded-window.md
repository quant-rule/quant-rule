# AI Transparency Log — Retain the Last 1000 Candles in a Bounded Window

- **Issue:** [#47](https://github.com/quant-rule/quant-rule/issues/47)
- **Date:** 2026-10-23 (UTC)
- **Execution plan:** [`docs/execution-plans/2026-10-23/47-bounded-window.md`](../../execution-plans/2026-10-23/47-bounded-window.md)

## Entry: planning and preflight

- **Date and tool:** 2026-10-23 (UTC). opencode (DeepSeek V4.1 Flash Free), running in a local shell with `git`, `gh`, and filesystem tools.
- **Intent and boundary:** Start #47 after #46 merged. The boundary is the bounded window only: no CSV source, shell command, or NDJSON. No implementation had started when this entry was written.
- **Verification and evidence:**
  - `main` was at `e05b52e`, the project root was clean, and no other worktrees were present.
  - The data crate at `e05b52e` exposes the candle entity, timestamp functions, and `Timeframe::M1`; it has no window type.
  - Issue #47 was moved to `In Progress` with start date 2026-10-23.
  - The worktree `quant-rule--47-bounded-window` was created on `feat/47-bounded-window` from `origin/main`.
  - No build, test, lint, or doc result is claimed in this entry; the implementation had not been written.
- **Learning:**
  - The window must assign `series_index`, because the shell should not own series identity. The candle's public parser still accepts an explicit index for other callers.
  - Rejected candles must not consume an index, so parsing and ordering checks precede the increment.
  - The last accepted timestamp must be tracked separately from the retained candles; otherwise an out-of-order row could pass after the previous candle is evicted.

## Entry: implementation and verification

- **Date and tool:** 2026-10-23 (UTC). opencode (DeepSeek V4.1 Flash Free), running in the PR2 worktree.
- **Intent and boundary:** Implement the bounded window in #47 and verify it. No CSV source, shell command, or NDJSON was added; those remain #48.
- **Implemented:**
  - `CandleWindow` in `crates/quantrule-data/src/window.rs`, with `DEFAULT_WINDOW = 1000`.
  - `push`, `len`, `is_empty`, `first`, `last`, `get`, and oldest-first `iter`.
  - Stable `series_index` assignment on acceptance and eviction-safe `last_timestamp`.
  - `WindowError` for invalid candles, duplicate timestamps, and out-of-order timestamps.
  - Exported the window and constant from the data crate and updated the crate docs.
  - Unit tests for the required counts and invariants.
- **Verification and evidence (observed in this worktree):**
  - `cargo fmt --all -- --check` exited 0.
  - `cargo clippy --all-targets -- -D warnings` exited 0 with no warnings.
  - `cargo test` exited 0: 50 unit tests in `quantrule-data`, 0 in `quantrule-model`, 0 in `quantrule-shell`, and 2 doctests passed.
  - `cargo doc --no-deps` exited 0 and generated documentation for all three crates.
  - The tests use generated timestamps and in-code candle fields; they do not read `data/`.
- **Learning:**
  - The first test compile failed because the local `raw` binding shadowed the `raw` test helper. The helper was renamed to `raw_candle`, and the suite then passed. This is a test-only correction; no production behavior changed.
  - **Rejected:** making the window capacity configurable. The issue fixes the product bound at 1000, and a configurable capacity would create an untested way to violate it.
  - **Accepted:** rejecting a candle before incrementing `next_series_index`, so a bad row cannot create a gap in the series identity.
  - No commit, push, or pull request was created before this entry. The human authorized the commit, push, and pull request on 2026-10-23; this verified tree is what they carry.
