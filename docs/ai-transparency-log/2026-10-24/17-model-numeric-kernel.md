# AI Transparency Log — The Model Numeric Kernel

- **Issue:** [#17](https://github.com/quant-rule/quant-rule/issues/17)
- **Date:** 2026-10-24 (UTC)
- **Execution plan:** [`docs/execution-plans/2026-10-24/17-model-numeric-kernel.md`](../../execution-plans/2026-10-24/17-model-numeric-kernel.md)
- **Umbrella plan:** [`docs/execution-plans/2026-10-21/14-model-engine.md`](../../execution-plans/2026-10-21/14-model-engine.md)

## Entry: preflight and planning

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash Free), running in a local shell with `git`, `gh`, and filesystem tools.
- **Intent and boundary:** Record the approved plan for M1, the shared numeric kernel of the model engine, and start it in its own worktree. The boundary is #17 only: no reading, axis, event, aggregate, market state, shell command, or data-engine change. M2 (#18) and later M-items are out of scope.
- **Verification and evidence:**
  - `main` was clean and equal to `origin/main` at `9c7845f`; no open pull request and no existing task worktree.
  - The data engine was complete: #9 and #46–#50 were closed, and PRs #51–#57 were merged. #14 is the model container; #17–#30 were all `Todo`.
  - Issue #17 was moved to `In Progress` with `Start date` 2026-10-24 on the `QuantRule` board.
  - Worktree `/home/zantgo/Proyectos/quant-rule--17-model-numeric-kernel` was created on `feat/17-model-numeric-kernel` from `origin/main`; the project root stays on `main`.
  - The contract was read at §9 (numeric output, readiness, percentile, counters), §12 (numeric contract), and §14 (code registries). The model umbrella plan was read for the M1 row and dependency order.
  - No implementation or test result is claimed in this entry; only the plan existed when it was written, and the contract clarification followed in the implementation entry.
- **Learning:**
  - **Accepted:** one cohesive `quantrule-model::kernel` module rather than a crate or a helper per file. The kernel is small, and the module boundary already separates it from the axis code.
  - **Accepted:** `Option<f64>` and `Option<i8>` for missing values. A sentinel zero would violate the contract's null-versus-zero rule and would be indistinguishable from a measured zero.
  - **Accepted:** `clamp_unit` instead of a generic clamp, because §12 fixes the bounds and call sites must not re-derive them.
  - **Flagged:** the contract's wording "closed bars the state has held" did not state the age on the change bar or what a null bar does to the change window. The chosen convention (first code-bearing bar age 1, change resets to 1, null freezes both counters and does not advance the window) was written into contract §9 so later axes do not reinterpret it. This is the only local interpretation in M1.

## Entry: implementation and verification

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash Free), running in the #17 worktree.
- **Intent and boundary:** Implement the kernel and cover issue #17's acceptance criteria, then verify the workspace. No axis, reading, event, aggregation, shell, or dependency change was made.
- **Implemented:**
  - `crates/quantrule-model/src/kernel.rs`: `UnavailableReason` (1..6) and `Readiness` (0..2) with `ALL`, `code`, and `from_code`; `PERCENTILE_WINDOW` (252), `PERCENTILE_MIN_SAMPLES` (50), `STATE_CHANGE_WINDOW` (50), and `COMPACT_STATE_THRESHOLD` (0.1); `clamp_unit`, `sign`, `compact_state`, `empirical_percentile`, `StateCounters`, and `StateStats`.
  - `crates/quantrule-model/src/lib.rs`: exposes `pub mod kernel` and updates the crate docs.
  - One clarifying sentence in contract §9 for the counter age, null freeze, and code-bearing change window.
- **Verification and evidence (observed in this worktree):**
  - `cargo fmt --check`: exit 0, no output.
  - `cargo clippy --all-targets -- -D warnings`: exit 0, no warnings. An earlier run failed on `clippy::manual_clamp`; `clamp_unit` was changed to `value.clamp(-1.0, 1.0)` and the check then passed.
  - `cargo test`: exit 0, 265 tests passed, 0 failed — 90 data unit, 23 model unit, 93 shell unit, 47 shell integration, 2 data doctests, and 10 model doctests.
  - `cargo doc --no-deps`: exit 0, no warnings.
  - Toolchain: `rustc 1.99.1` and `cargo 1.99.1`, invoked through `~/.cargo/bin/cargo` because `cargo` is not on the default `PATH`.
  - The model unit tests cover the registry codes and round trips, clamp and sign boundaries including NaN and infinity, compact-state inclusivity at ±0.1, the hand-computed percentile fixture, the 49/50 sample floor, the 252-sample bound, order-independent determinism, non-finite filtering, inclusive comparison, a valid zero, counter start/age/change/freeze/window eviction, and an all-null counter series.
  - No benchmark was added. The kernel's largest allocation is the bounded 252-element percentile copy.
