# AI Transparency Log: Progression Axis (M6)

- Work item: [#22](https://github.com/quant-rule/quant-rule/issues/22) — feat: build the progression axis
- Plan: [`docs/execution-plans/2026-10-24/22-progression-axis.md`](../../execution-plans/2026-10-24/22-progression-axis.md)
- Program plan: [`docs/execution-plans/2026-10-24/14-model-engine-completion.md`](../../execution-plans/2026-10-24/14-model-engine-completion.md)
- Date: 2026-10-24 (UTC)

## Entry: plan and repository assessment

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash Free).
- **Intent and boundary:** Read the M5 Structure axis and registry, the M2 pivot series, the M1 kernel, the model contract §5, §8.4, §9, §14, and §15, issue #22, the program plan, and the umbrella plan, then write the M6 execution plan and this log. No implementation file was changed in this entry. The four M6 defaults D1–D4 were approved by the human's `continue`; the plan records them and the implementation interpretations D5–D9 for review.
- **Verification and evidence:**
  - `main` is at `e8fa6f6`; the M6 worktree `../quant-rule--22-progression-axis` was created from that commit on branch `feat/22-progression-axis`.
  - The M5 merge at `e8fa6f6` passed 466 tests with `fmt`, `clippy -D warnings`, and `doc -D warnings` clean, as recorded in PR #62.
  - `crates/quantrule-model/src/axes/mod.rs` exposes `direction`, `momentum`, and `structure`; `Readings` exposes the three EMAs, RSI, ATR, RVOL, and pivots; `ConfirmedLevelRegistry::from_pivots` is available in `axes::structure::registry`.
  - The checked-in `readings_golden.csv` has 260 bars, 75 minor candidate highs, 84 minor candidate lows, 61 major candidate highs, 66 major candidate lows, and 44 minor plus 24 major confirmed pivots. Those counts are the input coverage for the golden fixture.
  - Issue #22 is `In Progress` on the board with `Start date` 2026-10-24.
- **Learning:**
  - Progression must rebuild the confirmed-level registry from `Readings.pivots` rather than reading `StructureSeries`, because §8.4 makes the pivots the source and an axis may not read another axis's output. Reusing the registry type is not a cross-axis read.
  - The contract's "invalidated structural level" has no representation in the append-only registry. D4 records that as unrepresentable rather than inventing a removal path.
  - Two publication questions are not settled by §8.4: whether a terminal leg stays published, and whether a breached forming leg is invalidated immediately or only when confirmation fails. The plan records both interpretations (D5, D6) so the human can reject them without reading the code.
  - The contract lists `timeframe_secs` on every leg but the axis has no timeframe input. The plan adds an optional configuration value that the assembling layer fills; the axis never infers it from timestamps.
  - The Stage 5 code must remain observable. The ledger exposed by `ProgressionSeries::legs()` makes every lifecycle and stage code reachable for the issue's acceptance test without adding a per-bar market-state field.

