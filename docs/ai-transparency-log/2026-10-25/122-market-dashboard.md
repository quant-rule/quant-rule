# AI Transparency Log — Market-Dashboard

- Date: 2026-10-25 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Work item: [#122](https://github.com/quant-rule/quant-rule/issues/122) — container
- Plan: [`docs/execution-plans/2026-10-25/122-market-dashboard.md`](../../execution-plans/2026-10-25/122-market-dashboard.md)
- Branches/worktrees: `docs/123-dashboard-view-contract` in `quant-rule--dashboard-view-contract` (merged, PR #132); `feat/124-dashboard-view-types` in `quant-rule--dashboard-view-types` (merged, PR #133); `feat/125-dashboard-assembler` in `quant-rule--dashboard-assembler` (merged, PR #134); `feat/126-bias-tenure-activity` in `quant-rule--bias-tenure-activity` (merged, PR #135); `feat/127-progression-pace` in `quant-rule--progression-pace` (merged, PR #136); `feat/128-dashboard-readings` in `quant-rule--dashboard-readings` (PR #137)
- Status: Phase 0 complete; Phase 1 contract approved 2026-10-25 and merged in PR #132; Phase 2 value types merged in PR #133; Phase 3 core assembler merged in PR #134 as `f118118`; Phase 4 Direction tenure, Structure activity, and reference prices merged in [PR #135](https://github.com/quant-rule/quant-rule/pull/135) as `7aa9502`; Phase 5 Progression recent pace, average net displacement, and terminal freeze merged as `5da8ec8` in [PR #136](https://github.com/quant-rule/quant-rule/pull/136) after `verify` (1m5s) and `sync` (12s) passed. Phase 6 typed readings are implemented in [PR #137](https://github.com/quant-rule/quant-rule/pull/137), with `verify` (1m7s) and `sync` (14s) passing; merge authorization is pending.

## Intent and boundary

The planning session defined an additive, single-instrument CLI dashboard over QuantRule's existing four-axis model. The scope is a pure `DashboardState` projection, a static `market-dashboard` text/JSON command, and view-only Direction bias tenure, Progression recent pace, Structure activity, typed readings, cross-timeframe agreement, and churn.

Boundary of this entry:

- Phase 0 setup: parent and sub-issues, project board statuses, first worktree, approved plan, and this log.
- Phase 1: the `DashboardState` contract document and its `docs/README.md` index entry. The human approved the contract on 2026-10-25; it is marked `Approved`; PR #132 is merged.
- Phase 2: the `quantrule-model::dashboard` value types and their unit tests. No assembler, shell command, or rendered output.
- Phase 3: the core `DashboardState` assembler and its unit tests. No bias tenure, recent pace, Structure activity, typed readings, derived agreement/churn, shell command, or rendered output.
- Phase 4: the Direction bias-tenure state machine and percentile, the Structure event-to-axis mapping, activity window, and reference prices, plus their unit, integration, and golden tests. No recent pace, typed readings, derived agreement/churn, shell command, or rendered output.
- Phase 5: the Progression recent-pace state machine, its absolute percentile, the average net displacement detail, and the terminal freeze, plus their unit, edge-case, causality, determinism, and golden tests. No typed readings, derived agreement/churn, shell command, or rendered output.
- Phase 6: the per-axis dated Event/Transition readings, the compact-state conversion, synthesized Progression stage transitions, availability filtering, ordering, and the 50-item cap, plus their unit, assembler, causality, determinism, and golden tests. No derived agreement/churn, shell command, or rendered output.
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
17. Committed the assembler and tests as `15da01d`, pushed `feat/125-dashboard-assembler`, and opened [PR #134](https://github.com/quant-rule/quant-rule/pull/134). `verify` passed in 1m5s; `sync` passed in 13s. The human authorized the merge; PR #134 merged as `f118118`. Removed the Phase 3 worktree and local/remote branch.
18. Created `feat/126-bias-tenure-activity` from `f118118` in `quant-rule--bias-tenure-activity`, set #126 to `In Progress` with start date `2026-10-26`, and moved the session to that worktree.
19. Read-only investigation for Phase 4: inspected contract §§6.1, 6.3, 7.1–7.3, 13, and 15; the execution-plan rows 13–20; `DirectionSeries` and its bar record; `EventRecord`, event types, and Signal source/subtype codes; `ConfirmedLevel`; `TimeframeModel`; and the existing assembler.
20. Added `crates/quantrule-model/src/dashboard/assemble/direction.rs`:
    - A pure replay of the published Direction bars: first establishment, genuine reversal, neutral pause that increments tenure, unavailable freeze, same-sign and neutral resume after a gap, and opposite-sign new origin at the first observed bar.
    - A percentile of the current tenure against prior observed tenure samples, using the existing kernel's 252 cap, 50-sample minimum, inclusive ties, and stable sort.
    - Measured zeros and nulls are preserved; the current bar is never its own sample.
21. Added `crates/quantrule-model/src/dashboard/assemble/structure.rs`:
    - The contracted event-to-axis mapping: Level and Divergence to Structure, threshold crossing to Momentum, EMA crossover to Structure, compact-state change by source code, unknown codes unassigned.
    - Activity over Structure-mapped Event records whose `available_bar_index` is in the last `min(5, bar + 1)` bars, with `activity_per_bar = count / window`; zero is a measured zero.
    - Reference prices in fixed order: EMA20/50/200 at the view bar, then the latest confirmed minor high/low and major high/low knowable by the view bar, each with its own `available_at`; missing references are omitted.
22. Wired both modules into `assemble.rs`: Direction size is the tenure percentile and `magnitude_pct` remains the existing percentile detail; Structure detail carries the references and activity. Added `TimeframeModel::direction()` so the view reads the published series instead of recomputing the axis.
23. Added 12 Direction unit tests and 6 Structure unit tests, 5 new assembler integration tests, and 2 integration golden tests. Added `dashboard_direction_tenure_golden.csv`, generated by an independent Python replay of the tenure state machine over the independently generated `direction_golden.csv`; the Structure golden cross-checks against the existing independently generated `structure_golden.csv`, `structure_levels_golden.csv`, and readings EMA columns. No existing axis fixture changed.
24. Committed the Phase 4 work as `cf799a7`, pushed `feat/126-bias-tenure-activity`, and opened [PR #135](https://github.com/quant-rule/quant-rule/pull/135). `verify` passed in 1m16s; `sync` passed in 11s. PR #135 merged as `7aa9502`; the Phase 4 worktree and local/remote branch were removed.
25. Created `feat/127-progression-pace` from `7aa9502` in `quant-rule--progression-pace`, set #127 to `In Progress` with start date `2026-10-26`, and moved the session to that worktree.
26. Read-only investigation for Phase 5: inspected #127, contract §6.4, plan §6.2, `ProgressionSeries`/`ProgressionBar`/`ProgressionLeg`, the leg lifecycle and freeze, the kernel percentile, and the existing assembler. `progression_golden.csv` has 258 published legs but only one confirmed published bar (bar 57), zero valid same-leg pace bars, and zero terminal published bars, so it cannot exercise the pace.
27. Added `TimeframeModel::progression()` so the view reads the published series instead of recomputing the axis.
28. Added `crates/quantrule-model/src/dashboard/assemble/progression.rs`:
    - A causal replay of the published bars: same-leg two-endpoint pace, leg-change reset, forming/missing typed absence, terminal freeze, and the average net displacement per elapsed bar.
    - A percentile of the absolute pace against prior measured paces, using the kernel's 252 cap, 50-sample minimum, inclusive ties, and no current-bar sample.
    - The six ordinal stage anchors, the stage absence reason, the pace radius reason, and the Progression detail mapping.
29. Wired `assemble.rs` to `progression_axis(timeframe, &model_bar)` and removed the placeholder pace values. The model's displacement percentile, progress, stage, lifecycle, scale, target state, and readiness are copied unchanged.
30. Added 16 Progression unit, edge-case, causality, determinism, and golden tests; updated the existing assembler test for the pace radius; added a confirmed-real-fixture test at bar 57. The assembler test now checks the forming latest leg's average, the absent pace, the `OriginUnconfirmed` radius reason, and the separate displacement percentile.
31. Generated `crates/quantrule-model/tests/fixtures/dashboard_progression_pace_golden.csv` with an independent Python replay of the contract. The fixture has 105 rows and 19 columns: 4 no-leg bars, 7 forming bars, 82 active bars, and 12 terminal bars (4 completed, 4 invalidated, 4 superseded); 83 rows have a signed pace, 33 have a percentile, 3 are flat, 3 are against the leg, 77 are with the leg, and the first percentile is at bar 60.
32. Ran the Phase 5 local verification: formatting, clippy, the targeted assembler tests, the full workspace test suite, rustdoc with warnings denied, and `git diff --check`.
33. Committed the Phase 5 work as `728e146`, pushed `feat/127-progression-pace`, and opened [PR #136](https://github.com/quant-rule/quant-rule/pull/136). `verify` passed in 1m5s; `sync` passed in 12s. The human authorized the commit, push, and pull request; merge authorization was not requested in that instruction and was not performed at that time. The human later authorized the merge; PR #136 merged as `5da8ec8`, and the Phase 5 worktree and local/remote branch were removed.
34. Created `feat/128-dashboard-readings` from `5da8ec8` in `quant-rule--dashboard-readings`, set #128 to `In Progress` with start date `2026-10-26`, and moved the session to that worktree.
35. Read-only investigation for Phase 6: inspected contract §7, the event type/subtype/source codes, the compact-state Signal values, the per-bar Progression stage series, `TimeframeState::at`, the existing assembler tests, and the independent Level/Divergence/Signal/Progression fixtures.
36. Added `crates/quantrule-model/src/dashboard/assemble/readings.rs`:
    - A total display mapping: Level and Divergence to Structure, threshold crossing to Momentum, EMA crossover to Structure, compact-state change by source code, and Progression stage changes to Progression.
    - Compact-state Signal events become `DatedReading::Transition` with `from_code` taken from `reference_value` and `to_code` from `primary_value`, so one change is one reading.
    - Progression stage transitions are synthesized from the published stage series, including a null endpoint when a stage appears or disappears; the first bar is never a transition.
    - Events are filtered by `available_bar_index <= view bar`, then ordered by availability, occurrence, Event-before-Transition, and the fixed event key with deterministic final tie-breakers.
    - The last 50 readings per axis are retained after ordering.
37. Wired `axis_readings` into `TimeframeState::at`, updated the module and method documentation, and clarified contract §7.3 with the Event-before-Transition rule and the final tie-breakers.
38. Added 15 readings unit tests, an assembler test that places the readings on their axes at the latest real-fixture bar and keeps the Level references in Structure detail, and a two-test integration golden with `dashboard_readings_golden.csv` generated by an independent Python replay of contract §7. The fixture has 531 data rows across nine view bars and uses the major-scale divergence events that the default run actually computes; the coverage test pins the divergence availability boundary at views 34 and 37.
39. Committed the Phase 6 work as `8406f42`, pushed `feat/128-dashboard-readings`, and opened [PR #137](https://github.com/quant-rule/quant-rule/pull/137). `verify` passed in 1m7s; `sync` passed in 14s.

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

Phase 4 implementation choices, made inside the approved contract:

- **A tenure sample is one prior observed bar's published tenure.** The percentile compares the current running tenure with the tenures that prior bars actually published. A bar with no tenure contributes no sample, and the current bar is appended only after its percentile is computed. This measures how unusual the current age is against the observed age distribution; it does not sample only completed bias runs. The alternative would compare against completed runs only and would answer a different question (typical completed duration), which the approved contract does not define.
- **A neutral first bar after a gap resumes the stored tenure.** The contract's neutral pause preserves the origin and tenure; a gap freezes the counter. Reading the neutral return as a pause keeps the origin and marks the tenure `interrupted`. A later opposite sign on an observed bar is then a genuine `Reversal`, because the neutral bar was observed.
- **An opposite sign hidden by a gap starts a fresh continuous tenure.** The new origin is the first observed bar, the origin kind is `NewOriginAfterUnavailable`, and continuity is `Continuous` for the new origin. The gap remains visible in the origin kind; the reversal is never backdated into the gap.
- **A missing tenure percentile keeps the Direction reason when Direction is unavailable.** If the axis is present but has fewer than 50 prior tenure samples, the size is `Undefined { InsufficientBars }`; if the axis itself is unavailable, the size carries the Direction's own reason. The raw tenure detail remains visible whenever it exists.
- **Structure activity counts Event records, by availability.** It does not synthesize transitions and it does not read an axis counter. An event with `available_bar_index` in the last `min(5, window_len)` bars counts once; a zero count publishes `Some(0.0)` per bar and never becomes `None`.
- **Reference prices separate current EMAs from confirmed swings.** An EMA reference is knowable at the view bar, so `available_bar` is the view bar. A swing reference is knowable only at its `available_at`, so `available_bar` is that confirmation bar, even when a later view reads it. Missing prices are omitted, never replaced by zero.
- **The golden fixture is independent of the Rust implementation.** The tenure fixture was generated by a Python replay from the independent Direction fixture. The Structure golden test reads only the public dashboard view and compares it to the existing independently generated Structure, level, and readings fixtures.

Phase 5 implementation choices, made inside the approved contract:

- **The same confirmed leg is required at both endpoints.** The pace compares displacement at `t` and `t - 3`, so the leg identifier must match and both records must be confirmed and knowable at their own bars. Checking only the current endpoint would let a forming prior bar contribute a value the model never published as a confirmed leg.
- **One sample per fresh pace, after its percentile opportunity.** A newly measured pace is appended to the percentile buffer only after that bar's own view would have been computed, and a frozen value carried through later bars is not re-sampled. A long terminal tail therefore cannot make one measurement dominate the history. The current bar is never its own sample.
- **The average net displacement is published for a forming leg.** The contract's formula only needs displacement and elapsed bars, so it is defined before confirmation; recent pace remains confirmation-gated because the three-bar window needs two confirmed endpoints. A valid zero average is a measured zero, not absence.
- **A missing radius names its blocker.** An absent or forming published leg is `OriginUnconfirmed`; a confirmed leg whose pace percentile has fewer than 50 prior samples is `InsufficientBars`. Readiness remains the model's readiness; the view does not invent a second readiness.
- **A terminal leg freezes both pace and average, and a leg change discards the freeze.** The freeze is set on the bar the published record becomes completed, invalidated, or superseded, and it stays while that terminal leg is published. A new published leg resets the tracker; the later values are not inherited from the prior leg.
- **The golden fixture is synthetic, and the real fixture supplies wiring evidence.** `progression_golden.csv` cannot produce a single valid same-leg pace, so the golden test drives the pure projection with 105 synthetic `ProgressionBar` rows. The real fixture still exercises `progression_axis` at the forming latest bar and at the one confirmed published bar, so the golden is not the only integration evidence.

Phase 6 implementation choices, made inside the approved contract:

- **One published event has one display axis.** The mapping is a match on the event type and, for Signals, the subtype and source code. An unmapped code is treated as a model defect and left out rather than assigned to a guessed axis. The unit test proves the valid codes are partitioned without duplication.
- **A compact-state Signal is one Transition, not an Event plus a Transition.** The event already carries the prior and current compact codes as `reference_value` and `primary_value`. Converting it directly avoids publishing the same change twice on the same axis, and a non-integral or out-of-range measured value becomes a typed absent code rather than a truncated one.
- **A Progression stage change is synthesized from the published series.** The stage series has no event record, so the view compares consecutive published stage codes. A null endpoint is kept because a stage appearing or disappearing is a published change; the first bar is the baseline and never a transition.
- **Ordering is total and independent of the event batch order.** Availability, occurrence, and Event-before-Transition come first; events then use the contract key with `direction_code` and `reference_value` as final tie-breakers. Without the final tie-breakers two otherwise identical events could swap when the batch is reordered. The cap is applied after ordering, so it always trims the oldest reading.
- **The integration fixture uses only the configured divergence scale.** The default model run computes divergence at `PivotScale::Major`. The event fixture contains both minor and major scale events, so the readings fixture takes scale 2 only; including the minor events would add readings the model never publishes.
- **The golden mixes early, cap, and availability views.** The nine view bars are 5, 20, 34, 37, 57, 58, 100, 200, and 259. The latest views exercise the 50 cap; view 37 includes the major divergence that occurred at bar 32, while view 34 excludes it; views 57 and 58 carry the two Progression stage changes.

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
- PR #134 merged as `f118118`; the Phase 3 worktree and local/remote branch were removed.

Observed Phase 4 evidence, run on `feat/126-bias-tenure-activity`:

- `cargo fmt --all -- --check` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed with no warnings.
- `cargo test -p quantrule-model --lib dashboard::assemble` — 39 passed, including 12 Direction tests and 6 Structure tests.
- `cargo test -p quantrule-model --test dashboard_direction_golden --test dashboard_structure_golden` — 2 passed.
- `cargo test` — 361 model unit tests, 2 new dashboard golden integration tests, 33 model doctests, 182 shell unit tests, all other integration tests, 0 failures; 3 ignored benchmarks.
- `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps` — passed with no warnings.
- `git diff --check` — passed.
- Plan verification rows 13–20 are checked for Phase 4.
- The new fixture `crates/quantrule-model/tests/fixtures/dashboard_direction_tenure_golden.csv` has 260 rows: 61 established tenures, 199 null tenures, and 11 rows with a percentile. It was generated by the independent Python replay described above.
- The Structure golden test walks every fixture bar and checks the dashboard detail against the existing relation, scale, EMA distance/cross/retest, score, size, readiness, EMA price, confirmed-swing price, and availability fixtures.
- Commit `cf799a7`; branch `feat/126-bias-tenure-activity`; PR #135 open.
- `gh pr checks 135 --repo quant-rule/quant-rule --watch` — `verify` passed in 1m16s; `sync` passed in 11s.

Observed Phase 5 evidence, run on `feat/127-progression-pace` (2026-10-26):

- `cargo fmt --all -- --check` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed with no warnings.
- `cargo test -p quantrule-model --lib dashboard::assemble` — 57 passed, including 16 Progression unit, edge-case, causality, determinism, and golden tests and the updated assembler test.
- `cargo test` — 379 model unit tests, 33 model doctests, 182 shell unit tests, 71 shell CLI tests, all other integration tests, 0 failures; 4 ignored pre-existing benchmark or slow tests.
- `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps` — passed with no warnings.
- `git diff --check` — passed.
- Plan verification rows 21–29 are checked for Phase 5.
- The new fixture `crates/quantrule-model/tests/fixtures/dashboard_progression_pace_golden.csv` has 105 rows and 19 columns: 4 no-leg bars, 7 forming bars, 82 active bars, and 12 terminal bars (4 completed, 4 invalidated, 4 superseded); 83 rows have a signed pace, 33 have a percentile, 3 are flat, 3 are against the leg, 77 are with the leg, and the first percentile is at bar 60.
- `rg -n "coiling|ignition|exhaust" crates/quantrule-model/src/dashboard/assemble/progression.rs crates/quantrule-model/src/dashboard/assemble/progression/tests.rs` — no hits. The only model/shell hits are the pre-existing context-code registry (`context.rs`, `dashboard/mod.rs`) and its shell label mapping, which the approved contract keeps as diagnostics, not Progression labels.
- Commit `728e146`; branch `feat/127-progression-pace`; [PR #136](https://github.com/quant-rule/quant-rule/pull/136) merged as `5da8ec8`.
- `gh pr checks 136 --repo quant-rule/quant-rule --watch` — `verify` passed in 1m5s; `sync` passed in 12s.

Observed Phase 6 evidence, run on `feat/128-dashboard-readings` (2026-10-26):

- `cargo fmt --all -- --check` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed with no warnings.
- `cargo test -p quantrule-model --lib dashboard::assemble` — 73 passed, including the 15 new readings tests and the new assembler axis-placement test.
- `cargo test -p quantrule-model --test dashboard_readings_golden` — 2 passed: the independent golden comparison and the fixture-coverage test.
- `cargo test` — 395 model unit tests, 33 model doctests, 182 shell unit tests, 71 shell CLI tests, all other integration tests, 0 failures; 4 pre-existing ignored benchmark or slow tests.
- `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps` — passed with no warnings.
- `git diff --check` — passed.
- Plan verification rows 30–34 are checked for Phase 6.
- The new fixture `crates/quantrule-model/tests/fixtures/dashboard_readings_golden.csv` has 531 data rows across nine view bars. Direction has no dated readings; Momentum retains 50 at views 200 and 259; Structure retains 50 at views 57, 58, 100, 200, and 259; Progression has one reading at view 57 and two at views 58, 100, 200, and 259. The major divergence at view 37 occurred at bar 32 and is absent from view 34.
- Commit `8406f42`; branch `feat/128-dashboard-readings`; [PR #137](https://github.com/quant-rule/quant-rule/pull/137) open.
- `gh pr checks 137 --repo quant-rule/quant-rule --watch` — `verify` passed in 1m7s; `sync` passed in 14s.

Not verified / not run:

- No Phase 6 merge yet; merge authorization is pending.
- No derived cross-timeframe agreement majority or churn verdict; that is #129. The assembler still publishes only the envelope, coverage, and configured thresholds.
- No shell `market-dashboard` command, text rendering, JSON output, or dashboard CLI run; those are #130 and #131.
- No independent agent review. The investigation, implementation, and verification were sequential in one session; that is disclosed here.

## Learning

- The trader-useful distinction is between what price has done, how it is behaving now, and how trustworthy the measurement is. Position, current-movement radius, and evidence must not collapse into one channel.
- A cumulative average is not current momentum. A fixed three-bar signed change with an absolute-speed percentile separates direction from speed and avoids conflating them.
- Neutral is a state, not a missing value and not a reset. A neutral pause must preserve tenure and be displayed explicitly, or a large dot can imply an active bias that is not there.
- An unavailable interval means the true reversal time is unknown. Marking continuity as interrupted is more honest than backdating a reversal.
- Structure alignment and event activity answer different questions. Event density must not inflate the Structure radius.
- Cross-timeframe majority needs coverage and a minimum valid-reading rule; otherwise one timeframe can masquerade as agreement.
- A maximum state-change count across different durations is churn evidence, not a universal market-stability verdict.
- An event's occurrence bar and availability bar answer different questions. Filtering a dated history by availability is what keeps a later confirmation from reaching an earlier view, even when the underlying occurrence is older.
- A state change is one reading, not an event plus a transition. Publishing both would double-count the same change in the axis history and make the 50-item window hold less history than it appears to.
- A deterministic history needs a total order. The contract key alone can tie; the final tie-breakers are what make the retained order independent of the event batch order.
- A types-only slice is useful when it fixes names and numeric code domains before the assembler. Its tests check variant and code mappings, not values that the assembler has not produced yet.
- Normalizing the timeframe ladder at the assembly boundary turns an accidental dependency on caller order into a tested invariant. If the view is meant to be deterministic, the view should own the normalization rather than trusting every caller to sort.
- A view of bar `t` must report the window up to `t`, not the run's total length. If it reports the latter, a full run and a prefix run produce different views of the same bar, and the causality test cannot prove what it claims.
- An envelope that carries real coverage and configured thresholds but no verdict is honest at a phase boundary. It keeps the type stable for downstream work without publishing a majority the data has not earned; the doc comment states that no consumer should read it as a verdict.
- Typed absence is more than nulls. An absent row still has to say which reason code applies, which readiness it has, and which scale the run used, or the renderer will invent an answer.
- A running-age percentile and a completed-duration percentile answer different questions. The contract asks for the current tenure's rank against prior observed tenure samples; sampling only completed runs would measure typical completed duration instead, and the two must not be conflated.
- Neutral is a pause, not a reset, and it stays a pause after a gap. If the first observed bar after a gap is neutral, preserving the stored tenure is the only reading consistent with the neutral-pause rule; the later opposite sign is then an observed reversal, not a hidden one.
- Origin kind carries information that continuity alone cannot. A fresh origin after a gap is continuous as a new tenure but must say `new_origin_after_unavailable`, or the view would imply the reversal was observed.
- Activity is evidence, not a vote. Counting Event records by availability in a fixed window, and asserting that the score and size are unchanged, makes the no-double-count and isolation rules testable rather than merely stated.
- Current references and confirmed references have different availability. An EMA is knowable at the view bar; a swing is knowable only when its confirmation lookback closes. Collapsing both to the view bar would leak a future confirmation into an earlier row.
- A golden fixture is most useful when its oracle is independent of the code under test. A Python replay from the independent Direction fixture catches a Rust state-machine regression that a second Rust helper would reproduce instead of expose.
- A same-leg pace window is a two-endpoint fact. The leg must be the same and confirmed at both ends; checking only the current endpoint lets a forming prior bar contribute a number the model never published as confirmed.
- A frozen terminal value is one measurement. Sampling a carried value on every terminal bar would let a long tail dominate the percentile and make a completed leg look faster than it was.
- Recent pace and average net displacement answer different questions. The average is defined before confirmation because the formula needs only displacement and elapsed bars; recent pace is confirmation-gated because it needs two confirmed endpoints.
- A synthetic golden fixture is the right oracle when the real fixture cannot exercise the behavior. The real Progression fixture has one confirmed published bar and no valid same-leg pace, so it can prove wiring and typed absence but not the pace arithmetic or percentile.
- A missing radius should name the blocker that actually applies. Collapsing a forming leg and a confirmed leg with insufficient history into one reason hides the difference between "not yet known" and "not enough samples".
