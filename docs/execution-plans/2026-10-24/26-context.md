# Execution Plan: Volatility and Participation Context (M10)

- Issue: [#26](https://github.com/quant-rule/quant-rule/issues/26) — feat: build the volatility and participation context
- Branch: `feat/26-context`
- Pull request: [#86](https://github.com/quant-rule/quant-rule/pull/86)
- Status: Verified
- Date: 2026-10-24 (UTC)
- Program plan: [Completion plan (M2–M14)](14-model-engine-completion.md)
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/26-context.md`](../../ai-transparency-log/2026-10-24/26-context.md)

## 1. Intent

Implement the volatility and participation diagnostic: `atr_ratio`, `rvol_ratio`, and the four-bucket `context_code`. It is outside the four axes, no axis reads it, and it never votes.

## 2. Current state

- `main` is at `a51a7f3`; Level, Divergence, and Signal events are merged.
- The readings publish ATR 14 and RVOL 20 as per-bar series.
- The contract §12 fixes the ratio formulas and the four-bucket table.

## 3. Scope

Included:
- `atr_ratio = ATR14 / SMA20(ATR14)`.
- `rvol_ratio = RVOL20`.
- The expanding, coiling, settling, and high-range/low-participation codes.
- Golden fixture, causality, determinism, no-axis-read, and edge-case tests.
- Contract clarification for the 20-value baseline and the zero baseline.

Excluded:
- Axis, event, aggregation, or shell changes.
- Any change to the ATR or RVOL formulas.

## 4. Assumptions and uncertainty

- The ATR baseline is the 20 most recent ATR values including the current bar; it is null before 20 values exist.
- A zero baseline makes `atr_ratio` null; a null ratio makes `context_code` null, while the other ratio is retained.
- `rvol_ratio` is null when volume or the volume baseline is unavailable.
- The code is a pure function of the two ratios; no axis reads it.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | ATR baseline window | Exclude current bar; include it | Include the current bar | `SMA20(ATR14)` at the current bar is the natural baseline and is causal |
| D2 | Zero baseline | Divide and get infinity; null | Null ratio and code | A zero denominator has no usable ratio; null is not zero |
| D3 | Missing ATR in the window | Average available values; null | Null | A partial baseline would invent a ratio |
| D4 | Raw ratios on null code | Null both; retain | Retain both | The contract says consumers retain the underlying values |
| D5 | Module location | Under `events`; top-level `context` | Top-level `context` | The context is not an event and is read by no axis |

## 6. Acceptance criteria

- The four codes match the contract's table, including the `>= 1` and `< 1` boundaries.
- `atr_ratio` is null before 20 ATR values and when the baseline is zero.
- `rvol_ratio` is null when volume or its baseline is unavailable.
- The code is null when either ratio is unavailable.
- The raw ratios are retained when the code is null.
- A causality test, a determinism test, a golden fixture, and edge cases pass.
- No axis output depends on the context.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Read contract §5 and §12 and the readings API.
2. Add `context.rs` with the bar, series, baseline, and code table.
3. Add 10 unit tests for the four codes, baseline boundaries, zero baseline, missing values, empty/single, causality, determinism, and no axis reads.
4. Generate an independent golden fixture from the readings fixture.
5. Add the golden, length, retention, causality, and determinism integration tests.
6. Clarify contract §12 for the baseline and zero denominator.
7. Run the full workspace checks.
8. Commit, push, open the PR, move #26 to Done, merge, and clean up.

## 8. Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 637 passed, 0 failed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Context unit tests: 10 passed.
- Context golden tests: 5 passed.

The golden fixture has 260 bars: 33 null codes before the ATR baseline exists, then 35 expanding, 75 coiling, 45 settling, and 72 high-range/low-participation bars. All four codes are covered.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| Baseline boundary off by one | A test asserts null at bar 18 and a ratio at bar 19 |
| Zero baseline divides by zero | The ratio is null and a test covers it |
| A missing ATR is averaged away | Any missing value in the window returns null |
| Context leaks into an axis | The module takes only readings; a test recomputes the axes before and after |
| Golden fixture is a Rust snapshot | Independent Python generator; checked-in data |

## 10. Stop conditions

- A contract change beyond the recorded clarification.
- A new dependency.
- An ATR/RVOL formula change outside #26.
- A failing check that cannot be corrected within scope.

## 11. Delivery

| Item | Value |
|---|---|
| Commit | `a224d09` — `feat: build the volatility and participation context` |
| Branch | `feat/26-context` |
| Pull request | [#86](https://github.com/quant-rule/quant-rule/pull/86), open against `main` |
| Board | Issue #26 moved to `Done`; the workflow closes the issue |

Merge, release, tag, and deployment remain with the human.
