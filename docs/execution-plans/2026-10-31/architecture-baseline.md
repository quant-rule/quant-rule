# Execution Plan: Architecture Baseline Documentation

- Date: 2026-10-31 (UTC)
- Work item: `architecture-baseline`
- Status: Verified (2026-10-31)
- Branch: `docs/architecture-baseline`
- Worktree: `/home/zantgo/Proyectos/quant-rule--architecture-baseline`
- Base: `main` at `c8dd519` before the branch
- Quant validation: Not applicable; docs-only, no model or strategy behavior change

## 1. Intent

Create a living system-architecture documentation set:

- a one-page, solution-agnostic product kata;
- a measurable architecture-characteristics record;
- a short ADR that adopts them as the architecture baseline.

Move the kata and characteristics out of the ADR form because they are reference
documentation, not decisions. Make `docs/architecture/` the system documentation
entry point and update the repository rules and index so the new kind is
discoverable. No runtime behavior changes.

Affected users: the human owner, validation and audit roles, and future agents.

## 2. Current state

- `main` was clean at `c8dd519`; the task worktree was created from it on branch
  `docs/architecture-baseline`. During the work, `main` advanced to `265341f`
  (PR #296, a diagnosis gap check). The branch remains based on `c8dd519`; the new
  code is unrelated to these docs, but the branch must be brought up to date and
  re-verified before a pull request.
- No `docs/architecture/` folder and no kata or characteristics document existed.
- `docs/README.md` listed documentation kinds and current ADRs; it had no
  architecture kind.
- `AGENTS.md` section 5 used only the dated `docs/<kind>/<YYYY-MM-DD>/<file>.md`
  layout; it had no living architecture kind.
- Existing ADRs are dated and decision-focused; the diagnosis ADR is a recent
  format reference.
- Observed snapshot in the worktree on 2026-10-31: 98,240 Rust lines including
  integration tests; 83,729 outside `tests/` directories; the shell is 34,896
  outside its integration tests (41,387 including them). The v0.3.0 release note
  records 1,391 tests passed, 0 failed, 6 ignored. Diagnosis v1 is implemented and
  awaiting human acceptance.

## 3. Scope

Included:

- `docs/architecture/README.md`
- `docs/architecture/kata.md`
- `docs/architecture/characteristics.md`
- `docs/adr/2026-10-31/adopt-architecture-baseline.md`
- `docs/execution-plans/2026-10-31/architecture-baseline.md`
- `docs/ai-transparency-log/2026-10-31/architecture-baseline.md`
- `AGENTS.md` section 5 and a one-line pointer in section 2
- `docs/README.md`

Excluded:

- Any code, configuration, model, strategy, execution, portfolio, or analytics
  change.
- Any command or report contract change.
- Extracting shell modules.
- New dependencies.
- A new issue or board item unless requested.
- Commit, push, pull request, merge, release, or deployment without separate
  authorization.

## 4. Assumptions and uncertainty

- The human approved the `docs/architecture/` split and the `AGENTS.md` section 5
  living-kind amendment on 2026-10-31.
- Living architecture docs are not dated work-item folders. If the project rejects
  that exception, the fallback is `docs/architecture/2026-10-31/`; that fragments
  a living document and is not recommended.
- The product kata is an adaptation of the architecture-kata practice: a one-page
  product definition, not a small-group workshop exercise. `kata.md` states this.
- The v0.3.0 evidence numbers are snapshots. The 2026-10-31 run is recorded in
  section 14; release notes remain the record for released runs.
- No issue number exists; the slug `architecture-baseline` is used.
- If separate review agents are unavailable, the Architect and Code Reviewer
  passes are performed sequentially and disclosed as non-independent.

## 5. Decisions

| Decision | Choice | Why |
|---|---|---|
| Documentation kind | `docs/architecture/` as a living folder | System docs are living, not dated work items; `architecture` is the conventional name. |
| Kata naming | `kata.md`, singular; title "The Trustworthy Backtest Kata" | One kata; "katas" is the collection (Architectural Katas). |
| ADR role | Short adoption decision only; links to the docs | An ADR records one decision; supporting material is linked, not embedded. |
| Style and quantum | A section inside `characteristics.md` | Keeps the characteristics cohesive; split later only if it grows. |
| Repository layout | Add `docs/architecture/<file>.md` to `AGENTS.md` section 5 | Makes the new kind discoverable and consistent with the repository. |

## 6. Acceptance criteria

- [x] `docs/architecture/kata.md` exists, is one page, and names no language,
      crate, database, command, or test tool.
- [x] `docs/architecture/characteristics.md` contains the method, derivation,
      three driving characteristics, enabling Testability, supporting
      characteristics, deliberate non-goals, style and quantum, trade-offs,
      watch items, and references. Every characteristic has a fitness function
      and current evidence.
- [x] `docs/architecture/README.md` is the system entry point and links to the
      kata, characteristics, ADR, and the model, configuration, and report
      contracts.
- [x] `docs/adr/2026-10-31/adopt-architecture-baseline.md` is about one page and
      has context, decision, alternatives, consequences, and links. It does not
      embed the kata or the characteristics.
- [x] `AGENTS.md` section 5 includes `docs/architecture/<file>.md`; `docs/README.md`
      includes the Architecture kind and the new ADR row.
- [x] `docs/ai-transparency-log/2026-10-31/architecture-baseline.md` exists and
      matches this plan.
- [x] All relative links resolve and `git diff --check` is clean.
- [x] `cargo test` and `cargo clippy --all-targets -- -D warnings` pass, or the
      actual result and any unrelated failure are recorded. No code changed.
- [x] No code, configuration, model, strategy, execution, portfolio, or analytics
      change exists in the diff.

## 7. Work breakdown

| Step | Action | Path | Role | Depends on |
|---|---|---|---|---|
| 0 | Preflight and worktree creation | — | Orchestrator | — |
| 1 | Write plan of record and transparency log | `docs/execution-plans/...`, `docs/ai-transparency-log/...` | Implementation | 0 |
| 2 | Write the kata | `docs/architecture/kata.md` | Implementation, Architect pass | 1 |
| 3 | Write the characteristics | `docs/architecture/characteristics.md` | Implementation, Architect pass | 2 |
| 4 | Write the architecture index | `docs/architecture/README.md` | Implementation | 2, 3 |
| 5 | Write the short ADR | `docs/adr/2026-10-31/adopt-architecture-baseline.md` | Implementation | 2, 3, 4 |
| 6 | Update repository rules and index | `AGENTS.md`, `docs/README.md` | Implementation | 4, 5 |
| 7 | Verification | — | Test Engineer, Code Reviewer pass | 6 |
| 8 | Commit, push, pull request | — | Orchestrator | 7 and authorization |

Step details:

- **Step 0.** Inspect status, branch, and worktrees. Create the task worktree from
  the latest `main` and leave the project directory on `main`.
- **Step 1.** Write this plan and the transparency log before the documentation
  files. The log records actual usage, not idealized examples.
- **Step 2.** Write the one-page kata. Add the "About this kata" note. Keep
  technology out.
- **Step 3.** Write the characteristics with the corrected crate map and the
  baseline snapshot. Label the evidence as recorded. Link to `AGENTS.md` for
  normative rules instead of duplicating them.
- **Step 4.** Write the system overview and links. Keep it a map, not a copy.
- **Step 5.** Write the short adoption decision. Enforce about one page.
- **Step 6.** Add the living architecture kind to `AGENTS.md` section 5 and the
  pointer in section 2. Add the Architecture kind, current architecture table,
  and ADR row to `docs/README.md`.
- **Step 7.** Run the checks in section 8 and record actual outputs.
- **Step 8.** Propose a commit and pull request only after explicit authorization.

## 8. Verification

| Check | Command | Expected |
|---|---|---|
| Branch and worktree | `git status --short --branch`, `git worktree list` | Correct branch; `main` untouched |
| Whitespace | `git diff --check` | Clean |
| Links | Extract links from the new docs and verify every relative path | All resolve |
| Kata purity | `rg -n -e Rust -e crate -e SQLite -e cargo -e clippy -e tokio -e serde docs/architecture/kata.md` | No technology terms |
| Formatting | `cargo fmt --all -- --check` | Clean |
| Tests | `cargo test` | Pass; record counts |
| Lints | `cargo clippy --all-targets -- -D warnings` | No warnings |
| Docs build | `cargo doc --no-deps` | No warnings, if required by the pull request gate |
| Scope | `git status --short` | Only docs and `AGENTS.md`; no code or configuration |

Limitations: the docs change exercises no runtime behavior. Test and lint runs
prove the tree still builds; they do not prove the documentation is correct. Link
checking is static and manual. Evidence numbers are snapshots; release notes are
authoritative.

## 9. Risks and recovery

| Risk | Impact | Mitigation |
|---|---|---|
| The living-kind change is rejected | Docs have no approved home | Human approved in conversation; fallback to a dated folder |
| The ADR grows into a second reference doc | Recreates the hybrid problem | Enforce about one page; link to the docs |
| The kata names technology | Breaks the solution-agnostic constraint | Purity check; move technology to the characteristics |
| Stale numbers | Misleading evidence | Label snapshots; link release notes; watch items |
| Broken links | Poor navigation | Link check in verification |
| Duplication with `AGENTS.md` | Two sources of truth | Link to normative rules; do not restate |
| An unrelated test or lint failure | Blocks the pull request | Report; do not fix unrelated code; stop and ask |
| Unauthorized commit or pull request | Process violation | Stop before Git gates; require explicit authorization |

Recovery: revert the docs commit if needed. The change is docs-only and has no
runtime rollback requirement. If the new kind is rejected, move the files under
the approved kind and update links.

## 10. Stop conditions

- Approval for `docs/architecture/` or the `AGENTS.md` section 5 amendment is
  withdrawn.
- A required link target does not exist and creating it expands scope.
- A code, configuration, model, strategy, execution, portfolio, or analytics
  change becomes necessary.
- `cargo test` or clippy fails for unrelated reasons; report, do not fix.
- Commit, push, pull request, merge, or release authorization is absent.
- A request changes the approved documentation structure materially.

## 11. AI transparency

Path: `docs/ai-transparency-log/2026-10-31/architecture-baseline.md`.

Record the date and tool, intent and boundary, verification and evidence, and
learning. Rejected options include one hybrid ADR, `katas` plural, `docs/system/`,
and dated architecture folders. Accepted options include `docs/architecture/`,
`kata.md` singular, and a short adoption ADR. Corrected items include the crate
map, test and line-count claims, the top-three inconsistency, and the diagnosis
acceptance status. Do not invent test results.

## 12. Approval and Git gates

- Plan: approved by the human on 2026-10-31 in conversation.
- Implementation: proceeds after approval, in the task worktree.
- Commits, pushes, pull requests, and merges: separate explicit authorization per
  `AGENTS.md` section 9. The plan includes them as gated steps, not pre-authorized.
- Steps 0-7 are implemented and verified. A docs commit was created on this branch;
  push and pull request remain gated.
- Board: no issue exists, so no board action. If an issue is created, move it to
  `In Progress` before edits and `Done` once the pull request exists.

## 13. Quant validation

Not applicable. This is a docs-only change: no model, strategy, execution,
portfolio, analytics, configuration, or command contract changes. The pull request
will state: `Quant validation: not applicable (docs-only; no model/strategy
behavior change).`

## 14. Verification record

Observed in the task worktree on 2026-10-31:

| Check | Observed result |
|---|---|
| `cargo fmt --all -- --check` | No output; clean |
| `cargo test -q` | Exit 0; 1,506 passed, 0 failed, 7 ignored across 47 test binaries |
| `cargo clippy --all-targets -- -D warnings` | Finished; no warnings |
| `cargo doc --no-deps` | Finished; no warnings |
| `git diff --check` | Clean |
| Kata purity check | No technology terms in `docs/architecture/kata.md` |
| Relative-link check | All relative targets in the new docs exist |
| Scope check | Only `AGENTS.md`, `docs/README.md`, and the new documentation files changed |

The test count is a snapshot. The v0.3.0 release note remains the record for that
released run. The test and lint runs are on `c8dd519`; `main` advanced to `265341f`
with unrelated diagnosis code during the work, so the branch must be updated and
re-verified before a pull request. No code, configuration, command, model, strategy,
execution, portfolio, or analytics behavior changed on this branch.
