# <Plan Title>

- Status: Proposed | Approved | In Progress | In Review | Verified | Superseded
- Date: <YYYY-MM-DD> (UTC)
- Issue: <#issue>
- Claim type: Problem Claim | Behaviour Claim | Outcome Claim | ADR (Architecture Decision)
- Owner: <role>
- Workflow: [Execution Plan](../execution-plan.md)

## Intent

## Current state

## Scope

Included:

Excluded:

## Assumptions and uncertainty

## Decisions

| ID | Decision | Consequence | Approval |
|---|---|---|---|
| D1 |  |  |  |

## Acceptance criteria

- [ ] 

## Work breakdown

| Phase | Work | Role | Depends on | Affected paths | Output | Gate |
|---|---|---|---|---|---|---|
| 1 |  |  |  |  |  |  |

## Verification

| Check | Command | Expected |
|---|---|---|
|  |  |  |

## Risks and recovery

| Risk | Control | Recovery |
|---|---|---|
|  |  |  |

## Stop conditions

- 

## Verification table

At approval, fill `ID`, `Plan item`, `Acceptance criterion`, `Evidence /
command`, and `Expected`. The implementer fills `Observed`; the reviewer fills
`Status`, `Reviewer`, and `Date`. One row covers each work-breakdown phase,
acceptance criterion, and closure step.

Run the workspace gate with `tools/gate.sh`; it uses a target directory inside
the worktree. Do not share a Cargo target directory across worktrees; a cached
test binary can carry another worktree's absolute paths and fail a correct tree.

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Phase 1 / <artifact> | <criterion> | `<command>` | <expected> |  |  |  |  |  |
| V2 | Closure / worktree and branch cleanup | Worktree and branches removed; the `Done` board item remains | `python3 tools/worktree_cleanup.py check <worktree>` | check passes; board item present |  |  |  |  |  |

## Related documents

- [`docs/workflows/execution-plan.md`](../execution-plan.md)
- [`AGENTS.md`](../../../AGENTS.md)