- **Learning:**
  - **Rejected:** `HashMap` or `HashSet` anywhere in the kernel. The change window is a fixed `[bool; 50]` ring, so iteration order cannot affect the result.
  - **Rejected:** sorting the caller's slice in place. The percentile copies at most 252 finite values and sorts its own buffer, so the caller's data and order are untouched.
  - **Rejected:** a `last_code` accessor on `StateCounters`. It was removed after review because no acceptance criterion needed it and it exposed internal state without a consumer.
  - **Rejected:** `Default` on `StateStats`. It was removed after review so a missing result cannot be silently defaulted to zero counters.

## Entry: review and corrections

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash Free). The review was a read-only AI pass in the same session; it was not an independent human review or an independent model judgment.
- **Intent and boundary:** Inspect the complete diff for contract deviations, correctness defects, missing tests, and unstated assumptions. The review was read-only; corrections were applied only within #17's approved scope.
- **Findings and corrections:**
  - **Major — percentile scanned the full history.** The first version iterated every sample while retaining only the last 252 finite values, which would be O(n²) if M2 passed full history each bar. Corrected to scan backward and stop after 252 finite values, then reverse into bar order. The added `non_finite_samples_do_not_consume_a_window_slot` test covers the interaction between filtering and the bound.
  - **Minor — NaN pass-through was under-documented.** `clamp_unit` and `sign` return NaN unchanged. The rustdoc now states that an axis must check availability first and that the NaN pass-through is a defect signal, not a supported output.
  - **Minor — docs narrowed the contract.** `UnavailableReason` now says "a measurement" rather than "its primary value"; reason 3 mentions a degenerate progression target; readiness 2 mentions the percentile sample requirement; `compact_state` names Direction and Momentum and excludes Structure and Progression.
  - **Minor — test gaps.** Added an empty/one-sample percentile case, the non-finite window-slot case, an all-null counter series, and `Default` matching `new`. The counter determinism test was already present.
  - **Not changed — generic counter codes.** The counter accepts any `i8` because it is deliberately axis-agnostic; the axis tests own the published code ranges. This keeps axis knowledge out of the kernel.
- **Verification and evidence:** After the corrections, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo doc --no-deps` were run again and all exited 0. The model test count rose from 19 to 23 unit tests; the doctest count remained 10.
- **Learning:**
  - The contract's percentile definition is precise about the 252 sample window and the 50-sample floor but does not state the scan direction. Under the documented bar-order precondition, scanning backward selects the same 252 values as scanning forward and avoids a per-bar full-history walk; the caller still owns the "strictly before the current bar" precondition, and that is stated in the rustdoc.
  - **Not claimed:** no axis, reading, event, aggregate, market state, or shell behavior was demonstrated. Those remain M2–M13, and the plan records that limitation.

## Entry: commit and pull request

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash Free).
- **Intent and boundary:** Deliver the verified #17 work after explicit human authorization for the commit, push, and pull request. No code or contract change was made in this entry.
- **Verification and evidence:**
  - Commit `a751ac0` (`feat: add the model numeric kernel`) contains the kernel, the crate module exposure, the contract clarification, the plan, and this log.
  - The branch `feat/17-model-numeric-kernel` was pushed to `origin`.
  - Pull request [#58](https://github.com/quant-rule/quant-rule/pull/58) is open against `main`, closes #17, and is mergeable.
  - The `sync` check on the pull request passed.
  - Issue #17 was moved to `Done` on the `QuantRule` board; the board shows `Done`, `Issue`, `Closed`, and `Closed date` 2026-10-24. The issue was closed by the project automation at `2026-10-24T02:53:59Z`.
- **Learning:**
  - The board's GitHub automation filled the pull request's `Item type` after it appeared, as the project setup intends. The pull request's board `Status` remains `Todo` while it is open; prior pull requests show `Done` once merged and closed.
  - The human still owns review and merge authorization. The pull request is not merged.

## Current state

- The worktree is clean on `feat/17-model-numeric-kernel` at the delivery commit.
- The work is committed, pushed, and open for review as PR #58. Issue #17 is `Done` on the board and closed on GitHub.
- Merge is not performed; it awaits human approval.
