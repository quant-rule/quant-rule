# Refactor Workflow

- Date: 2026-11-06 (UTC)
- Status: Current; approved by the 2026-11-06 refactor workflow plan
- Scope: structure, module boundaries, tests, documentation, tooling, and
  repository layout

## Purpose

The Refactor Workflow improves the structure of the program without changing
observable behavior. It answers one question: **is the program structured so the
next correct change is cheap, local, and provable?**

It improves cohesion and modularity, reduces coupling, keeps files and modules
small enough to read, and keeps the repository understandable by a human. It is
not a rewrite, a feature, a defect fix, or a performance optimization by
assumption. A change that alters model or strategy behavior is not a refactor; it
belongs to the service or model workflow that owns that behavior.

The workflow is an engineering workflow. Its roles specialize the Software
Architect, Planner, Implementation, and Code Reviewer roles; they are not a fifth
decision authority.

## Stages

```text
Refactor Assessment -> Refactor Plan -> Execution -> Verification -> Loop Decision
```

| Stage | Owner | Input | Output | Gate |
|---|---|---|---|---|
| 1. Refactor Assessment | Structure Auditor | Fixed baseline commit, code, tests, docs, tools, layout | `docs/refactors/<YYYY-MM-DD>/<scope>-refactor.md` with findings in `Open` | Read-only; no code, test, issue, or PR changes |
| 2. Refactor Plan | Refactor Planner | Assessment and current code | Approved execution plan with finding-to-issue-to-PR-to-verification traceability | Human approves the plan; no implementation before approval |
| 3. Execution | Refactor Implementer | Approved plan and refactor issues | Branch, one behavior-neutral seam, tests, docs, PR; finding moves to `Refactored` after merge | Scope limited to the plan; no behavior change; no test weakening |
| 4. Verification | Refactor Verifier | Merged refactor, baseline manifest, named metric | Finding moves to `Verified`, or returns to remediation | Read-only; cannot verify its own assessment or implementation |
| 5. Loop Decision | Structure Auditor and human | Re-scan and before/after scorecard | Next cycle, stop, or new campaign | Budget and stop rule respected; human accepts |

### 1. Refactor Assessment

The Structure Auditor reads a fixed baseline and records structural findings
with reproducible evidence. The assessment carries a baseline table, a
before-change scorecard, and one finding per structural problem.

The assessment does not modify the system. It does not create fix issues; the
Refactor Planner does that after the plan is approved.

### 2. Refactor Plan

The Refactor Planner reproduces or code-reads every finding, marks stale or
duplicate findings, finds the structural root cause, and compares options. The
result is one execution plan at
`docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md` with a traceability table:

```text
finding -> issue/sub-issue -> PR -> verification
```

The plan names the one seam being changed, the metric expected to improve, the
behavior invariants, the diff budget, the rollback, and the stop conditions. The
human approves the plan before any implementation starts. The Refactor Planner
does not implement and does not approve.

### 3. Execution

The Refactor Implementer changes only the approved seam. One PR carries one
seam. Moves and mechanical edits are separate commits where practical. Tests and
module documentation move with the code they cover.

The implementer stops if the seam needs a logic edit, a new dependency, a schema
change, or a module-boundary decision outside the plan.

### 4. Verification

The Refactor Verifier independently checks that the merged refactor preserved
behavior and improved the named metric. It confirms the baseline and after
commands, test count, golden fixtures, representative output hashes, the
complete diff, the absence of unrelated changes, and the named metric delta. It
then moves the finding to `Verified` or returns it for remediation.

Only the Refactor Verifier moves a finding to `Verified`; only the human decides
`Deferred` or `Won't refactor`.

### 5. Loop Decision

After verification, the Structure Auditor re-scans the new HEAD and compares the
scorecard with the campaign baseline. The loop continues only within the approved
campaign and budget. A new finding outside the campaign stops the loop until a
new plan is approved. No High or Medium finding left is a valid stop.

The loop automates scanning and verification. It does not automate approval,
commits, merges, or scope expansion.

## Roles

| Role | Responsibility | Boundary |
|---|---|---|
| Structure Auditor | Scans structure, captures the baseline, records reproducible findings | Read-only; no plan, code, or verification |
| Refactor Planner | Verifies findings, deduplicates, finds the structural root cause, writes the plan and issues after approval | No implementation; no approval |
| Refactor Implementer | Executes the approved behavior-neutral seam and updates tests and docs | No scope expansion; no test weakening |
| Refactor Verifier | Independently checks equivalence, gates, and the named metric | Read-only; cannot verify its own work |

