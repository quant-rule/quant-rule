# Market-Dashboard — Execution Plan

- Date: 2026-10-25 (UTC)
- Issue: [#122](https://github.com/quant-rule/quant-rule/issues/122) — the container; the work is tracked as [#123](https://github.com/quant-rule/quant-rule/issues/123) through [#131](https://github.com/quant-rule/quant-rule/issues/131).
- Status: Approved. Contract approved 2026-10-25 and delivered in [PR #132](https://github.com/quant-rule/quant-rule/pull/132). Phase 2 value types are implemented in [PR #133](https://github.com/quant-rule/quant-rule/pull/133); the assembler and command are not started. Not Verified.
- Contract to be written in #123: `docs/model/2026-10-25/dashboard-state.md`.
- AI transparency log: `docs/ai-transparency-log/2026-10-25/122-market-dashboard.md`.
- No ADR: this is an additive view over the existing model pipeline; it does not change the pipeline stages, dependencies, or storage schema.

## 1. Intent

Give the operator one deterministic terminal view of the four axes for the single instrument in a candle CSV. The view makes the vertical read possible: one row per timeframe, four tracks, with position, size, readiness, and typed readings visible together.

The view must answer trader questions without pretending the model measures something it does not:

| Axis | Trader question | Position | Radius | Evidence |
|---|---|---|---|---|
| Direction | Which way, and how durable? | signed EMA-alignment score | bias-tenure percentile | current bias state, tenure, origin, magnitude percentile, compact-state age/changes |
| Momentum | Accelerating or decelerating, with how much force? | signed acceleration | force percentile | force, intensification, state age/changes |
| Structure | Higher highs/lows, or lower? | signed swing relation | swing-alignment magnitude percentile | high/low relations, scale, EMA distance/cross/retest, reference prices, event activity |
| Progression | How far along, and is recent movement with or against the leg? | six ordinal anchors | absolute recent-pace percentile | signed recent pace, direction, net average pace, displacement percentile, lifecycle, scale, target state |

This is a descriptive view. It is not a strategy, a forecast, an entry/exit, a stop, a target, a confidence value, or a probability.

## 2. Current state

- Repository base: `main` at `8228e86` when the plan was drafted (it was originally drafted against `231ac03`; the later commits are documentation-only changes to `AGENTS.md` and docs, plus tests already merged; no source path used by this plan changed). Phase 2 starts from `fb4918c` after PR #132.
- `ModelRun` already computes one candle series per timeframe and the model is single-instrument. There is no pair map.
- `MarketState` publishes latest per-timeframe numeric snapshots, readiness codes, reasons, size percentiles, events, and cross-timeframe aggregates.
- Direction, Momentum, Structure, and Progression publish the raw values listed in section 1. Phase 2 adds the `DashboardState` value types; there is still no assembler, bias tenure, recent pace, per-axis typed readings, or cross-timeframe agreement/churn view.
- The shell has `market-state`, `market-chart`, `axis`, `events`, `legs`, and `model-series`. There is no dashboard command.
- The store persists the existing numeric model fields. This plan adds no column and no migration.
- The multi-pair surface does not exist and is not introduced.

## 3. Scope

Included:

- A pure `DashboardState` projection in `quantrule-model`.
- View-only measurements: Direction bias tenure, Progression recent pace, Structure activity.
- Per-axis typed readings: Level references, Event occurrences, Transitions.
- Per-instrument cross-timeframe agreement and churn.
- A `market-dashboard <csv-path> [--window full|latest] [--json]` command.
- Deterministic text and numeric JSON output.
- Unit, golden, causality, determinism, edge-case, and CLI tests.
- Contract, README/help, execution plan, and AI log updates.

Excluded:

- Multi-pair support, pair maps, or `cross_pair_state`.
- Changes to existing axis formulas or existing command contracts.
- Store schema/migration changes.
- New dependencies.
- Full-screen TUI, ANSI color, live refresh, or clock reads.
- Coiling, ignition, exhaustion, or other categorical trading interpretations.
- Strategy, entry/exit, stop, target, risk/reward, confidence, probability, verdict, or recommendation.

## 4. Assumptions and uncertainty

- The four-timeframe ladder is the existing M1/M5/M15/H1 set. The view receives the ladder explicitly; it does not assume a source format.
- The existing per-bar model series is causal and available through `TimeframeModel::bar`.
- The view can iterate the model series up to the latest bar to compute tenure and pace; the cost is bounded by the existing input cap.
- The percentile convention is the existing `empirical_percentile`: prior valid samples only, 252 cap, minimum 50, inclusive ties, sorted with `f64::total_cmp`.
- `origin_occurred_at` is the bar the pivot occurred; `origin_available_at` is the bar it became knowable. The view must never publish a confirmed-leg measurement before the availability bar.
- The board and workflow own `State`, `Item type`, and `Closed date`; the agent sets only leaf `Status` and `Start date`.
- The six-change churn threshold and the two-valid-timeframe agreement minimum are initial parameters, not established properties. They are exposed through `DashboardConfig` for calibration.

## 5. Decisions

| ID | Decision | Status |
|---|---|---|
| D1 | Structure keeps swing relations as position and swing-alignment magnitude as radius; event activity and swing evidence are separate details. | Approved |
| D2 | Direction uses bias tenure as radius; neutral pauses preserve tenure but display the current state explicitly; unavailable bars freeze the counter. | Approved |
| D3 | Progression keeps six anchors; radius uses absolute recent-pace percentile; signed pace, net average pace, displacement percentile, and lifecycle are details. | Approved |
| D4 | Cross-timeframe agreement uses Direction, Momentum, and Structure (`of = 3`), strict majority, at least two valid timeframe readings, and explicit coverage. | Approved |
| D5 | Churn publishes the maximum state changes, all source axes/timeframes, history completeness, and a configurable threshold (default 6). | Approved |
| D6 | The projected type is `DashboardState`, not `ModelState`. | Approved |
| D7 | Existing command contracts remain unchanged; the dashboard is a new command. | Approved |
| D8 | The first release is a static text renderer; a TUI can wrap the same view later. | Approved |
| D9 | No new dependency and no store schema change. | Approved |
| D10 | Level references live in Structure detail; the dated 50-item history holds Event and Transition readings. | Approved |
| D11 | No data-version flag in the first slice; the CSV path and model version identify the run. | Approved |
| D12 | One worktree and one PR per sub-issue; one sub-issue in flight at a time. | Approved |
| D13 | Direction after an unavailable interval preserves the stored tenure but marks continuity as interrupted. If the bias returns with the opposite sign, a new origin is established at the first observed bar; the actual reversal time during the gap is unknown. | Approved clarification |
| D14 | The historical pace field is named `average_net_displacement_atr_per_bar`; it is net displacement per elapsed bar, not total path distance. | Approved clarification |

## 6. Metric definitions

### 6.1 Direction bias tenure

- `bias_state`: `bullish` (> +0.1), `bearish` (< -0.1), `neutral` (within ±0.1 inclusive), or `undefined`.
- `bias_tenure_bars`: bars since the last genuine establishment or reversal, including neutral bars; unavailable bars do not increment the counter.
- `tenure_origin_kind`: `first_establishment`, `reversal`, `new_origin_after_unavailable`, or `none`.
- `tenure_origin_bar`: the bar the current origin was set.
- `bias_continuity`: `continuous`, `interrupted`, or `undefined`.
- `bias_tenure_pct`: percentile of `bias_tenure_bars` against prior valid tenure samples, 252 cap, minimum 50, inclusive ties, never the current sample.
- On an unavailable interval, preserve the stored tenure and origin, set continuity to `interrupted`, and publish `—` for the current tenure. On the first available bar:
  - same sign as the stored origin side: resume the stored tenure, increment by one, and keep the original origin;
  - opposite sign: set a new origin on the observed bar, tenure 1, origin kind `new_origin_after_unavailable`; the actual reversal time is unknown and is not published.
- `magnitude_pct` remains the existing size percentile for detail.
- `compact_state_age_bars` and `compact_state_changes_50` are details and are not the same as bias tenure.

### 6.2 Recent progression pace

For the same confirmed published leg at `t` and `t - 3`:

```text
recent_pace_signed_atr_per_bar = leg_direction × (displacement_atr(t) - displacement_atr(t-3)) / 3
recent_pace_direction = with_leg if > 0, against_leg if < 0, flat if = 0
recent_pace_speed_pct = percentile of abs(recent_pace_signed_atr_per_bar)
                         against prior abs values, 252 cap, minimum 50, inclusive ties
```

- Radius uses `recent_pace_speed_pct`.
- `recent_pace_signed_atr_per_bar` and `recent_pace_direction` are details.
- The same `leg_id` must be published at both endpoints; a leg change resets the measurement.
- A forming leg or missing displacement is `Undefined`.
- A terminal leg freezes the pace at the bar its lifecycle becomes terminal; later bars do not decay it.
- `average_net_displacement_atr_per_bar = abs(displacement_atr) / max(1, t - origin_occurred_at)`; frozen at terminal; historical detail only.
- `displacement_pct` and `progress` remain separate details.
- `lifecycle` distinguishes `completed`, `invalidated`, and `superseded`.
- No categorical coiling/ignition/exhaustion labels are published.

### 6.3 Structure activity

- `activity_count`: Structure-mapped Event readings with `available_bar_index` in the last `min(5, window_len)` closed bars.
- `activity_per_bar = activity_count / window`.
- `activity_window = min(5, window_len)`.
- Activity is detail only. It never changes position or size.
- The radius remains the existing swing-alignment magnitude percentile. The evidence line shows high/low relations, scale, EMA distance/cross/retest, reference prices, and activity.

### 6.4 Cross-timeframe agreement

- Direction, Momentum, Structure; Progression excluded; `of = 3`.
- For each axis, classify valid timeframes as bullish (> +0.1), bearish (< -0.1), or neutral (inside ±0.1 inclusive).
- A strict majority requires one class to have more than half of the valid timeframes.
- At least `min_cross_tf_members` valid timeframe readings (default 2) are required; below that, the axis has `insufficient_coverage` and no majority.
- Missing timeframes are reported as absent and lower coverage; they are never counted as neutral.
- Neutral is a measured class, not a missing value.
- Ties are not a strict majority.
- `progression_leg_direction` is published separately.

### 6.5 Cross-timeframe churn

- `max_changes_50`: maximum `state_changes_50` across all axes and present timeframes.
- `sources`: every `(timeframe_secs, axis)` tied at the maximum, sorted fastest-to-slowest then axis order.
- `threshold`: default 6, supplied through `DashboardConfig`.
- `window`: 50.
- `history_complete`: every required timeframe has at least 50 code-bearing bars for the compared axes.
- `churn_class`: `Building` when history is incomplete, `Unstable` at or above the threshold, otherwise `Holding`.
- Coverage is published; the text labels this `churn`, not universal market stability.

### 6.6 Readiness and absence

- Readiness maps the model's existing code: `Ready` → solid; `Provisional`/`Unavailable` → dashed.
- The exact readiness code and unavailable reason remain in detail.
- A missing view-only size is `—` in the size channel; it does not silently change the model readiness.
- A missing or absent timeframe has `present = false` and typed `Undefined` positions/sizes.
- No absence becomes zero, NaN, a default stage, or an origin dot.

## 7. View contract

```text
DashboardState
  ladder: Vec<u64>
  timeframes: Vec<TimeframeState>
  cross_tf_state: CrossTfState

TimeframeState
  timeframe_secs
  present
  latest_bar
  latest_open_time
  window_start
  window_len
  axes[4]
  diagnostics
  derived

AxisState
  axis
  position: AxisValue
  size: AxisValue
  readiness: solid | dashed
  readings[]
  detail: DirectionDetail | MomentumDetail | StructureDetail | ProgressionDetail

AxisValue
  signed { value, domain }
  rank { pct }
  ordinal { stage }
  undefined { reason }

Domain: directional_bias | acceleration | structure_relation
Stage: Against | Early | EarlyMid | Mid | Late | Terminal
```

- `readings` holds the last 50 dated Event/Transition readings per axis, oldest first.
- Level references live in `StructureDetail`.
- `CrossTfState` holds configured/present timeframes, existing aggregates, agreement, churn, and progression leg direction.
- `DashboardConfig` holds `churn_threshold` (default 6) and `min_cross_tf_members` (default 2).

`--json` prints one deterministic numeric object:

```json
{
  "model_version": "…",
  "view_contract": "2026-10-25",
  "dashboard_state": {
    "ladder": [60, 300, 900, 3600],
    "timeframes": [],
    "cross_tf_state": {}
  }
}
```

The text renderer uses the same `DashboardState`. It adds labels, track glyphs, and the evidence/readings lines.

## 8. Acceptance criteria

- `market-dashboard` parses and prints the text dashboard and `--json` view.
- One row per configured timeframe; absent timeframes are typed absence, not omitted silently.
- Four identical track geometries; no axis reversed; position, radius, and outline channels are distinct.
- Direction uses bias tenure; neutral preserves tenure and is displayed as neutral; genuine reversal resets; unavailable freezes; continuity interruption is explicit.
- Progression uses six anchors; recent pace is same-leg, 3-bar, signed in detail, absolute percentile for radius; terminal pace freezes; lifecycle is visible.
- Structure radius is swing-alignment magnitude; activity is detail and never changes position/size.
- Readings map every event exactly once; 50 cap; availability filter; Progression stage transition is a Transition.
- Agreement uses strict majority, minimum two valid timeframes, coverage, missing not neutral, ties not majority.
- Churn publishes max changes, sources, threshold, window, and history completeness.
- Repeated runs are byte-identical; interactive and non-interactive output are byte-identical.
- Existing commands and store behavior are unchanged.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo doc --no-deps` pass.
- The verification table is complete and the plan is marked Verified.

## 9. Work breakdown

| Order | Issue | Deliverable | Depends on | Branch / PR |
|---|---|---|---|---|
| Parent | #122 | Container | — | none |
| 1 | #123 | Contract | — | `docs/123-dashboard-view-contract` / [PR #132](https://github.com/quant-rule/quant-rule/pull/132) |
| 2 | #124 | View types | #123 | `feat/124-dashboard-view-types` / [PR #133](https://github.com/quant-rule/quant-rule/pull/133) |
| 3 | #125 | Core assembler | #124 | `feat/125-dashboard-assembler` / PR 3 |
| 4 | #126 | Direction tenure + Structure activity | #125 | `feat/126-bias-tenure-activity` / PR 4 |
| 5 | #127 | Progression recent pace | #125 | `feat/127-progression-pace` / PR 5 |
| 6 | #128 | Readings | #125 | `feat/128-dashboard-readings` / PR 6 |
| 7 | #129 | Agreement + churn | #126–#128 | `feat/129-dashboard-aggregates` / PR 7 |
| 8 | #130 | Shell command | #129 | `feat/130-market-dashboard` / PR 8 |
| 9 | #131 | Verification closeout | #130 | `docs/131-dashboard-closeout` / PR 9 |

One sub-issue is in flight at a time. Each PR includes its tests and AI-log update. No long-lived feature branch.

### Phases and roles

| Phase | Work | Roles | Gate |
|---|---|---|---|
| 0 | Issues, board, first worktree, plan, AI log | Orchestrator; Human | This plan approved |
| 1 | Contract #123 | Domain analyst, architect, implementer, reviewer | Contract review; implementation cannot start before it |
| 2 | Types #124 | Architect, implementer, test engineer, reviewer | `cargo test -p quantrule-model`, `cargo doc` |
| 3 | Assembler #125 | Implementer, test engineer, reviewer | Mapping, absence, causality, determinism |
| 4 | Direction/Structure #126 | Implementer, test engineer, reviewer | Tenure, neutrality, activity |
| 5 | Progression #127 | Implementer, test engineer, reviewer | Pace, six anchors, terminal freeze |
| 6 | Readings #128 | Implementer, test engineer, reviewer | Total mapping, 50 cap |
| 7 | Agreement/churn #129 | Implementer, test engineer, reviewer | Coverage, majority, churn |
| 8 | Command #130 | Implementer, test engineer, reviewer | Text/JSON goldens, CLI |
| 9 | Closeout #131 | Implementer, test engineer, reviewer, release verifier, orchestrator | Full verification; human acceptance |

Security/privacy review is not triggered: local CSV input, stdout output, no network, no credentials, no authentication, no external data.

## 10. Verification

The full checklist is section 11. Required commands:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

Required manual checks:

- Run `market-dashboard` on a small fixture, a short fixture, and the 100k M1 fixture.
- Run the same command twice and compare bytes.
- Compare interactive and non-interactive output.
- Confirm existing command tests remain green.
- Confirm no store schema change and no new dependency.
- Record observed times for the 100k full-window run.

Limitations:

- The 100k timing is a smoke measurement, not a formal benchmark; no benchmark harness exists.
- The board's `State`/`Item type` fields are workflow-owned; the agent does not set them.
- The Phase 2 value types and their unit tests pass. The assembler, shell command, and end-to-end tests are pending; no end-to-end result is claimed.

## 11. Verification checklist

| # | Phase | Check | Evidence | Expected | Status |
|---|---|---|---|---|---|
| 1 | 0 | Plan approved; issues opened; board set; first worktree created | Issues #122–#131; board: #122 and #123 `In Progress`, #124–#131 `Todo`, all with `State` and `Item type`; worktree `quant-rule--dashboard-view-contract` | Complete | [x] |
| 2 | 1 | Contract exists and is linked | `docs/model/2026-10-25/dashboard-state.md`; linked from `docs/README.md` and this plan; PR #132 | Complete | [x] |
| 3 | 1 | Contract covers types, metrics, readings, agreement, churn, absence, deviations | Human review approved 2026-10-25; contract covers §§1–16; PR #132 CI passes | No contradictions | [x] |
| 4 | 2 | `DashboardState` types exist; no `ModelState` | `cargo test` — 322 model unit tests passed, including 12 `dashboard` type tests; `cargo doc`; PR #133 CI passes | Compiles | [x] |
| 5 | 2 | `AxisValue` four variants; domains exact | `dashboard::tests::axis_values_keep_their_variant_and_domain`, `domains_stay_distinct` | Signed/Rank/Ordinal/Undefined | [x] |
| 6 | 2 | Six stage anchors; no arithmetic | `dashboard::tests::stages_are_the_six_contract_codes` | Against…Terminal | [x] |
| 7 | 2 | Public items documented | `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps` — no warnings | No warnings | [x] |
| 8 | 3 | Four positions map correctly | Unit tests | As specified | [ ] |
| 9 | 3 | Readiness maps to solid/dashed with reason | Unit tests | Correct | [ ] |
| 10 | 3 | Missing timeframe typed absence | Unit tests | No zero | [ ] |
| 11 | 3 | Repeated/reordered determinism | Tests | Identical | [ ] |
| 12 | 3 | Causality at bar `t` | Test | Unchanged by future bars | [ ] |
| 13 | 4 | Direction neutral pause: `neutral`, tenure 46 | Test | Correct | [ ] |
| 14 | 4 | Direction reversal resets tenure | Test | Tenure 1 | [ ] |
| 15 | 4 | Direction unavailable freezes tenure and marks continuity interrupted | Test | Correct | [ ] |
| 16 | 4 | Opposite sign after gap creates new origin at first observed bar | Test | Correct | [ ] |
| 17 | 4 | Tenure percentile 252/50/ties/no current sample | Test | Exact | [ ] |
| 18 | 4 | Direction radius = tenure; magnitude detail | Golden | Correct | [ ] |
| 19 | 4 | Structure activity never changes score/size | Test | Unchanged | [ ] |
| 20 | 4 | Structure evidence includes relations, scale, EMA, levels | Golden | Present | [ ] |
| 21 | 5 | All six progression anchors | Test | Correct | [ ] |
| 22 | 5 | Recent pace signed with/against/flat | Test | Correct | [ ] |
| 23 | 5 | Same-leg requirement; leg change resets | Test | Typed absence | [ ] |
| 24 | 5 | Recent pace speed percentile is the radius | Golden | Absolute | [ ] |
| 25 | 5 | Average net displacement is detail | Golden | Present | [ ] |
| 26 | 5 | Displacement percentile remains separate | Golden | Present | [ ] |
| 27 | 5 | Terminal pace freezes for completed/invalidated/superseded | Tests | Frozen | [ ] |
| 28 | 5 | Lifecycle visible and distinct from stage | Golden | Correct | [ ] |
| 29 | 5 | No coiling/ignition/exhausted labels | Grep/test | Zero hits | [ ] |
| 30 | 6 | Every event maps exactly once | Test | Total | [ ] |
| 31 | 6 | Event/Transition history; Level references in detail | Test | Correct split | [ ] |
| 32 | 6 | 50 cap trims oldest | Test | 50 max | [ ] |
| 33 | 6 | Availability filter | Test | No future event | [ ] |
| 34 | 6 | Progression stage transition | Test | Present | [ ] |
| 35 | 7 | Agreement strict majority, of=3, Progression excluded | Test | Correct | [ ] |
| 36 | 7 | Minimum 2 valid timeframes; one reading cannot form a majority | Test | Insufficient coverage | [ ] |
| 37 | 7 | Missing timeframe not neutral; coverage lowered | Test | Correct | [ ] |
| 38 | 7 | Neutral is a measured class; tie is not a majority | Test | Correct | [ ] |
| 39 | 7 | Churn max, sources, threshold, window | Test | Complete | [ ] |
| 40 | 7 | Churn history completeness; Building when incomplete | Test | Correct | [ ] |
| 41 | 7 | Churn threshold default 6 and boundary | Test | Correct | [ ] |
| 42 | 8 | `market-dashboard` parses/refuses | CLI tests | Correct | [ ] |
| 43 | 8 | Text golden matches | CLI golden | Exact bytes | [ ] |
| 44 | 8 | Four tracks, one row per timeframe, no reversal | Golden/manual | Correct | [ ] |
| 45 | 8 | Typed absence prints `—`, never origin/zero | Golden | Correct | [ ] |
| 46 | 8 | `--json` numeric, parseable, deterministic | JSON test | Stable | [ ] |
| 47 | 8 | Interactive/non-interactive byte-identical | CLI test | Identical | [ ] |
| 48 | 8 | No strategy vocabulary | Grep/test | Zero hits | [ ] |
| 49 | 8 | Help/README updated | Help test | Complete | [ ] |
| 50 | 9 | `cargo fmt --all -- --check` | Command | Pass | [ ] |
| 51 | 9 | `cargo clippy --all-targets -- -D warnings` | Command | Pass | [ ] |
| 52 | 9 | `cargo test` | Command | Pass | [ ] |
| 53 | 9 | `cargo doc --no-deps` | Command | Pass | [ ] |
| 54 | 9 | Release run: small, short, 100k | Manual/CLI | Correct, time recorded | [ ] |
| 55 | 9 | Existing commands unchanged | Existing tests | Pass | [ ] |
| 56 | 9 | No new dependency | `Cargo.toml`/lock review | Unchanged | [ ] |
| 57 | 9 | No store schema change | Store tests | Pass | [ ] |
| 58 | 9 | Plan Verified; AI log updated | Docs | Complete | [ ] |
| 59 | 9 | Parent Done; sub-issues closed; worktrees cleaned | Board/Git | Complete | [ ] |

## 12. Risks and recovery

- **Semantic drift:** the view's Direction and Progression sizes differ from the existing `market-state` size percentiles. Mitigation: name the fields distinctly, document the roles, and test that `market-state` is unchanged.
- **Stepwise Structure percentile:** only three absolute magnitudes exist. Mitigation: retain it, label it, and show the relations and activity separately.
- **Neutral tenure misread:** a large radius can appear bullish at a neutral score. Mitigation: explicit `neutral` label and continuity/tenure text; test the 40+6 example.
- **Unavailable interval:** the true reversal time is unknown. Mitigation: mark continuity interrupted and set a new origin only at the first observed opposite bar; never backdate the reversal.
- **Recent pace conflation:** speed and direction are separate. Mitigation: radius uses absolute pace; signed pace and direction are details; test both.
- **Terminal pace decay:** a frozen leg must not decay with later bars. Mitigation: freeze at the terminal transition and test all three terminal lifecycles.
- **Low coverage majority:** two of four timeframes can form a majority. Mitigation: publish coverage and minimum valid count; missing is not neutral.
- **Churn overclaim:** max changes across different durations is not universal stability. Mitigation: label it `churn`, publish sources/threshold/history, and make the threshold configurable.
- **Performance:** the view iterates the model series for tenure/pace. Mitigation: bounded percentile windows; measure the 100k run; keep `--window latest`.
- **Recovery:** all work is additive. Revert the new module, command, docs, and tests if the contract is rejected; existing commands and store are untouched.

## 13. Stop conditions

Stop and request human direction if:

- A metric definition in this plan conflicts with the model contract.
- A change would alter an existing command contract.
- A store schema or migration change is required.
- A new dependency is required.
- Causality or determinism fails and cannot be fixed without changing the model.
- The full-window 100k run is unacceptably slow.
- A source file exceeds 1,000 lines without a clean seam.
- Absence cannot be represented without a zero or default.
- The work exceeds the approved sub-issue scope.

## 14. AI transparency

Material AI contributions for this work item are recorded in
[`docs/ai-transparency-log/2026-10-25/122-market-dashboard.md`](../../ai-transparency-log/2026-10-25/122-market-dashboard.md).

The log records the planning discussion, the rejected alternatives, the Phase 0 setup, and the fact that no code or test result exists yet.
