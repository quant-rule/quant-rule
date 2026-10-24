# AI Transparency Log: Model Hardening (M14)

- Work item: [#30](https://github.com/quant-rule/quant-rule/issues/30) — test: harden the model layer against the contract
- Plan: [`docs/execution-plans/2026-10-24/30-hardening.md`](../../execution-plans/2026-10-24/30-hardening.md)
- Program plan: [`docs/execution-plans/2026-10-24/14-model-engine-completion.md`](../../execution-plans/2026-10-24/14-model-engine-completion.md)
- Date: 2026-10-24 (UTC)

## Entry: hardening and performance

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Add cross-cutting tests, audit A1–A11, benchmark the long-history path, and fix the performance defect the benchmark exposes. No new dependency.
- **Verification and evidence:**
  - Added `tests/hardening.rs`: 8 passing tests and 1 ignored 100k benchmark.
  - Added `LevelIndex` to target selection and a 1024-leg per-scale retention bound.
  - All progression golden and causality tests still pass.
  - `cargo test --workspace`: 677 passed, 0 failed, 1 ignored.
  - `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
  - 100k release benchmark: **2.381 s**; before the fix the same run took 308–333 s.
  - `grep` found no `HashMap` or `HashSet` in `quantrule-model/src` or `quantrule-data/src`.
- **Learning:**
  - The benchmark exposed an O(n²) progression path: every leg creation and revision scanned the registry, and every bar iterated an unbounded leg list. The price-sorted index and the 1024-leg bound reduce the 100k run by two orders of magnitude.
  - The 20k debug hardening test takes about 5.4 s; the 100k benchmark is ignored by default and run in release to keep the regular suite practical.
  - A8 is a live-history bound, not a batch-output restriction; the contract note now states the distinction. A11's command names are the M13 proposal implemented as the shell contract.
  - The 1024-leg bound removes only old terminal legs; the newest per-scale record and all recent transitions are preserved. The golden progression fixture has 172 legs, below the bound.

## Final verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 677 passed, 0 failed, 1 ignored.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- 100k release benchmark: 2.381 s.
- No `HashMap`/`HashSet` in the model or data signal path.

Status: `Verified`. Commit, push, pull request, and board transition follow the delivery step.
