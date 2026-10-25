# AI Transparency Log: Harden the Analytical Contracts and Model Run

- Work item: `harden-analytics` (no issue number)
- Plan: [`docs/execution-plans/2026-10-25/harden-analytics.md`](../../execution-plans/2026-10-25/harden-analytics.md)
- Date: 2026-10-25 (UTC)

## Entry: input, model run, retention, and store lifecycle

- **Date and tool:** 2026-10-25 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Fix the current analytical half only. No strategy,
  execution, portfolio, backtest, schema migration, or new dependency.
- **Implementation:**
  - Added full-series model input by default and `--window latest` for a fast
    1,000-bar tail.
  - Added `ModelRun`, `TimeframeModel`, and `ModelBar`; analytics now consumes
    the model run instead of rebuilding readings and axes.
  - Added `model-series`, full event batches, bounded 1,024-event history,
    model output provenance, `store remove`, uninitialized-store rejection,
    fetch cap warning, and `market-chart --window latest`.
  - Added the history/retention ADR, CI, dataset verification, and corrected
    stale plan statuses.
- **Verification and evidence:**
  - `cargo fmt --all -- --check`: clean.
  - `cargo clippy --all-targets -- -D warnings`: clean.
  - `cargo test`: 882 passed, 4 ignored.
  - `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
  - 100k release model benchmark: 2.349 s.
  - 100k release layer benchmark: build 4.182 s, render 3.983 s.
  - `data/processed/harden-analytics`: layer1=333, layer2=333, layer3=260,
    raw=260; two same-root exports byte-identical; two summaries identical.
- **Learning:**
  - The first model-run refactor made the 100k layer build 6.573 s because the
    snapshot lookup scanned every confirmed level for every M1 bar. Walking
    back from the latest knowable level restored it to 4.182 s.
  - The event bound is a record count; `events` must use the full batch, not
    the latest snapshot, or old events disappear.
  - `main.rs`/`command.rs` dispatch duplication remains; it was not hidden
    behind a claim of completion.
- **Remaining:** D1/D2 command dispatch/module split and the full cross-surface
  column agreement test. No push, PR, or merge was performed.
