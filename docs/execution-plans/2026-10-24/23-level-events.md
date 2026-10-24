# Execution Plan: Level Events (M7)

- Issue: [#23](https://github.com/quant-rule/quant-rule/issues/23) — feat: build level events
- Branch: `feat/23-level-events`
- Pull request: pending
- Status: Verified
- Date: 2026-10-24 (UTC)
- Program plan: [Completion plan (M2–M14)](14-model-engine-completion.md)
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Umbrella plan: [`docs/execution-plans/2026-10-21/14-model-engine.md`](../2026-10-21/14-model-engine.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/23-level-events.md`](../../ai-transparency-log/2026-10-24/23-level-events.md)

## 1. Intent

Implement the Level event type: price interacting with a named reference as a test/retest, a cross, or a break. The event record is shared with Divergence and Signal, records both occurrence and availability bars, and never alters an axis value.

## 2. Current state

- `main` is at `143fd19`; the four axes and the shared readings are merged.
- `crates/quantrule-model/src/` has no `events` module.
- `ConfirmedLevelRegistry` already assigns the contract's `1000 + n` reference codes in confirmation-bar order.
- `PivotSeries::confirmed_at` is indexed by availability bar, so a pivot cannot be read before it is knowable.
- The contract's §7.1 record and §7.2 Level rules are the authority.

## 3. Scope

Included:
- The shared `EventType` and `EventRecord`.
- The Level detector with EMA 20/50/200 and the latest confirmed swing high/low at both scales.
- Break, cross, and test/retest precedence.
- A golden fixture, causality, determinism, and edge-case tests.
- The contract clarification for ATR availability, closed zones, and reference order.

Excluded:
- Divergence and Signal events (#24, #25).
- Aggregation, market state, shell exposure (#27–#29).
- Any change to an axis value or to a reading formula.

## 4. Assumptions and uncertainty

- The reference set is exactly seven references per timeframe: three EMAs plus the latest confirmed high and low at each scale.
- A Level event is emitted only when ATR 14 is available: without it the break and test predicates, and therefore the precedence, cannot be resolved. This is recorded in contract §7.2.
- A closed proximity zone means a range that touches the boundary intersects it.
- The latest swing reference is the most recently available confirmed pivot of that scale and kind, not the highest or lowest.
- Event order inside a bar is fixed by the reference order, so the output is deterministic.
- `LevelEventSeries` is a batch result over the input; the bounded live history of 1024 records is a later snapshot concern.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | ATR availability | Emit a cross without ATR; skip all Level events | Skip all when ATR is unavailable | A cross cannot be distinguished from a break without the ATR threshold, so precedence would be violated |
| D2 | Test-zone boundary | Half-open; closed | Closed | The contract says "intersects"; touching the boundary is an intersection |
| D3 | Same-bar event ordering | Reference-code order; fixed reference order | EMA20, EMA50, EMA200, minor high, minor low, major high, major low | Matches the contract's reference-set order and is stable |
| D4 | Event series retention | Batch `Vec`; bounded 1024 only | Batch `Vec`; bounded snapshot later | #23 needs the full event evidence; the snapshot bound belongs to the market state/aggregation work |
| D5 | Swing reference source | Recompute pivots; reuse registry | Reuse `ConfirmedLevelRegistry` | One source of reference codes and availability |

## 6. Acceptance criteria

- The shared event record carries type, subtype, source, timeframe, occurrence, availability, direction, reference, and measured values.
- Seven references are eligible at most; the latest confirmed high/low at each scale is used only from its availability bar.
- Break, cross, and test/retest are detected with the contract's predicates and precedence.
- No more than one event is emitted per reference and bar; an event is appended once.
- A missing ATR produces no Level event; a zero ATR remains a measured value.
- A causality test proves later bars do not change earlier events.
- A determinism test proves repeated runs are identical.
- A golden fixture covers every subtype and both pivot scales.
- Empty, single-candle, short-history, missing-ATR, and zero-ATR cases are covered.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Read the contract, registry, pivots, and readings. (orchestrator)
2. Add `events/mod.rs`: shared event type and record. (implementer)
3. Add `events/level.rs`: reference set, latest-level scan, predicates, precedence. (implementer)
4. Add unit tests for each subtype, precedence, direction, reference codes, availability, ATR, and edge cases. (test engineer)
5. Generate an independent golden fixture from the checked-in readings fixture. (test engineer)
6. Add the golden, causality, determinism, and source-code integration tests. (test engineer)
7. Clarify contract §7.2 for ATR availability, closed zones, and event order. (implementer)
8. Run the full workspace checks and record the evidence. (orchestrator)
9. Commit, push, open the PR, and move #23 to Done. (orchestrator)

## 8. Verification

Commands actually run at the verified state:

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 575 passed, 0 failed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- `cargo test -p quantrule-model --test level_events_golden`: 5 passed.
- Level unit tests: 16 passed.

The golden fixture contains 438 events over 260 bars: 334 tests, 43 crosses, and 61 breaks. It exercises EMA sources 1 and 2, minor pivots (source 7), and major pivots (source 8).

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| ATR unavailable handling changes a published meaning | Clarified in contract §7.2 and covered by tests |
| The latest swing level is read before availability | The registry scan advances only on `available_at <= bar`; a test asserts no swing event before bar 7 in the fixture |
| Precedence is wrong | Dedicated unit tests for break over cross/test and cross over test |
| Event ordering is unstable | Fixed reference order; golden comparison is order-sensitive |
| The golden fixture is a snapshot of the Rust output | The generator is an independent Python implementation; the fixture is checked in as data |
| A later bar changes an earlier event | Prefix and later-candle causality tests |

## 10. Stop conditions

- A contract change beyond the recorded clarifications.
- A new dependency.
- A reading/axis change outside #23.
- A failing check that cannot be corrected within scope.
