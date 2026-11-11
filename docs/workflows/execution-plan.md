# Execution Plan Workflow

- Date: 2026-11-11 (UTC)
- Status: Current
- Scope: execution plans for substantial multi-step work
- Owner: Planner (plan), Implementer (execution), Reviewer (verification), Human (approval)
- Contract: [`AGENTS.md`](../../AGENTS.md) section 11
- Claim type: Behaviour Claim
- Related: [`requirements-design.md`](requirements-design.md), [`recordkeeping.md`](recordkeeping.md), [`release-operations.md`](release-operations.md)

## 1. Purpose

An execution plan turns an approved change into an ordered, reviewable sequence
of work. It answers three questions:

- What is being changed, and why?
- What is deliberately excluded?
- How will a reviewer know the plan was actually implemented?

The plan is not a proposal to be improvised from. It is the plan of record.

## 2. When to use

Use it when the work:

- spans multiple components or several dependent steps;
- changes an architecture, contract, schema, or workflow;
- has security, privacy, migration, or recovery implications;
- needs more than one issue, sub-issue, or pull request;
- contains material uncertainty about the solution.

Do not use it for a small fix that fits a single issue and a single pull request.

## 3. Stages

```text
Proposed -> Approved -> In Progress -> In Review -> Verified
                                      -> Superseded
```

| Stage | Question | Owner | Gate |
|---|---|---|---|
| Proposed | Is the scope and evidence clear? | Planner | Human approval |
| Approved | Is the plan authorized? | Human | Plan status `Approved` |
| In Progress | Are the issues being executed? | Implementer | First issue `In Progress` |
| In Review | Can the reviewer check the result? | Reviewer | Verification table populated |
| Verified | Does every row pass? | Reviewer + Human | No unresolved `Fail` or `Blocked` |
| Superseded | Is the plan replaced? | Human | A newer plan links forward |

## 4. Required contents

Every plan includes:

- **Intent:** problem, outcome, affected users.
- **Current state:** files, behavior, constraints.
- **Scope:** included and excluded.
- **Assumptions and uncertainty:** what could invalidate the plan.
- **Decisions:** options, consequences, trade-offs, approval state.
- **Acceptance criteria:** observable conditions.
- **Work breakdown:** ordered steps, dependencies, roles, paths.
- **Verification:** commands, expected evidence, limitations.
- **Verification table:** the reviewer checklist defined below.
- **Risks and recovery:** failure modes, rollback, recovery.
- **Stop conditions:** conflicts, new decisions, missing permissions.

## 5. Verification table

The plan ends with a table with many rows and columns. It is not a summary; it is
the reviewer's checklist.

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
|  |  |  |  |  |  |  |  |  |  |

Rules:

- One row per work-breakdown phase, per acceptance criterion, and per closure
  step. A closure step includes the issue or sub-issue, pull request, board
  transition, changelog, AI transparency record, and worktree/branch cleanup.
- `Plan item` names the specific file, command, issue, or artifact, and starts
  with the work-breakdown phase ID.
- At approval, every row names the plan item, acceptance criterion, evidence or
  command, and expected result. `Observed`, `Status`, `Reviewer`, and `Date` are
  filled during execution and review.
- The table is presented with the plan at approval and again with observed
  results in the completion report.
- `Status` is `Pass`, `Fail`, `Blocked`, `Pending`, or `Accepted risk`.
- The implementer fills `Observed`; the reviewer fills `Status`, `Reviewer`, and
  `Date`.
- A plan is not `Verified` while any row is `Pending`, `Fail`, or `Blocked`.
- An `Accepted risk` requires the human to record the residual risk and the next
  action.
- Failed or blocked rows stay visible; they are not deleted.

## 6. Roles and boundaries

| Role | Responsibility | Boundary |
|---|---|---|
| Planner | Scope, order, acceptance criteria, verification table | No implementation; no approval |
| Implementer | Execute the approved scope and fill observed evidence | No scope expansion; no test weakening |
| Reviewer | Check every row, the diff, and the evidence | Read-only; no self-verification |
| Human | Approve the plan and accept residual risk | Sole final authority |

The implementer cannot mark the plan `Verified` for its own work. If separate
agents are unavailable, the sequential reviewer pass is disclosed.

## 7. Delivery sequence

An approved plan is delivered in this order:

| Step | Action | Owner | Record |
|---|---|---|---|
| 1 | Open the issue with one claim type | Planner | Issue |
| 2 | Create the worktree and branch | Implementer | `git worktree list` |
| 3 | Commit the approved plan | Planner | Plan file |
| 4 | Implement the approved scope | Implementer | Code, docs, tests |
| 5 | Open the pull request with the required fields | Implementer | Pull request |
| 6 | Move the issue to `In Review` when checks are green | Implementer | Board |
| 7 | Review every verification row and the diff | Reviewer | Plan table |
| 8 | Merge after human acceptance | Human / author | Merge commit |
| 9 | Verify acceptance criteria and clean up the worktree and branches | Implementer | Cleanup record |
| 10 | Move the issue to `Done`; the board item stays | Implementer | Board |

The normative worktree, branch, pull request, review, merge, and cleanup rules
are in [`AGENTS.md`](../../AGENTS.md) sections 7 and 9. This sequence orders them;
it does not restate them.

## 8. Board lifecycle

The plan's issues follow the four-stage board lifecycle in `AGENTS.md` section 9:

- `Todo` with a claim type;
- `In Progress` with a start date;
- `In Review` when the pull request is open and the human gate is requested;
- `Done` after acceptance, merge, verification, and cleanup, with a close date.
  The board item stays `Done` permanently.

A plan is not `Verified` while any of its issues is not `Done` unless the human
records an exception.

## 9. Human gates

| Gate | Human action |
|---|---|
| Plan approval | Approve scope, order, and verification table |
| Review | Accept, request changes, or reject |
| Residual risk | Accept a recorded risk or stop |
| Supersede | Approve a replacement plan |

## 10. Green criteria

A plan is green when:

- every required section is present;
- the verification table has a row for every work-breakdown phase, acceptance
  criterion, and closure step;
- every row is `Pass` or a human-approved `Accepted risk`;
- the issues are `Done` and their pull requests are merged;
- the worktrees and branches are removed, and the `Done` board items remain on
  the board;
- the changelog and AI transparency record are updated.

## 11. Artifacts and records

| Artifact | Path |
|---|---|
| Plan | `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md` |
| Issues | GitHub Project |
| Pull requests | GitHub |
| AI log | `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md` |
| Changelog | `CHANGELOG.md` |
| Release note | `docs/releases/<YYYY-MM-DD>/<tag>.md`, when released |

## 12. Stop conditions

Stop and request human direction when:

- the plan needs a new claim type, scope, dependency, or architecture decision;
- a verification row fails and the fix changes the approved scope;
- an issue is `In Review` without green checks;
- the plan cannot name an observable acceptance criterion;
- a cleanup step would delete, archive, remove, or bulk-clean a board item;
- a `Done` item would be reopened or moved back to an earlier status;
- a worktree or branch cannot be cleaned safely.

## 13. Related documents

- [`docs/workflows/README.md`](README.md)
- [`AGENTS.md`](../../AGENTS.md)
- [`requirements-design.md`](requirements-design.md)
- [`recordkeeping.md`](recordkeeping.md)
- [`release-operations.md`](release-operations.md)
