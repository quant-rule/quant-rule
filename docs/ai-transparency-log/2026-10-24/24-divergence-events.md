# AI Transparency Log: Divergence Events (M8)

- Work item: [#24](https://github.com/quant-rule/quant-rule/issues/24) — feat: build divergence events
- Plan: [`docs/execution-plans/2026-10-24/24-divergence-events.md`](../../execution-plans/2026-10-24/24-divergence-events.md)
- Program plan: [`docs/execution-plans/2026-10-24/14-model-engine-completion.md`](../../execution-plans/2026-10-24/14-model-engine-completion.md)
- Date: 2026-10-24 (UTC)

## Entry: implementation

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Implement the Divergence event detector under `crates/quantrule-model/src/events/divergence.rs`. No shell change, no axis change, no new dependency.
- **Verification and evidence:**
  - Added the configurable-scale detector, the four-subtype matrix, the two-bar RSI window, and the `EventRecord` output.
  - Added 15 unit tests covering all four subtypes, no previous pivot, no divergence, equal RSI, missing RSI, unavailable ATR, zero ATR, availability, causality, determinism, and no axis mutation.
  - Added `tests/divergence_events_golden.rs` with both scales, source/availability checks, prefix causality, and determinism.
  - `cargo test --workspace`: 595 passed, 0 failed.
  - `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
- **Learning:**
  - The first ATR-unavailable/zero tests used an equal-RSI series, so no divergence existed and the tests failed for the right reason. The tests now use the regular-bullish RSI windows.
  - The detector takes only `Readings`, not candles: pivots carry prices and the RSI/ATR windows come from the readings. This makes the no-axis-mutation test direct.
  - The golden fixture covers three of the four subtypes; the unit test covers hidden bullish explicitly. Coverage claims name the view that proves them.
  - The contract needed four clarifications: scale selection, source/reference meaning, incomplete-window behavior, and ATR-as-evidence.

## Entry: golden fixture

- **Date and tool:** 2026-10-24 (UTC). opencode with an external Python generator.
- **Intent and boundary:** Produce independent expected divergence events at both scales from `readings_golden.csv`. The generator is `/tmp/opencode/divergence_fixture.py` and is not checked in.
- **Verification and evidence:**
  - `divergence_events_golden.csv`: 11 events, 12 columns, LF.
  - By scale: 5 minor, 6 major.
  - By subtype: 2 regular bullish, 3 regular bearish, 6 hidden bearish, 0 hidden bullish.
  - First event at bar 34; last at bar 242.
- **Learning:**
  - The registry order (minor before major at one bar) must be reproduced before filtering by scale; otherwise reference codes drift.
  - The two-bar window is evaluated only for confirmed pivots; for minor pivots the right edge equals the availability bar, and for major it is earlier.

## Final verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 595 passed, 0 failed (data 90, model unit 256, direction 14, divergence 4, level 5, momentum 14, progression 18, readings 11, structure 15, shell unit 93, shell CLI 47, data doc 2, model doc 26).
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.

Status: `Verified`. Commit `a6ca63a` is pushed on `feat/24-divergence-events`, pull request [#84](https://github.com/quant-rule/quant-rule/pull/84) is open, and #24 is `Done` on the board. Merge and cleanup remain with the human.
