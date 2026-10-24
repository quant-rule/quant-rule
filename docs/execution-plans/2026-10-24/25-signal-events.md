# Execution Plan: Signal Events (M9)

- Issue: [#25](https://github.com/quant-rule/quant-rule/issues/25) — feat: build signal events
- Branch: `feat/25-signal-events`
- Pull request: pending
- Status: Verified
- Date: 2026-10-24 (UTC)
- Program plan: [Completion plan (M2–M14)](14-model-engine-completion.md)
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/25-signal-events.md`](../../ai-transparency-log/2026-10-24/25-signal-events.md)

## 1. Intent

Implement the Signal event type: RSI threshold crossings, configured EMA/EMA crossovers, and compact Direction, Momentum, and Structure state changes. A price/EMA crossing remains a Level event; a Signal is evidence and never alters an axis.

## 2. Current state

- `main` is at `58e8130`; Level (#23) and Divergence (#24) are merged.
- The four axes publish per-bar `state_code` values; the readings publish RSI and the three EMAs.
- The shared event record and the bounded source-code registry exist.

## 3. Scope

Included:
- RSI 30/50/70 threshold crossings.
- EMA pairs 20/50, 50/200, 20/200.
- Direction, Momentum, and Structure state changes.
- Golden fixture, unit, causality, determinism, no-axis-mutation, and edge-case tests.
- Contract clarification for thresholds, EMA pairs, equality, state-change boundaries, and condition ordering.

Excluded:
- Level and Divergence changes (#23, #24).
- Aggregation, market state, shell (#27–#29).
- Any change to an axis value or reading formula.

## 4. Assumptions and uncertainty

- A threshold crossing is strict: equality on either bar is not a crossing.
- An EMA cross is up when the previous fast is at or below the slow and the current fast is above it; down is symmetric. Equality on the current bar is not a cross.
- A compact state change fires only between two code-bearing bars. The first code, a transition from null, and a transition to null do not emit.
- Distinct signal conditions can coincide on one bar. They are not duplicates because each names a different defined relationship; the state change still fires exactly on the bar the code changes.
- A price/EMA crossing is never emitted by the Signal detector; the Level detector owns it.
- The golden fixture's Direction code never changes after its first appearance, so Direction state changes are covered by the unit test; the fixture covers Momentum and Structure.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Cross-category dedup | Suppress state changes on threshold/EMA bars; emit all distinct conditions | Emit all distinct conditions | The state change must fire exactly when the code changes; each condition is a different defined relationship |
| D2 | EMA source/reference | Source 9 for all; faster/slower | Faster EMA source, slower EMA reference | Keeps the pair traceable through the existing source/reference registries |
| D3 | Previous equality | Strict previous side; at-or-equal | At-or-equal on the previous bar | A cross from equality is still a crossing; current equality remains no cross |
| D4 | State null boundaries | Count null as a state; ignore | Ignore | A null is not a published code; the counters also freeze |
| D5 | Structure direction | Use the code as direction; use its sign | Sign of the code | Direction code is -1, 0, +1; primary/reference retain the -2..+2 codes |
| D6 | Event order | Reference-code order; fixed condition order | Thresholds, EMA pairs, state axes | Matches the detector's fixed evaluation order and is deterministic |

## 6. Acceptance criteria

- The three signal subtypes and their sources match the contract.
- Threshold crossings are strict; EMA crosses use the recorded equality rule.
- State changes fire exactly when the published code changes, and not on first/null transitions.
- A price/EMA crossing produces no Signal.
- No underlying condition emits twice on one bar.
- A causality test, a determinism test, a golden fixture, and edge cases pass.
- No axis value changes.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Read contract §7.4 and the axis `state_code` interfaces.
2. Add `events/signal.rs` with the config, subtype registry, thresholds, EMA pairs, and state changes.
3. Add 20 unit tests for thresholds, EMA pairs, equality, state boundaries, no-axis mutation, causality, determinism, and edge cases.
4. Generate an independent golden fixture from the checked-in readings and axis fixtures.
5. Add the golden, source coverage, causality, and determinism integration tests.
6. Clarify contract §7.4.
7. Run the full workspace checks.
8. Commit, push, open the PR, move #25 to Done, merge, and clean up.

## 8. Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 621 passed, 0 failed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Signal unit tests: 20 passed.
- Signal golden tests: 5 passed.

The golden fixture contains 109 events: 38 threshold crossings (source 4), 2 EMA crossovers (source 1), 56 Momentum state changes (source 11), and 13 Structure state changes (source 10). Direction state changes and the 50/200 and 20/200 EMA pairs are covered by unit tests.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| A state change is suppressed as a duplicate | The detector emits every code change; a unit test asserts a threshold and a Momentum state change both fire |
| Equality is handled inconsistently | Strict threshold rule and at-or-equal previous EMA rule are tested |
| A price/EMA cross becomes a Signal | The detector only compares EMA to EMA; a test asserts no series event |
| Event order drifts | Fixed condition order; the golden comparison is order-sensitive |
| Direction coverage is absent from the fixture | Explicit Direction unit test; the post-mortem records the fixture gap |

## 10. Stop conditions

- A contract change beyond the recorded clarification.
- A new dependency.
- An axis/reading change outside #25.
- A failing check that cannot be corrected within scope.