The roles map to the existing Architect, Planner, Implementation, and Code
Reviewer roles. When one agent performs more than one role, the orchestrator
discloses the sequential passes and does not claim independent review.

## Refactor Lenses

The lenses are the measurable definitions of modular, atomic, simple, low-coupling,
high-cohesion code. They guide the assessment; only the gates listed as hard are
CI failures.

| Lens | What "good" means | Evidence or metric | Enforcement |
|---|---|---|---|
| Boundary and dependency direction | Crates and modules depend one way; no cycles; no stage reaches upstream | Crate graph and module import scan | Hard gate |
| Cohesion and atomicity | One module, one responsibility; the name predicts the contents; one seam per PR | Module map, diff scope, review checklist | Assessment and review |
| Coupling and surface | Narrow `pub` surface; no sibling internals; no glob re-exports; explicit inputs | `pub`/`pub(crate)` counts, cross-module imports, fan-in and fan-out | Assessment and review |
| Size and complexity | Files at or under 1,000 lines; over 600 is a scan trigger; functions and parameter lists stay reasonable | `check_file_size.py`, structural report | Hard at 1,000; advisory at 600 |
| Testability and determinism | Pure engines; no I/O, clock, or randomness; causal and deterministic tests; test code is code | Engine-purity scan, test inventory, causality and determinism tests | Hard purity gate; test-count guard |
| Duplication and abstraction | Extract only on a third real repetition or a named invariant; no speculative abstraction | Duplicate review, shared-helper count | Assessment and review |
| Error and panic hygiene | No `unsafe`; no new `unwrap`/`expect` in library code; no panic on invalid input | `forbid(unsafe_code)`, clippy, source scan | Hard unsafe rule; ratchet for panics |
| Documentation and layout | Module docs, indexes, kind/date/slug paths, data allowlist, deterministic order, human-readable tree | `check_links`, `check_terms`, structural gate | Hard objective rules; review for content |
| Performance, measured only | No speculative optimization; benchmark before and after; identical numeric output | Existing bench harness, baseline and target, output hashes | Plan-specific gate |

Line count is a prompt to look, not a target. A cohesive module is not split
merely to hit a number.

## Equivalence Contract

A change is a refactor only if all of the following hold:

- Commands, arguments, stdout and stderr, exit codes, report and artifact
  bytes, configuration and report schemas, numeric values, and error strings are
  unchanged.
- No arithmetic reassociation, type change, rounding or clamping change, sort or
  iteration-order change, `HashMap`/`BTreeMap` swap, or `serde` field-order
  change.
- No test is removed, skipped, or weakened; the test count does not decrease;
  golden fixtures and expected values are unchanged.
- No new dependency; no unexplained `Cargo.toml` or lockfile change.
- No `unsafe`; no new `unwrap` or `expect` in library code; no new `#[allow]`
  used to silence a lint.
- A move that requires a logic edit is not a refactor. Stop and re-plan.

## Rust Move Rules

- Use `git mv` for moves and keep file history where practical.
- Follow the module style already used by the surrounding module; do not
  mass-convert `mod.rs` layouts in a move-only cycle.
- Use `pub(super)` or `pub(crate)` for child modules; widen to `pub` only when
  the item is part of the intentional crate API.
- Move tests with the code they cover. Keep shared fixtures in a support module,
  not in a catch-all helper.
- Do not add `#[path]`, `include!`, or a new glob import to make a split work.
- Do not use `#[allow]` to silence a new warning. If a lint is genuinely
  inapplicable, allow it at the narrowest scope with a comment explaining why.
- A test name describes the behavior under test, not the implementation
  (`rejects_zero_volume_candle`, not `test_1`).

## The Loop

- A **campaign** is bounded: a fixed baseline commit, a finite finding list,
  metric targets, a cycle cap, and a stop rule.
- **One seam per cycle and per PR.** A campaign may contain several cycles, but
  each cycle is planned and verified on its own.
- The default cap is three cycles per campaign, with a hard stop at five unless
  the human renews it. The approved plan states the actual cap.
- After each verified cycle, re-scan the new HEAD. Continue only within the
  approved campaign; a new finding outside it requires a new plan.
- If the named metric does not improve, close the finding. Do not keep moving
  code.
- The loop stops when no High or Medium finding remains, the budget is
  exhausted, a behavior change is required, or the human stops it.

## Finding Statuses

