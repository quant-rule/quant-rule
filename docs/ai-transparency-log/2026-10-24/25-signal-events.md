# AI Transparency Log: Signal Events (M9)

- Work item: [#25](https://github.com/quant-rule/quant-rule/issues/25) — feat: build signal events
- Plan: [`docs/execution-plans/2026-10-24/25-signal-events.md`](../../execution-plans/2026-10-24/25-signal-events.md)
- Program plan: [`docs/execution-plans/2026-10-24/14-model-engine-completion.md`](../../execution-plans/2026-10-24/14-model-engine-completion.md)
- Date: 2026-10-24 (UTC)

## Entry: implementation

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Implement the Signal event detector under `crates/quantrule-model/src/events/signal.rs`. No shell change, no axis change, no new dependency.
- **Verification and evidence:**
  - Added threshold crossings, the three EMA pairs, and Direction/Momentum/Structure state changes.
  - Added 20 unit tests covering strict equality, all three EMA pairs, state-change boundaries, coincident conditions, no-axis mutation, causality, determinism, and empty/single.
  - Added `tests/signal_events_golden.rs` with a 109-event fixture, source coverage, causality, and determinism.
  - `cargo test --workspace`: 621 passed, 0 failed.
  - `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
- **Learning:**
  - The first EMA test expected the faster EMA's reference code; the slower EMA is the reference, so the assertion was corrected to EMA 50's code.
  - The issue's two clauses — "one underlying occurrence never produces two signal events" and "the compact state change fires exactly when the published code changes" — conflict if a state change is suppressed on a threshold bar. The implementation emits every code change and treats distinct conditions as distinct relationships; the plan records the interpretation and a test proves both events fire.
  - The checked-in Direction axis has no state change after its first code appears, so the fixture cannot cover a Direction signal. The unit test covers it; the fixture covers Momentum and Structure.
  - The fixture's EMA crossovers only exercise the 20/50 pair; the unit test covers all three pairs.

## Entry: golden fixture

- **Date and tool:** 2026-10-24 (UTC). opencode with an external Python generator.
- **Intent and boundary:** Produce independent expected Signal events from `readings_golden.csv` and the checked-in direction, momentum, and structure fixtures. The generator is `/tmp/opencode/signal_fixture.py` and is not checked in.
- **Verification and evidence:**
  - `signal_events_golden.csv`: 109 events, 11 columns, LF.
  - Subtypes: 38 threshold, 2 series crossover, 69 state changes.
  - Sources: RSI 38, EMA 20 2, Momentum 56, Structure 13.
  - First event at bar 15; last at bar 256.
- **Learning:**
  - The axis fixtures make the signal fixture independent of the Signal detector while staying downstream of the already-verified axis fixtures.
  - The Direction state code is `-1` for 61 bars and null otherwise, with no change after the first code; an all-axis fixture cannot cover every axis state change.

## Final verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 621 passed, 0 failed (data 90, model unit 276, direction 14, divergence 4, level 5, momentum 14, progression 18, readings 11, signal 5, structure 15, shell unit 93, shell CLI 47, data doc 2, model doc 27).
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.

Status: `Verified`. Commit `8193fb4` is pushed on `feat/25-signal-events`, pull request [#85](https://github.com/quant-rule/quant-rule/pull/85) is open, and #25 is `Done` on the board. Merge and cleanup remain with the human.