## Entry: implementation

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Implement the approved M6 plan: the Progression axis under `crates/quantrule-model/src/axes/progression/`, wired from `axes/mod.rs` and described in `lib.rs`. No file outside the model crate, the contract, the plan, and this log was changed. No commit, push, or pull request was made.
- **Verification and evidence:**
  - Added `target.rs` (target selection, basis codes, fallback finite check), `leg.rs` (the `Leg` state machine, lifecycle, stability, progress, stage, invalidation), `mod.rs` (public types, compute loop, publication, readiness, `transition_prior`), and `tests.rs` (series-level tests).
  - The implementation reads `Readings.pivots` and `Readings.atr14`; it rebuilds `ConfirmedLevelRegistry::from_pivots` rather than reading `StructureSeries`.
  - `cargo test -p quantrule-model progression` passed 67 unit tests and 18 golden tests at the final state.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` were clean.
- **Learning:**
  - The first draft published `scales.minor.last()` and `scales.major.last()` without a lifecycle filter, which is the approved D1 "retained leg" rule. The contract's earlier "forming or active" wording implied a narrower rule; the contract was clarified, not the code, because D1/D6 were explicitly approved.
  - The first draft froze a terminal leg before advancing its target stability clock. The review caught that a leg revised within three bars of termination never stabilised; the fix advances the clock before the frozen check and makes readiness depend on a stable target.
  - A terminal leg frozen on a missing-candle bar kept a null progress and was labelled reason 3 on later bars. The fix records the missing close on the leg and reports reason 1, so the reason describes the actual cause.
  - The earlier "immediately preceding retained leg" prior-transition rule let a discarded candidate shield an older active leg. `transition_prior` now finds the most recent active leg and skips forming and terminal legs (D10).

## Entry: golden fixture

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017) with an external Python generator.
- **Intent and boundary:** Produce an independent expected-output fixture for §8.4 and §9 from the checked-in `readings_golden.csv` ATR and pivot columns, under D1–D10. The generator is `/tmp/opencode/progression_fixture.py`; it does not call or import the Rust code and is not checked in.
- **Verification and evidence:**
  - `progression_golden.csv`: 260 bars, 52 columns, LF, 71,372 bytes; `progression_legs_golden.csv`: 172 legs, 18 columns, LF, 16,816 bytes.
  - Rerunning the generator reproduced both files byte-identically (`sha256sum -c` reported a match for both).
  - Ledger lifecycle codes: 0: 5, 1: 2, 2: 51, 3: 108, 4: 6. Ledger stage codes: 4: 2, 5: 165, null: 5.
  - Per-bar scale stage codes: 0: 1, 1: 10, 2: 5, 3: 4, 4: 5, 5: 8, null: 487. Published stage: 4: 1, null: 259.
  - Readiness: 0: 38, 1: 222, 2: 0. Reasons: 6: 38, null: 222. Percentile bars: 196. Revisions: 10. Basis: 1: 169, 2: 1, null: 2. Published scale: 1: 61, 2: 197, null: 2. Published direction: -1: 118, 1: 140, null: 2. Maximum `state_changes_50`: 0.
  - The fixture's published leg is forming on 259 of 260 bars, so the crafted unit test covers published stages 0–4, readiness 2, and nonzero state changes; the fixture covers stage 5 and every lifecycle code through the ledger and the per-bar scale records.
- **Learning:**
  - The fixture's coverage is a property of the D1 publication rule, not a defect: a forming candidate is usually the newest leg. Coverage claims therefore distinguish the ledger, the per-bar scale records, and the published fields.
  - The generator had to be updated twice after semantic corrections (terminal stability and readiness). Regenerating it after each change kept the fixture an independent oracle rather than a snapshot of the Rust output.

## Entry: independent review and corrections

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017), with two read-only reviewer subagent passes.
- **Intent and boundary:** Review the complete diff against contract §8.4/§9/§14/§15, the approved plan, and `AGENTS.md`, then correct in-scope findings. No commit, push, merge, or board transition was performed.
- **Verification and evidence:**
  - First pass: no production blocker; one major behavioral defect (terminal stability freeze), one stale required artefact (this log), and test/documentation gaps. The reviewer confirmed the fixture is independent and reproduced it byte-identically.
  - Corrections: terminal stability now advances before the frozen check; readiness requires a stable target; a terminal leg frozen on a missing close reports reason 1; the equal-highs/lows test now asserts the candidate/discard path; the gap and zero-volume tests were split and strengthened; `a_zero_atr_is_not_a_usable_origin_distance`, `a_missing_candle_without_a_leg_reports_an_unconfirmed_origin`, `an_overflowing_fallback_is_not_a_target`, `a_farther_confirmed_high_does_not_replace_the_nearest`, `two_levels_at_the_same_price_yield_that_price`, `a_same_target_does_not_manufacture_a_revision`, `changing_a_later_candle_does_not_change_earlier_records`, `irregular_timestamps_do_not_affect_the_output`, `every_stage_code_is_reachable_in_the_scale_records`, and `the_percentile_starts_at_fifty_prior_samples` were added.
  - Second pass verified the behavioral fixes and reported the remaining plan/log/contract text as stale. The plan's §4, §6, §8, and §11 and contract §8.4's published-leg sentence were updated; this log now records the implementation and review entries.
  - The reviewer's claim that the per-bar fixture had zero stage-5 rows was checked and rejected: the minor per-bar records contain 8 stage-5 rows. The stage-coverage test was added anyway to make the claim explicit.
  - The reviewer's suggestion to change the contract was accepted: contract §8.4 now says the published leg is a retained leg (forming, active, or terminal), matching the approved D1/D6 behavior.
  - The reviewer's note that reasons 2 and 5 are unused was checked and recorded as not a defect: §14/A2 does not require them for Progression.
  - A third delta review was attempted after the final missing-close reason refinement, but the reviewer session reached its usage limit before it could run. The delta was self-verified with the new tests and the final checks; this log does not claim a third independent review.
- **Learning:**
  - The terminal-freeze defect was a real interaction between two approved rules (freeze at termination and three-bar stability). A state that is frozen for measurement is not necessarily frozen for reference tracking.
  - Readiness is a conjunction across progress, stage, target stability, and percentile; testing only the stage and percentile leaves the stability gate unverified.
  - A fixture that covers every code in one view can still miss a code in the published view. Coverage claims must name the view they measure.

## Final verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test`: 553 passed, 0 failed (data 90, model unit 225, direction 14, momentum 14, progression 18, readings 11, structure 15, shell unit 93, shell CLI 47, doc 2 + 24).
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Progression filter: 67 unit tests and 18 golden tests passed.
- The two fixtures reproduce byte-identically from the external generator.

## Entry: delivery

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** The human approved the commit, push, pull request, and board transition. The implementation, fixtures, contract clarification, plan, and log were committed and pushed; the pull request was opened; issue #22 was moved to `Done`. No merge, release, tag, or deployment was performed.
- **Verification and evidence:**
  - Commit `0de472e` — `feat: build the progression axis` — on `feat/22-progression-axis`, pushed to `origin`.
  - Pull request [#63](https://github.com/quant-rule/quant-rule/pull/63), open against `main`, with the summary, scope, verification, risks, and AI-transparency reference.
  - Issue #22 moved to `Done` on the project board after the pull request existed; the board's workflow owns the resulting issue state.
  - The delivery evidence itself is committed in a follow-up commit on the same branch so the plan and log carry the commit identifier and PR number.
- **Learning:**
  - The board rule requires the pull request to exist before the item moves to `Done`; the order was commit, push, PR, then board.
  - The commit and PR evidence belongs in the branch, not in the merge commit, so the plan and log are updated in a second commit rather than amended into the first.

## Current state

- M6 is implemented, independently reviewed, locally verified, committed, pushed, and open as PR #63.
- The plan is `Verified` and records the delivery evidence.
- Issue #22 is `Done` on the board.
- No merge, release, tag, or deployment was performed. Merge authorization remains with the human.
