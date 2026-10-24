# Execution Plan: Cross-Timeframe Aggregation (M11)

- Issue: [#27](https://github.com/quant-rule/quant-rule/issues/27) — feat: build cross-timeframe aggregation
- Branch: `feat/27-aggregation`
- Pull request: [#87](https://github.com/quant-rule/quant-rule/pull/87)
- Status: Verified
- Date: 2026-10-24 (UTC)
- Program plan: [Completion plan (M2–M14)](14-model-engine-completion.md)
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/27-aggregation.md`](../../ai-transparency-log/2026-10-24/27-aggregation.md)

## 1. Intent

Aggregate the per-timeframe snapshots over the ordered ladder. Direction and Momentum use means and shares; Structure and Progression use histograms and spans; events use counts. Ordinal codes are never averaged, every aggregate names its source timeframes, and the result is order-independent.

## 2. Current state

- `main` is at `3c22247`; #23–#26 are merged.
- The four axes publish per-bar values and diagnostics; the context and event records exist.
- No cross-timeframe input or aggregate output exists yet.

## 3. Scope

Included:
- `TimeframeSnapshot` input with the latest bars, ATR, bounded reference set, events, and leg origin times.
- Direction, Momentum, Structure, Progression, and Event aggregates.
- Level clustering and cross-timeframe leg matching.
- Order-independence, determinism, empty-ladder, and no-double-count tests.
- Contract clarification for share denominators, gradients, cluster ties, stage modes, leg matching, and event windows.

Excluded:
- The assembled market-state snapshot (#28).
- Shell commands (#29).
- Any change to an axis formula.

## 4. Assumptions and uncertainty

- The caller supplies one snapshot per configured timeframe; duplicate timeframes are not expected.
- The aggregator sorts by `timeframe_secs`, so input order does not affect the result.
- Shares use valid values for Direction, Momentum, and Structure; a missing axis is not neutral. Readiness shares use all timeframes.
- Gradients use the configured fastest and slowest endpoints; a null endpoint gives a null gradient.
- Level clusters use references with a positive ATR; the largest cluster is chosen by distinct timeframe count, then span.
- Stage histograms use the retained leg at each scale; the mode breaks ties toward the lower stage.
- A leg match uses active legs only, the same scale and direction, origin times within one slower-timeframe bar, and prices within `0.5 * max(ATR_i, ATR_j)`.
- Event window counts use `available_bar_index` in the last five closed bars.
- The synthetic cases carry explicit expected values in the unit tests; the checked-in CSV market-state fixture is delivered with #28, which assembles these aggregates into the published state.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Snapshot input | Per-bar series; latest snapshot | Latest snapshot with origin times | #27 aggregates the published state; #28 assembles it |
| D2 | Share denominator | All timeframes; valid values | Valid values for axis shares; all timeframes for readiness shares | A missing axis is not neutral; readiness is a completeness share |
| D3 | Gradient endpoint | Nearest valid; configured endpoints | Configured fastest/slowest | The contract names those timeframes; a missing endpoint is null |
| D4 | Stage mode tie | Highest; lowest | Lowest | Deterministic and consistent with ordinal order |
| D5 | Cluster tie | First; largest span | Largest timeframe count, then span | More timeframes is the stronger cluster; span breaks ties |
| D6 | Leg match scale | Cross-scale; same scale | Same scale | Scales are never blended |
| D7 | Active leg | Any retained; lifecycle 1 | Lifecycle 1 | Forming/terminal legs are not active |

## 6. Acceptance criteria

- No ordinal code is averaged anywhere.
- Every aggregate names the timeframes it came from.
- Means and shares use only valid values; a missing axis is not neutral.
- The level clusters use the bounded set and the `0.5 * max(ATR_i, ATR_j)` rule; the span is in ATR.
- The event counts cover the five-bar window and are never treated as votes.
- The aggregates are independent of input order.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Read contract §10, §11, and the axis/event types.
2. Add `aggregate/mod.rs` with the snapshot and simple axis aggregates.
3. Add `aggregate/levels.rs`, `aggregate/events.rs`, and `aggregate/progression.rs`.
4. Add 15 unit tests covering the axis methods, level clusters, leg matches, event windows, empty ladder, order independence, and determinism.
5. Clarify contract §10 for the recorded interpretations.
6. Run the full workspace checks.
7. Commit, push, open the PR, move #27 to Done, merge, and clean up.

## 8. Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 652 passed, 0 failed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Aggregate unit tests: 15 passed.

Limitation: the multi-timeframe CSV fixture is part of #28's assembled market-state golden test. #27 proves the aggregates with explicit expected values, order independence, and determinism.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| A share includes a missing axis | Valid-value denominators; a missing-direction test |
| An ordinal code is averaged | The code only counts codes; stage/code outputs are integers |
| Input order changes the output | Sorting by timeframe and a reversed-input equality test |
| Level chaining is hidden | The span in ATR is published; a chain test |
| A leg match crosses scales | Same-scale requirement and a cross-scale negative test |
| Event window off by one | Window is `latest-4..=latest`; a boundary test |
| Fixture coverage is deferred | The limitation is recorded; #28 adds the CSV state fixture |

## 10. Stop conditions

- A contract change beyond the recorded clarification.
- A new dependency.
- An axis/event formula change outside #27.
- A failing check that cannot be corrected within scope.

## 11. Delivery

| Item | Value |
|---|---|
| Commit | `18ffc5e` — `feat: build cross-timeframe aggregation` |
| Branch | `feat/27-aggregation` |
| Pull request | [#87](https://github.com/quant-rule/quant-rule/pull/87), open against `main` |
| Board | Issue #27 moved to `Done`; the workflow closes the issue |

Merge, release, tag, and deployment remain with the human.
