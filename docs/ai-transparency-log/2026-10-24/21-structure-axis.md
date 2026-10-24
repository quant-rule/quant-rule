# AI Transparency Log: Structure Axis (M5)

- Work item: [#21](https://github.com/quant-rule/quant-rule/issues/21) — feat: build the structure axis
- Plan: [`docs/execution-plans/2026-10-24/21-structure-axis.md`](../../execution-plans/2026-10-24/21-structure-axis.md)
- Program plan: [`docs/execution-plans/2026-10-24/14-model-engine-completion.md`](../../execution-plans/2026-10-24/14-model-engine-completion.md)
- Date: 2026-10-24 (UTC)

## Entry: plan and repository assessment

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash Free).
- **Intent and boundary:** Read the M4 Momentum axis, the M2 readings, the M1 kernel, the model contract §7.2, §8.3, §9, and §14, issue #21, the program plan, and the umbrella plan, then write the M5 execution plan and this log. No implementation file was changed in this entry. The plan was `Proposed` and waited for human approval.
- **Verification and evidence:**
  - The M4 pull request #61 was merged as `970e120`; its worktree and local and remote branch were removed after a content comparison against `origin/main` was empty.
  - The M5 worktree `../quant-rule--21-structure-axis` was created from `origin/main` at `970e120` on branch `feat/21-structure-axis`.
  - Issue #21 is `In Progress` on the board with `Start date` 2026-10-24.
  - `crates/quantrule-model/src/axes/mod.rs` exposed `direction` and `momentum`; `Readings` exposes `ema20`, `ema50`, `ema200`, `rsi14`, `atr14`, `rvol20`, and `pivots`; `PivotSeries` stores confirmed pivots by availability bar.
  - The checked-in `readings_golden.csv` has 260 bars, 44 confirmed minor pivots, and 24 confirmed major pivots (12 highs and 12 lows). At the major scale it yields 34 null-score bars, 226 valid score bars, structure codes -2/0/+2 only, 13 code changes, 176 percentile bars, and 5 bars with the high relation available while the low relation is null. It has no equal highs or equal lows.
  - The M4 merge at `970e120` passed 418 tests with `fmt`, `clippy -D warnings`, and `doc -D warnings` clean, as recorded in PR #61.
- **Learning:**
  - Structure needs the candle series as well as the readings, because `Readings` does not carry close, high, or low. The plan passes both inputs explicitly and documents the alignment precondition.
  - The confirmed-level registry must hold both scales for M6's same-scale targets and M7's bounded latest-high/low selection; the configured scale applies to the per-bar swing fields, not to the registry.
  - The readings fixture reaches only the -2, 0, and +2 structure codes and has no equal relations. The plan records that coverage and leaves the ±1 and equal-relation paths to unit tests rather than claiming the golden fixture proves them.
  - Three contract points are genuinely ambiguous and were presented as decisions: the incomplete-pair reason (1 or 2), the retest code when the close finishes exactly on the EMA, and whether the registry's `1000 + n` sequence starts at 0 or 1. The human approved reason 2, code 0, and 1-based numbering (D4–D6).
  - The EMA features are auxiliary: a null EMA feature must not make the axis unavailable while the swing score is valid, and §9 has no per-feature reason field. The plan keeps the axis reason tied to the primary score and records the limitation.

