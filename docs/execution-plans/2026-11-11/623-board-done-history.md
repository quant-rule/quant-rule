# Protect Board Done History and Strengthen the Execution-Plan Checklist

- Status: In Progress
- Date: 2026-11-11 (UTC)
- Issue: #623
- Claim type: Behaviour Claim
- Owner: Planner / Implementer / Reviewer
- Workflow: [Execution Plan](../../workflows/execution-plan.md)

## Intent

Make the project board append-only history so a `Done` item is never deleted,
archived, removed, bulk-cleaned, reopened, or moved back to an earlier status.
Make the execution-plan verification table a complete reviewer checklist: one
row per work-breakdown phase, acceptance criterion, and closure step, including
the issue, pull request, board transition, changelog, AI log, and cleanup.

## Current state

- `AGENTS.md` section 9 says a `Done` item that returns to active work clears
  `Closed date` and moves to `In Progress`. It has no rule against deleting,
  archiving, removing, or bulk-cleaning a board item.
- `.github/scripts/project-sync.sh` can move an item out of `Done`. An open
  `Done` leaf falls through to `Todo`, and a `Done` container is recomputed to
  `In Progress` when a child reopens.
- `docs/workflows/execution-plan.md` section 5 requires one row per plan item and
  acceptance criterion, but not a row for each work-breakdown phase or closure
  step. `tools/check_execution_plans.py` checks the header, columns, and at least
  one data row; it does not check unique IDs, core cell content, or unresolved
  rows on a `Verified` plan.
- `docs/workflows/templates/execution-plan.md` has a work breakdown of
  `Phase | Work | Output | Gate`; `AGENTS.md` section 11 requires dependencies,
  roles, and affected paths.
- `.github/ISSUE_TEMPLATE/task.md` lists historical `Policy change` and omits
  current `Simulation research` and `Research swarm`.
- `tools/check_workflows.py`, `tools/check_execution_plans.py`,
  `tools/check_claims.py`, `tools/check_links.py`, and `tools/test_check_*` pass
  on `main` at `6fdc971`.

## Scope

Included:

- Add the board permanence rule to `AGENTS.md`, recordkeeping, the execution-plan
  workflow, and the workflow index.
- Make `Done` terminal in the lifecycle and in `project-sync.sh`.
- Add the delivery sequence and verification-table coverage rules.
- Update the plan template, plan checker, issue template, docs index, and CI.
- Add a board permanence checker and tests.
- Record the changelog and AI transparency log.

Excluded:

- Model, strategy, execution, portfolio, and analytics behavior.
- Rust source changes.
- Board schema changes, view changes, or field renames.
- Deleting, archiving, removing, or bulk-cleaning any existing board item.

## Assumptions and uncertainty

- The board's built-in automation adds an issue or pull request to the project;
  `project-sync.sh` backstops it with `ensure_item`.
- The project board's `Done` option and views are configured in GitHub and are
  not changed by this work.
- A `Done` item may still be open on GitHub; the board status and the issue state
  remain independent.
- `gh` has the `project` scope and can set the issue's board fields.
- The shell test can source `project-sync.sh` without network access once the
  field loading and status decision are separated into functions.

## Decisions

| ID | Decision | Consequence | Approval |
|---|---|---|---|
| D1 | The board is append-only history | No delete, archive, remove, or bulk-clean; `Done` items stay | Human, 2026-11-11 |
| D2 | `Done` is terminal | Follow-up work is a new linked issue; `Closed date` is not cleared | Human, 2026-11-11 |
| D3 | Cleanup means worktree and branches only | `worktree_cleanup.py` never touches the board | Human, 2026-11-11 |
| D4 | Verification table covers phases, criteria, and closure | The reviewer has a row for every deliverable and every closure step | Human, 2026-11-11 |
| D5 | Extend the execution-plan workflow, do not add a second workflow | One source of truth for plan and delivery | Human, 2026-11-11 |
| D6 | Keep the ten-column table | No rewrite of existing plan tables; `Plan item` names the artifact | Human, 2026-11-11 |

## Acceptance criteria

- `AGENTS.md` states the board permanence rule in absolute terms and makes `Done`
  terminal.
- `project-sync.sh` never moves a `Done` item out of `Done`; a shell test covers a
  Done leaf, a Done container, a closed leaf, an active leaf, and an open PR.
- The execution-plan workflow states the delivery sequence and requires a row for
  every work-breakdown phase, acceptance criterion, and closure step.
- The template names roles, dependencies, and affected paths, and the verification
  table is last.
- `check_execution_plans.py` rejects duplicate IDs, empty core cells, and a
  `Verified` plan with an unresolved row.
