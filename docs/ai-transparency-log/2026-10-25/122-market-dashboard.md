# AI Transparency Log — Market-Dashboard

- Date: 2026-10-25 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Work item: [#122](https://github.com/quant-rule/quant-rule/issues/122) — container
- Plan: [`docs/execution-plans/2026-10-25/122-market-dashboard.md`](../../execution-plans/2026-10-25/122-market-dashboard.md)
- Branches/worktrees: `docs/123-dashboard-view-contract` in `quant-rule--dashboard-view-contract` (merged, PR #132); `feat/124-dashboard-view-types` in `quant-rule--dashboard-view-types` (merged, PR #133); `feat/125-dashboard-assembler` in `quant-rule--dashboard-assembler` (Phase 3)
- Status: Phase 0 complete; Phase 1 contract approved 2026-10-25 and merged in PR #132; Phase 2 value types merged in PR #133; Phase 3 core assembler implemented and locally verified; committed as `15da01d` in [PR #134](https://github.com/quant-rule/quant-rule/pull/134), with `verify` and `sync` passing. No Phase 3 merge yet.

## Intent and boundary

The planning session defined an additive, single-instrument CLI dashboard over QuantRule's existing four-axis model. The scope is a pure `DashboardState` projection, a static `market-dashboard` text/JSON command, and view-only Direction bias tenure, Progression recent pace, Structure activity, typed readings, cross-timeframe agreement, and churn.

Boundary of this entry:

- Phase 0 setup: parent and sub-issues, project board statuses, first worktree, approved plan, and this log.
- Phase 1: the `DashboardState` contract document and its `docs/README.md` index entry. The human approved the contract on 2026-10-25; it is marked `Approved`; PR #132 is merged.
- Phase 2: the `quantrule-model::dashboard` value types and their unit tests. No assembler, shell command, or rendered output.
- Phase 3: the core `DashboardState` assembler and its unit tests. No bias tenure, recent pace, Structure activity, typed readings, derived agreement/churn, shell command, or rendered output.
- No repository change on `main`. The project directory remains on `main`; each phase is worked in its own worktree.

## Work performed

1. Read-only preflight of `quant-rule`:
   - `git status --short --branch` — clean on `main`.
   - `git worktree list` — one worktree at `main`.
   - `git log -1 --oneline` — `8228e86`.
   - `git diff --stat 231ac03..8228e86` — the later commits touch `AGENTS.md`, docs, and tests; no file under the planned model/shell source paths changed.
2. Opened the parent issue #122 and nine sub-issues #123–#131 under it.
3. Added the issues to the `QuantRule` project. Set the parent to `In Progress` with start date `2026-10-25`; set #123 to `In Progress` with start date `2026-10-25`; left the remaining leaves in `Todo`.
4. Created the first worktree from `main`:
   - branch `docs/123-dashboard-view-contract`;
   - path `quant-rule--dashboard-view-contract`;
   - no commits.
5. Moved the session to the worktree and wrote the approved execution plan and this log.
6. Wrote `docs/model/2026-10-25/dashboard-state.md` and indexed it in `docs/README.md`. The contract covers the view types, all four axis roles, Direction tenure and the interrupted-continuity rule, Progression recent pace and terminal freeze, Structure activity, readings, agreement, churn, absence, causality, edge cases, and deviations.
7. Did not write any Rust code. The human approved the contract on 2026-10-25; the contract status was changed from `Proposed` to `Approved`, and the plan's Phase 1 review row was checked.
8. Committed the documentation as `db3dc3e`, pushed `docs/123-dashboard-view-contract`, and opened [PR #132](https://github.com/quant-rule/quant-rule/pull/132). The `verify` and `sync` checks pass; PR #132 is merged.
9. Created `feat/124-dashboard-view-types` from the updated `main` (`fb4918c`) in `quant-rule--dashboard-view-types`, with #124 set `In Progress` and its start date set to `2026-10-26`.
10. Added `crates/quantrule-model/src/dashboard/mod.rs` and `tests.rs`, and registered the module in `lib.rs`:
    - `DashboardState`, `TimeframeState`, `DiagnosticsView`, `TimeframeDerived`, `TimeframeConsensusView`, `AxisState`, `AxisValue`, `Domain`, `Stage`, `AxisReadiness`, `DashboardConfig`.
    - `DirectionDetail`, `MomentumDetail`, `StructureDetail`, `ProgressionDetail`, `LevelReading`, `DatedReading`, `EventReading`, `TransitionReading`.
    - `CrossTfState`, `LegDirectionReading`, `CrossTfAgreement`, `AxisAgreement`, `Coverage`, `CrossTfChurn`, `ChurnSource`, `ChurnClass`, `MajorityClass`.
    - Constants for the fixed 3-bar recent-pace window and the default churn/coverage thresholds.
    - The module is types only: no I/O, candle computation, axis formula, assembler, or rendering.
11. Committed the types and tests as `fe97ddb`, pushed `feat/124-dashboard-view-types`, and opened [PR #133](https://github.com/quant-rule/quant-rule/pull/133). The `verify` and `sync` checks pass. PR #133 was merged as `001b0d7`.
12. Removed the Phase 2 worktree, deleted the merged local and remote branch, updated `main`, and created `feat/125-dashboard-assembler` in `quant-rule--dashboard-assembler` from `001b0d7`; moved the session to that worktree. Set #125 to `In Progress` with start date `2026-10-26`.
13. Read-only investigation for Phase 3: inspected `ModelRun`, `TimeframeModel`, `ModelBar`, `MarketState`, the four axis bars, `ContextBar`, `CrossTimeframeAggregates`, `ReferenceSet`, `EventRecord`, and `ProgressionLeg`.
14. Added `crates/quantrule-model/src/dashboard/assemble.rs`:
    - `DashboardState::from_run(&ModelRun, &[u64], &DashboardConfig)`.
    - `TimeframeState::at(&TimeframeModel, bar)` for a causal per-bar view; the loaded window is `0..=bar`.
    - The four axis positions, the initial sizes, the exact readiness mapping, the diagnostics, the timeframe consensus, and typed absence.
    - An absent row for every ladder member without a closed bar, including the run's structure scale when one is available.
    - Ladder normalization by ascending sort and deduplication.
    - The cross-timeframe envelope: configured/present timeframes, existing aggregates, published leg directions, configured thresholds, and coverage. Agreement and churn remain explicit envelopes until their work item.
15. Added `crates/quantrule-model/src/dashboard/assemble/tests.rs` with 16 tests: envelope mapping, all four axis positions/sizes/readiness, stage anchors, missing rows, empty run, single candle, readiness variants, future-bar causality (appended and altered), reordered inputs/ladders, zero preservation, momentum force without acceleration, progression origin reason, level-count availability, configured thresholds, and the absent-row structure scale.
16. Added the `mod assemble;` boundary and updated the dashboard module documentation. No existing axis formula, command, or store schema was changed.
17. Committed the assembler and tests as `15da01d`, pushed `feat/125-dashboard-assembler`, and opened [PR #134](https://github.com/quant-rule/quant-rule/pull/134). `verify` passed in 1m5s; `sync` passed in 13s.

## Decisions and corrections

The human approved the architecture and decisions D1–D12, with two clarifications recorded as D13 and D14:

- D13 — Direction after an unavailable interval preserves the stored tenure but marks continuity as interrupted. If the bias returns with the opposite sign, a new origin is established at the first observed bar. The actual reversal time during the unavailable interval is unknown and is not backdated.
- D14 — The historical pace field is named `average_net_displacement_atr_per_bar`; it measures net displacement per elapsed bar, not total path distance.

Suggestions rejected or corrected during planning:

- **Average leg speed as the sole current-momentum measure** — rejected. It describes historical pace, not whether the leg is accelerating, stalling, or moving against its intended direction.
- **Signed recent-pace percentile as the radius** — rejected. It would make a large dot mean “with the leg” rather than “fast.” The radius uses the absolute recent-pace percentile; the signed pace and direction remain details.
- **Coiling / ignition / exhausted labels** — rejected for this release. They require separate volatility/range or failure-to-continue evidence and historical validation.
- **`ModelState` as the projected type name** — rejected to avoid ambiguity with the existing `MarketState`. The type is `DashboardState`.
- **Cross-timeframe `consensus` / `stability` names** — rejected. The view uses `cross_tf_agreement` and `churn` to avoid confusion with the existing per-timeframe `TimeframeConsensus` and to avoid claiming universal market stability.
- **One available timeframe forming a majority** — rejected. The agreement requires at least two valid timeframe readings and publishes coverage.
- **Missing timeframe treated as neutral** — rejected. Missing is absent and lowers coverage; neutral is a measured class.
- **Level interaction events as persistent Level readings** — corrected. The dated history holds Event/Transition readings; Level references are current EMA/confirmed-swing evidence in Structure detail.
- **Five progression anchors** — rejected. QuantRule publishes six ordinal codes, including `against`; the visual retains six.
- **Changing existing command contracts** — rejected. The dashboard is a new command; `market-state`, `axis`, `events`, `legs`, `model-series`, and `market-chart` remain unchanged.

Phase 3 implementation choices, made inside the approved contract:

- **Ladder normalized at the assembly boundary.** `DashboardState::from_run` sorts the ladder ascending and removes duplicates before building rows. Two callers that pass the same set in a different order, or repeat a member, get the same view, the same row order, and the same coverage denominator. Normalizing at the boundary makes determinism a property of the view rather than a convention the caller must remember.
- **`window_len` is `bar + 1`, not the loaded series length.** `TimeframeState::at` reports the window actually observed up to that bar, so the state at bar `t` is identical whether the run contained later bars or ended at `t`. This is what lets the causality test compare a full run against a prefix run at the same bar.
- **A past bar view is the test surface.** `TimeframeState::at` is the per-bar entry point; `from_run` uses it for the latest bar. There is no second code path that could drift.
- **Typed absence keeps the run's meaning.** A ladder member without a closed bar gets an explicit absent row: `present = false`, nulls rather than zeros, `Readiness::Unavailable`, and the four axis detail variants. Its Structure detail carries the scale the run actually used; the model default major scale is used only when the entire run is empty, where no measurement exists to describe.
- **Progression absence names the stage's own blocker.** When the model reports progress but withholds the stage, the view reuses the leg's state: origin unconfirmed, no target, or target unstable. It does not collapse every absent stage into insufficient bars.
- **Agreement and churn stay explicit envelopes.** The core assembler publishes real coverage, the configured thresholds, the existing aggregate, and the published leg directions, but no per-axis majority and no churn verdict. A missing timeframe is not neutral and is not counted. The envelopes exist so consumers have stable types, but nothing in Phase 3 claims agreement or churn that has not been derived.

## Verification and evidence

Observed Phase 0 and Phase 1 evidence:

- `gh issue view 122 --json subIssues` — nine sub-issues #123–#131 linked.
- `gh project item-list 1 --owner quant-rule` — parent #122 `In Progress`; #123 `Todo` before the status edit; the other leaves `Todo`.
- `gh project item-edit` — #123 set to `In Progress` and start date `2026-10-25`; parent start date `2026-10-25`.
- `git worktree add` — worktree created at `quant-rule--dashboard-view-contract` on branch `docs/123-dashboard-view-contract` from `main`/`8228e86`.
- The parent body was edited to list #123–#131.
- Some new project items showed a null `State` or `Item type` immediately after creation. Those fields are workflow-owned; the agent did not set them. The `Project sync` runs for #124, #128, and #130 had been cancelled. The cancelled runs were rerun through GitHub Actions (run ids 40235967266, 40236073343, 40236095637) and completed successfully. The board then showed all ten items with `State` and `Item type`; #122 and #123 are `In Progress`, #124–#131 are `Todo`.
- `docs/model/2026-10-25/dashboard-state.md` exists; its header status is `Approved — reviewed 2026-10-25`.
- `docs/README.md` carries the new contract in `Current model contracts` as `Approved (#123)`.
- `docs/execution-plans/2026-10-25/122-market-dashboard.md` verification rows 2 and 3 are checked: the contract exists, is linked, and was approved by the human.
- Commit `db3dc3e`; branch `docs/123-dashboard-view-contract`; PR #132 merged as `fb4918c`.
- `gh pr checks 132 --repo quant-rule/quant-rule --watch` — `verify` passed in 53s; `sync` passed in 11s.
- The contract's section headings were inspected for the required coverage: purpose, boundary, conventions, envelope, per-timeframe state, axis state, readings, cross-timeframe state, absence/readiness, text, JSON, invariants, edge cases, deviations, verification, and open items.
- Phase 2 local checks, all observed on `feat/124-dashboard-view-types`:
  - `cargo fmt --all -- --check` — passed.
  - `cargo clippy --all-targets -- -D warnings` — passed.
  - `cargo test` — 322 model unit tests passed, including 12 `dashboard` type tests; 31 model doctests passed, including 3 `dashboard` doctests; 182 shell unit tests and all integration tests passed; 0 failures; 2 ignored benchmarks.
  - `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps` — passed with no warnings.
  - `git diff --check` — passed.
- Plan verification rows 4–7 are checked for Phase 2.
- Commit `fe97ddb`; branch `feat/124-dashboard-view-types`; PR #133 open.
- `gh pr checks 133 --repo quant-rule/quant-rule --watch` — `verify` passed in 48s; `sync` passed in 10s.

Observed Phase 3 evidence, run on `feat/125-dashboard-assembler`:

- `cargo fmt --all -- --check` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed with no warnings.
- `cargo test` — 338 model unit tests passed, including the 16 `dashboard::assemble` tests; 33 model doctests passed; 182 shell unit tests and all integration tests passed; 0 failures; 3 ignored benchmarks. The suite includes the pre-existing goldens for Direction, Momentum, Structure, Progression, readings, context, and events, so the additive view did not move an existing axis value.
- `cargo test -p quantrule-model dashboard::assemble --lib` — 16 passed: envelope mapping, all four axis mapping, stage anchors, missing rows, empty run, single candle, readiness variants, neutral-corridor bounds, future-bar causality (appended and altered), reordered inputs and ladders, zero preservation, momentum force without acceleration, progression origin reason, level-count availability, configured thresholds, and absent-row structure scale.
- `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps` — passed with no warnings; the two new doc examples compile and run in the doctest suite.
- `git diff --check` — passed.
- Plan verification rows 8–12 are checked for Phase 3.
- Commit `15da01d`; branch `feat/125-dashboard-assembler`; PR #134 open.
- `gh pr checks 134 --repo quant-rule/quant-rule --watch` — `verify` passed in 1m5s; `sync` passed in 13s.

Not verified / not run:

- No Direction bias tenure or continuity, Progression recent pace, Structure activity, or per-axis typed readings; those are #126, #127, and #128.
- No derived cross-timeframe agreement majority or churn verdict; those are #129. Phase 3 publishes only the envelope, coverage, and configured thresholds.
- No shell `market-dashboard` command, text rendering, JSON output, or dashboard CLI run; those are #130 and #131.
- No Phase 3 merge yet.
- No independent agent review. The investigation, implementation, and verification were sequential in one session; that is disclosed here.

## Learning

- The trader-useful distinction is between what price has done, how it is behaving now, and how trustworthy the measurement is. Position, current-movement radius, and evidence must not collapse into one channel.
- A cumulative average is not current momentum. A fixed three-bar signed change with an absolute-speed percentile separates direction from speed and avoids conflating them.
- Neutral is a state, not a missing value and not a reset. A neutral pause must preserve tenure and be displayed explicitly, or a large dot can imply an active bias that is not there.
- An unavailable interval means the true reversal time is unknown. Marking continuity as interrupted is more honest than backdating a reversal.
- Structure alignment and event activity answer different questions. Event density must not inflate the Structure radius.
- Cross-timeframe majority needs coverage and a minimum valid-reading rule; otherwise one timeframe can masquerade as agreement.
- A maximum state-change count across different durations is churn evidence, not a universal market-stability verdict.
- A types-only slice is useful when it fixes names and numeric code domains before the assembler. Its tests check variant and code mappings, not values that the assembler has not produced yet.
- Normalizing the timeframe ladder at the assembly boundary turns an accidental dependency on caller order into a tested invariant. If the view is meant to be deterministic, the view should own the normalization rather than trusting every caller to sort.
- A view of bar `t` must report the window up to `t`, not the run's total length. If it reports the latter, a full run and a prefix run produce different views of the same bar, and the causality test cannot prove what it claims.
- An envelope that carries real coverage and configured thresholds but no verdict is honest at a phase boundary. It keeps the type stable for downstream work without publishing a majority the data has not earned; the doc comment states that no consumer should read it as a verdict.
- Typed absence is more than nulls. An absent row still has to say which reason code applies, which readiness it has, and which scale the run used, or the renderer will invent an answer.
