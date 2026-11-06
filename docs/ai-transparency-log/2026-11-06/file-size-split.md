# AI Transparency Log — File-Size Split

- Date: 2026-11-06 (UTC)
- Plan: [`docs/execution-plans/2026-11-06/file-size-split.md`](../../execution-plans/2026-11-06/file-size-split.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash), single session
- Intent and boundary: behavior-neutral refactor of the oversized Rust source files plus a file-size CI gate. No formula, event, strategy, or output change.

## 1. Preflight

The following files exceeded 1,000 lines:

| File | Lines |
|---|---:|
| `crates/quantrule-model/src/events/extended.rs` | 2,317 |
| `crates/quantrule-model/src/readings/extended.rs` | 1,713 |
| `crates/quantrule-backtest/src/execution/engine/mod.rs` | 1,197 |
| `crates/quantrule-backtest/src/execution/engine_tests.rs` | 1,049 |
| `crates/quantrule-model/src/spec/v2.rs` | 1,305 |
| `crates/quantrule-data/src/timeframe_set.rs` | 1,259 |
| `crates/quantrule-model/src/events/signal.rs` | 1,035 |

## 2. Implemented work

- Moved inline tests out of `spec/v2.rs`, `timeframe_set.rs`, and `events/signal.rs` into child `tests.rs`/`edge_tests.rs` files.
- Split `events/extended.rs` into `{mod,shared,vwap,profile,participation,flow,levels,divergence,tests}.rs`.
- Split `readings/extended.rs` into `{mod,shared,vwap,profile,participation,flow,structure,anatomy,tests}.rs`.
- Split `execution/engine/mod.rs` into `{mod,entry,exit,records}.rs`; kept `bracket_entry.rs`.
- Split `engine_tests.rs` into `{mod,entry,bracket,risk,determinism}.rs` with shared fixtures in `mod.rs`.
- Added `tools/check_file_size.py` and wired it into `README.md` and CI.
- Updated the architecture size snapshot and the test evidence line.

## 3. Verification

Observed on 2026-11-06:

| Check | Result |
|---|---|
| `python3 tools/check_file_size.py` | pass; every Rust source file is at or under 1,000 lines |
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace --no-fail-fast` | pass; all test binaries and doctests, 0 failed |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | pass |
| `python3 tools/check_registry.py` | pass |
| `python3 tools/check_validation.py` | pass |
| `python3 tools/check_links.py` | pass |
| `python3 tools/check_terms.py` | pass |
| `python3 tools/check_legacy.py` | pass |
| `git diff --check` | pass |

No fixture changed. Test counts remain the same as before the split. The largest remaining source file is 973 lines.

## 4. Learning and corrections

- Extracting a function by a line range cut the closing brace of `ExtendedEventSeries::events` and split `push`; both were corrected before compiling.
- The first `shared.rs` extraction lost the `pub(super)` visibility added earlier; recreated with the visibility in place.
- `readings/extended/flow.rs` initially kept an `#[allow(clippy::type_complexity)]` attribute that belonged to `participation_series`; it was moved to the correct module.
- `records.rs` initially had an extra closing brace from the parent `impl`; removed.
- `bracket_entry.rs` imported the moved helpers from `super`; changed to `super::records` and the parent re-export list.
- No formula, fixture, or public behavior changed.
