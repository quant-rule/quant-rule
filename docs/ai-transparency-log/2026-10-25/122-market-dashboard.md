# AI Transparency Log — Market-Dashboard

- Date: 2026-10-25 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Work item: [#122](https://github.com/quant-rule/quant-rule/issues/122) — container
- Plan: [`docs/execution-plans/2026-10-25/122-market-dashboard.md`](../../execution-plans/2026-10-25/122-market-dashboard.md)
- Branches/worktrees: `docs/123-dashboard-view-contract` in `quant-rule--dashboard-view-contract` (merged, PR #132); `feat/124-dashboard-view-types` in `quant-rule--dashboard-view-types` (Phase 2)
- Status: Phase 0 complete; Phase 1 contract approved 2026-10-25 and merged in PR #132. Phase 2 value types are implemented in PR #133; local checks and CI pass; no Phase 2 merge.

## Intent and boundary

The planning session defined an additive, single-instrument CLI dashboard over QuantRule's existing four-axis model. The scope is a pure `DashboardState` projection, a static `market-dashboard` text/JSON command, and view-only Direction bias tenure, Progression recent pace, Structure activity, typed readings, cross-timeframe agreement, and churn.

Boundary of this entry:

- Phase 0 setup: parent and sub-issues, project board statuses, first worktree, approved plan, and this log.
- Phase 1: the `DashboardState` contract document and its `docs/README.md` index entry. The human approved the contract on 2026-10-25; it is marked `Approved`; PR #132 is merged.
- Phase 2: the `quantrule-model::dashboard` value types and their unit tests. No assembler, shell command, or rendered output.
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
11. Committed the types and tests as `fe97ddb`, pushed `feat/124-dashboard-view-types`, and opened [PR #133](https://github.com/quant-rule/quant-rule/pull/133). The `verify` and `sync` checks pass.

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

Not verified / not run:

- No assembler, shell command, text rendering, JSON output, or dashboard CLI run.
- No Phase 2 merge yet.
- No bias tenure, recent pace, readings, agreement, churn, or shell golden tests; those are later sub-issues.
- No independent agent review. The planning and implementation roles were sequential in one session; that is disclosed here.

## Learning

- The trader-useful distinction is between what price has done, how it is behaving now, and how trustworthy the measurement is. Position, current-movement radius, and evidence must not collapse into one channel.
- A cumulative average is not current momentum. A fixed three-bar signed change with an absolute-speed percentile separates direction from speed and avoids conflating them.
- Neutral is a state, not a missing value and not a reset. A neutral pause must preserve tenure and be displayed explicitly, or a large dot can imply an active bias that is not there.
- An unavailable interval means the true reversal time is unknown. Marking continuity as interrupted is more honest than backdating a reversal.
- Structure alignment and event activity answer different questions. Event density must not inflate the Structure radius.
- Cross-timeframe majority needs coverage and a minimum valid-reading rule; otherwise one timeframe can masquerade as agreement.
- A maximum state-change count across different durations is churn evidence, not a universal market-stability verdict.
- A types-only slice is useful when it fixes names and numeric code domains before the assembler. Its tests check variant and code mappings, not values that the assembler has not produced yet.