- The board permanence checker rejects a missing rule or a forbidden board
  mutation, and CI runs it.
- The issue template workflow list matches the current workflow index.
- The changelog and AI transparency log are updated.
- The issue, worktree, branch, PR, review gate, merge, and cleanup are recorded;
  the `Done` board item remains on the board.

## Work breakdown

| Phase | Work | Role | Depends on | Affected paths | Output | Gate |
|---|---|---|---|---|---|---|
| 1 | Board permanence and lifecycle rule | Implementer | Plan approval | `AGENTS.md`, `docs/workflows/recordkeeping.md`, `docs/workflows/execution-plan.md`, `docs/workflows/README.md` | Normative append-only rule | Docs review |
| 2 | Delivery sequence and table coverage | Implementer | Phase 1 | `docs/workflows/execution-plan.md`, `docs/workflows/templates/execution-plan.md` | Ordered delivery and checklist template | Docs review |
| 3 | Plan checker hardening | Implementer | Phase 2 | `tools/check_execution_plans.py`, `tools/test_check_execution_plans.py` | Checker and tests | `python3 tools/check_execution_plans.py` |
| 4 | Board automation Done stickiness | Implementer | Phase 1 | `.github/scripts/project-sync.sh`, `tools/test_project_sync.sh` | Done is terminal in automation | `bash tools/test_project_sync.sh` |
| 5 | Board permanence guard | Implementer | Phase 1 | `tools/check_board_permanence.py`, `tools/test_check_board_permanence.py`, `.github/workflows/ci.yml` | Guard and CI step | Checker test |
| 6 | Index and template drift | Implementer | Phases 2-5 | `docs/README.md`, `.github/ISSUE_TEMPLATE/task.md` | Index and workflow list agree | `check_workflows.py`, `check_links.py` |
| 7 | Records | Implementer | Phases 1-6 | `CHANGELOG.md`, `docs/ai-transparency-log/2026-11-11/623-board-done-history.md` | Release entry and AI log | `check_changelog.py` |
| 8 | Delivery | Implementer / Reviewer | Phase 7 | Issue, branch, PR, board | PR `In Review` | Human review |

## Verification

