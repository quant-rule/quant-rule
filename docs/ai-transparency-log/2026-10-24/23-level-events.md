# AI Transparency Log: Level Events (M7)

- Work item: [#23](https://github.com/quant-rule/quant-rule/issues/23) — feat: build level events
- Plan: [`docs/execution-plans/2026-10-24/23-level-events.md`](../../execution-plans/2026-10-24/23-level-events.md)
- Program plan: [`docs/execution-plans/2026-10-24/14-model-engine-completion.md`](../../execution-plans/2026-10-24/14-model-engine-completion.md)
- Date: 2026-10-24 (UTC)

## Entry: implementation

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Implement the Level event detector and the shared event record under `crates/quantrule-model/src/events/`. No shell change, no axis change, no new dependency.
- **Verification and evidence:**
  - Added `events/mod.rs` and `events/level.rs`, and exposed `events` from `lib.rs`.
  - Added 16 unit tests covering test, cross, break, precedence, direction, reference codes, bounded reference set, ATR unavailable, zero ATR, empty, single candle, availability, causality, and determinism.
  - Added `tests/level_events_golden.rs` with a golden fixture, source-code check, availability check, prefix causality, and determinism.
  - `cargo test --workspace`: 575 passed, 0 failed.
  - `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
- **Learning:**
  - The first test drafts changed the EMA level between bars to manufacture a cross; the previous close was still above the level, so the test produced a test event instead. The corrected tests move the previous close, not the level.
  - A single candle with manually supplied EMA/ATR legitimately produces a test event; the edge-case test now uses the real `Readings::compute`, where the readings are unavailable.
  - The ATR availability rule is an interpretation needed to preserve precedence: a cross cannot be known to be a cross rather than a break without the ATR threshold. It is recorded in contract §7.2 with the closed-zone and event-order clarifications.
  - `ConfirmedLevelRegistry` already provides the contract's stable `1000 + n` codes and the availability order; reusing it avoids a second source of truth.

## Entry: golden fixture

- **Date and tool:** 2026-10-24 (UTC). opencode with an external Python generator.
- **Intent and boundary:** Produce an independent expected-event fixture from the checked-in `readings_golden.csv` readings and pivots, without calling the Rust implementation. The generator is `/tmp/opencode/level_events_fixture.py` and is not checked in.
- **Verification and evidence:**
  - `level_events_golden.csv`: 438 events, 11 columns, LF.
  - Subtypes: 334 tests, 43 crosses, 61 breaks.
  - Sources: EMA20 143, EMA50 81, minor pivots 132, major pivots 82.
  - First event at bar 14, the first bar with ATR 14; last at bar 259.
- **Learning:**
  - The fixture's large test count is a consequence of the wide 0.2 ATR zone around the seven references, not a defect. The golden test compares every event in order, so a precedence or boundary change is visible.
  - The generator reproduces the registry's confirmation order (minor before major at one bar) and the fixed reference order used by the detector.

## Final verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 575 passed, 0 failed (data 90, model unit 241, direction 14, level 5, momentum 14, progression 18, readings 11, structure 15, shell unit 93, shell CLI 47, data doc 2, model doc 25).
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.

Status: `Verified`. Commit, push, pull request, and board transition remain gated by §9 of `AGENTS.md`.
