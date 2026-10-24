# Execution Plan: Divergence Events (M8)

- Issue: [#24](https://github.com/quant-rule/quant-rule/issues/24) — feat: build divergence events
- Branch: `feat/24-divergence-events`
- Pull request: [#84](https://github.com/quant-rule/quant-rule/pull/84)
- Status: Verified
- Date: 2026-10-24 (UTC)
- Program plan: [Completion plan (M2–M14)](14-model-engine-completion.md)
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/24-divergence-events.md`](../../ai-transparency-log/2026-10-24/24-divergence-events.md)

## 1. Intent

Implement the Divergence event type: consecutive confirmed price pivots of the same kind disagree with RSI 14 around them. The event is emitted only when the newer pivot is confirmed, records both occurrence and availability bars, and never changes an axis value.

## 2. Current state

- `main` is at `6ca964d`; Level events (#23) are merged.
- The readings expose RSI 14 and confirmed pivots at both scales.
- `ConfirmedLevelRegistry` assigns stable reference codes and is ordered by confirmation bar.
- `EventRecord` and `EventType` are available from #23.

## 3. Scope

Included:
- The four-subtype matrix from contract §7.3.
- Consecutive confirmed pivots at one configured scale, same kind only.
- The two-bar RSI window, min for lows and max for highs.
- Golden fixtures at both scales; unit, causality, determinism, availability, and no-axis-mutation tests.
- Contract clarification for scale, source/reference, missing RSI, and ATR evidence.

Excluded:
- Signal events (#25).
- Aggregation, state, shell (#27–#29).
- Any change to the RSI formula or an axis value.

## 4. Assumptions and uncertainty

- The detector runs at one scale per run; the default is major. Minor and major pivots are never compared with each other.
- The event's source is RSI (`source_code` 4); `reference_code` is the newer confirmed swing level, whose scale is recoverable from the registry.
- If any bar in either two-bar RSI window has no value, the window is unavailable and no divergence is inferred.
- ATR is evidence, not a condition: `price_delta_atr` is null when ATR is unavailable or zero, but the event is still emitted.
- The RSI window is clamped to the series; a confirmed pivot's window is in bounds.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Pivot scale | Minor; major; both; configurable | Configurable, default major | Major is the structural sequence; a run may request minor without blending scales |
| D2 | Source/reference | Source 7/8, reference level; source 4, reference null | Source 4, reference = newer level | Contract says RSI is the source; the level keeps the scale traceable |
| D3 | Missing RSI in a window | Use available values; unavailable | Unavailable, no event | Min/max over an incomplete window would invent evidence |
| D4 | ATR unavailable | Skip event; null evidence | Null `price_delta_atr`, keep event | The divergence does not depend on ATR; ATR is recorded evidence |
| D5 | Pivot comparison | Same kind across scales; same scale and kind | Same scale and kind | The contract compares consecutive pivots of the same kind; scales are distinct sequences |
| D6 | Event order | Reference-code order; availability order | Availability order | Each pivot confirms once; the order is deterministic |

## 6. Acceptance criteria

- The four subtypes and direction codes match the contract's matrix.
- No event is emitted before the newer pivot is confirmed.
- No event when a comparable pivot or either RSI window is unavailable.
- The event changes no axis value and is not a second RSI vote.
- The newer level's reference code is recorded; source is RSI 14.
- A causality test, a determinism test, a golden fixture at both scales, and edge cases pass.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Read contract §7.3, the pivot series, registry, and shared event record.
2. Add `events/divergence.rs` with the config, subtype registry, window summary, and matrix.
3. Add 15 unit tests for every subtype, missing RSI, unavailable/zero ATR, availability, causality, determinism, and no axis mutation.
4. Generate an independent golden fixture at both scales from the readings fixture.
5. Add the golden, source, availability, causality, and determinism integration tests.
6. Clarify contract §7.3 for scale, source/reference, missing RSI, and ATR evidence.
7. Run the full workspace checks.
8. Commit, push, open the PR, move #24 to Done, merge, and clean up.

## 8. Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 595 passed, 0 failed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Divergence unit tests: 15 passed.
- Divergence golden tests: 4 passed.

The golden fixture contains 11 events: 5 minor, 6 major. Subtypes covered: regular bullish (1), regular bearish (2), hidden bearish (4). Hidden bullish (3) is covered by the unit test; the checked-in readings have no such pair.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| Comparing across scales | The config filters one scale; a test computes both separately |
| Event before confirmation | Availability is the pivot's `available_at`; an events-at-bar test asserts no event at 9 |
| Incomplete RSI window | Any missing value returns `None`; a unit test covers it |
| ATR evidence changes the event decision | ATR is only evidence; unavailable/zero tests keep the event with a null primary value |
| Hidden bullish not in the fixture | Unit test covers the subtype explicitly |
| Golden fixture is a Rust snapshot | Independent Python generator; checked-in data |

## 10. Stop conditions

- A contract change beyond the recorded clarifications.
- A new dependency.
- An RSI or pivot change outside #24.
- A failing check that cannot be corrected within scope.

## 11. Delivery

| Item | Value |
|---|---|
| Commit | `a6ca63a` — `feat: build divergence events` |
| Branch | `feat/24-divergence-events` |
| Pull request | [#84](https://github.com/quant-rule/quant-rule/pull/84), open against `main` |
| Board | Issue #24 moved to `Done`; the workflow closes the issue |

Merge, release, tag, and deployment remain with the human.