| Check | Command | Expected |
|---|---|---|
| Workflow docs | `python3 tools/check_workflows.py` | pass |
| Execution plans | `python3 tools/check_execution_plans.py` | pass |
| Execution-plan tests | `python3 -m unittest discover -s tools -p 'test_check_execution_plans.py'` | pass |
| Board permanence | `python3 tools/check_board_permanence.py` | pass |
| Board permanence tests | `python3 -m unittest discover -s tools -p 'test_check_board_permanence.py'` | pass |
| Project sync syntax | `bash -n .github/scripts/project-sync.sh` | pass |
| Project sync behavior | `bash tools/test_project_sync.sh` | pass |
| Claim types | `python3 tools/check_claims.py` | pass |
| Changelog | `python3 tools/check_changelog.py` | pass |
| Links | `python3 tools/check_links.py` | no broken links |
| Terms | `python3 tools/check_terms.py` | pass |
| Legacy | `python3 tools/check_legacy.py` | pass |
| Rust format | `cargo fmt --all -- --check` | clean |
| Rust lints | `cargo clippy --all-targets -- -D warnings` | clean |
| Rust tests | `cargo test` | pass |

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Phase 1: Board permanence rule | `AGENTS.md` prohibits delete, archive, remove, and bulk-clean in absolute terms | `python3 tools/check_board_permanence.py` | pass | board permanence ok: append-only rule present; no delete or archive mutation | Pass | Agent (sequential) | 2026-11-11 | Sequential pass; not independent |
| V2 | Phase 1: Done lifecycle | `Done` is terminal; cleanup never touches the board | `rg "Done.*terminal\|append-only" AGENTS.md docs/workflows/recordkeeping.md` | both rules present | `AGENTS.md` lines 408 and 476; `recordkeeping.md` line 91 | Pass | Agent (sequential) | 2026-11-11 | Sequential pass; not independent |
| V3 | Phase 1: Recordkeeping | Append-only rules cover board items | `rg "board" docs/workflows/recordkeeping.md` | rule present | `recordkeeping.md` line 91: append-only rule covers board items | Pass | Agent (sequential) | 2026-11-11 | Sequential pass; not independent |
| V4 | Phase 2: Delivery sequence | Issue to Done sequence names owner and record | `rg "Delivery sequence" docs/workflows/execution-plan.md` | section present | `execution-plan.md` line 107: Delivery sequence | Pass | Agent (sequential) | 2026-11-11 | Sequential pass; not independent |
| V5 | Phase 2: Verification coverage | Row per work-breakdown phase, acceptance criterion, and closure step | `python3 tools/check_execution_plans.py` | pass | execution plans ok: current plans carry a claim type and verification table | Pass | Agent (sequential) | 2026-11-11 | Sequential pass; not independent |
| V6 | Phase 2: Template | Work breakdown has role, dependencies, paths; table is last | `rg "Depends on\|Affected paths" docs/workflows/templates/execution-plan.md` | columns present | template line 34: `Phase \| Work \| Role \| Depends on \| Affected paths \| Output \| Gate` | Pass | Agent (sequential) | 2026-11-11 | Sequential pass; not independent |
| V7 | Phase 3: Plan checker | Duplicate IDs, empty core cells, and Verified unresolved rows fail | `python3 -m unittest discover -s tools -p 'test_check_execution_plans.py'` | pass | 8 tests pass | Pass | Agent (sequential) | 2026-11-11 | Sequential pass; not independent |
| V8 | Phase 4: Done stickiness | `project-sync.sh` keeps Done leaves and containers Done | `bash tools/test_project_sync.sh` | pass | 8 cases pass; project sync ok | Pass | Agent (sequential) | 2026-11-11 | Sequential pass; not independent |
| V9 | Phase 5: Board guard | Forbidden board mutations and missing rule fail | `python3 -m unittest discover -s tools -p 'test_check_board_permanence.py'` | pass | 4 tests pass | Pass | Agent (sequential) | 2026-11-11 | Sequential pass; not independent |
| V10 | Phase 6: Issue template | Workflow list matches the current index | `rg "Simulation research\|Policy change" .github/ISSUE_TEMPLATE/task.md` | simulation present; policy absent | Simulation research present at line 25; Policy change absent | Pass | Agent (sequential) | 2026-11-11 | Sequential pass; not independent |
| V11 | Phase 6: Docs index | Board guard is listed in the tooling table | `rg "check_board_permanence" docs/README.md` | present | `docs/README.md` line 233: board permanence checker | Pass | Agent (sequential) | 2026-11-11 | Sequential pass; not independent |
| V12 | Phase 7: Records | Changelog and AI log are updated | `python3 tools/check_changelog.py`; `test -f docs/ai-transparency-log/2026-11-11/623-board-done-history.md` | pass | changelog ok: 5 releases, 90 entries; AI log present | Pass | Agent (sequential) | 2026-11-11 | Sequential pass; not independent |
| V13 | Phase 8: Delivery | Issue In Progress; PR open; issue In Review; Done item intact after merge | GitHub Project and PR | board and PR states match | | Pending | | | PR open; human review and merge pending |
| V14 | Phase 8: Regression guard | The board permanence guard fails before the rule and passes after | `python3 tools/check_board_permanence.py` against the pre-change tree | fail before; pass after | Pre-change tree: `AGENTS.md is missing the board permanence rule`; post-change: pass | Pass | Agent (sequential) | 2026-11-11 | Sequential pass; not independent |

## Risks and recovery

| Risk | Control | Recovery |
|---|---|---|
| The script refactor changes status behavior | Sourceable functions, shell test, idempotent script | Revert the script commit; the docs rule still stands |
| The checker rejects a current plan | Run the checker on all current plans before the PR | Narrow the rule or fix the plan; do not weaken the test |
| A board item is removed by a manual command | Append-only rule, guard, and CI check | Restore the item if possible; record the incident; do not repeat |
| `gh` lacks project permission | Check `gh auth status` before the PR | Ask the human to set the field in the browser |
| The plan table becomes theatre | Reviewer fills every row and the completion report links the plan | Return failed rows; do not mark Verified |

## Stop conditions

- A change would delete, archive, remove, or bulk-clean a board item.
- A `Done` item would be reopened or moved back to an earlier status.
- The plan needs a new claim type, scope, dependency, or architecture decision.
- A verification row fails and the fix changes the approved scope.
- The issue is `In Review` without green checks.
- A worktree or branch cannot be cleaned safely.

## Related documents

- [`AGENTS.md`](../../../AGENTS.md)
- [`docs/workflows/execution-plan.md`](../../workflows/execution-plan.md)
- [`docs/workflows/recordkeeping.md`](../../workflows/recordkeeping.md)
- [`docs/workflows/README.md`](../../workflows/README.md)
- [`docs/workflows/templates/execution-plan.md`](../../workflows/templates/execution-plan.md)
- [`tools/check_execution_plans.py`](../../../tools/check_execution_plans.py)
