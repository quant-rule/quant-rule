# Completion Plan: The Model Engine (M2–M14)

- **Status:** Approved
- **Date:** 2026-10-24 (UTC)
- **Approved:** 2026-10-24 by the human, with D1 model-only and D3 milestone cadence
- **Container issue:** [#14](https://github.com/quant-rule/quant-rule/issues/14)
- **Contract:** [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- **Umbrella plan:** [`docs/execution-plans/2026-10-21/14-model-engine.md`](../../execution-plans/2026-10-21/14-model-engine.md)
- **Baseline:** `main` at `2a845b9`; M1 ([#17](https://github.com/quant-rule/quant-rule/issues/17)) merged; data engine ([#9](https://github.com/quant-rule/quant-rule/issues/9)) complete
- **Scope:** #14 and the 13 open model sub-issues #18–#30
- **Out of scope:** #10–#13 (execution, strategy, portfolio, analytics)

## 1. Intent

Complete the model engine from its verified numeric kernel (M1) through the full market-state output and its shell surface, and close every model-engine issue through its own reviewed pull request.

The intended outcome is a model layer that publishes the contract's complete numeric market state per timeframe and across the timeframe ladder, plus the three event families, with no labels, no cross-axis averaging, no look-ahead, and deterministic output. The shell exposes that state through documented commands whose output is byte-identical interactively and non-interactively.

Affected users: the human developer, who owns the contract and the command vocabulary; and the strategy engine ([#11](https://github.com/quant-rule/quant-rule/issues/11)), which consumes the market state after this program.

## 2. Current state

Observed on 2026-10-24 UTC:

| Fact | Evidence |
|---|---|
| `main` is clean and equal to `origin/main` at `2a845b9` | `git status`, `git log` |
| M1 kernel is merged; #17 is Done/Closed | PR #58; `crates/quantrule-model/src/kernel.rs`, 740 lines |
| The model crate has only `kernel.rs` and `lib.rs` | `crates/quantrule-model/src/` |
| Data engine #9 is Done/Closed; children #46–#50 and source #55 are Closed | GitHub issues and board |
| Data ladder is M1, M5, M15, H1; H4 and D1 are explicitly unsupported | `crates/quantrule-data/src/timeframe.rs`; #50 plan |
| 18 open issues: #14; #18–#30; #10–#13 | `gh issue list` |
| Board: #14 In Progress; #18–#30 Todo; #10–#13 Todo | project board |
| Baseline checks pass | `fmt`, `clippy`, 265 tests, `doc` |

## 3. Scope

Included:

- M2–M14 ([#18](https://github.com/quant-rule/quant-rule/issues/18)–[#30](https://github.com/quant-rule/quant-rule/issues/30)), one work item at a time.
- Per-item plan, branch, worktree, transparency log, tests, PR, and board transition.
- Contract and README updates required by the implementation, in the same PR as the behavior.
- A final contract audit and the closure of #14 and #18–#30.
- The program plan itself, recorded in the repository on approval.

Excluded:

- #10–#13. They are downstream engines, not model-engine work. #11 depends on #14 and is unblocked by this program, but it is a separate effort.
- New data-engine capabilities: H4/D1, real-time data, new sources, or a different ladder. A need for any of these stops the program.
- Semantic changes to the four axes, published fields, event model, aggregation rules, or code values.
- New dependencies. The kernel and the model use the standard library; any dependency need stops the program.
- Command vocabulary changes without the human's decision before M13.
- Parallel work on more than one leaf issue, unless the human explicitly authorizes it.

## 4. Assumptions and uncertainty

- The contract is authoritative. Ambiguity is escalated, not resolved by invention.
- The M1 kernel API is stable. Only defect fixes required by a downstream item are allowed; each is tested and called out in that item's PR.
- The data engine supplies M1/M5/M15/H1. The model aggregates over that configured ladder; H4/D1 are not required.
- One leaf issue is In Progress at a time; a container does not occupy the slot.
- The board automation closes an issue when its item reaches Done, so the PR must exist first.
- The human retains merge authorization. The cadence is decision D3.
- M2's readings are the foundation: an error there propagates to every axis and event.
- The highest-risk items are M6 (leg lifecycle and target policy) and M11–M12 (aggregation and assembled state).
- M13's command names are reserved to the human; implementation waits for that decision.

## 5. Decisions

| ID | Decision | Options | Recommendation | Status |
|---|---|---|---|---|
| D1 | Program scope | Model issues only; model plus downstream #10–#13 | Model only. #10–#13 are a second program after #14. | Approved |
| D2 | Execution order | Numeric M2→M14; a different topological order | Numeric order, which is topological and keeps one item in flight. | Approved |
| D3 | Merge authorization cadence | Human merges every PR; standing authorization to merge after checks with milestone checkpoints; one batch at the end | Review M2 per PR, then a standing authorization per milestone if the human is satisfied. The agent never merges without explicit authorization. | Approved |
| D4 | M13 command names | `market-state`, `axis`, `events`, `legs`; other names | Human decides before M13 starts; the plan does not harden a name. | Open |
| D5 | Repository artifact for this program | New dated completion plan; addendum to the existing #14 plan | New `docs/execution-plans/2026-10-24/14-model-engine-completion.md`, linked from the #14 umbrella plan. | Approved |
| D6 | Kernel changes after M1 | Freeze; defect fixes only; allow API evolution | Defect fixes only, with a test and a note in the item's PR. | Approved |
| D7 | Data ladder | M1/M5/M15/H1; add H4/D1 | Use the existing four-timeframe ladder. H4/D1 require a new data-engine issue. | Approved |

## 6. Acceptance criteria

The program is complete when all of the following are observably true:

- #14 and #18–#30 are Done on the board and Closed on GitHub; no model issue remains open.
- The model crate implements readings, the four axes, the three event families, context, aggregation, and the assembled market state, with the module boundaries named in the umbrella plan.
- Every field of the contract's §9 output table exists with its stated type and range; missing measurements are null with the correct reason code; no model-layer value is a string.
- No axis averages another axis; ordinal codes are counted, never averaged; events never alter an axis value.
- Every late-arriving measurement respects `available_at`; no value reaches an earlier bar.
- The shell exposes the market state through commands whose output is asserted, whose help is discoverable, and whose interactive and non-interactive bytes are identical.
- The command reference and README are current; no stale model documentation remains.
- M14's property, determinism, anti-double-counting, A1–A11, and benchmark evidence is reported.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo doc --no-deps` are clean.
- No open model PR, task worktree, or task branch remains; each merged worktree and branch is cleaned up.
- #10–#13 remain open unless a separate program is approved.

## 7. Work breakdown

### 7.1 Execution order

| Step | Phase | Issue | Deliverable | Depends on | Exit gate |
|---|---|---|---|---|---|
| 1 | A — Inputs | #18 (M2) | Indicator readings: EMA 20/50/200, RSI 14, ATR 14, RVOL 20, confirmed pivots at both scales, role registry | #17 | Golden fixture per reading; causality and determinism tests; output length equals input; no pivot before `available_at`; empty/single/short edge cases |
| 2 | B — Axes | #19 (M3) | Direction axis: normalized EMA spreads, score, compact code, percentile, readiness, reason, counters | #18 | Null on missing EMA/ATR; valid zero stays neutral; percentile null under 50; no EMA-distance read; causality/determinism/golden |
| 3 | B — Axes | #20 (M4) | Momentum axis: force, acceleration, intensification, diagnostics | #18 | All values in [-1,+1]; three-bar window; zero tie; null on missing RSI; causality/determinism/golden |
| 4 | B — Axes | #21 (M5) | Structure axis: swing relations, score/code, EMA features, confirmed-level registry | #18 | Score null on incomplete pair; relation codes retained; equal highs/lows give 0; EMA features never enter the score; causality/determinism/golden |
| 5 | B — Axes | #22 (M6) | Progression axis: major/minor legs, target policy, versioning, progress, lifecycle, stage | #18, #21 | Stage null until origin confirmed and target stable; terminal lifecycle forces stage 5; revision increments version without price change; unclamped progress; every code reachable |
| 6 | C — Events | #23 (M7) | Level events and the shared event record | #18, #21 | Break/cross/test precedence; one event per level per bar; no event before `available_at`; bounded A6 references; causality/determinism/golden |
| 7 | C — Events | #24 (M8) | Divergence events | #18, #21 | No event before the newer pivot confirms; four-subtype matrix; no inference when a pivot or RSI is unavailable; no axis mutation |
| 8 | C — Events | #25 (M9) | Signal events | #18, #19, #20, #21 | One underlying occurrence never duplicates; price/EMA crossing stays a Level event; signal adds no vote |
| 9 | C — Context | #26 (M10) | Volatility and participation context | #18 | Pure function of the two ratios; null when either is unavailable; raw ratios retained; no axis reads context |
| 10 | D — Assembly | #27 (M11) | Cross-timeframe aggregation | #19–#26 | No ordinal code averaged; every aggregate names its timeframes; A6/A7 level clustering; order independence; five-bar event counts |
| 11 | D — Assembly | #28 (M12) | Consensus diagnostic and full market-state output | #19–#22, #27 | Every §9 field present; missing axis not neutral; no field averages two axes; no strings; golden market-state fixture |
| 12 | E — Surface | #29 (M13) | Shell commands, label mapping, command reference, README | #28, #9 | Each command asserted; interactive/non-interactive byte-identical; every command in help; no words in the model layer |
| 13 | F — Hardening | #30 (M14) | Property, determinism, anti-double-counting, A1–A11 audit, benchmarks | #17–#29 | All hardening tests pass; benchmark reported; deviations fixed or contract corrected with approval; full checks clean |
| 14 | F — Closure | #14 | Container close-out | All children | #14 Done/Closed by the workflow; board audit; no open model PR/worktree/branch |

### 7.2 Per-item protocol

Every step follows the same gate:

1. Confirm no other leaf issue is In Progress. Move the issue to In Progress and set its Start date to the UTC date.
2. Create or reuse the worktree `../quant-rule--<issue>-<slug>` and branch `<type>/<issue>-<slug>` from the latest `origin/main`.
3. Write or refresh the item plan under `docs/execution-plans/<date>/<issue>-<slug>.md`, and the matching transparency log.
4. Implement only the issue's scope. Update the contract and README only where the behavior or its documentation requires it.
5. Add the tests the issue requires: unit, golden, causality, determinism, and edge cases. Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo doc --no-deps`.
6. Run a read-only review against the contract, the issue's Done-when list, and the diff. Correct within scope; record the findings.
7. Commit, push, and open a PR closing the issue. Record the actual command results in the PR and the transparency log.
8. Move the issue to Done only after the PR is open. Merge under decision D3, then remove the worktree and delete the local and remote branch after verifying no work is lost.
9. At each milestone, run the full workspace checks, audit the board and branches, and stop for the human checkpoint.

### 7.3 Milestones

| Milestone | Contains | Human checkpoint |
|---|---|---|
| A — Foundation | M2 (#18) | Confirm readings and fixtures before any axis is built |
| B — Axes | M3–M6 (#19–#22) | Confirm all four axes publish the contract's fields with no cross-axis reads |
| C — Events and context | M7–M10 (#23–#26) | Confirm availability, precedence, and no-double-count behavior |
| D — Assembly | M11–M12 (#27–#28) | Confirm the full numeric market state and the golden fixture |
| E — Surface | M13 (#29) | Confirm command names, output, help, and documentation |
| F — Hardening | M14 (#30), #14 | Confirm the audit, benchmarks, and final board closure |

## 8. Verification

### 8.1 Per-item verification

Each item is verified against its own Done-when list and the model-layer rules in `AGENTS.md` section 4:

- **Readings:** one golden fixture per reading; a causality test that appends, changes, or removes later bars; a determinism test; empty, single-candle, and short-history cases; output length equals input length; no pivot before `pivot_available_at`.
- **Axes:** a causality test, a determinism test, and a golden fixture; null-versus-zero; percentile null under 50 samples; readiness and reason codes; state counters; a test proving no cross-axis reads.
- **Events:** no event before its availability bar; one underlying occurrence never emitted twice; precedence respected; no axis value changes; golden and determinism tests.
- **Context:** a pure function of the two ratios; null when unavailable; no axis reads it; golden and determinism tests.
- **Aggregation:** no ordinal averaging; source timeframes named; A6/A7 level clustering; five-bar event counts; order independence; multi-timeframe golden fixtures.
- **State:** every §9 field, type, and range; a missing axis is not neutral; no field averages two axes; no strings; golden fixture.
- **Shell:** each command's printed output asserted; interactive and non-interactive output byte-identical; help listing; command reference and README updated; no labels in the model.
- **Hardening:** property tests, determinism across repeated runs and reordered collections, anti-double-counting regressions, the A1–A11 decision audit, and benchmarks for long-history paths.

### 8.2 Program verification

- `cargo fmt --check`: exit 0.
- `cargo clippy --all-targets -- -D warnings`: exit 0.
- `cargo test`: all unit, integration, and doc tests pass.
- `cargo doc --no-deps`: builds without warnings.
- Contract audit: every §9 field is produced; every §7 event rule is exercised; every §10 aggregate names its sources; §12 determinism rules hold.
- Board audit: #14 and #18–#30 Done/Closed; no model issue open.
- Git audit: no open model PR, no task worktree, no task branch; `main` clean and equal to `origin/main`.
- Documentation audit: each item has a plan and a transparency log; the README and command reference match the shipped commands; no stale model behavior remains.

Evidence is the actual command output recorded in the PR and the transparency log. A claim is not made beyond what the test exercises.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| A contract ambiguity changes a published meaning | Stop; present the options and consequences to the human; do not invent a rule |
| A foundational error in M2 propagates to every later item | Golden fixtures per reading and causality tests; do not start M3 until milestone A is accepted |
| M6's leg lifecycle and target policy are the largest source of edge cases | Extra review, explicit transition tests, and a golden fixture covering every lifecycle and stage code |
| M11–M12 integration hides a scale or ordering defect | Multi-timeframe golden fixture; order-independence tests; compare the assembled state field by field with §9 |
| M13 hardens an unapproved command name | Human decision gate D4 before implementation |
| Determinism drifts through unordered collections or accumulation order | Ordered collections, fixed accumulation order, no `HashMap`/`HashSet` in the signal path, determinism tests |
| A long-history path regresses performance | M14 benchmarks; report the result; fix within scope |
| Board automation closes an issue before its PR exists | Move to Done only after the PR is open; if a PR is rejected, reopen the issue |
| A merge proceeds without authorization | The program stops at D3's gate; no merge without explicit human authorization |
| Scope creeps into #10–#13 or new data sources | The scope table is explicit; a new need becomes a new issue, not an expansion |
| The data ladder cannot supply a required timeframe | Stop; H4/D1 or a new ladder is a data-engine issue, not a model change |
| Recovery from a bad merge | Revert with a new commit; the item branch is preserved until verification; no migrations or persistent state are involved |

## 10. Stop conditions

Stop and request human direction if:

- The contract is ambiguous or contradicts itself, or an implementation choice would change a published meaning.
- A new dependency is needed.
- The data engine must change: H4/D1, a different ladder, new fields, or a new source.
- A §9 field cannot be computed from the existing readings without a semantic contract change.
- The M13 command names are not decided before M13 starts.
- The toolchain cannot run `fmt`, `clippy`, `test`, or `doc`.
- The work would exceed the issue's scope or require a new issue.
- Merge authorization is withheld or a PR is rejected.
- M14 finds a semantic deviation that requires human approval to resolve.
- A security or privacy concern appears in the shell's data access. The model layer itself performs no I/O.

## 11. Approval gates and first action

Human approval is required for:

1. This plan and its scope (D1).
2. The merge cadence (D3).
3. The M13 command names (D4), before M13 starts.
4. Any semantic contract change, new dependency, or data-engine change.
5. Each merge under the chosen cadence.

After approval:

1. Record this program plan in the repository at `docs/execution-plans/2026-10-24/14-model-engine-completion.md` and link it from the #14 umbrella plan.
2. Move #18 to In Progress with its UTC start date.
3. Create the #18 worktree and branch.
4. Begin M2, the indicator readings layer.
