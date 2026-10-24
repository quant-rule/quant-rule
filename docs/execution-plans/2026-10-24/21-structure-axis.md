# Execution Plan: The Structure Axis

- Issue: [#21](https://github.com/quant-rule/quant-rule/issues/21) — feat: build the structure axis
- Branch: `feat/21-structure-axis`
- Pull request: pending
- Status: Verified
- Date: 2026-10-24 (UTC)
- Program plan: [Completion plan (M2–M14)](14-model-engine-completion.md)
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Umbrella plan: [`docs/execution-plans/2026-10-21/14-model-engine.md`](../2026-10-21/14-model-engine.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/21-structure-axis.md`](../../ai-transparency-log/2026-10-24/21-structure-axis.md)

## 1. Intent

Build the Structure axis as a pure per-bar series. Structure turns confirmed swing pivots into the contract's higher/lower swing relations, the signed swing score, and the compact swing code, and it publishes the EMA distance, cross, and retest features for the three configured periods. It also maintains the confirmed-level registry that the Progression axis and the Level events consume.

The outcome is a `quantrule-model::axes::structure` module that publishes one `StructureBar` per input bar plus one `ConfirmedLevelRegistry` per input series. The swing fields are computed at a configured scale that defaults to major; the EMA features are additional context and never enter `structure_score`.

Affected users: the agents implementing M6, M7, and M12, and the human reviewing the Structure axis against the contract.

## 2. Current state

- `main` at `970e120` contains the M1 kernel, the M2 readings, the M3 Direction axis, and the M4 Momentum axis.
- `crates/quantrule-model/src/axes/mod.rs` exposes `direction` and `momentum` only.
- `Readings` publishes `ema20`, `ema50`, `ema200`, `rsi14`, `atr14`, `rvol20`, and `pivots`. It does not carry the candle series, so the EMA distance, cross, and retest features need the candles passed alongside the readings.
- `PivotSeries` stores confirmed pivots at their availability bar and exposes `confirmed_at`, `confirmed_as_of`, and `all_confirmed`. A pivot is never visible before `available_at`.
- The contract fixes the Structure formulas and fields in §8.3, the output and diagnostic fields in §9, and the reference, scale, and reason codes in §14. The bounded seven-reference set for Level events is §7.2, not the registry itself.
- The checked-in `readings_golden.csv` holds 260 bars with 44 confirmed minor pivots and 24 confirmed major pivots. The major scale has 12 confirmed highs and 12 lows. At the default major scale the fixture yields 34 bars with a null score, 226 valid score bars, structure codes -2/0/+2 only, 13 code changes, and 176 bars with the percentile available. It has 5 bars where the high relation is available while the low relation is null. It has no equal highs or equal lows, so the ±1 and equal-relation paths need unit tests.
- The M4 merge at `970e120` passed 418 tests, with `fmt`, `clippy -D warnings`, and `doc -D warnings` clean (PR #61).
- Issue #21 is `In Progress` on the board with `Start date` 2026-10-24.

## 3. Scope

Included:

- `quantrule-model::axes::structure` with the swing relations, `structure_score`, `structure_code`, `structure_scale_code`, the nine EMA distance/cross/retest fields, the size percentile, readiness, unavailable reason, state age, and state changes as per-bar values.
- `StructureConfig` with a `scale` field whose default is `PivotScale::Major`.
- `ConfirmedLevel` and `ConfirmedLevelRegistry`, built from both pivot scales and ordered by confirmation bar.
- Null rules for an incomplete relation pair, missing EMA or ATR inputs, and invalid ATR denominators.
- Unit, causality, determinism, edge-case, no-double-count, and golden-fixture tests, plus a registry golden fixture.
- The M5 plan and transparency log.

Excluded:

- Progression, events, context, cross-timeframe aggregation, market state, and shell commands (M6–M13).
- The bounded seven-reference selection for Level events (§7.2); M5 publishes the full registry, and M7 selects from it.
- The progression target policy and leg lifecycle (M6).
- Changes to the contract's formulas, thresholds, windows, or code values.
- Changes to the readings layer, its golden fixture, the kernel, or the data engine.
- New dependencies, I/O, clock access, randomness, or shell changes.

## 4. Assumptions and uncertainty

The contract is authoritative. The interpretations below are recorded so a reviewer can reject them without reading the code.

- **Scale.** `StructureConfig::default()` uses `PivotScale::Major`. `structure_scale_code` is always published from the configuration, even on bars where the score is null. The swing fields use only the configured scale; the registry holds both scales.
- **Latest pair.** For each kind, the axis keeps the two most recent confirmed pivots at the configured scale with `available_at <= t`, ordered by occurrence bar. Confirmation lag is constant within a scale, so occurrence order and availability order agree. The relation compares the newer pivot with the older one.
- **Relations.** `high_relation_code` is +1 when the newer high is higher, -1 when it is lower, and 0 when the two prices are equal. `low_relation_code` uses the same comparison for lows. Each relation is resolved independently.
- **Incomplete pair.** If either relation is null, `structure_score` and `structure_code` are null, readiness is 0, and the state counters freeze. The available relation is still published. The unavailable reason is 2, insufficient confirmed pivots.
- **Score and code.** `structure_score = (high_relation_code + low_relation_code) / 2`, published as a discrete-valued continuous number in `[-1, +1]`. `structure_code = round(2 * structure_score)` is exact because the score is always a half-integer; it is published as an integer in `[-2, +2]`.
- **EMA features.** Each period is independent. `ema_distance_atr_n = (Close - EMA_n) / ATR14`. The cross compares the previous and current close with the previous and current EMA. The retest tests whether `[low, high]` intersects `EMA_n ± 0.2 * ATR14`, then publishes the side of the close. A no-cross or no-test is a measured 0, not null.
- **EMA null rules.** Distance and retest are null when a required candle, EMA, or ATR value is unavailable, non-finite, or when ATR is zero or negative. The cross does not use ATR, so it remains available when ATR is invalid. A cross is null when the previous or current close or EMA is unavailable.
- **Retest equality.** When the zone is tested and the close finishes exactly on the EMA, the retest code is 0. The contract defines only above (+1), below (-1), and no qualifying test (0); §7.2 uses the same 0 for a close exactly on a level. This conflates a neutral test with no test, and the plan records it as an interpretation.
- **Auxiliary nulls.** The EMA features are additional context, not the axis primary. Their nulls do not change `structure_score`, readiness, or the axis unavailable reason. §9 publishes one reason per axis, so there is no separate reason field for an auxiliary null.
- **Percentile.** The history holds the absolute `structure_score` of every valid score bar, in bar order, including measured zeros. A bar whose score is null contributes no sample. The kernel selects the most recent 252 samples and applies the 50-sample minimum.
- **Readiness.** Structure is unavailable (0) when the score is null. It is provisional (1) when the score exists but the percentile has fewer than 50 valid prior samples. It is ready (2) when the score and the percentile are both available.
- **State counters.** The axis passes `structure_code` to `StateCounters`. A null code freezes both counters. The first code-bearing bar has age 1; a change resets the age and counts once.
- **Registry scope.** The registry holds every confirmed pivot at both scales, because M6 needs same-scale structural targets and M7 needs the bounded latest high/low set. It is ordered by confirmation bar. At a bar where a minor and a major pivot confirm together, the minor level receives the lower sequence number. Within one scale, at most one pivot confirms per bar because the readings layer discards a bar that qualifies as both a high and a low.
- **Reference codes.** A level's `reference_code` is `1000 + n`, where `n` is its 1-based sequence number in confirmation-bar order. The first level is 1001. The plan flags the 1-based choice because the contract does not state whether `n` starts at 0 or 1.
- **Registry availability.** `as_of(bar)` yields only levels with `available_at <= bar`, in registry order. The sequence numbers are assigned by confirmation order, so a later append never renumbers an earlier level.
- **Mismatched lengths.** The axis iterates over `readings.len()`, the canonical input length. A missing candle at a bar makes that bar's EMA features null rather than panicking. Aligned inputs are the documented precondition; a mismatch is outside the contract.
- **Golden reference.** The expected Structure columns are produced by an independent Python implementation of §8.3 and §9 from the checked-in readings fixture's independently generated EMA, ATR, and pivot columns. The fixture is not derived from the Rust code under test.

## 5. Decisions

| ID | Decision | Options | Choice | Status |
|---|---|---|---|---|
| D1 | Series shape and names | Contract-prefixed fields; axis-scoped fields | One `StructureBar` per bar in a `StructureSeries`; `score`, `state_code`, `scale_code`, the relation fields, the nine EMA fields, and the shared diagnostics. The contract names map one to one | Approved |
| D2 | Scale configuration | A `compute` argument; a configuration type; a hard-coded major scale | `StructureConfig { scale }` with a major default, `compute` for the default, and `compute_with_config` for an explicit scale | Approved |
| D3 | Diagnostics representation | Publish raw `u8` codes; publish the kernel enums | Publish `Readiness` and `UnavailableReason` in the engine record; callers publish `.code()` | Approved |
| D4 | Incomplete pair | Reason 1 for a short series and reason 2 for a missing pivot; reason 2 throughout | Reason 2 (`InsufficientConfirmedPivots`) whenever the score is null; the missing input is a confirmed pivot, not a bar | Approved |
| D5 | Registry scope and numbering | Configured scale only; both scales. 0-based or 1-based `n` | Both scales; confirmation-bar order; minor before major on a tie; `reference_code = 1000 + n` with 1-based `n` | Approved |
| D6 | EMA null and equality rules | Null on any unavailable input; a 0 for a neutral retest | Distance and retest null on an invalid ATR; cross independent of ATR; a tested close exactly on the EMA publishes 0 | Approved |
| D7 | Golden fixtures | One combined CSV; a per-bar CSV plus a registry CSV | `structure_golden.csv` for the per-bar fields at the default major scale, and `structure_levels_golden.csv` for the registry | Approved |
| D8 | No-double-count proof | Source-text scan; behavior test over unused readings | A behavior test that changes EMA and ATR and asserts the swing fields unchanged, and changes the pivots and asserts the EMA features unchanged | Approved |
| D9 | File layout | One `structure.rs`; a `structure/` module | A `structure/` module with `mod.rs`, `ema.rs`, and `registry.rs`, so the implementation and its unit tests stay under the file-size limit | Approved |

## 6. Acceptance criteria

- `high_relation_code` and `low_relation_code` compare the latest two confirmed pivots of each kind at the configured scale and publish +1, 0, or -1, with equal prices giving 0.
- `structure_score` equals `(high_relation_code + low_relation_code) / 2` and lies in `[-1, +1]`.
- `structure_code` matches the contract's table for every relation pair, including the mixed and both-unchanged cases.
- `structure_score` is null when either relation is null, while an individually available relation code is retained; readiness is 0 and the counters freeze.
- `structure_scale_code` publishes the configured scale, defaults to 2, and never changes with the data.
- The nine EMA fields match their formulas and null rules for each period; a no-cross and a no-test are 0; a zero ATR nulls distance and retest but not cross.
- A test proves that changing the EMA and ATR readings does not change `structure_score`, `structure_code`, or either relation, and that changing the pivots does not change the EMA features.
- The size percentile is null under 50 valid prior score samples and available at 50; it uses only bars strictly before the current bar.
- Readiness is 0 when the score is null, 1 when the score exists without the percentile, and 2 when both exist.
- The state counters use `structure_code`, give the first code-bearing bar age 1, reset the age on a change, count changes over code-bearing bars, and freeze on a null code.
- The registry holds every confirmed level at both scales, ordered by confirmation bar, with price, scale, kind, occurrence bar, and availability bar; reference codes start at 1001 and are stable across prefixes.
- `as_of(bar)` never exposes a level whose `available_at` is later than `bar`.
- A causality test proves that appending or changing later bars does not change an earlier Structure bar or an earlier `as_of` registry view.
- A determinism test proves that repeated computation yields the same series and registry.
- A golden fixture covers every Structure field and the registry over the 260-bar reference series, including null scores, codes -2/0/+2, one-relation-only bars, percentile availability, and state changes; unit tests cover the ±1 and equal-relation paths the fixture does not reach.
- Edge cases are covered: empty series, single candle, a series shorter than the required pivot history, an unaligned timeframe boundary, a gap, a zero-volume candle, a flat series, equal highs and lows, a bar that is both a candidate high and low, a zero ATR, and a missing EMA.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.

## 7. Work breakdown

| Step | Action | Paths | Depends on |
|---|---|---|---|
| 1 | Record the M5 plan and the initial transparency-log entry | `docs/execution-plans/2026-10-24/21-structure-axis.md`, `docs/ai-transparency-log/2026-10-24/21-structure-axis.md` | Approval |
| 2 | Add the Structure module, its EMA features, and the registry; expose it from the axes module | `crates/quantrule-model/src/axes/mod.rs`, `crates/quantrule-model/src/axes/structure/mod.rs`, `ema.rs`, `registry.rs`, `crates/quantrule-model/src/lib.rs` | 1 |
| 3 | Add unit tests for the relations, score, code, null rules, EMA features, percentile, readiness, counters, and registry | the three Structure files | 2 |
| 4 | Generate the independent golden fixtures and add the integration tests | `crates/quantrule-model/tests/fixtures/structure_golden.csv`, `structure_levels_golden.csv`, `crates/quantrule-model/tests/structure_golden.rs` | 2 |
| 5 | Run the full checks and review the diff against the contract and the issue | all changed files | 3–4 |
| 6 | Record the review and evidence in the plan and log | this plan, the transparency log | 5 |
| 7 | Commit, push, open the PR, and move #21 to Done | all changed files | 6 and the commit/PR authorization |

## 8. Verification

- **Relations:** hand-computed pairs for higher/higher (+2), higher/equal (+1), higher/lower (0), equal/equal (0), lower/equal (-1), and lower/lower (-2); equal highs and equal lows give 0.
- **Incomplete pair:** only one high, only one low, and one relation available while the other is null; the score is null, the reason is 2, readiness is 0, and the available relation survives.
- **Scale:** the default is major; a minor configuration uses the minor pivots and publishes 1; the same candles produce different relations at the two scales where the fixture allows it.
- **EMA distance:** hand-computed positive, negative, and zero distances; null on a missing EMA, a missing candle, a non-finite input, and a zero or negative ATR.
- **EMA cross:** +1, -1, 0, a previous close exactly on the EMA in each direction, a current close exactly on the EMA, and null on missing previous or current inputs.
- **EMA retest:** +1 and -1, a zone boundary touched inclusively, a close exactly on the EMA giving 0, a bar outside the zone giving 0, and null on a zero ATR or a missing EMA.
- **No double counting:** changing EMA and ATR leaves the swing fields unchanged; changing the pivots leaves the EMA features unchanged; changing RSI and RVOL leaves the whole series unchanged.
- **Percentile:** 49 prior score samples give null and readiness 1; 50 give a value and readiness 2; the current bar is excluded; a 298-bar series proves the 252-sample trim.
- **Counters:** a same-code run, a change, and a null bar; age and `changes_50` are asserted directly.
- **Registry:** both scales, confirmation-bar order, the minor-first tie, reference codes starting at 1001, `as_of` never exposing a later level, and stable codes across prefixes.
- **Causality:** every prefix matches the corresponding prefix of the full series; changing a later candle leaves earlier bars and earlier `as_of` views unchanged.
- **Determinism:** two computations over the same candles are equal; timestamps do not affect the output.
- **Golden:** every per-bar field and every registry row, with `1e-9` for continuous values and exact equality for codes and counters.
- **Edge cases:** empty, single, short, unaligned timestamps, gap, zero volume, flat, equal highs/lows, a both-candidate bar, zero ATR, and a missing EMA.

Commands:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`

The evidence is the observed command output, recorded in the PR and the transparency log. No claim is made about an axis, event, aggregate, or shell behavior this item does not implement.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| A pivot is used before it is knowable | The axis reads `confirmed_at` at the current bar only and the registry records `available_at`; `as_of` filters on it; the prefix and later-candle tests assert the boundary |
| A relation compares the wrong two pivots | The axis keeps only the two most recent confirmed pivots of each kind and compares newer to older; a hand-computed test covers the replacement of the older pivot |
| The score is inferred from an incomplete pair | Each relation is resolved separately; the score is null when either is null; a test keeps one relation available |
| The EMA features leak into the score | The swing fields are computed from pivots only; a mutation test changes EMA and ATR and asserts the swing fields unchanged |
| A zero ATR is treated as a valid denominator | Distance and retest return null; the cross, which does not use ATR, stays available; a dedicated test covers it |
| A neutral retest is confused with no test | The contract provides only -1/0/+1; a tested close exactly on the EMA publishes 0, recorded as an interpretation |
| The registry renumbers earlier levels when later pivots confirm | Sequence numbers are assigned in confirmation-bar order; a prefix test compares the earlier `as_of` views |
| The 1-based reference numbering is not the intended one | The plan flags it; changing to 0-based is a one-line change and a fixture regeneration |
| The golden fixture does not reach ±1 or equal relations | The fixture's coverage is measured and recorded; unit tests cover the missing paths |
| A file grows past the file-size limit | The module is split into `mod.rs`, `ema.rs`, and `registry.rs` along its own seams |
| A merge proceeds without authorization | The program stops at D3's gate; no merge without explicit human authorization |

## 10. Stop conditions

Stop and request human direction if:

- The contract requires a semantic change to the relations, score, code table, EMA features, or codes.
- The retest equality, the incomplete-pair reason, the registry scope, or the 1-based reference numbering cannot be reconciled with the contract.
- A new dependency is needed.
- The readings layer lacks a value Structure requires, or its availability semantics cannot support the pair or the registry.
- The golden fixture cannot be reproduced from the checked-in candles and readings.
- The toolchain cannot run `fmt`, `clippy`, `test`, or `doc`.
- A merge authorization is withheld or the PR is rejected.

## 11. Evidence

Observed on 2026-10-24 (UTC) in the worktree `../quant-rule--21-structure-axis` on branch `feat/21-structure-axis`, based on `main` at `970e120`. Toolchain: `cargo 1.99.1 (5f94df478 2026-08-27)`, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`.

### Implementation

| Path | Lines | Content |
|---|---|---|
| `crates/quantrule-model/src/axes/structure/mod.rs` | 859 | `StructureBar`, `StructureConfig`, `StructureSeries`, the relation/score/code helpers, 14 unit tests |
| `crates/quantrule-model/src/axes/structure/ema.rs` | 429 | The nine EMA distance/cross/retest features, 11 unit tests |
| `crates/quantrule-model/src/axes/structure/registry.rs` | 242 | `ConfirmedLevel`, `ConfirmedLevelRegistry`, 5 unit tests |
| `crates/quantrule-model/tests/structure_golden.rs` | 899 | 15 golden, causality, determinism, and edge-case tests |
| `crates/quantrule-model/tests/fixtures/structure_golden.csv` | 261 lines | 260 bars, 26 columns, LF |
| `crates/quantrule-model/tests/fixtures/structure_levels_golden.csv` | 69 lines | 68 levels, 7 columns, LF |
| `crates/quantrule-model/src/axes/mod.rs` | — | Exposes `structure` |
| `crates/quantrule-model/src/lib.rs` | — | Crate docs name the three implemented axes |

All three source files are under the 1,000-line limit; the largest test file is 899 lines.

### Fixture provenance

The fixtures were produced by `/tmp/opencode/structure_fixture.py`, an external Python script that implements contract §8.3 and §9 directly from the checked-in `readings_golden.csv` EMA, ATR, and pivot columns. It does not call or mirror the Rust code. The generator is not checked in, so its output is the reproducible artefact; re-running it reproduced both CSVs byte-identically, including LF line endings. Observed fixture coverage:

| Measure | Value |
|---|---|
| Bars | 260 |
| Null-score bars | 34 |
| Valid-score bars | 226 |
| Score values | -1.0, 0.0, +1.0 |
| Code values | -2, 0, +2 |
| Code transitions | 13 |
| Provisional bars | 50 |
| Ready bars | 176 |
| Percentile bars | 176 |
| One-relation-only bars | 5 |
| Levels | 68 (44 minor, 24 major; major 12 highs, 12 lows) |
| First / last reference code | 1001 / 1068 |

The fixture reaches only the -2, 0, and +2 codes and has no equal relations. The ±1 and equal-relation paths are covered by the unit tests, as recorded in §9.

### Checks

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Clean, no output |
| `cargo clippy --all-targets -- -D warnings` | Clean, no warnings |
| `cargo test` | 466 passed, 0 failed: data 90, model unit 158, direction 14, momentum 14, readings 11, structure 15, shell unit 93, shell CLI 47, doc 24 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean, no warnings |

### Independent review

A separate read-only review session (a subagent, same model family; disclosed as not an independent model judgement) checked the diff against §7.2, §8.3, §9, §14, the issue, and this plan. It reported **no blocker and no major code defect** and confirmed the relation selection, score and code table, EMA null rules, causality and prefix behaviour, determinism, no-double-count separation, percentile window, readiness and reason 2, counters, registry ordering and reference codes, and test coverage.

It raised these findings, all addressed before this record:

| Finding | Correction |
|---|---|
| The plan and log contradicted the tree | This section and the log's implementation entry now record the observed state |
| No Structure-level test of the 252-sample trim | Added `the_percentile_uses_only_the_most_recent_252_scores` over a 298-bar varied zigzag that asserts the cap changes the result |
| No test that a current close exactly on the EMA is not a cross | Added both above and below cases to `a_touch_or_a_one_sided_bar_is_not_a_cross` |
| The fixture generator was outside the repository | Recorded under fixture provenance above, with the byte-identical reproduction |
| The fixtures used CRLF | Both regenerated with LF and verified to contain zero CRLF |
| `ConfirmedLevelRegistry::get` named its parameter `sequence` while taking a zero-based index | Renamed to `position` |
| `pub mod ema` exposed an internal module | Reduced to `mod ema` |
| Weak edge-test names | Renamed the missing-series and gap tests |

### Acceptance criteria

All criteria in §6 are met by the implementation and the observed checks, with these recorded limitations:

- The golden fixture does not reach the ±1 codes or an equal relation; the unit tests do.
- The generator is external to the repository, so fixture regeneration depends on `/tmp/opencode/structure_fixture.py` being retained or re-derived.
- The retest equality and the 1-based reference numbering remain the recorded interpretations in §4; the contract does not settle them.

Status: `Verified`.

## 12. Delivery

The commit, push, and pull request await explicit human authorization. Once authorized, the change is committed on `feat/21-structure-axis`, a pull request is opened against `main`, and #21 moves to `Done` on the board.
