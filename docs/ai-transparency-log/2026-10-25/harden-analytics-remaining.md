# AI Transparency Log: Remaining Analytical Hardening

- Work item: `harden-analytics-remaining`
- Plan: [`docs/execution-plans/2026-10-25/harden-analytics-remaining.md`](../../execution-plans/2026-10-25/harden-analytics-remaining.md)
- Date: 2026-10-25 (UTC)

## Entry: evidence, dispatch, provenance, and column guard

- **Date and tool:** 2026-10-25 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Close the evidence, dispatch, provenance, and
  column-agreement gaps in the merged analytical hardening. No trading,
  schema migration, or new dependency.
- **Implementation:**
  - Regenerated `data/processed/harden-analytics` in the main checkout.
  - Corrected stale plan statuses and the progression delivery line.
  - Added `parse_argv` and `execute`; `main.rs` now delegates both entry points.
  - Added window metadata to `market-state` and `model-series`, and event
    occurrence/availability timestamps.
  - Added a `bar` DDL versus research column-order test.
- **Verification and evidence:**
  - `cargo test`: 883 passed, 4 ignored.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo doc --no-deps` with `-D warnings`: clean.
  - Dataset files present under `data/processed/harden-analytics`.
- **Learning:**
  - The first argv-dispatch migration passed the command but dropped the
    command name before `parse_argv`; the CLI suite caught it. The entry point
    now passes the already-stripped argv exactly once.
  - The dataset artifact is ignored by Git and must be generated in the main
    checkout; a worktree-local artifact disappears with the worktree.
- **Remaining:** D2 command-module split and the exact 1,500/100,001/1,200/
  >1,024 fixtures listed in the predecessor plan. No push, PR, or merge.