| Status | Meaning | Moved by |
|---|---|---|
| `Open` | Recorded, not yet reviewed for action | Structure Auditor |
| `Triaged` | Verified against the current code; impact and route agreed | Refactor Planner |
| `Planned` | Covered by an approved plan and refactor issue | Refactor Planner after human approval |
| `In progress` | Implementation is underway | Refactor Implementer |
| `Refactored` | Refactor merged; equivalence evidence still to be checked | Refactor Implementer or PR |
| `Verified` | Equivalence and metric improvement independently checked | Refactor Verifier only |
| `Deferred` | Accepted, but not scheduled | Human only |
| `Won't refactor` | Accepted residual structural debt | Human only |
| `Duplicate` | Covered by another finding | Refactor Planner |
| `Stale` | No longer reproduces or no longer applies | Refactor Planner |

`Refactored` is not `Verified`. A merge is evidence that a change landed; it is
not evidence that behavior was preserved or the structure improved.

## Green Criteria

A cycle is green when:

- every selected finding is `Verified`, or the human has recorded a dated
  `Deferred` or `Won't refactor`;
- the full gate passes before and after: `cargo fmt`, `cargo clippy
  --all-targets`, `cargo test`, `cargo doc`, the file-size gate, the structural
  gate, and every documentation and registry check;
- the test count has not decreased and no test was weakened, skipped, or removed;
- golden fixtures and their expected values are unchanged;
- representative command output and report artifacts are byte-identical;
- the named structural metric improved in the expected direction;
- the diff contains no unrelated change and no unapproved scope;
- documentation, module docs, indexes, and command references match the changed
  structure;
- the Refactor Verifier has recorded the observed commands and results.

A campaign is green when all its cycles are green or closed by the human, and no
High finding remains open.

## Artifacts

| Artifact | Path | Owner |
|---|---|---|
| Refactor assessment | `docs/refactors/<YYYY-MM-DD>/<scope>-refactor.md` | Structure Auditor |
| Execution plan | `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md` | Refactor Planner |
| Refactor issues and sub-issues | Project board | Refactor Planner after approval |
| Pull request and verification record | Repository and execution plan | Refactor Implementer |
| Repo-wide checker | `tools/check_structure.py` | Engineering |
| AI transparency log | `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md` | Orchestrator |
| Finding status and links | Refactor assessment | Planner, Implementer, Verifier |

The assessment is the status record. The project board tracks the refactor
issues and pull requests, not the assessment itself.

## Naming and Terminology

Use the names in `AGENTS.md` section 3 and the following terms consistently:

| Term | Meaning | Do not use as a synonym |
|---|---|---|
| Candle | OHLCV record | bar, row |
| Bar | Time position or index | candle |
| Reading | Indicator computation | signal, event |
| Event | Published event record | reading, signal |
| State | Discrete per-indicator code | event |
| Level | Confirmed price level | event |
| Progression | Published leg series | trend |
| Stage | Pipeline engine | layer, tier |
| Tier | Improvement model, strategy, or portfolio level | stage |
| Campaign | Bounded sequence of refactor cycles | program, sprint |
| Cycle | One assessment-plan-execute-verify pass | iteration, loop |
| Seam | One structural boundary changed in a cycle | boundary |
| Finding | One recorded structural issue, `REF-...` | defect, bug |

Rust has no classes; the same naming intent applies to structs, enums, traits,
and modules. Research names describe the domain, not the implementation. Do not
add `utils`, `helpers`, `common`, `misc`, `manager`, `data2`, `tmp`, or `foo`
modules. `shared.rs` is allowed only when it has a module doc and one cohesive
purpose. A rename is a finding; it is not a drive-by edit in a move-only cycle.

## Rules

- Evidence must be reproducible: a command or a code reference, not an
  impression.
- Do not rewrite published evidence; correct it with a dated note.
- Do not mix a refactor with a defect fix, a feature, a schema change, or a
  model or strategy change.
- Do not weaken a test, regenerate a golden fixture, or change an expected value
  to make a refactor pass.
- Do not add a dependency to make a split work.
- Do not claim a performance improvement without a before-and-after benchmark.
- Do not claim independent verification when one agent performed several roles;
  disclose the sequential passes.
- A refactor that needs a behavior change stops and routes to the Audit,
  feature, or Quant workflow that owns the behavior.

## Related Documents

- [Refactor assessment format](../refactors/README.md)
- [Refactor Workflow roles](../roles/2026-11-06/refactor-workflow-roles.md)
- [Audit Workflow](audit-workflow.md)
- [Improvement Workflow](improvement-workflow.md)
- [Architecture characteristics](../architecture/characteristics.md)
- [`AGENTS.md` section 11](../../AGENTS.md)
