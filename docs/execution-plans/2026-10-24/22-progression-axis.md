# Execution Plan: The Progression Axis

- Issue: [#22](https://github.com/quant-rule/quant-rule/issues/22) — feat: build the progression axis
- Branch: `feat/22-progression-axis`
- Pull request: pending
- Status: Verified (implementation, independent review, and checks complete; delivery awaits §9 authorization)
- Date: 2026-10-24 (UTC)
- Program plan: [Completion plan (M2–M14)](14-model-engine-completion.md)
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Umbrella plan: [`docs/execution-plans/2026-10-21/14-model-engine.md`](../2026-10-21/14-model-engine.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/22-progression-axis.md`](../../ai-transparency-log/2026-10-24/22-progression-axis.md)
- Approval basis: the program plan's decision D1/D3 and the human's `continue` after the four M6 defaults D1–D4 were presented.

## 1. Intent

Build the Progression axis as a pure per-bar series. Progression tracks a structural leg from its pivot candidate through confirmation, target selection, progress, and a terminal lifecycle. It maintains a separate minor and a separate major leg per timeframe, publishes the most recently created leg across the two scales, and publishes the full record of the leg at each scale.

The outcome is a `quantrule-model::axes::progression` module that publishes one `ProgressionBar` per input bar. Each bar carries the published leg's `progress`, ordinal `stage_code`, direction and scale, the per-scale leg records, and the axis diagnostics. The two scales are never blended, and no stage code is ever averaged.

Affected users: the agents implementing M7 (Level events), M12 (assembled market state), and M13 (shell rendering), and the human reviewing the Progression axis against §8.4 and §9.

## 2. Current state

- `main` at `e8fa6f6` contains the M1 kernel, the M2 readings, the M3 Direction axis, the M4 Momentum axis, and the M5 Structure axis.
- `crates/quantrule-model/src/axes/mod.rs` exposes `direction`, `momentum`, and `structure`; `lib.rs` names the three implemented axes.
- `Readings` publishes `ema20`, `ema50`, `ema200`, `rsi14`, `atr14`, `rvol20`, and `pivots`. `PivotSeries` exposes the candidate at each occurrence bar and the confirmed pivot at each availability bar.
- `axes::structure::registry::ConfirmedLevelRegistry` already holds every confirmed level at both scales, ordered by confirmation bar, with price, kind, scale, occurrence bar, and availability bar. Progression reuses the registry type and rebuilds it from `Readings.pivots`; it never reads `StructureSeries`.
- The contract fixes the leg fields and transitions in §8.4, the output and diagnostic fields in §9, and the reason, scale, and basis codes in §14. Append-only facts: the registry has no level removal, so an "invalidated structural level" cannot revise a target.
- The checked-in `readings_golden.csv` holds 260 bars. It has 75 minor candidate highs, 84 minor candidate lows, 61 major candidate highs, 66 major candidate lows, and 44 minor plus 24 major confirmed pivots. It is the input for the independent Progression golden fixture.
- The M5 merge at `e8fa6f6` passed 466 tests, with `fmt`, `clippy -D warnings`, and `doc -D warnings` clean (PR #62).
- Issue #22 is `In Progress` on the board with `Start date` 2026-10-24.

## 3. Scope

Included:

- `quantrule-model::axes::progression` with the major and minor leg state machines, leg creation from candidates, origin confirmation, completion, supersession, invalidation, and the per-scale leg records.
- The target policy: nearest same-scale confirmed structural level beyond the origin, the ATR fallback, the basis codes, the target version, and the three-bar stability rule.
- `progress`, `displacement_price`, `displacement_atr`, `stage_code`, `target_reached_code`, the published leg's direction and scale, the size percentile, readiness, unavailable reason, state age, and state changes as per-bar values.
- The full record of the leg at each scale, including `leg_id`, `timeframe_secs`, origin, target, progress, lifecycle, stage, and target-reached fields.
- `ProgressionConfig` with `timeframe_secs`, defaulting to `None`, so the assembling layer stamps the timeframe without the axis inferring it from timestamps.
- Unit, golden, causality, determinism, lifecycle-reachability, no-averaging, and edge-case tests, plus the independent golden fixture.
- The M6 plan and transparency log.

Excluded:

- Level, divergence, and signal events (M7–M9), context (M10), cross-timeframe aggregation (M11), the assembled market state (M12), and shell commands (M13).
- Any change to Structure, the readings layer, the kernel, the data engine, the contract's formulas, thresholds, windows, or code values.
- A live leg reference set for events; M7 consumes the registry, not this axis.
- New dependencies, I/O, clock access, randomness, or shell changes.

## 4. Assumptions and uncertainty

The contract is authoritative. The interpretations below are recorded so a reviewer can reject them without reading the code.

- **Leg creation.** Every pivot candidate creates a forming leg at its occurrence bar, with the direction +1 for a low candidate and -1 for a high candidate. The origin price is the candidate high or low. A bar that is both a candidate high and a low is already discarded by the readings layer.
- **Retention and publication.** Each scale keeps its legs in creation order. A leg that fails confirmation is removed unless it is invalidated; terminal legs stay in the list. The per-scale record is the most recently created retained leg at that scale. The published leg is the more recent of the two scale records; when both were created on the same bar, the major scale wins (D1). The full ledger of retained legs is exposed by `ProgressionSeries::legs()` for inspection and reachability tests; it is not a per-bar market-state field.
- **Leg identity.** `leg_id` is one per-series counter in creation order, shared by both scales, starting at 1. A discarded leg consumes its id and is not reused, so a retained id is stable across prefixes (D2).
- **Confirmation.** A confirmed pivot promotes its own leg from lifecycle 0 to 1 and sets `origin_confirmed_code` to 1 and `origin_available_at` to the confirmation bar. A leg that is already terminal keeps its terminal lifecycle; its origin fields still become confirmed when the pivot confirms.
- **Prior transition.** At a confirmation, the most recent active leg before the confirming leg at the same scale becomes lifecycle 2 when its direction opposes the new leg and lifecycle 4 when it matches. Forming and terminal legs are skipped: a discarded candidate must not shield an older active leg from the confirmation that ends it, and a terminal leg is final. The prior leg freezes its measured fields at that bar. This is D10, corrected during verification.
- **Invalidation.** An active leg is invalidated when a close is beyond its origin against the leg by strictly more than `0.2 * ATR_at_origin`, with an ATR that is finite and greater than zero. A forming leg records the breach but is invalidated only when its candidate fails confirmation; if the candidate confirms, the breach is forgotten and the active rule applies from the confirmation bar. No invalidation occurs while ATR at origin is unavailable (D3).
- **Target selection.** At creation the axis selects, among the confirmed levels of the same scale knowable at that bar, the nearest high strictly above the origin for an up leg, or the nearest low strictly below it for a down leg. "Nearest" is the minimum price above the origin or the maximum price below it; an exact price tie goes to the earliest confirmed level. A structural target always takes precedence over the ATR fallback. The fallback is `origin + direction * 2 * ATR_at_origin` with basis 2 when ATR is usable; with no structural level and no usable ATR the target is null with basis null.
- **Target revision.** Only a newly confirmed level can revise a target; the registry is append-only, so the contract's "invalidated structural level" is unrepresentable (D4). Revision applies to every retained non-terminal leg of the scale and direction the new level can target. The nearest is recomputed; the price, basis, version, and stability change only when the selected price differs or a fallback becomes structural. A revision increments `target_version` and sets `target_stable_code` to 0.
- **Stability.** A target that exists at creation is stable. A revision makes it unstable at the revision bar, and it becomes stable again at the third following closed bar with no further revision. A leg with no target has `target_stable_code` 0.
- **Progress and displacement.** `progress = (Close - origin) / (target - origin)` is unclamped and signed. It is null with reason 4 when there is no target, and with reason 3 when the target equals the origin or the absolute distance is below `0.2 * ATR_at_origin`. When ATR at origin is unavailable the threshold test is skipped and progress is computed whenever a target exists and differs from the origin; `displacement_atr` is then null. `displacement_price` and `displacement_atr` are computed even when the target is null. A missing candle makes progress, displacement, and invalidation measurements null with reason 1; on a terminal leg that freezes on the missing bar, the null progress keeps reason 1 on later bars.
- **Terminal freeze.** A leg freezes its progress, displacement, target, stage, and target-reached fields at the bar it becomes terminal. Its target stability clock keeps advancing, because stability is a property of the unchanged reference rather than of the frozen measurements. A terminal lifecycle publishes stage 5 on every later bar it remains the per-scale record. This keeps a completed record from drifting with later price action; the alternative is recorded in §5.
- **Stage.** A terminal lifecycle has precedence and publishes 5. Otherwise a forming leg publishes null, an active leg with an unstable target publishes null, and an active leg with null progress publishes null. The five buckets are 0 for progress below 0, 1 for `[0, 0.25)`, 2 for `[0.25, 0.50)`, 3 for `[0.50, 0.75)`, and 4 for `0.75` and above. `target_reached_code` is 1 when progress is at least 1, 0 when it is below, and null when progress is null.
- **Readiness and reason.** A null progress on a forming leg reports reason 6 (origin unconfirmed), which takes precedence over the target reasons so that reason 6 is reachable from a leg created by a candidate; without a published leg the axis also reports reason 6 because there is no origin to confirm. With a published leg and a null progress, a missing candle reports reason 1 before any leg reason; a frozen terminal leg keeps its available progress and reports no reason. An active or terminal leg with null progress is unavailable with reason 4 when there is no target, reason 3 when the target degenerates or the progress arithmetic is non-finite, and reason 1 when the close is missing (including a terminal leg frozen on a missing-candle bar). Progress with a null stage is provisional with no axis reason, because the primary value exists while the reference is incomplete. Progress and stage with a null percentile or a null `displacement_atr` are provisional. Ready requires an available progress, an available stage, a stable target, and an available percentile.
- **Percentile.** The history holds `abs(displacement_atr)` of the published leg on every bar where it is finite, in bar order, including a measured zero. The current bar is appended only after its percentile is computed. A terminal leg contributes its frozen magnitude on each bar it remains published. The kernel applies the 252-sample window and the 50-sample minimum.
- **State counters.** The axis passes `stage_code` to `StateCounters`. A null stage freezes both counters. The first stage-bearing bar has age 1, and a stage change resets the age and counts once.
- **Timeframe.** `ProgressionConfig::timeframe_secs` defaults to `None`; the axis does not derive a timeframe from candle timestamps. Each leg record carries the configured value. The assembling layer supplies it when the per-timeframe state is built.
- **No cross-axis read.** Targets are rebuilt from `Readings.pivots` through `ConfirmedLevelRegistry::from_pivots`, not read from `StructureSeries`. Reusing the registry type is not a cross-axis read; it is the same pivot reading consumed through the same definition.
- **Mismatched lengths.** The axis iterates over `readings.len()`, the canonical input length. A missing candle nulls that bar's measurements rather than panicking. Aligned inputs are the documented precondition.
- **Golden reference.** The expected columns are produced by an independent Python implementation of §8.4 and §9 over the checked-in readings fixture's independently generated ATR and pivot columns. The fixture is not derived from the Rust code under test.

## 5. Decisions

| ID | Decision | Options | Choice | Status |
|---|---|---|---|---|
| D1 | Leg retention and publication | Only the active leg; every candidate; the most recent retained leg per scale | Keep every unconfirmed candidate, publish the most recently created retained leg per scale, apply 2/4 to the prior leg on confirmation, tie-break publication toward major | Approved 2026-10-24 (`continue`) |
| D2 | Leg identity | Per-scale counters; one per-series counter; no id | One per-series counter in creation order, unique across scales and stable across prefixes | Approved 2026-10-24 (`continue`) |
| D3 | Missing ATR at origin | No leg; null target; ATR fallback only | Create the leg with a null target and null progress; reason 6 while forming and reason 4 once active; no invalidation until ATR exists | Approved 2026-10-24 (`continue`) |
| D4 | Revised or invalidated levels | Rebuild the target set; append-only | The registry is append-only; only newly confirmed levels revise a target; level invalidation is unrepresentable and recorded | Approved 2026-10-24 (`continue`) |
| D5 | Forming-leg invalidation | Invalidate immediately on a breach; invalidate only when confirmation fails | Record the breach while forming; invalidate at the failed confirmation, forget it on confirmation, then apply the active rule | Applied — reviewed 2026-10-24 |
| D6 | Terminal measurements | Keep updating progress; freeze at termination | Freeze progress, displacement, target price, stage, and target-reached at the terminal bar; the target stability clock still advances; a terminal record publishes stage 5 on later bars | Applied — reviewed 2026-10-24 |
| D7 | Reason precedence | Axis reason whenever the stage is null; reason only when progress is null | Reason only when the primary progress is null; a forming or unstable leg with a valid progress is provisional with no reason | Applied — reviewed 2026-10-24 |
| D8 | Golden fixture | Per-bar published fields only; published fields plus both leg records | `progression_golden.csv` with the candle, the published fields, and the full minor and major leg record on every bar | Applied — reviewed 2026-10-24 |
| D9 | File layout | One `progression.rs`; a `progression/` module | `progression/` with `mod.rs`, `leg.rs`, `target.rs`, and `tests.rs` along the state-machine, publication, target, and test seams | Applied — reviewed 2026-10-24 |
| D10 | Prior-leg transition target | Immediately preceding retained leg; most recent active leg | Most recent active leg before the confirming leg; skip forming and terminal legs | Applied — reviewed 2026-10-24 (corrected during verification) |

## 6. Acceptance criteria

- A candidate creates a forming leg with lifecycle 0, `origin_confirmed_code` 0, a null `origin_available_at`, and a null stage; a confirmed pivot promotes it to lifecycle 1 at the confirmation bar and fills `origin_available_at`.
- A confirmed pivot of the opposite kind completes the prior retained leg at the same scale with lifecycle 2; a pivot of the same kind supersedes it with lifecycle 4; the prior leg freezes at that bar.
- A close beyond the origin against an active leg by more than `0.2 * ATR_at_origin` invalidates it with lifecycle 3; no invalidation occurs without a usable ATR.
- The target is the nearest confirmed same-scale level beyond the origin, with basis 1; with none it is the ATR fallback with basis 2; with neither it is null.
- A target revision increments `target_version`, sets `target_stable_code` to 0, and changes no candle price; stability returns at the third unchanged closed bar.
- `progress` is unclamped and signed; a degenerate target gives null progress with reason 3 and a missing target gives null progress with reason 4.
- The stage buckets follow §8.4, a terminal lifecycle forces 5, and the stage is null while the origin is unconfirmed or the target is unstable.
- The published leg is the most recently created retained leg across both scales, with its direction and scale published; the per-scale records never blend the two.
- The size percentile samples `abs(displacement_atr)` of the published leg, excludes the current bar, and applies the kernel window and minimum.
- Readiness is 0 when progress is null, 1 when progress is available without a stable stage, a stable target, or a percentile, and 2 when progress, stage, stable target, and percentile are all available.
- The state counters use `stage_code`, give the first stage-bearing bar age 1, reset on a change, count changes over the last 50 code-bearing bars, and freeze on a null stage.
- Every lifecycle code 0..4 is reachable in the ledger; every stage code 0..5 is reachable in the per-bar scale records; no stage is averaged with another leg's stage.
- A causality test proves that appending or changing later bars does not change an earlier bar; a determinism test proves repeated computation is identical and timestamps do not affect the output.
- A golden fixture covers every published field and both leg records over the 260-bar series; unit tests cover the lifecycle, stage, target, stability, and edge paths the fixture does not reach.
- Edge cases are covered: empty series, single candle, a series shorter than the pivot history, an unaligned timeframe boundary, a gap, a zero-volume candle, a flat series, equal highs or lows, a zero ATR, a missing candle, and an unconfirmed origin at the end of the series.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.

## 7. Work breakdown

| Step | Action | Paths | Depends on |
|---|---|---|---|
| 1 | Record the M6 plan and the initial transparency-log entry | `docs/execution-plans/2026-10-24/22-progression-axis.md`, `docs/ai-transparency-log/2026-10-24/22-progression-axis.md` | Approval |
| 2 | Add the Progression module, the leg state machine, and the target policy; expose it from the axes module and the crate docs | `crates/quantrule-model/src/axes/mod.rs`, `crates/quantrule-model/src/lib.rs`, `crates/quantrule-model/src/axes/progression/mod.rs`, `leg.rs`, `target.rs` | 1 |
| 3 | Add unit tests for creation, confirmation, prior transitions, invalidation, targets, revision, stability, progress, stage, readiness, percentile, counters, and every code | the three Progression files | 2 |
| 4 | Generate the independent golden fixture and add the integration tests | `crates/quantrule-model/tests/fixtures/progression_golden.csv`, `crates/quantrule-model/tests/progression_golden.rs` | 2 |
| 5 | Run the full checks and review the diff against the contract and the issue | all changed files | 3–4 |
| 6 | Record the review and evidence in the plan and log | this plan, the transparency log | 5 |
| 7 | Commit, push, open the PR, and move #22 to Done | all changed files | 6 and the commit/PR authorization |

## 8. Verification

- **Creation:** a minor and a major candidate each create a forming leg; two candidates on the same bar create two legs with consecutive ids; a bar that is a candidate at both scales creates both.
- **Confirmation:** a confirmed pivot promotes its leg at `occurred_at + lookback`; a leg that is already terminal keeps its lifecycle; `origin_available_at` is the confirmation bar.
- **Prior transitions:** opposite kind gives 2, same kind gives 4, a terminal prior is untouched, a forming prior is not transitioned, and the most recent active leg is found past a discarded candidate.
- **Invalidation:** an active leg closes beyond the origin by more than the threshold; exactly at the threshold does not invalidate; a forming leg that fails and breached invalidates, one that confirms forgets the breach, and one without ATR is discarded instead.
- **Targets:** nearest high above and low below; an exact price tie yields the same price, because the target record stores the price rather than the level's identity; no structural level uses the fallback; no structural level and no ATR leaves the target null; a fallback becomes structural on the first qualifying confirmation.
- **Revision:** a nearer level increments the version, sets stability 0, and leaves the close unchanged; a farther level does not revise; stability returns at the third unchanged bar and a second revision resets the count.
- **Progress:** a positive, a negative, and an above-one progress; a target equal to the origin, a target closer than `0.2 * ATR`, a null target, and a missing candle.
- **Stage:** every bucket boundary at 0, 0.25, 0.5, and 0.75; null while forming; null while unstable; 5 for each terminal lifecycle.
- **Publication:** the most recent retained leg wins; a same-bar tie chooses major; a discarded latest leg falls back to the prior retained leg; an empty scale is absent.
- **Diagnostics:** 49 prior samples give null and readiness 1; 50 give a value and readiness 2; the current bar is excluded; a null stage freezes the counters; a stage change resets the age; a crafted path reaches stage 0, stages 1–4, readiness 2, and a nonzero `state_changes_50`.
- **Causality:** every prefix matches the corresponding prefix of the full series; changing a later candle leaves earlier bars unchanged.
- **Determinism:** two computations over the same candles are equal; changed timestamps leave the output unchanged.
- **Golden:** every published field and every per-scale leg field, with `1e-9` for continuous values and exact equality for codes, ids, bars, and counters.
- **Edge cases:** empty, single, short, unaligned timestamps, gap, zero volume, flat, equal highs/lows, zero ATR, missing candle, and an unconfirmed candidate at the end.

Commands:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`

The evidence is the observed command output, recorded in the PR and the transparency log. No claim is made about an event, aggregate, or shell behavior this item does not implement.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| A leg is created or promoted from a pivot before it is knowable | Legs are created from `candidate(scale, bar)` at the occurrence bar and promoted from `confirmed_at(scale, bar)` at the availability bar; the prefix test asserts the boundary |
| The prior-leg transition touches the wrong leg | The scale list is in creation order; the most recent active leg is transitioned, forming and terminal legs are skipped, and hand-built tests cover a terminal prior, a forming prior, and a discarded candidate before an older active leg |
| A forming leg's breach is handled inconsistently | The breach is recorded while forming and resolved once at the failed confirmation; a confirmed leg forgets it and is then subject to the active rule |
| A target revision silently changes a price or misses a change | Revision is a pure function of the newly confirmed level and the as-of registry; the version, stability, basis, and price are asserted directly |
| A target is revised by a level that is not yet knowable | Revision uses `as_of(bar)` and the registry's availability filter; a later-level test asserts no earlier revision |
| A terminal leg's progress drifts | Progress, displacement, target price, stage, and target-reached freeze at the termination bar; only the target stability clock advances, and a test compares the frozen record on later bars |
| The percentile samples a null or a duplicate | Only finite `abs(displacement_atr)` values enter the buffer; a null contributes no sample; the current bar is appended after its own percentile |
| The two scales are blended | The published leg is one record chosen by creation order; the per-scale records are separate; no stage arithmetic crosses scales |
| The golden generator and the Rust code share the same bug | The generator is an independent Python implementation from the checked-in readings columns, not a call into the Rust code |
| The fixture does not reach every published code | Coverage is measured and recorded; the per-bar scale records reach every stage code, and the crafted stage test covers published stages 0–4, readiness 2, and nonzero state changes |
| A file grows past the file-size limit | The module is split into `mod.rs`, `leg.rs`, `target.rs`, and `tests.rs` along its own seams |
| A merge proceeds without authorization | The program stops at D3's gate; no merge without explicit human authorization |

## 10. Stop conditions

Stop and request human direction if:

- The contract requires a semantic change to leg creation, the prior transition, invalidation, the target policy, progress, the stage table, or the codes.
- The forming-leg breach rule, terminal freeze, or reason precedence cannot be reconciled with §8.4 and §9.
- The append-only registry cannot express a target revision the contract requires.
- The published-leg tie or the ledger accessor is judged outside the approved scope.
- A new dependency is needed.
- The readings layer lacks a value Progression requires, or its availability semantics cannot support the leg or the target.
- The golden fixture cannot be reproduced from the checked-in candles, ATR, and pivots.
- The toolchain cannot run `fmt`, `clippy`, `test`, or `doc`.
- A merge authorization is withheld or the PR is rejected.

## 11. Evidence

Implementation, independent review, and local verification are complete. The commit, push, pull request, and board transition remain gated by §9 of `AGENTS.md`.

### Implementation

| Path | Lines | Content |
|---|---|---|
| `crates/quantrule-model/src/axes/progression/mod.rs` | 545 | `ProgressionLeg`, `ProgressionBar`, `ProgressionConfig`, `ProgressionSeries`, the compute loop, publication, readiness, and `transition_prior` |
| `crates/quantrule-model/src/axes/progression/leg.rs` | 870 | `Lifecycle`, the `Leg` state machine, target revision and stability, progress, stage, invalidation, and 30 unit tests |
| `crates/quantrule-model/src/axes/progression/target.rs` | 297 | Nearest same-scale structural target, ATR fallback, basis codes, and 10 unit tests |
| `crates/quantrule-model/src/axes/progression/tests.rs` | 856 | 27 series-level unit tests, including the crafted stage/readiness path |
| `crates/quantrule-model/tests/progression_golden.rs` | 685 | 18 golden, causality, determinism, and edge-case tests |
| `crates/quantrule-model/tests/fixtures/progression_golden.csv` | 261 lines | 260 bars, 52 columns, LF |
| `crates/quantrule-model/tests/fixtures/progression_legs_golden.csv` | 173 lines | 172 retained legs, 18 columns, LF |
| `crates/quantrule-model/src/axes/mod.rs` | — | Exposes `progression` |
| `crates/quantrule-model/src/lib.rs` | — | Crate docs name the four implemented axes |

All five source and test files are under the 1,000-line limit; the largest is `leg.rs` at 870 lines. The unit tests total 67 in the module (`leg.rs` 30, `target.rs` 10, `tests.rs` 27) and the integration test adds 18.

### Fixture provenance

The fixtures were produced by `/tmp/opencode/progression_fixture.py`, an external Python script that implements contract §8.4 and §9 directly from the checked-in `readings_golden.csv` ATR and pivot columns under the approved D1–D10 interpretations. It does not call or mirror the Rust code. The generator is not checked in, so its output is the reproducible artefact; re-running it reproduced both CSVs byte-identically with LF line endings. Observed fixture coverage:

| Measure | Value |
|---|---|
| Bars | 260 |
| Retained legs | 172 |
| Lifecycle codes in the ledger | 0: 5, 1: 2, 2: 51, 3: 108, 4: 6 |
| Stage codes in the ledger | 4: 2, 5: 165, null: 5 |
| Stage codes in the per-bar scale records | 0: 1, 1: 10, 2: 5, 3: 4, 4: 5, 5: 8, null: 487 |
| Published stage codes | 4: 1, null: 259 |
| Readiness codes | 0: 38, 1: 222, 2: 0 |
| Unavailable reasons | 6: 38, null: 222 |
| Percentile bars | 196 |
| Target revisions | 10 |
| Target basis codes | 1: 169, 2: 1, null: 2 |
| Published scale codes | 1: 61, 2: 197, null: 2 |
| Published direction codes | -1: 118, 1: 140, null: 2 |
| Maximum `state_changes_50` | 0 |

The fixture reaches every lifecycle code and every stage code across its per-bar scale records, but its published leg is forming on almost every bar, so the published stage is only 4 or null, readiness is never 2, and the published state never changes. That is a consequence of the approved D1 publication rule, not a defect. The crafted test `every_published_stage_and_ready_are_reachable_on_a_crafted_path` covers published stages 0–4, readiness 2, and a nonzero `state_changes_50`; the terminal stage 5 is covered by the ledger and the per-bar scale records.

### Corrections made during implementation

| Correction | Reason |
|---|---|
| `transition_prior` now finds the most recent **active** leg before the confirming leg, skipping forming and terminal legs | The earlier "immediately preceding retained leg" reading let a discarded candidate shield an older active leg from completion or supersession. Recorded as D10 |
| A forming leg with null progress reports reason 6, not reason 4 | Reason 6 must be reachable from a leg created by a candidate (§8.4, A2); a missing candle still reports reason 1 first. The plan's earlier statement that no published leg reports reason 6 was wrong |
| The module tests moved from `mod.rs` to `tests.rs` | `mod.rs` had grown past the 1,000-line limit; the split follows the production/test seam |
| `ProgressionConfig` derives `Default` | Clippy `derivable_impls`; the derived value is the same `None` |
| The unused `created_at` field was removed | Publication uses the creation-order identifier, not a separate bar field |
| `ProgressionSeries::bars()` was added | The golden and causality tests need the full per-bar slice without repeated `get` calls |
| A terminal leg now advances its target stability clock after its measurements freeze | Readiness requires a stable target; a leg revised within three bars of termination previously stayed unstable forever and published stage 5 as provisional/ready incorrectly |
| Readiness requires a stable target | A terminal leg inside the three-bar stability window publishes stage 5 but is not complete; the earlier rule marked it Ready as soon as stage 5 and a percentile existed |
| A terminal leg frozen with a null progress on a missing-candle bar now reports reason 1 | The missing close, not a degenerate target, caused the null progress; the earlier code reported reason 3 for any terminal leg with a target |
| Contract §8.4's published-leg sentence now says retained (forming, active, or terminal) | D1/D6 publish a terminal leg with frozen measurements; the earlier wording implied only forming or active legs are published |
| The target module records that only a nearer structural level can revise a target | The append-only registry allows a farther level to confirm without changing the selected nearest price; the earlier "move closer" wording was imprecise |

### Checks

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Clean, no output |
| `cargo clippy --all-targets -- -D warnings` | Clean, no warnings |
| `cargo test` | 553 passed, 0 failed: data 90, model unit 225, direction 14, momentum 14, progression 18, readings 11, structure 15, shell unit 93, shell CLI 47, doc 2 + 24 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean, no warnings |

The Progression filter alone reports 67 unit tests and 18 golden tests passing.

### Independent review

Two read-only review passes were run against the contract, the plan, and the code. The first pass found one behavioral defect and several test/documentation gaps; the second pass verified the fixes and identified the remaining documentation and test gaps. All findings are now closed or recorded as limitations below.

| Finding | Severity | Status |
|---|---|---|
| A terminal leg never regained target stability after freezing, so it could publish Ready with an unstable target | Major | Fixed: `Leg::update` advances the stability clock before the frozen check; readiness requires a stable target. Tests: `a_terminal_leg_regains_target_stability`, `ready_requires_a_stable_target` |
| The transparency log still described implementation as not started | Major | Fixed: the implementation, testing, review, and evidence entries are recorded in the log |
| Changing a later candle's price was untested | Major | Fixed: `changing_a_later_candle_does_not_change_earlier_records` changes one candle at four positions and asserts all earlier bars are unchanged |
| Per-bar stage coverage was unasserted | Major | Fixed: the fixture's per-bar scale records reach stages 0–5; `every_stage_code_is_reachable_in_the_scale_records` asserts the set |
| The 49/50 percentile boundary and current-bar exclusion were unasserted | Major | Fixed: `the_percentile_starts_at_fifty_prior_samples` proves 50 prior samples, 49 prior gives null, and the current bar is excluded |
| Contract §8.4 said the published leg is forming or active, while D1/D6 publish terminal legs | Minor | Fixed: the contract now says retained (forming, active, or terminal) and describes the terminal freeze |
| Plan readiness, freeze, line-count, and test-count text was stale | Minor | Fixed: §4, §6, §8, and §11 updated to the final behavior and counts |
| No test for a farther level failing to revise, an exact-price tie, or a second-revision reset | Minor | Fixed: `a_farther_confirmed_high_does_not_replace_the_nearest`, `two_levels_at_the_same_price_yield_that_price`, `a_same_target_does_not_manufacture_a_revision`, `a_second_revision_restarts_the_stability_clock` |
| Zero-ATR end-to-end leg absent | Minor | Accepted limitation: a strict origin requires a non-zero true range, so a zero ATR at a pivot origin is unreachable; `a_zero_atr_is_not_a_usable_origin_distance` covers the gate directly |
| Reasons 2 and 5 are unused; internal field names differ from the contract's prose | Not a defect | Recorded: reason 2 is not required by §14/A2 for Progression, and the Rust API is internal; the published codes match the contract |

After the second pass, the missing-close reason refinement (the `missing_close` field, the axis-level guard, and the tests it drove) and the documentation updates were self-verified. A third independent delta review was attempted, but the reviewer session reached its usage limit before it could run. The two independent passes cover the production logic before that small refinement; the refinement is covered by `a_missing_close_nulls_progress_with_reason_one`, `a_terminal_leg_frozen_on_a_missing_close_keeps_reason_one`, the existing missing-candle tests, and the final check run.

### Acceptance criteria

All criteria in §6 are met by the implementation and the observed checks, with these recorded limitations:

- The golden fixture's published leg is forming on 259 of 260 bars, so the published stage is 4 once and null otherwise; readiness 2 and published stages 0–3 are covered by the crafted unit test, not by the fixture.
- The fixture generator is external to the repository, so fixture regeneration depends on `/tmp/opencode/progression_fixture.py` being retained or re-derived. The generator's output, not the generator, is the checked-in artefact; rerunning it reproduced both CSVs byte-identically with LF.
- The exact-price target tie is tested only through the selected price, because the target record stores the price rather than the level's identity; the strict comparison retains the earliest level.
- The forming-leg reason precedence (6 before 3 and 4) is an interpretation of §8.4/A2 recorded in §4; the contract does not state the precedence explicitly.
- A zero ATR at a pivot origin is unreachable from candles, so the zero-ATR path is tested at the `usable_atr` gate rather than end to end.

Status: `Verified`. The commit, push, pull request, and board transition remain gated by §9 of `AGENTS.md`.

## 12. Delivery

The working tree is ready for delivery. The following operations are proposed, not performed, and each requires the human's explicit authorization under §9 of `AGENTS.md`:

- Commit the Progression axis, the golden fixtures, the contract clarification, this plan, and the transparency log on `feat/22-progression-axis`.
- Push the branch and open the pull request against `main`.
- Move issue #22 to `Done` on the project board once the pull request is open.

No merge, release, or tag is requested by this plan. The branch and worktree cleanup follows the merged-PR cleanup rule in §9 after acceptance.
