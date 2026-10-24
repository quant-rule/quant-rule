# Execution Plan: The Model Engine and Its Metrics Layer

- Issue: [#14](https://github.com/quant-rule/quant-rule/issues/14) — feat: model engine, contract and implementation breakdown
- Branch: `docs/14-model-engine-contract` (this change) and one branch per sub-issue
- Status: Verified
- Date: 2026-10-21 (UTC; 2026-10-20 local time)
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Decision: [`docs/adr/2026-10-21/four-axis-analytical-model.md`](../../adr/2026-10-21/four-axis-analytical-model.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-21/14-model-engine.md`](../../ai-transparency-log/2026-10-21/14-model-engine.md)
- Follow-up: [Completion plan (M2–M14)](../2026-10-24/14-model-engine-completion.md) — approved 2026-10-24

## Intent

QuantRule's model engine does not exist, and the way it was described would have produced the wrong system. `AGENTS.md` section 2 called the engine a stage that "computes indicators, which are categorized on four axes", with "every indicator declares the axis it belongs to", and issue #14 asked for an indicator abstraction carrying a name, an axis, required history, and the metrics it emits. That framing makes an indicator the owner of an axis. It also leaves the engine's product as "the combined time series" — candles beside indicator metrics, one series, no cross-timeframe view, no events, no notion of a market state.

The intended outcome is a model engine that describes market state along four independent axes, with a shared three-type event model, numeric-only output, and cross-timeframe aggregation in which each axis is aggregated by a method that suits it. The human asked for the metrics layer of that engine to be planned and built, and for the work to be tracked inside the two existing open engine issues rather than as a new one. This change plans it; the fourteen sub-issues build it.

Affected users: the human developer, who owns the model's shape and the command vocabulary; and the agents who will implement the fourteen sub-issues described here.

## Current state

Evidence gathered on 2026-10-20 local time (2026-10-21 UTC), `origin/main` at commit `4b11f01`.

- The repository contains no Rust source, no `Cargo.toml`, no `tests/` directory, and no CI configuration. `cargo --version` returns `cargo 1.99.0` and `rustc --version` returns `rustc 1.99.0`, which establishes toolchain availability only. No build, test, lint, or doc run applies to this change, and none is claimed.
- The repository contains `AGENTS.md` (577 lines, fourteen numbered sections), `README.md` (5 lines), and `docs/` with five execution plans and five transparency logs under the kinds `execution-plans` and `ai-transparency-log`.
- `AGENTS.md` section 2, before this change, described the model engine as "MODEL ENGINE — indicators", stated that "every indicator declares the axis it belongs to", and described the output as "the combined time series". Its crate-layout note said: "Concrete module paths are not listed here because the Rust workspace does not exist yet."
- `AGENTS.md` section 2's invariant list held three items: multi-timeframe, candles-only, and layer isolation. Nothing stated that the four axes are independent, that a late-arriving measurement must record both its occurrence and its availability, or that a correlated reading is not a second vote.
- `AGENTS.md` section 3 had no rule about axis ownership, numeric-only model output, ordinal codes, or event record shape. Section 4 had no model-layer, event, progression, or aggregation test rule. Section 5 listed two documentation kinds and marked the ADR path as a future location. Section 12 had no rule fixing the percentile definition, the accumulation order, or the retention bound.
- Issue #14 was open on the board in `Todo`. Its body read, in full:

  > Goal: Build the model engine — the stage that takes a candle series and computes indicators, each declared on one of four axes: direction, momentum, progression, and structure. It reads only the series it is given and returns indicator metrics aligned to it.
  >
  > Depends on: #9 (data engine), because indicators consume the normalized per-timeframe series it produces.
  >
  > Scope:
  > - Define the indicator abstraction: name, axis, required history, and the metrics it emits.
  > - Implement the first indicators on each of the four axes, each with rustdoc and a compiled example.
  > - Keep output length aligned with input length, with an explicit value for bars that lack history.
  > - Keep the engine pure and causal: no I/O, no clock, no randomness, and no reading beyond the current bar.
  > - Expose indicators to the shell as commands, each with a documented output shape.
  >
  > Done when:
  > - Every indicator declares its axis and its required history.
  > - `cargo test` passes, with a causality test per indicator: appending, changing, or removing later bars leaves earlier output unchanged.
  > - A golden fixture covers indicator output, and a determinism test covers repeated and reordered runs.
  > - Edge cases are covered: empty series, single candle, and input shorter than the required history.
  > - Each indicator is reachable from the shell by a command whose output is asserted in a test. Issue #9 (the data engine) was open in `Todo` and states that the engine "is reachable from the shell by at least one command whose output is asserted in a test", which presupposes a shell that no issue creates.
- The project board has three single-select fields: `Status` (Todo, In Progress, Done), `Item type` (Issue, Pull request), and `State` (Open, Closed), plus a `Parent issue` field and a `Sub-issues progress` field. The GraphQL API exposes `Issue.parent`, `Issue.subIssues`, and the `addSubIssue` and `createIssue(parentIssueId)` mutations, so the sub-issue workflow is natively supported.
- The board's automation closes a GitHub issue when its board item reaches `Done`. This was observed on issue #5 and recorded in that work item's transparency log, and it applies to the sub-issues created here.

## Scope

Included in this change:

- Amend `AGENTS.md` sections 2, 3, 4, 5, and 12 so the architecture, conventions, testing obligations, documentation kinds, and determinism rules are consistent with the four-axis model. No other section changes.
- Record the analytical model's contract in `docs/model/2026-10-21/four-axis-analytical-model.md`: the four axes, the numeric output contract, the event model, the aggregation rules, the code registries, and the explicit exclusions.
- Record the architecture decision in `docs/adr/2026-10-21/four-axis-analytical-model.md`, with the alternatives considered and the consequences.
- This plan, and the AI transparency log that matches it.
- Update `docs/README.md` so the kinds index names the two new kinds.
- Rewrite issue #14 as the four-axis model, in the body format of `AGENTS.md` section 8, and amend issue #9 so the data-engine prerequisites it implies are explicit.
- Create fourteen sub-issues M1 (issue #17) to M14 (issue #30) under #14, each with acceptance criteria, each with its dependencies named, and each added to the board in `Todo` with `Item type` = Issue and `State` = Open.

Explicit non-goals:

- No Rust source, no `Cargo.toml`, no `tests/` directory, no CI configuration. The user directed that the documentation and issue structure come first and that no Rust code is written as part of this task.
- No implementation of M1 (issue #17) to M14 (issue #30). Each is a separate work item with its own branch, plan, and pull request.
- No change to the fourteen numbered sections of `AGENTS.md`, their numbering, their titles, or their cross-references.
- No change to the command vocabulary. Section 3 of `AGENTS.md` reserves it to the human, so the names proposed in M13 (issue #29) are proposals.
- No merge of any pull request.

## Assumptions and uncertainty

The specification that drives this work left eleven points open. The human directed that the recommended default is applied to each, that each is recorded here and in the transparency log, and that each is flagged in the pull request so it can be reviewed and overridden. They are listed as A1 to A11 in the Decisions table below, and each is applied in the section of the contract document named in the table's last column. Two of them change what the model publishes, and two change what the implementation does, so they are called out in the risk section as well.

Beyond those eleven, the following assumptions carry the plan:

- **The data engine lands first.** The metrics layer consumes a normalized candle series per timeframe, which issue #9 produces and nothing else does. The human chose this sequencing, and the metrics implementation does not duplicate or prematurely introduce the workspace, the candle type, or the timeframe type.
- **The model engine is one crate.** The workspace is expected to hold one crate per pipeline stage: a data crate, a model crate, and the shell binary, with dependencies pointing from the shell to the model to the data. The concrete names are recorded in `AGENTS.md` section 2 in the change that creates the workspace, per section 2's own rule, and the crate-layout decision is recorded in the ADR.
- **The shell skeleton belongs to the data engine.** Issue #9's done-when already requires a command whose output is asserted in a test, and no other issue creates the shell. The shell skeleton is therefore D4 in issue #9 rather than a fifteenth sub-issue of #14.
- **The model computes per-bar series and publishes a snapshot.** Percentiles, state ages, and change counts are all history-dependent, so the engine keeps a per-bar series aligned to the input length and publishes the latest snapshot of each field. The shell renders the snapshot.
- **The specification's field names are the contract.** The numeric output table in section 9 of the contract document is binding. Where the implementation needs a name the specification does not give, it is a local choice that does not change the published shape, and it is recorded in the sub-issue that introduces it.
- **No new dependency is needed.** The model layer needs floating-point arithmetic, sorting, and collection handling, all of which the standard library provides. A percentile over a window needs a sort, not a statistics crate.

## Decisions

### The eleven specification refinements (A1 to A11)

Each of these was an open point in the source specification. Each is applied in the contract document, in the section named in the last column, so the contract is self-contained and the implementation follows one document.

| # | Open point | Options | Choice | Rationale | Applied in (contract section or work item) |
|---|---|---|---|---|---|
| A1 | Which leg a timeframe publishes, given that the contract has one `progress` field and the specification keeps major and minor legs separate | Current leg as the headline with a scale code, plus a full record per scale; two separate field sets per scale; the active leg with major preferred | Current leg as the headline with `progression_leg_scale_code`, plus a full record per scale, and cross-timeframe progression computed per scale | The contract has one progression field, so a headline is required. The headline is the current leg, which is the one a consumer means by "the leg" — and it is the leg that carries a forming state. The full per-scale records keep both scales inspectable, and the per-scale aggregation honours "a minor counter-move inside a major leg is not blended into the major leg's stage distribution" | Contract §8.4, §9, §10 |
| A2 | Whether `lifecycle_code` 0 (forming) and unavailable-reason 6 (origin unconfirmed) are reachable | Create legs from left-side candidate pivots, so a leg exists before its pivot confirms; create legs only from confirmed pivots, which leaves codes 0 and 6 unreachable and requires amending the contract | Create legs from left-side candidate pivots, and publish the candidate as a reading | A pivot is knowable only when its right-side bars have closed, so a leg whose origin is a confirmed pivot is confirmed the instant it is created, and codes 0 and 6 would be dead vocabulary. A candidate is a bar whose extreme is strict against the k bars to its left; the leg is created then, confirmed when the k bars to its right close, discarded when the candidate fails, and invalidated when price closes beyond the origin by more than 0.2 ATR against the leg. The candidate is a reading, so the leg logic never recomputes it. The alternative would have required amending the published contract | Contract §5, §8.4 |
| A3 | The exact empirical percentile | Inclusive or strict comparison, and whether the current bar is in its own history | `100 * count(samples <= value) / n`, where the samples are the up to 252 most recent valid magnitudes strictly before the current bar, and `n < 50` gives null | "The preceding 252 valid bars" excludes the current bar, and an inclusive comparison gives a value the share of history at or below it, which is the empirical distribution function. The 50-sample floor is the specification's own | Contract §9 |
| A4 | The state counters when the compact code is null, which happens whenever a progression stage is null | Freeze the counters; reset the age; count null as a state | Freeze the counters, and report readiness 0 or 1 while the code is null | A null stage is an absence, not a transition. Resetting the age would destroy the information that the leg has been in its current state for a long time, and counting null as a state would put a state code on the axis that the contract does not define | Contract §9 |
| A5 | The thresholds for the cross-timeframe structure and momentum shares | Key them on `structure_score` at ±0.1, or on the sign of `structure_code`; leave them undefined | Key them on `structure_score` at ±0.1, and on `momentum_force` at ±0.1 | Section 11 already fixes ±0.1 as the common sign threshold for `direction`, `momentum_force`, and `structure_score`. Using the same threshold keeps the consensus and the aggregation consistent, and `structure_code` remains the compact code used for the histogram and the state counters | Contract §10, §11 |
| A6 | Which levels are eligible references for Level events | Every confirmed pivot ever confirmed; the three EMAs plus the latest confirmed swing high and low at each scale | The three EMAs plus the latest confirmed swing high and low at each scale, seven references per timeframe | The specification says "confirmed minor/major swing levels" without a bound, and an unbounded reference set makes the event stream grow without limit and the event store unbounded, which section 12 now forbids. The bounded set is also the set the clustering uses, so the two agree on what a level is | Contract §7.2 |
| A7 | Which levels feed the cross-timeframe level clusters, and how they are grouped | The same bounded set as A6; every level the model retains; single linkage or complete linkage | The same bounded set, clustered with single linkage, with the span reported in ATR | One definition of a level, used by both the events and the clusters. Single linkage is the only rule the specification states, and the reported span makes any chaining visible rather than hidden | Contract §10 |
| A8 | How long the event history is retained | Unbounded; bounded at a stated size | A bounded buffer of 1024 events per timeframe, with the five-bar window counts computed from the retained records | Section 12 now requires a bounded retention whose size is recorded in the contract. 1024 is far larger than the published five-bar window needs, and it is stated rather than implied | Contract §14 |
| A9 | Whether the model publishes per-bar series, a snapshot, or both | Snapshot only; per-bar series only; both | Both: the engine computes per-bar series aligned to the input length and publishes the latest snapshot of each field | The percentile, the state age, and the change count are all history-dependent, and the causality tests compare a bar's output across runs, which needs a per-bar series to compare. The contract's field table is a snapshot, so the snapshot is what the shell renders | Contract §9 |
| A10 | When the crate names are fixed | Now, in `AGENTS.md`; in the change that creates the workspace | In the change that creates the workspace, with the crate-layout decision recorded in the ADR | Section 2 states that no crate name is assumed before the workspace exists and that the names are recorded in the same change that introduces it. Writing names into section 2 now would contradict the rule in the same file, and the module paths in this plan are therefore written without a crate prefix | ADR "Crate layout" |
| A11 | The shell command names for the market state | Fixed here; proposed and reserved to the human | Proposed only, and recorded in M13 (issue #29) as proposals | Section 3 of `AGENTS.md` reserves a change to what a command means to the human, and the command vocabulary has never been fixed in this repository. The M13 (issue #29) sub-issue names candidates — `market-state`, `axis`, `events`, `legs` — as proposals, and asks the human to decide them before any of them hardens into a contract | M13 (issue #29) |

### The three work-item and process decisions (A12 to A14)

These are not specification points. They record how this work item was carried out, and they are not open to override as contract changes.

| # | Open point | Options | Choice | Rationale |
|---|---|---|---|---|
| A12 | How the fourteen work items are tracked | A checklist inside #14; GitHub sub-issues; a new umbrella issue | GitHub sub-issues of #14 | The human chose independently tracked items. The board already carries a `Parent issue` field and a `Sub-issues progress` field, and the API exposes `createIssue(parentIssueId)`, so the native mechanism is available |
| A13 | How much of `AGENTS.md` this change amends | Section 2 only; sections 2 and 3; sections 2, 3, 4, 5, and 12 | Sections 2, 3, 4, 5, and 12 | A contract in section 2 with no numeric-output rule in section 3, no test rule in section 4, and no determinism rule in section 12 leaves the file's rules not binding on the code that implements it |
| A14 | Whether the data engine is implemented first | #9 first, then the metrics layer; a minimal workspace pulled forward; everything on one branch | #9 first, then the metrics layer | The metrics layer consumes the normalised series that #9 produces. Pulling a minimal workspace forward duplicates the candle and timeframe types that #9 will also define, and section 2's layer isolation makes that duplication a second source of truth |

## Acceptance criteria

Observable conditions for completion of this change:

- `AGENTS.md` section 2 describes the analytical model's three stages, states that a reading is not owned by an axis, defines the output as the market state, and carries the three new invariants.
- `AGENTS.md` section 3 states that the model publishes numbers only, that missing data is null and never zero, that ordinal codes are counted and not averaged, and that an event does not alter an axis value.
- `AGENTS.md` section 4 requires the model-layer, event, progression, and aggregation test rules.
- `AGENTS.md` section 5 lists the `model` and `adr` documentation kinds.
- `AGENTS.md` section 12 fixes the numeric contract and the bounded retention.
- The fourteen numbered sections, their numbering, their titles, and every cross-reference in `AGENTS.md` are unchanged, and every cross-reference still resolves.
- `docs/model/2026-10-21/four-axis-analytical-model.md` states the axes, the formulas, the numeric output contract, the event model, the aggregation rules, the code registries, and the exclusions.
- `docs/adr/2026-10-21/four-axis-analytical-model.md` states the decision, the alternatives, and the consequences.
- `docs/execution-plans/2026-10-21/14-model-engine.md` exists, is marked `Approved`, and carries the ten required contents of `AGENTS.md` section 11.
- `docs/ai-transparency-log/2026-10-21/14-model-engine.md` exists and carries the four required items of `AGENTS.md` section 6.
- `docs/README.md` names the `model` and `adr` kinds.
- Issue #14's body describes the four-axis model, names the contract, and lists the fourteen sub-issues.
- Issue #9's body names its four prerequisite items, including the shell skeleton.
- Fourteen sub-issues exist under #14, numbered in dependency order, each on the board in `Todo` with `Item type` = Issue and `State` = Open, each with acceptance criteria and its dependencies named.
- A pull request exists against `main` that closes #14, states the verification performed, and states the risks.

## Work breakdown

### This change — the contract and the issue structure

| Step | Action | Role | Paths affected | Depends on |
|---|---|---|---|---|
| 1 | File the ambiguity list and the plan; confirm the decisions; move #14 to `In Progress` on the board | Orchestrator | this plan, board | — |
| 2 | Create `docs/14-model-engine-contract` from the latest `origin/main` | Implementation | git state | 1 |
| 3 | Amend `AGENTS.md` sections 2, 3, 4, 5, and 12 | Implementation | `AGENTS.md` | 2 |
| 4 | Write the model contract | Implementation | `docs/model/2026-10-21/four-axis-analytical-model.md` | 2 |
| 5 | Write the architecture decision | Implementation | `docs/adr/2026-10-21/four-axis-analytical-model.md` | 2 |
| 6 | Write this plan | Orchestrator | `docs/execution-plans/2026-10-21/14-model-engine.md` | 2 |
| 7 | Write the transparency log | Orchestrator | `docs/ai-transparency-log/2026-10-21/14-model-engine.md` | 2 |
| 8 | Update the kinds index | Implementation | `docs/README.md` | 2 |
| 9 | Rewrite #14's body; amend #9's body | Orchestrator | GitHub issues | 3–8 |
| 10 | Create the fourteen sub-issues with board membership and field values | Orchestrator | GitHub issues, board | 9 |
| 11 | Verification pass, including an independent review of the diff and the documents against the acceptance criteria | Test Engineer, Code Reviewer | all changed files | 3–10 |
| 12 | Commit, push, open the pull request | Orchestrator | git state, PR | 11 |
| 13 | Move #14 to `Done` on the board | Orchestrator | board | 12 |
| 14 | Stop for the human's merge authorization | Orchestrator | — | 13 |

`AGENTS.md` section 9 requires an issue to be moved to `In Progress` before the first change is made to the working tree, so step 1 moves #14. That rule was not met on this change — the branch and the first edits existed before the move — and the deviation is recorded in the Verification section below and in the transparency log.

### The fourteen sub-issues

Each is a separate work item with its own branch, plan, transparency log, and pull request. One is in flight at a time, in the order below.

| ID | Title | Depends on | Problem and resolution | Affected components | Done when |
|---|---|---|---|---|---|
| M1 (issue #17) | feat: build the shared numeric kernel of the model | — | *Problem:* the contract needs null-versus-zero discipline, unavailable-reason codes, readiness codes, clamping, the compact-state bucketing, the percentile, and the state counters, and every axis would otherwise define its own. *Resolution:* one kernel module, with the percentile and the compact-state function defined exactly once, and the reason-code registry published with it | `model crate :: kernel` | Every helper has unit tests; the percentile matches a hand-computed fixture; no helper can return 0 for a missing measurement; the reason-code registry is a checked-in table |
| M2 (issue #18) | feat: build the indicator readings layer | M1 (issue #17) | *Problem:* the readings are the model's only calculation inputs, and issue #14's "one axis per indicator" abstraction is the wrong shape. *Resolution:* per-bar series aligned to the input length, null before the required history, a role registry in which each use of a reading states its own definition, and both pivot scales carrying occurrence and availability bars, together with the pivot candidate that decision A2 makes the basis of every leg | `model crate :: readings` | A golden fixture per reading; a causality test per reading; a determinism test; edge cases for an empty series, a single candle, and a series shorter than the required history; no pivot is used before its availability bar |
| M3 (issue #19) | feat: build the direction axis | M2 (issue #18) | *Problem:* the Direction formulas and their null rule are specified but not implemented. *Resolution:* `d_short`, `d_long`, the direction score, the compact code, the percentile, readiness, and the persistence counters | `model crate :: axes::direction` | A null whenever a required EMA or ATR is unavailable; a valid 0 is neutral and not missing; the percentile is null under 50 samples; the axis does not read the EMA distances |
| M4 (issue #20) | feat: build the momentum axis | M2 (issue #18) | *Problem:* force, acceleration, and intensification are specified but not implemented. *Resolution:* the three published fields with their clamp and their null rule | `model crate :: axes::momentum` | All three in [-1, +1]; intensification is 0 exactly when the magnitude is unchanged; null when a required RSI value is unavailable; the three-bar comparison window is honoured |
| M5 (issue #21) | feat: build the structure axis | M2 (issue #18) | *Problem:* the swing relations and the EMA dynamic-level features must not be summed into one score, and the relation codes must survive an incomplete pair. *Resolution:* `high_relation_code` and `low_relation_code`, `structure_score`, `structure_code`, the per-EMA distance, cross, and retest features, and the confirmed-level registry | `model crate :: axes::structure` | The score is null on an incomplete pair while the individual relation codes are retained; the code matches the published table; a test proves the EMA features never enter `structure_score`; equal highs and equal lows produce the 0 relation |
| M6 (issue #22) | feat: build the progression axis | M2 (issue #18), M5 (issue #21) | *Problem:* progression is the largest part of the contract — leg lifecycle, target policy, target versioning and stability, unclamped progress, and ordinal stages — and it depends on the confirmed levels M5 (issue #21) produces. *Resolution:* the leg store per timeframe and scale, the target policy with its ATR fallback, the progress arithmetic, the five lifecycle codes, and the six stage codes, with the decisions A1 and A2 applied | `model crate :: axes::progression` | The stage is null while the origin is unconfirmed or the target is unstable; a terminal lifecycle forces stage 5; a target revision increments the version and changes no price; the progress is unclamped; a degenerate target gives null progress with reason 3 or 4; every lifecycle code is reachable |
| M7 (issue #23) | feat: build level events | M2 (issue #18), M5 (issue #21) | *Problem:* the event record and the Level subtype rules are specified but not implemented. *Resolution:* the shared event record, the source and reference registries, the test, cross, and break detection with ATR scaling, and the break-cross-test precedence | `model crate :: events::level` | One event per level per bar; the precedence holds; a test, cross, or break never precedes its availability bar; the reference universe is the bounded set of A6 |
| M8 (issue #24) | feat: build divergence events | M2 (issue #18), M5 (issue #21) | *Problem:* divergence is the one relationship that spans two axes, and it must never be a second RSI vote. *Resolution:* the RSI window around a confirmed pivot, the four-subtype matrix, and the numeric evidence fields | `model crate :: events::divergence` | No event before the newer pivot is confirmed; no event when a pivot or an RSI value is unavailable; the subtype and direction codes match the published matrix; the divergence changes no axis value |
| M9 (issue #25) | feat: build signal events | M2 (issue #18), M3 (issue #19), M4 (issue #20), M5 (issue #21) | *Problem:* the Signal subtypes overlap with the Level events, and a duplicated occurrence would inflate the evidence. *Resolution:* RSI threshold crossings, the three EMA crossover pairs, and the compact state changes, with the most specific event emitted and duplicates avoided | `model crate :: events::signal` | No duplicate emission for one underlying occurrence; a price/EMA crossing stays a Level event; a signal adds no vote to its source reading |
| M10 (issue #26) | feat: build the volatility and participation context | M2 (issue #18) | *Problem:* the context diagnostic is specified but has no home, and it must not vote. *Resolution:* `atr_ratio`, `rvol_ratio`, and the four-bucket `context_code` | `model crate :: context` | The code is a pure function of the two ratios; the code is null when either ratio is unavailable, including when volume is absent; no axis reads the context |
| M11 (issue #27) | feat: build cross-timeframe aggregation | M3 (issue #19) through M10 (issue #26) | *Problem:* the aggregation rules differ per axis, and a single method would destroy the ordinal codes. *Resolution:* the Direction and Momentum means, shares, and gradients; the Structure histogram, shares, span, and conflict count; the per-scale Progression histograms, mode, and span; the leg matching; the level clustering; and the event counts over the five-bar window | `model crate :: aggregate` | No ordinal code is ever averaged; every aggregate names its source timeframes; the level clustering uses the A6 set and the A7 rule; the event counts are over the five-bar window and are not a vote |
| M12 (issue #28) | feat: build the consensus diagnostic and the market-state output contract | M3 (issue #19), M4 (issue #20), M5 (issue #21), M6 (issue #22), and M11 (issue #27) | *Problem:* the published field list, the readiness and reason codes, and the cross-axis counts have no home, and the four axes must never be summed. *Resolution:* the consensus counts, the weakest-axis code, the progression leg direction published separately, and the assembled market-state structure | `model crate :: state` | Every field of the contract's output table exists with the stated type and range; a missing axis is not counted as neutral; no field averages two axes |
| M13 (issue #29) | feat: expose the market state through the shell | M12 (issue #28), #9 | *Problem:* a capability the user cannot reach by typing a command does not exist, and the words that render the numbers belong to the shell. *Resolution:* the commands, the label mapping in the shell, the command reference, and the README | shell binary, `docs/` | Each command's output is asserted in a test; the interactive and the non-interactive output are byte-identical; no word appears in the model layer; the command names are decided by the human before they harden into a contract |
| M14 (issue #30) | test: harden the model layer against the contract | M1 (issue #17) through M13 (issue #29) | *Problem:* the contract has many invariants that no single sub-issue is responsible for. *Resolution:* the property tests, the determinism tests across reordering and split work, the anti-double-counting regression tests, the benchmarks, and a line-by-line check of the implementation against the contract | `tests/`, benchmarks | The property tests cover output length, the absence of values never supplied, the stage ordering, and the percentile bounds; the determinism tests cover repeated runs, reordered collections, and split work; the anti-double-counting tests are explicit; the benchmark result is reported in the pull request |

## Verification

### Verification of this change

Commands actually executed on 2026-10-20 local time (2026-10-21 UTC), with observed results:

- `git fetch origin --prune`, `git log --oneline -1 main`, `git log --oneline -1 origin/main`, `git rev-list --left-right --count main...origin/main` — the local `main` and `origin/main` were both at `4b11f01`, with a count of `0 0`, so the local branch was not stale.
- `git checkout -b docs/14-model-engine-contract origin/main`, then `git merge-base --is-ancestor origin/main HEAD` — exits zero, so the branch contains the current `origin/main`; `git diff --stat origin/main..HEAD` was empty, so the branch started from an identical tree.
- `grep -c "^## " AGENTS.md` — 14 top-level headings after the change, unchanged, and the headings remain numbered 1 to 14 with no gaps and no duplicates.
- A script parsed every `## N.` heading and every `section N` reference in `AGENTS.md`: the headings are contiguous 1 through 14, the referenced set is {2, 3, 4, 6, 8, 9, 11, 12, 14}, and each reference resolves to exactly one heading. No unresolved reference.
- `git diff --check` — no whitespace errors. A trailing-newline check on every changed and added file returns a newline for each.
- `git status --short` — the change modifies `AGENTS.md` and `docs/README.md`, and adds the contract, the ADR, this plan, and the transparency log. `git diff --stat` shows only the two modified files, because the four new documents are untracked until they are staged. No Rust source exists, so no `cargo` command applies and no build, test, lint, or doc result is claimed.
- The plan is checked against the ten required contents in `AGENTS.md` section 11: Intent, Current state, Scope, Assumptions and uncertainty, Decisions, Acceptance criteria, Work breakdown, Verification, Risks and recovery, and Stop conditions are all present by name.
- The transparency log is checked against the four required items in `AGENTS.md` section 6: Date and tool, Intent and boundary, Verification and evidence, and Learning with the rejected and corrected suggestions nested under it.
- The contract document is checked against the source specification field by field: the four axes, the input set, the shared calculations, the operational map, the event model and its three types, the axis formulas, the numeric output table, the cross-timeframe aggregation, the consensus diagnostic, the context diagnostic, and the exclusions. The source specification is the four-axis analytical model given in the work request for issue #14; it is not a file in this repository, so the check is a manual read of the contract against it, and the eleven decisions A1 to A11 are the record of what that read changed.
- The board was read before and after the change with a GraphQL query against the project: the `Status` field offers Todo, In Progress, and Done; `Item type` offers Issue and Pull request; `State` offers Open and Closed; and a `Parent issue` field and a `Sub-issues progress` field exist, so sub-issues are natively supported.
- The sub-issues are read back after creation: `issue(number: 14).subIssues` lists M1 (issue #17) to M14 (issue #30) in dependency order, each carries the parent link, and each board item shows `Status` = Todo, `Item type` = Issue, and `State` = Open.
- Issue #14 was moved to `In Progress` on the board, and the transition was confirmed by the mutation's response and by a read-back showing `Status` = In Progress, `Item type` = Issue, and `State` = Open. The move to `Done` waits for the pull request to exist. `AGENTS.md` section 9 requires the move to `In Progress` before the first change is made to the working tree, and it was not made in time: the branch and the `AGENTS.md` amendment already existed when the omission was found by the review pass. The deviation is recorded here rather than presented as compliance.
- An independent review pass was run by a separate agent reading the full diff, the four documents, the live board, and the fourteen issue bodies. It found eleven blockers, ten major findings, thirteen minor findings, and five nits, and it confirmed the categories that were clean. The blockers were the contract omitting the decisions it was supposed to carry, a count of "eleven" against a fourteen-row table, two dangling references in the ADR, an `AGENTS.md` rule that forbade what the contract does, a crate-name promise with nothing behind it, a percentile that could not be compared across three axes, a board status that was never moved, three verification claims that were false, a mitigation that was never applied, and an issue body over the section 8 line budget. Every one of them is corrected in this change, and the corrections are recorded in the AI transparency log.
- The `AGENTS.md` heading and cross-reference script was run again after the corrections: the headings are still contiguous 1 through 14, and every `section N` reference still resolves to exactly one heading.
- A second independent review pass was run over the corrected change, reading the same files, the live board, and all fourteen issue bodies. It confirmed that the first pass's ten findings were fixed, and it found one blocker, nine major findings, eleven minor findings, and six nits. The blocker was a new false statement in this plan — it said issue #14 had been moved to `In Progress` before the first working-tree change, which the log contradicted and the evidence disproves. The major findings were the contract's decision map naming sections that carry no decision, the published leg never being defined as a rule, the compact state codes being called published without being fields, the output table omitting quantities other sections call published, the per-axis diagnostics published as one row each, the readiness rule having no value for a short history, the pivot candidate being produced by no sub-issue, an ambiguous momentum gradient, and three aggregates with no name. All are corrected, and the corrections are recorded in the AI transparency log.
- The fourteen issue bodies were rebuilt after that pass, each in the section 8 shape with the dependency line directly after the goal, each within the roughly fifteen-line budget, and each carrying the contract changes: the pivot candidate in M2, the published-leg rule in M6, the named aggregates in M11, the per-axis diagnostics in M12, and the candidate command names in M13.
- The board was read back after every change: issue #14 shows `Status` = In Progress, `Item type` = Issue, and `State` = Open; the fourteen sub-issues show `Status` = Todo, `Item type` = Issue, and `State` = Open; issue #9 is unchanged in `Todo`.
- A script resolved every relative markdown link in the six changed and added files against the working tree. The first run found six broken links, because a document in `docs/<kind>/<date>/` needs two levels of parent to reach a sibling kind. The links were corrected and the second run reports zero broken links.

- The change was committed as `3678c7f` ("docs: define the four-axis analytical model contract"), pushed to `origin/docs/14-model-engine-contract`, and opened as pull request #31 against `main`, closing issue #14. `git ls-remote --heads origin docs/14-model-engine-contract` returns `3678c7f837860d18cbbb059a1daa697706d87bf8`, and `gh pr view 31` reports the pull request as OPEN from `docs/14-model-engine-contract` to `main`. Issue #14 was moved to `Done` on the project board, and the transition was confirmed by the mutation's response and by a read-back showing `Status` = Done, `Item type` = Issue, and `State` = Closed. Moving the item to `Done` also closed the GitHub issue two seconds later, at `2026-10-21T03:57:41Z`, because this project has the automatic close-on-Done setting enabled. The issue is therefore closed on GitHub while pull request #31 is still open, which diverges from the separation between board status and issue state that section 9 describes. If the pull request is not merged, the issue must be reopened. The fourteen sub-issues are in `Todo` with `State` = Open, and issue #9 is unchanged in `Todo`.

Acceptance criteria are evaluated against the evidence above. All are met, including the criterion that depends on the pull request existing, which is satisfied by pull request #31.

**The umbrella's own scope.** Issue #14 carries two scopes, and only one of them is this issue's. The build criteria — the contract implemented, `cargo test` passing, the market state reachable from the shell — belong to the fourteen sub-issues, which state them in full. Issue #14's own criteria are the contract, the decision, the plan, the sub-issue structure, and the consistency of `AGENTS.md` with the contract; those are met by pull request #31. The first version of the rewritten body kept the original build criteria under "Done when" while the scope section had already delegated the build to the sub-issues, so the issue's criteria and its board status contradicted each other. The body now states the umbrella's own scope, and the title was changed to "feat: model engine — contract and implementation breakdown" so the board reads honestly at a glance.

A parent's board status follows its children: it is `In Progress` while any sub-issue is open, and `Done` only when every sub-issue is done. This reverses the rule this plan originally recorded, and the reversal is the human's: the first version closed a parent when its own deliverable was merged, which left the board reading as though the model engine were built while its fourteen sub-issues sat in `Todo`. The human moved the parent back to `In Progress` and asked for the rule to be that way always. `AGENTS.md` section 9 now states it, and the single `In Progress` slot is scoped to work items so that a container mirroring its children does not occupy it. The work item that changed the rule is issue #36. The status of this plan was `Approved` in the commit that carried the change, because that criterion was not yet met at that point; it is changed to `Verified` in the follow-up commit that records the commit, the push, and the pull request, following the precedent set by the earlier entries in this repository. The merge itself is not part of the criteria and remains with the human.

Limitations, stated explicitly:

- The repository contains no Rust source, no `Cargo.toml`, and no `tests/` directory, so `cargo build`, `cargo test`, `cargo clippy`, and `cargo doc` do not apply to this change. `cargo --version` and `rustc --version` were run and establish toolchain availability only. No build, test, lint, or doc result is claimed.
- No automated markdown linter or link checker exists in the repository. Verification of a prose change is a manual read against the acceptance criteria, not a machine-checked result.
- The contract describes a model that does not exist yet. The document is a contract, and no claim is made that any of it has been implemented.
- The eleven refinements A1 to A11 are agent decisions. The human directed that they be applied and flagged rather than raised as blockers, and they are flagged in the pull request.
- The independent review was performed by a separate agent reading the diff, the documents, and the live board. It was not an independent judgment on the product decision itself, which is the human's.

### Verification of each sub-issue

Each sub-issue carries its own verification, and the section 4 rules of `AGENTS.md` apply to all of them: `cargo test` passes, unit tests sit beside the code, integration tests sit in `tests/`, and every reading and axis has a causality test, a determinism test, a golden fixture, and the edge cases named in section 4. A sub-issue that cannot show its evidence does not close.

## Risks and recovery

- **The eleven refinements are agent decisions, and two of them change what the model publishes.** A1 changes the shape of the progression headline and the per-scale aggregation, and A2 changes when a leg exists at all. Mitigation: each is one numbered row in the Decisions table, each is isolated in the implementation, and each is flagged in the pull request that introduces the contract. A2's alternative would require amending two codes in the published contract, so overriding it is a contract change and is called out as such.
- **Fourteen tracked sub-issues is more board surface than one issue.** Mitigation: one item is in flight at a time, the board's `Parent issue` and `Sub-issues progress` fields keep the tree visible, and this plan holds the dependency order so the board does not have to. The cost is the human's, and it was their choice.
- **A board item reaching `Done` closes its GitHub issue.** This project's automation does it, and it was observed on issue #5. Mitigation: the pull request for a sub-issue is opened before the item moves to `Done`, and a rejected pull request means the sub-issue is reopened by hand. The rule is recorded here and in `AGENTS.md` section 9, and the sub-issue bodies link to this plan, so the implementer reads it there.
- **`AGENTS.md` grows.** The file was 577 lines and is 591 after this change, a growth of 14 lines: 18 lines added against 4 lines replaced. The growth is reported in the pull request rather than left implicit.
- **The contract is a document and the code will drift from it.** Mitigation: `AGENTS.md` section 5 already requires that a change which invalidates a documented behaviour updates that document in the same pull request, and M14 (issue #30) checks the implementation against the contract line by line.
- **The metrics layer waits on the data engine.** The work is blocked until #9 lands. Mitigation: the dependency is stated in every affected sub-issue's body, and no metrics sub-issue duplicates the candle or timeframe type.
- **Recovery.** The change is a documentation commit on a short-lived branch merged through a pull request, so it can be reverted with a new commit on `main`. The sub-issues are removed with `removeSubIssue` and their issues closed, and the board items are removed with the same automation that added them.

## Stop conditions

Stop and request human direction if:

- A sub-issue cannot be created under #14 with the parent link the API requires, or the board cannot carry it with the required field values.
- The contract cannot be stated without a decision reserved to the human, in particular anything that would change the command vocabulary or the four axes themselves.
- A crate name is needed before the workspace exists, which section 2 forbids.
- Amending `AGENTS.md` would require changing another section, or a cross-reference would stop resolving.
- The data engine lands with a candle or timeframe shape that contradicts the assumptions in this plan, in which case the contract's input section changes before M2 (issue #18) starts.