## Entry: implementation

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash Free).
- **Intent and boundary:** Implement the approved M5 scope: the Structure axis module, its EMA features, the confirmed-level registry, the unit tests, the independent golden fixtures, and the integration tests. The scope excluded Progression, events, aggregation, market state, shell commands, the readings layer, the kernel, the data engine, new dependencies, and any change to the contract's formulas or codes.
- **Verification and evidence:**
  - Created `crates/quantrule-model/src/axes/structure/{mod.rs,ema.rs,registry.rs}` and exposed `structure` from `axes/mod.rs`; updated the `lib.rs` crate docs.
  - `mod.rs` publishes `StructureBar`, `StructureConfig`, and `StructureSeries`; `ema.rs` publishes the nine EMA features and keeps the retest half-width at 0.2 ATR; `registry.rs` publishes `ConfirmedLevel` and `ConfirmedLevelRegistry` with `reference_code = 1000 + n`.
  - Added 14 unit tests in `mod.rs`, 11 in `ema.rs`, and 5 in `registry.rs`, covering the relation table, the full score and code table, scale configuration, the percentile minimum and the 252-sample trim, readiness transitions, counter freezing, the two no-double-count mutation tests, empty, single, short, missing-series, flat, and both-candidate inputs.
  - Added 15 tests in `crates/quantrule-model/tests/structure_golden.rs`: the golden comparison of every per-bar field and every registry row, every-prefix causality, a changed later candle, repeated-run determinism, timestamp independence, contract bounds, and the edge cases the plan names.
  - Observed checks: `cargo fmt --all -- --check` clean, `cargo clippy --all-targets -- -D warnings` clean, `cargo test` 466 passed and 0 failed, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` clean.
- **Learning:**
  - The percentile buffer is bounded to the kernel window rather than the input length, so a long series does not retain every historical score. A test over a 298-bar zigzag proves the trim actually changes the published percentile; a shorter series cannot, because the difference never appears.
  - A close exactly on the EMA is ambiguous for the cross as well as the retest. The contract's `at or below` / `at or above` reading makes it a cross in both directions, which is now tested rather than assumed.
  - The relation pair is resolved per kind, so one relation can be published while the score is null. Five fixture bars exercise this, and the golden test asserts the count rather than only the individual rows.

## Entry: independent review and corrections

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash Free), a separate read-only subagent session.
- **Intent and boundary:** Review the uncommitted M5 diff against AGENTS.md sections 3, 4, 12, and 13, contract §7.2, §8.3, §9, and §14, the approved plan, and issue #21. The reviewer was instructed not to modify any file and not to approve, merge, or commit. The session is a different agent context but the same model family, so this was not an independent model judgement; that limitation is recorded here rather than presented as more than it was.
- **Verification and evidence:**
  - The reviewer found no blocker and no major code defect. It confirmed the relation selection and availability, the score and code table, the EMA formulas and null rules, causality across prefixes, the registry ordering, the reference codes, determinism, and the no-double-count separation.
  - It reported one major process finding and seven minor or nit findings; all were corrected before this entry. The corrections are listed in plan §11.
  - The reviewer independently re-ran the format, clippy, test, and doc commands and reproduced both fixtures from the external generator.
- **Learning:**
  - The most useful findings were about evidence rather than logic: the plan and log claimed a state the tree contradicted, and the percentile trim was asserted by a series too short to distinguish a capped from an uncapped window. A passing test that cannot reach the difference does not prove it.
  - Two test names described behaviour they did not assert: the missing-candle test passes an empty candle slice, and the gap test only checks a well-formed latest bar. Both are renamed to say what they do.
  - A `pub mod ema` exposed an internal feature helper, and a `get` parameter named `sequence` took a zero-based index. Neither was a defect, but both would have confused the next reader.

## Entry: evidence and handoff

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash Free).
- **Intent and boundary:** Record the observed results in plan §11, set its status, and prepare the Git handoff. No commit, push, or pull request was created; those remain gated on explicit human authorization.
- **Verification and evidence:**
  - Plan §11 now records the implemented paths, the fixture provenance and coverage, the observed check results, the review findings and their corrections, and the acceptance-criteria status. The plan status is `Verified`.
  - Fixture provenance: `/tmp/opencode/structure_fixture.py`, an external Python implementation of §8.3 and §9 over the checked-in readings fixture. It is not checked in; re-running it reproduced both CSVs byte-identically with LF line endings.
  - `git status --short --branch` on `feat/21-structure-axis` shows two modified files, six untracked implementation and fixture paths, and the two untracked documentation files. No unrelated file is modified.
- **Learning:**
  - The generator being outside the repository is a real reproducibility gap: the fixtures are checked in, but regenerating them depends on a file that is not. The plan records this instead of implying a checked-in generator exists.
  - The deliverable is verified but not delivered. The board stays `In Progress` until a pull request exists, per AGENTS.md section 9.

## Entry: delivery

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash Free).
- **Intent and boundary:** Commit, push, and open the pull request after explicit human authorization. Merge, release, and deployment were not authorized and were not performed.
- **Verification and evidence:**
  - Committed `0b9edb5 feat: add the structure axis` with 10 files and 3,087 insertions; the staged diff contained only the nine M5 implementation, fixture, and test paths plus the two documentation files.
  - Pushed `feat/21-structure-axis` to `origin`.
  - Opened [#62](https://github.com/quant-rule/quant-rule/pull/62) against `main`; GitHub reports it mergeable with the `sync` check passing.
  - Moved #21 to `Done` on the board and labelled both items with their `Item type`; the project workflow closed issue #21 and linked PR #62 to it.
- **Learning:**
  - Marking a `Done` board item closes its issue through the project workflow, so the issue is closed while the pull request is still open. That is the lifecycle the repository defines, not a premature close by the agent.
  - The PR item arrived from GitHub's automation with an empty `Item type` and a `Todo` status; it was labelled `Pull request` and moved to `In Progress` to reflect an open, unmerged change.

## Current state

- M5 is implemented, reviewed, verified, committed, pushed, and open as PR #62.
- Issue #21 is `Done` on the board and closed; the PR is open and awaits human review and merge authorization.
- No merge, tag, release, or deployment was performed.
