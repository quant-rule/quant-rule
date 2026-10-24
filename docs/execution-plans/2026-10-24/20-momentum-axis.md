# Execution Plan: The Momentum Axis

- Issue: [#20](https://github.com/quant-rule/quant-rule/issues/20) — feat: build the momentum axis
- Branch: `feat/20-momentum-axis`
- Pull request: [#61](https://github.com/quant-rule/quant-rule/pull/61)
- Status: Verified
- Date: 2026-10-24 (UTC)
- Program plan: [Completion plan (M2–M14)](14-model-engine-completion.md)
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Umbrella plan: [`docs/execution-plans/2026-10-21/14-model-engine.md`](../2026-10-21/14-model-engine.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/20-momentum-axis.md`](../../ai-transparency-log/2026-10-24/20-momentum-axis.md)

## 1. Intent

Build the Momentum axis as a pure per-bar series. Momentum turns RSI 14 into the contract's signed force, the signed three-bar change in force, the signed change in force magnitude, and the axis's compact state, size percentile, readiness, unavailable reason, and persistence counters.

The outcome is a `quantrule-model::axes::momentum` module that publishes one `MomentumBar` per input bar, aligned to the input length. A missing current RSI makes all three continuous values null; a missing RSI exactly three bars earlier makes only the two change measures null while the force remains published. A measured force of zero stays a neutral value.

Affected users: the agents implementing M5–M14, and the human reviewing the Momentum axis against the contract.

## 2. Current state

- `main` at `e85e90a` contains the M1 kernel, the M2 readings, and the M3 Direction axis.
- `crates/quantrule-model/src/axes/mod.rs` exposes `direction` only.
- `Readings` publishes `rsi14` as one `Option<f64>` per input bar. `ReadingSeries::get` returns `None` for an unavailable bar.
- The kernel provides `clamp_unit`, `sign`, `compact_state`, `empirical_percentile`, `StateCounters`, `Readiness`, and `UnavailableReason`.
- The contract fixes the formulas in §8.2, the published fields and diagnostics in §9, and the codes in §14.
- The checked-in `readings_golden.csv` holds 260 bars with `rsi14` valid from bar 14 through bar 259. Its RSI range is 19.42–69.45, so the derived force covers negative, neutral, and positive states and 56 compact-state changes.
- Baseline checks pass on `main`: 385 tests, with `fmt`, `clippy -D warnings`, and `doc` clean.

## 3. Scope

Included:

- `quantrule-model::axes::momentum` with `momentum_force`, `momentum_acceleration`, `momentum_intensification`, `momentum_state_code`, `momentum_size_percentile`, `momentum_readiness_code`, `momentum_unavailable_reason_code`, `momentum_state_age_bars`, and `momentum_state_changes_50` as per-bar values.
- The contract's three formulas with `clamp_unit`, `sign`, and the exact three-bar comparison window.
- Null rules for a missing current RSI and for a missing comparison RSI.
- Percentile history of up to 252 valid absolute force magnitudes strictly before the current bar.
- Unit, causality, determinism, edge-case, and golden-fixture tests, including a no-other-reading regression test.
- The M4 plan and transparency log.

Excluded:

- Structure, Progression, events, context, cross-timeframe aggregation, market state, and shell commands (M5–M13).
- Changes to the contract's formulas, thresholds, windows, or code values.
- New dependencies, I/O, clock access, randomness, or shell changes.
- Any change to the readings layer or its golden fixture.
- A signal event built from the momentum state; Signal events belong to M9.

## 4. Assumptions and uncertainty

- The contract is authoritative. The interpretations below are recorded so a reviewer can reject them without reading the code.
- **Force.** `force = (RSI14 - 50) / 50`. RSI 14 is bounded to `[0, 100]` by the reading layer, so force is bounded to `[-1, +1]` without a clamp. The axis rejects a non-finite or out-of-range RSI as `InsufficientBars` (1) rather than clamping a malformed reading into a valid extreme.
- **Three-bar window.** `force_(t-3)` means exactly the bar three positions earlier, not the third previous valid sample. A null RSI at `t-3` makes acceleration and intensification null even when an older valid RSI exists. The current bar is never used as its own comparison.
- **Null scope.** A missing current RSI makes force, acceleration, and intensification null. A missing comparison RSI makes only acceleration and intensification null; force and its state diagnostics remain published.
- **Reason.** The only reason Momentum produces is `InsufficientBars` (1), for a missing, non-finite, or out-of-range RSI. A bar whose force is available has no unavailable reason even when its change measures are null, because the axis's primary value is published.
- **Readiness.** Momentum is unavailable (0) when force is null. It is provisional (1) when force exists but the percentile has fewer than 50 valid prior samples. It is ready (2) when force and the percentile are both available. A null acceleration or intensification is a null output, not an incomplete axis: §9 names the percentile sample count as the completeness gate, and the change measures are derived values whose window can be incomplete while the force is still published.
- **Percentile.** The history holds the absolute force of every valid force bar, in bar order, including measured zeros. A bar whose force is null contributes no sample. The kernel selects the most recent 252 samples and applies the 50-sample minimum.
- **State counters.** The axis passes `compact_state(force)` to `StateCounters`. A null force freezes both counters. The first code-bearing bar has age 1; a change resets the age and counts once.
- **Intensification sign.** The formula is applied literally: `sign(force_t)` orients the magnitude change. When force is positive, a positive value means the magnitude is expanding; when force is negative, a negative value means the magnitude is expanding in the negative direction. The contract's sentence about "above 0" is read as describing the positive-force case; the formula and the issue both state the sign factor explicitly. A force of exactly zero reports exactly 0 whatever the comparison value.
- **No other-reading read.** The axis reads only `rsi14`. The input extraction is kept narrow so a later field cannot enter silently. The regression test changes every other reading and asserts identical Momentum output.
- **Golden reference.** The expected Momentum columns are produced by an independent Python implementation of §8.2 and §9 from the checked-in readings fixture's independently generated `rsi14` column. The fixture is not derived from the Rust code under test.

## 5. Decisions

| ID | Decision | Options | Choice | Status |
|---|---|---|---|---|
| D1 | Series shape | Parallel field vectors; one record per bar | One `MomentumBar` per bar in a `MomentumSeries`; each record carries every published Momentum field | Approved with this plan |
| D2 | Numeric diagnostics | Publish raw `u8` codes; publish the kernel enums | Publish `Readiness` and `UnavailableReason` in the engine record; callers publish `.code()` | Approved with this plan |
| D3 | Missing comparison RSI | Null all three values; null only the two change measures | Null only acceleration and intensification; keep force and its diagnostics | Approved with this plan |
| D4 | Readiness with a null change measure | Percentile-only; require every continuous value | Percentile-only: readiness 2 requires force and the percentile; a null change measure stays a null output | Approved with this plan |
| D5 | Out-of-range RSI | Clamp into range; reject as unavailable | Reject as `InsufficientBars` (1); a defect must not masquerade as a valid extreme | Approved with this plan |
| D6 | Golden fixture | Reuse the readings CSV; add a self-contained Momentum CSV | A self-contained `momentum_golden.csv` with candles and expected Momentum fields | Approved with this plan |
| D7 | No-double-count proof | Source-text scan; behavior test over unused readings | A behavior test that changes EMA, ATR, RVOL, and pivots and asserts identical Momentum output, plus a narrow input extraction | Approved with this plan |

## 6. Acceptance criteria

- `momentum_force` equals `(RSI14 - 50) / 50` and lies in `[-1, +1]`.
- `momentum_acceleration` and `momentum_intensification` use exactly the bar three positions earlier and are clamped to `[-1, +1]`.
- Acceleration and intensification are null when either required RSI is unavailable; force is null only when the current RSI is unavailable.
- A force of exactly zero is published as `Some(0.0)`, has state code 0, and reports intensification exactly 0.
- Intensification is exactly 0 when the force magnitude is unchanged across the window.
- Every available force has a state code matching the inclusive ±0.1 bucketing.
- The size percentile is null when fewer than 50 valid prior force samples exist, and readiness is then 1.
- The percentile is available once 50 valid prior samples exist, and readiness is then 2.
- A missing comparison RSI leaves force published, nulls only the change measures, and does not downgrade readiness once the percentile is available.
- The percentile uses only bars strictly before the current bar.
- The state counters start at age 0, give the first code-bearing bar age 1, reset age on a code change, count changes over code-bearing bars, and freeze on a null force.
- A test proves that changing EMA, ATR, RVOL, and pivots does not change Momentum.
- A causality test proves that appending or changing later bars does not change an earlier Momentum value.
- A determinism test proves that repeated computation yields the same series.
- A golden fixture covers every Momentum field over the 260-bar reference series, including positive, negative, neutral, and state-change paths.
- Edge cases are covered: empty series, single candle, a series shorter than the required RSI history, a gap, a zero-volume candle, and a flat series with equal highs and lows.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.

## 7. Work breakdown

| Step | Action | Paths | Depends on |
|---|---|---|---|
| 1 | Record the M4 plan and the initial transparency-log entry | `docs/execution-plans/2026-10-24/20-momentum-axis.md`, `docs/ai-transparency-log/2026-10-24/20-momentum-axis.md` | Approval |
| 2 | Add the Momentum module and expose it from the axes module | `crates/quantrule-model/src/axes/mod.rs`, `crates/quantrule-model/src/axes/momentum.rs`, `crates/quantrule-model/src/lib.rs` | 1 |
| 3 | Add unit tests for the formulas, null rules, percentile, readiness, and counters | `crates/quantrule-model/src/axes/momentum.rs` | 2 |
| 4 | Generate the independent golden fixture and add the integration tests | `crates/quantrule-model/tests/fixtures/momentum_golden.csv`, `crates/quantrule-model/tests/momentum_golden.rs` | 2 |
| 5 | Run the full checks and review the diff against the contract | all changed files | 3–4 |
| 6 | Record the review and evidence in the plan and log | this plan, the transparency log | 5 |
| 7 | Commit, push, open the PR, and move #20 to Done | all changed files | 6 and the commit/PR authorization |

## 8. Verification

- **Formula:** hand-computed force, acceleration, and intensification cases, including both clamp directions, a positive and a negative force, and a zero force.
- **Window:** a value at `t-4` or `t-2` does not change the `t` result; a null at `t-3` makes only the change measures null; a null at `t-3` with a valid third previous sample proves the exact-bar rule.
- **Nulls:** a missing current RSI, a missing comparison RSI, a non-finite comparison RSI, a non-finite current RSI, and an out-of-range RSI; the reason and readiness are asserted at the bar.
- **Zero:** a force of exactly zero is `Some(0.0)`, state 0, and intensification 0; readiness follows the percentile and window.
- **Percentile:** 49 samples give null and readiness 1; 50 samples give a value and readiness 2; a current value below all samples ranks at 0, proving strict-before; the inclusive comparison is asserted; a 300-sample series proves the axis trims to the most recent 252.
- **Counters:** a same-code run, a code change, and a null bar; age and `changes_50` are asserted directly.
- **Causality:** every prefix of the golden series matches the corresponding prefix of the full series; changing a later candle leaves earlier bars unchanged.
- **Determinism:** two computations over the same candles are equal; the series is compared field by field.
- **No other-reading read:** two `Readings` that differ only in EMA, ATR, RVOL, and pivots produce identical Momentum output.
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
| The comparison uses the third previous valid sample instead of bar `t-3` | The lookup is a direct `bar.checked_sub(3)`; tests vary `t-4` and `t-2` without changing the result |
| A null comparison RSI nulls the force | The current and comparison values are resolved separately; a test asserts force remains published |
| A null change measure is mistaken for an incomplete axis | Readiness is percentile-only per §9; the mid-series-gap test asserts ready while acceleration is null |
| A flat series is misdiagnosed as insufficient history | The readings layer publishes `None` for an undefined RSI, so the axis reports reason 1; the registry has no separate undefined-indicator code, and this limitation is recorded |
| A zero force is mistaken for a missing value | `force_at` returns `Some(0.0)` for RSI 50; the state is 0 and intensification is 0 |
| The percentile accidentally includes the current bar | The history is appended only after the percentile is computed; a test uses a current value below every sample and expects 0 |
| Momentum reads a correlated field | The input extraction names only `rsi14`; a behavior test changes every other reading |
| The golden fixture encodes the Rust bug | The expected columns come from an independent Python implementation over the independently generated `rsi14` column |
| Floating-point drift between Python and Rust | The contract's operation order is followed in both; the continuous comparison uses `1e-9` |
| The module grows past the file-size limit | The implementation and its unit tests are cohesive; the integration test is separate. Split only if the file approaches 1,000 lines |
| Recovery from a bad merge | Revert with a new commit; no migrations or persistent state are involved |

## 10. Stop conditions

Stop and request human direction if:

- The contract requires a semantic change to the formulas, thresholds, windows, readiness, or codes.
- The sign convention of intensification cannot be reconciled with the contract's prose and the formula.
- A new dependency is needed.
- The readings layer lacks an RSI value Momentum requires, or its null semantics cannot distinguish a measured zero.
- A test cannot distinguish a valid zero from an unavailable value.
- The golden fixture cannot be reproduced from the checked-in candles and readings.
- The toolchain cannot run `fmt`, `clippy`, `test`, or `doc`.
- A merge authorization is withheld or the PR is rejected.

## 11. Evidence

Observed on 2026-10-24 in the task worktree, after the implementation and the review corrections, and after the pull request was opened.

The delivery is PR [#61](https://github.com/quant-rule/quant-rule/pull/61) at commit `306e604`. The PR reads `MERGEABLE` with merge state `CLEAN`, and the project-sync check passed. Issue #20 was moved to `Done` on the board once the PR existed, and the PR's own board item carries `Item type = Pull request`.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Clean, exit 0. |
| `cargo clippy --all-targets -- -D warnings` | Clean, exit 0. |
| `cargo test` | 418 passed, 0 failed: data 90, model unit 128, direction golden 14, momentum golden 14, readings golden 11, shell unit 93, shell CLI 47, doc 21 (data 2, model 19). |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean, exit 0. |

The golden CSV holds 260 bars. Its expected Momentum columns were produced by an independent Python implementation of §8.2 and §9 from the independently generated `rsi14` column of the readings fixture. The fixture covers 14 unavailable bars (all reason 1), 50 provisional bars, 196 ready bars, 196 percentile values, 17 null accelerations, 111 negative states, 84 neutral states, 51 positive states, and 56 state changes. It contains 55 bars where acceleration and intensification differ, so the sign factor is exercised. It contains no exact-zero force and no exact-zero intensification; those are covered by unit tests. It reaches 245 prior force samples, so the 252-sample trim is covered by a unit test rather than by the fixture.

The independent read-only review recomputed all 260 rows from the readings fixture and reported a maximum absolute difference of `0.0` for force, acceleration, intensification, and percentile, with zero mismatches for the state, readiness, reason, age, and change counters.

The review found no numeric or formula defect. It found one readiness interpretation that was corrected: the implementation had required both change measures for readiness 2, while §9 names the percentile sample count as the completeness gate. Readiness is now percentile-only, and the mid-series-gap test asserts ready while acceleration is null. The review also led to a bounded 252-sample capacity, an exact-window test for `t-4` and `t-2`, a 252-sample trim test, an exact percentile assertion in the gap case, stronger gap assertions, and an invalid-comparison-RSI test.

The flat-series case reports reason 1 because the readings layer publishes `None` for an undefined RSI and the registry has no separate undefined-indicator code. This is recorded as a limitation, not hidden. The axis loop uses `Readings::len()`, the canonical input length, so a malformed synthetic `Readings` with mismatched series lengths is outside the contract. The fixture has no equal-high/equal-low tie; the flat-series edge test covers that path. No blocker remained.
