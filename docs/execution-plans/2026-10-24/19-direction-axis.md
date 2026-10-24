# Execution Plan: The Direction Axis

- Issue: [#19](https://github.com/quant-rule/quant-rule/issues/19) — feat: build the direction axis
- Branch: `feat/19-direction-axis`
- Pull request: [#60](https://github.com/quant-rule/quant-rule/pull/60)
- Status: Verified
- Date: 2026-10-24 (UTC)
- Program plan: [Completion plan (M2–M14)](14-model-engine-completion.md)
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Umbrella plan: [`docs/execution-plans/2026-10-21/14-model-engine.md`](../2026-10-21/14-model-engine.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/19-direction-axis.md`](../../ai-transparency-log/2026-10-24/19-direction-axis.md)

## 1. Intent

Build the Direction axis as a pure per-bar series. Direction turns the EMA 20/50/200 stack and ATR 14 into the contract's signed alignment score, its compact state, its size percentile, its readiness, its unavailable reason, and its persistence counters.

The outcome is a `quantrule-model::axes::direction` module that publishes one `DirectionBar` per input bar, aligned to the input length. A missing required reading is null with a reason code; a measured zero stays a neutral value. The axis reads only EMA 20/50/200 and ATR 14, so the EMA-distance features and every other axis's output cannot become a second vote.

Affected users: the agents implementing M4–M14, and the human reviewing the Direction axis against the contract.

## 2. Current state

- `main` at `70fc753` contains the M1 kernel and the M2 readings.
- `crates/quantrule-model/src/lib.rs` exposes `kernel` and `readings`; no axis module exists.
- `Readings` publishes `ema20`, `ema50`, `ema200`, `atr14`, `rsi14`, `rvol20`, and pivots. Each scalar series has one entry per input bar and uses `None` for an unavailable measurement.
- The kernel provides `clamp_unit`, `compact_state`, `empirical_percentile`, `StateCounters`, `Readiness`, and `UnavailableReason`.
- The contract fixes the formula in §8.1, the published fields in §9, the diagnostic semantics in §9, and the codes in §14.
- Baseline checks pass on `main`: 360 tests, with `fmt`, `clippy -D warnings`, and `doc` clean.

## 3. Scope

Included:

- `quantrule-model::axes::direction` with `direction`, `direction_state_code`, `direction_size_percentile`, `direction_readiness_code`, `direction_unavailable_reason_code`, `direction_state_age_bars`, and `direction_state_changes_50` as per-bar values.
- The contract's `d_short`, `d_long`, and `direction` formula with `clamp_unit`.
- Null rules for missing EMA/ATR and for a zero or otherwise invalid ATR denominator.
- Percentile history of up to 252 valid absolute Direction magnitudes strictly before the current bar.
- Unit, causality, determinism, edge-case, and golden-fixture tests, including a no-other-axis-read regression test.
- The M3 plan and transparency log.

Excluded:

- Momentum, Structure, Progression, events, context, cross-timeframe aggregation, market state, and shell commands (M4–M13).
- The `ema_distance_atr_*`, cross, and retest fields, which belong to the Structure axis (M5).
- Changes to the contract's formula, thresholds, windows, or code values.
- New dependencies, I/O, clock access, randomness, or shell changes.
- Any change to the readings layer or its golden fixture.

## 4. Assumptions and uncertainty

- The contract is authoritative. The interpretations below are recorded so a reviewer can reject them without reading the code.
- **Reason precedence.** A missing EMA or ATR, or a non-finite EMA, is `InsufficientBars` (1). A non-finite, zero, or negative ATR is `InvalidDenominator` (3). The EMA checks run before the ATR check, so a missing EMA wins when both conditions hold. This keeps a flat series with a measured zero ATR distinct from a series that has not yet produced ATR.
- **Non-finite readings.** The reading layer produces finite values from validated candles. The axis still checks finiteness so a defect cannot publish NaN as a score or a neutral state. A non-finite ATR is treated as an unusable denominator, not as missing history.
- **Percentile history.** The history holds the absolute Direction value of every valid Direction bar, in bar order, including measured zeros. A bar whose Direction is null contributes no sample. The kernel selects the most recent 252 samples and applies the 50-sample minimum.
- **Readiness.** Direction is ready (2) when its value and its percentile are both available. It is provisional (1) when its value is available but the percentile is null. It is unavailable (0) when its value is null.
- **State counters.** The axis passes its published compact code to `StateCounters`. A null Direction freezes both counters. The first code-bearing bar has age 1. A code that differs from the last code-bearing bar counts as a change even when null bars intervened, because changes are counted over code-bearing bars.
- **No other-axis read.** The axis is a function of `Readings` but reads only `ema20`, `ema50`, `ema200`, and `atr14`. The EMA-distance fields do not exist yet; the regression test changes every other reading and asserts that Direction is unchanged, and the input extraction is kept narrow so a later field cannot enter silently.
- **Golden reference.** The expected Direction columns are produced by an independent Python implementation of §8.1 and §9 from the checked-in readings fixture. The fixture's readings were themselves produced by an independent reference, so the Direction test does not derive its expectation from the Rust code under test.

## 5. Decisions

| ID | Decision | Options | Choice | Status |
|---|---|---|---|---|
| D1 | Series shape | Parallel field vectors; one record per bar | One `DirectionBar` per bar in a `DirectionSeries`; each record carries every published Direction field | Approved with this plan |
| D2 | Numeric diagnostics | Publish raw `u8` codes; publish the kernel enums | Publish `Readiness` and `UnavailableReason` in the engine record; callers publish `.code()` | Approved with this plan |
| D3 | ATR zero | Treat as a valid denominator; emit null with reason 3 | Emit null with `InvalidDenominator` (3); a zero Direction remains a measured zero | Approved with this plan |
| D4 | Percentile sample | Include the current bar; use strictly preceding bars | Strictly preceding valid magnitudes, per §9 | Approved with this plan |
| D5 | Golden fixture | Reuse the readings CSV; add a self-contained Direction CSV | A self-contained `direction_golden.csv` with candles and expected Direction fields | Approved with this plan |
| D6 | No-double-count proof | Source-text scan; behavior test over unused readings | A behavior test that changes RSI, RVOL, and pivots and asserts identical Direction output, plus a narrow input extraction | Approved with this plan |

## 6. Acceptance criteria

- `direction` is null whenever a required EMA or ATR is unavailable, with reason 1.
- `direction` is null with reason 3 when ATR is zero or negative.
- A valid Direction of zero is published as `Some(0.0)`, has state code 0, and is not treated as missing.
- Every available Direction is in `[-1, +1]`, and its state code matches the inclusive ±0.1 bucketing.
- The size percentile is null when fewer than 50 valid prior samples exist, and readiness is then 1.
- The size percentile is available once 50 valid prior samples exist, and readiness is then 2.
- The percentile uses only bars strictly before the current bar.
- The state counters start at age 0, give the first code-bearing bar age 1, reset age on a code change, count changes over code-bearing bars, and freeze on a null code.
- A test proves that changing RSI, RVOL, and pivots does not change Direction.
- A causality test proves that appending or changing later bars does not change an earlier Direction value.
- A determinism test proves that repeated computation yields the same series.
- A golden fixture covers every Direction field over the 260-bar reference series.
- Edge cases are covered: empty series, single candle, a series shorter than the required history, a gap, a zero-volume candle, and a flat series with equal highs and lows.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.

## 7. Work breakdown

| Step | Action | Paths | Depends on |
|---|---|---|---|
| 1 | Record the M3 plan and the initial transparency-log entry | `docs/execution-plans/2026-10-24/19-direction-axis.md`, `docs/ai-transparency-log/2026-10-24/19-direction-axis.md` | Approval |
| 2 | Add the axes module and the Direction implementation | `crates/quantrule-model/src/axes/mod.rs`, `crates/quantrule-model/src/axes/direction.rs`, `crates/quantrule-model/src/lib.rs` | 1 |
| 3 | Add unit tests for the formula, null rules, percentile, readiness, and counters | `crates/quantrule-model/src/axes/direction.rs` | 2 |
| 4 | Generate the independent golden fixture and add the integration tests | `crates/quantrule-model/tests/fixtures/direction_golden.csv`, `crates/quantrule-model/tests/direction_golden.rs` | 2 |
| 5 | Run the full checks and review the diff against the contract | all changed files | 3–4 |
| 6 | Record the review and evidence in the plan and log | this plan, the transparency log | 5 |
| 7 | Commit, push, open the PR, and move #19 to Done | all changed files | 6 and the commit/PR authorization |

## 8. Verification

- **Formula:** hand-computed `d_short`, `d_long`, and `direction` cases, including clamped legs and offsetting legs.
- **Nulls:** each missing EMA, a missing ATR, a zero ATR, and a flat series; the reason code and readiness are asserted at the bar.
- **Zero:** a measured zero Direction is `Some(0.0)`, state 0, and ready after 50 samples.
- **Percentile:** 49 samples give null and readiness 1; 50 samples give a value and readiness 2; a current value below all samples ranks at 0, proving strict-before; the inclusive comparison is asserted.
- **Counters:** a same-code run, a code change, and a null bar; age and `changes_50` are asserted directly.
- **Causality:** every prefix of the golden series matches the corresponding prefix of the full series; changing a later candle leaves earlier bars unchanged.
- **Determinism:** two computations over the same candles are equal; the series is compared field by field.
- **No other-axis read:** two `Readings` that differ only in RSI, RVOL, and pivots produce identical Direction output.
- **Golden:** every field on every bar of the 260-bar fixture, with `1e-9` for continuous values and exact equality for codes and counters.
- **Edge cases:** empty, single, short, gap, zero volume, and flat equal-high/equal-low series.

Commands:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`

The evidence is the observed command output, recorded in the PR and the transparency log. No claim is made about an axis, event, aggregate, or shell behavior this item does not implement.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| The percentile accidentally includes the current bar | The history is appended only after the percentile is computed; a test uses a current value below every sample and expects 0 |
| A zero ATR is mistaken for a valid score | The denominator is checked before the division; a flat-series test asserts null with reason 3 |
| A null bar advances the counters | `StateCounters::observe(None)` freezes both values; an axis-level test covers it |
| Direction reads a correlated field | The input extraction names only EMA 20/50/200 and ATR 14; a behavior test changes every other reading |
| The golden fixture encodes the Rust bug | The expected columns come from an independent Python implementation over the independently generated readings fixture |
| Floating-point drift between Python and Rust | The contract's operation order is followed in both; the continuous comparison uses `1e-9` |
| The module grows past the file-size limit | The implementation and its unit tests are cohesive; the integration test is separate. Split only if the file approaches 1,000 lines |
| Recovery from a bad merge | Revert with a new commit; no migrations or persistent state are involved |

## 10. Stop conditions

Stop and request human direction if:

- The contract requires a semantic change to the formula, thresholds, windows, readiness, or codes.
- A new dependency is needed.
- The readings layer lacks a value Direction requires, or its null semantics cannot distinguish a measured zero.
- A test cannot distinguish a valid zero from an unavailable value.
- The golden fixture cannot be reproduced from the checked-in candles and readings.
- The toolchain cannot run `fmt`, `clippy`, `test`, or `doc`.
- A merge authorization is withheld or the PR is rejected.

## 11. Evidence

Observed on 2026-10-24 in the task worktree, after the implementation and before the pull request.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Clean, exit 0. |
| `cargo clippy --all-targets -- -D warnings` | Clean, exit 0. |
| `cargo test` | 385 passed, 0 failed: data 90, model unit 110, direction golden 14, readings golden 11, shell unit 93, shell CLI 47, doc 20 (data 2, model 18). |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean, exit 0. |

The golden CSV holds 260 bars. Its expected Direction columns were produced by an independent Python implementation of §8.1 and §9 from the independently generated readings fixture. The fixture covers 199 unavailable bars (all reason 1), 50 provisional bars, 11 ready bars, and 11 percentile values. Its valid bars are all state -1 with no state change, so the positive, neutral, negative, change, zero-ATR, and clamp paths are covered by the unit and edge-case tests rather than by the golden fixture. The independent review recomputed the fixture from the CSV candles and found a maximum absolute difference of `1.08e-14` with zero mismatches.

The read-only review found one plan/code mismatch on the non-finite ATR reason code and several test-strengthening opportunities. The plan was corrected to classify a non-finite ATR as reason 3, matching the code and test. A one-leg-clamped formula case, a non-empty pivot mutation in the no-other-reading test, exact integer parsing in the golden parser, a safe `replace_candle` helper, and a bounded 252-sample magnitude history were added. The README sentence about the model was scoped to the shell. No blocker remained.
