# Execution Plan — Align the Board to the Kata

- Status: In Progress
- Claim type: ADR (Architecture Decision)
- Owner: Human
- Workflow: [Requirements & Design](../../workflows/requirements-design.md)
- Date: 2026-11-13 (UTC)

## 1. Intent

The kata and the architecture characteristics are the source of truth. The issue
tree was written before them and does not serve every requirement, while it
contains work the kata does not ask for. This plan re-scopes the board.

## 2. Current state

| Source | Content |
|---|---|
| Kata | six requirements: hypothesis record, exploration, costs, statistical significance, out-of-sample validation, comparison and reproduction |
| Characteristics | simplicity 5/5, modularity 4/5, maintainability 4/5, testability 5/5, configurability 4/5, evolvability 4/5, deployability 2/5, performance 3/5, reliability 4/5, responsiveness 2/5, scalability 2/5, elasticity 1/5, fault tolerance 2/5, overall cost 2/5 |
| Board | 61 issues, written for an earlier kata with eight requirements |

## 3. Gap analysis

| Kata requirement | Covered by | Gap |
|---|---|---|
| R1 define and record hypotheses | none | **missing** |
| R2 explore historical data | simulation issues, datasets screen | partial; no exploration query |
| R3 transaction costs | config parsing only | **missing as a requirement** |
| R4 statistical significance and expected returns | none | **missing, and it is the core requirement** |
| R5 out-of-sample validation | none | **missing** |
| R6 compare and reproduce | evidence, digest, verify | comparison missing |

## 4. Scope

**Included**

- Six new issues, one per kata requirement, that carry the domain work.
- Defer the work the kata does not ask for: profiles and the desktop shell.
- Re-scope the epic around the six requirements.
- Record the decision in the plan and the changelog.

**Excluded**

- Deleting or rewriting existing issues other than the deferred set.
- Any program change.
- Any change to the kata or the characteristics.

## 5. Decisions

| ID | Decision |
|---|---|
| K1 | The six kata requirements are the acceptance frame for the board |
| K2 | Profiles, the desktop shell, and the embedded desktop are **deferred**, not deleted: they serve no kata requirement |
| K3 | New domain issues are numbered `DIST-12x` and placed with the services |
| K4 | The epic counts only the six requirements; deferred work is listed but does not gate the release |

## 6. Work breakdown

| Step | Work | Paths |
|---|---|---|
| 1 | Write this plan | `docs/execution-plans/2026-11-13/` |
| 2 | Create `DIST-120` … `DIST-125` | GitHub |
| 3 | Label the deferred set | GitHub |
| 4 | Re-scope the epic | GitHub |
| 5 | Record the change in the changelog | `CHANGELOG.md` |

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Plan recorded | File exists with a verification table | `ls docs/execution-plans/2026-11-13/` | one file | one file | Pass | agent | 2026-11-13 | |
| V2 | Six issues created | One per kata requirement | `gh issue list --label kata` | 6 | pending | Pending | — | — | |
| V3 | Deferred set labelled | Profiles and desktop carry `deferred` | board query | 8 items | pending | Pending | — | — | |
| V4 | Epic re-scoped | Body names the six requirements | `gh issue view 630` | six rows | pending | Pending | — | — | |
| V5 | No contradiction | Every open issue maps to a requirement or is deferred | manual read | no orphan | pending | Pending | — | — | |
| V6 | Changelog | Entry under Unreleased | `check_changelog` | pass | pending | Pending | — | — | |
| V7 | Documentation gates | Links, claims, structure | `tools/gate.sh` checkers | pass | pending | Pending | — | — | |

## 7. Risks and recovery

| Risk | Mitigation |
|---|---|
| Deferring profiles and desktop hides work the user wanted | They stay on the board with a label and a reactivation trigger |
| Six new issues duplicate existing ones | Each names the requirement and the existing issue it builds on |
| The board grows while the code does not | The epic counts requirements, not issues |

## 8. Stop conditions

- A new requirement needs a decision outside the kata.
- An issue cannot be mapped to a requirement and is not infrastructure.
