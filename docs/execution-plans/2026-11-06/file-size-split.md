# File-Size Split — Behavior-Neutral Module Refactor

- Date: 2026-11-06 (UTC)
- Status: Verified
- Issue: none (human-approved task slug)
- Branch: `refactor/file-size-split`
- Worktree: `../quant-rule--file-size-split`
- Claim type: behavior-neutral refactor plus a CI file-size gate. No formula, event, strategy, or output change.

## 1. Intent

Bring every Rust source file at or under the 1,000-line rule in `AGENTS.md` section 3, split along each module's existing seams, and add a gate that prevents a new oversized source file from entering the tree.

## 2. Current state

| File | Lines | Problem |
|---|---:|---|
| `crates/quantrule-model/src/events/extended.rs` | 2,317 | Detector families and tests in one file |
| `crates/quantrule-model/src/readings/extended.rs` | 1,713 | Reading families and tests in one file |
| `crates/quantrule-backtest/src/execution/engine/mod.rs` | 1,197 | Entry, exit, record, and public API in one impl |
| `crates/quantrule-backtest/src/execution/engine_tests.rs` | 1,049 | All engine scenarios and shared fixtures in one file |
| `crates/quantrule-model/src/spec/v2.rs` | 1,305 | 813 production lines plus 492 inline test lines |
| `crates/quantrule-data/src/timeframe_set.rs` | 1,259 | 487 production lines plus 772 inline test lines |
| `crates/quantrule-model/src/events/signal.rs` | 1,035 | 530 production lines plus 505 inline test lines |

## 3. Scope

Included:

- Move inline tests to child `tests.rs` files for `spec/v2.rs`, `timeframe_set.rs`, and `events/signal.rs`.
- Split `events/extended.rs` into an orchestrator plus VWAP, profile, participation, flow, levels, divergence, and shared modules.
- Split `readings/extended.rs` into an orchestrator plus VWAP, profile, participation, flow, structure, anatomy, and shared modules.
- Split `execution/engine/mod.rs` into public API, entry, exit, and record modules; keep `bracket_entry.rs`.
- Split `engine_tests.rs` into scenario modules with shared fixtures retained in `mod.rs`.
- Add `tools/check_file_size.py` and wire it into CI and the README.
- Update the architecture size snapshot after the split.

Excluded:

- Any formula, threshold, event, strategy, portfolio, or output change.
- Any new dependency.
- Any fixture regeneration.
- Any change to the command set or public behavior.

## 4. Assumptions and uncertainty

- Golden, causality, determinism, and shell tests prove the refactor is behavior-neutral.
- A split that requires a logic change is outside this plan; stop and re-plan.
- Visibility changes are limited to `pub(super)`/`pub(crate)` needed for child modules to reach shared helpers.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Split direction | By existing detector/reading/engine seam | Each module keeps one cohesive responsibility |
| Test placement | Child `tests.rs` or scenario submodules | Production files shrink without moving tests out of the crate |
| Shared helpers | `shared.rs` or `mod.rs` with narrow visibility | No broad public API expansion |
| File-size gate | Python standard-library checker, 1,000-line threshold | No new dependency; fails CI on a new violation |
| Exceptions | None after the split | The rule is enforced rather than documented only |

## 6. Acceptance criteria

- `python3 tools/check_file_size.py` reports zero violations.
- No file under `crates/**/*.rs` exceeds 1,000 physical lines.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`, and `cargo doc` pass.
- No golden fixture changes; `git diff` contains moves, visibility changes, module declarations, and the new checker only.
- The existing event-order, MFI dedup, and determinism tests still pass.
- CI runs the file-size gate.

## 7. Work breakdown

| Step | Work | Paths |
|---:|---|---|
| 1 | Move inline tests for spec v2, timeframe set, and signal | `spec/v2/tests.rs`, `timeframe_set/tests.rs`, `events/signal/tests.rs` |
| 2 | Split extended events | `events/extended/{mod,vwap,profile,participation,flow,levels,divergence,shared,tests}.rs` |
| 3 | Split extended readings | `readings/extended/{mod,vwap,profile,participation,flow,structure,anatomy,shared,tests}.rs` |
| 4 | Split the execution engine | `execution/engine/{mod,entry,exit,records}.rs` |
| 5 | Split engine tests | `execution/engine_tests/{mod,entry,exits,risk,bracket}.rs` |
| 6 | Add the file-size checker and CI step | `tools/check_file_size.py`, `README.md`, `.github/workflows/ci.yml` |
| 7 | Update the architecture size snapshot | `docs/architecture/characteristics.md` |

## 8. Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
python3 tools/check_file_size.py
python3 tools/check_registry.py
python3 tools/check_validation.py
python3 tools/check_links.py
python3 tools/check_terms.py
python3 tools/check_legacy.py
git diff --check
```

Additional evidence: review the diff with `git diff --word-diff` and confirm no formula or fixture change.

## 9. Risks and recovery

- A moved function changes behavior: stop; the golden and determinism suites must pass without fixture edits.
- A visibility change exposes internals: keep it `pub(super)`/`pub(crate)` and review the boundary.
- A test module loses a shared fixture: keep fixtures in the child `mod.rs`.
- Rollback is the branch; no migration or remote state.

## 10. Stop conditions

- A split requires a logic change.
- A fixture changes.
- A test fails for a reason outside the move.
- The file-size gate needs an exception.


## 11. Verification record

Observed on 2026-11-06 in the task worktree:

| Check | Result |
|---|---|
| `python3 tools/check_file_size.py` | pass; every Rust source file is at or under 1,000 lines |
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace --no-fail-fast` | pass; all test binaries and doctests, 0 failed |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | pass |
| `python3 tools/check_registry.py` | pass; 46 indicators, 143 events, 101 parameters |
| `python3 tools/check_validation.py` | pass; 188 claims for 46 indicators and 142 active events |
| `python3 tools/check_links.py` | pass; 550 Markdown files |
| `python3 tools/check_terms.py` | pass; 325 files |
| `python3 tools/check_legacy.py` | pass; 353 current files |
| `git diff --check` | pass |

No golden fixture changed. The test counts are unchanged from the pre-refactor run, and the event-order, MFI dedup, determinism, and shell tests still pass. The largest remaining Rust source file is 973 lines (`portfolio/ledger_tests.rs`).
