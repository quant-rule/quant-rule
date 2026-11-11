# Requirements and Design Workflow

- Date: 2026-11-07 (UTC)
- Status: Current
- Scope: a new command, report, data source, capability, or contract
- Owner: Requirements Analyst (problem), Architect (design), Human (approval)
- Contract: [`AGENTS.md`](../../AGENTS.md) section 11
- Claim type: requirements and design process; no model, strategy, or edge claim.

## 1. Purpose

The Requirements and Design Workflow answers one question: **what should the new
capability be, and what is explicitly excluded?**

It turns an idea, request, or gap into a bounded proposal with observable
acceptance criteria, a command or artifact contract, and a design that respects
the pipeline boundaries. It does not implement the capability.

## 2. When to use

Use it when:

- adding a command, report, data source, configuration field, or artifact;
- changing a public contract in a way that needs a design decision;
- introducing a new capability that spans more than one module;
- a request is ambiguous about the user, the output, or the non-goals.

Do not use it when:

- the change is a defect: use the Audit Workflow;
- the change is behavior-neutral: use the Refactor Workflow;
- the change is a model or strategy hypothesis: use the Quant workflows;
- the change is a release or incident: use Release & Operations.

## 3. Stages

```text
Intake -> Discovery -> Requirements -> Design -> Approval -> Handoff
```

| Stage | Question | Owner | Output | Gate |
|---|---|---|---|---|
| 1. Intake | What is the request, and who needs it? | Requirements Analyst | Issue | The request is in scope |
| 2. Discovery | What exists today, and what is the gap? | Requirements Analyst + Architect | Current-state note | Evidence, not assumption |
| 3. Requirements | What is the thin end-to-end behavior? | Requirements Analyst | Requirements | Observable acceptance criteria |
| 4. Design | What is the command, artifact, boundary, and test strategy? | Architect | Design | Stage boundaries and determinism respected |
| 5. Approval | Does the human approve the scope and design? | Human | Approved plan | No implementation before approval |
| 6. Handoff | What does the implementation receive? | Orchestrator | Execution plan and issues | Links to the plan and records |

## 4. Requirements

A requirement states:

- the user and the problem;
- the current behavior and the gap;
- the thin end-to-end outcome;
- the observable acceptance criteria;
- the error behavior;
- the non-goals;
- the data, privacy, and determinism implications;
- the command or artifact contract, when applicable.

A requirement that cannot be observed is not a requirement. A capability the user
cannot reach from the shell does not exist.

## 5. Design

A design states:

- the owning crate or module;
- the stage boundary and the dependency direction;
- the command name, arguments, output, and error behavior;
- the artifact schema and provenance;
- the configuration fields and validation;
- the determinism and causality rules;
- the test strategy: unit, integration, golden, causality, determinism, edge;
- the documentation and tutorial updates;
- the migration or compatibility rule, if any.

A new capability arrives as a new command, not as a change to an old command's
meaning. A command-contract change is a human decision.

## 6. Roles and boundaries

| Role | Responsibility | Boundary |
|---|---|---|
| Requirements Analyst | User, problem, scope, acceptance criteria | No business rule invented; no scope expansion |
| Architect | Boundaries, contracts, trade-offs | Recommends; does not approve |
| Human | Approves requirements, scope, and design | Sole final authority |
| Orchestrator | Consolidates evidence and writes the plan | No silent resolution of conflicts |

## 7. Human gates

| Gate | Human action |
|---|---|
| Scope | Approve the problem and the non-goals |
| Design | Approve the command/artifact contract and any new dependency |
| Plan | Approve the execution plan |
| Contract change | Approve a change to an existing command's meaning |

## 8. Green criteria

A requirements/design change is green when:

- the problem and the user are stated;
- the acceptance criteria are observable;
- the non-goals are explicit;
- the command or artifact contract is stated;
- the stage boundary and dependency direction are correct;
- the determinism and causality rules are stated;
- the test strategy covers the acceptance criteria;
- the human approves the plan;
- the documentation and tutorial updates are named.

## 9. Artifacts and records

| Artifact | Path |
|---|---|
| Issue | Project board |
| Requirements/design record | The issue and the execution plan |
| Execution plan | `docs/execution-plans/<date>/<issue>-<slug>.md` |
| AI log | `docs/ai-transparency-log/<date>/<issue>-<slug>.md` |
| Changelog | `CHANGELOG.md` |

## 10. Stop conditions

Stop and request human direction when:

- the request conflicts with an invariant;
- the scope grows beyond one deliverable;
- a new dependency is required;
- a command contract would change meaning;
- the design crosses a stage boundary;
- the acceptance criteria cannot be observed;
- the human has not approved the plan.

## 11. Handoff

| To | Packet |
|---|---|
| Execution Plan | The approved scope, acceptance criteria, dependencies, and verification table |
| Implementation | The approved plan, acceptance criteria, and contract |
| Test Engineer | The acceptance criteria and edge cases |
| Code Reviewer | The diff and the contract |
| Recordkeeping | The issue, plan, PR, and changelog entry |

A substantial change hands off to [`execution-plan.md`](execution-plan.md). The
plan must include the reviewer verification table before implementation starts.

## 12. Related documents

- [`audit-workflow.md`](audit-workflow.md)
- [`refactor-workflow.md`](refactor-workflow.md)
- [`release-operations.md`](release-operations.md)
- [`../../AGENTS.md`](../../AGENTS.md)
- [`../../README.md`](../../README.md)
- [`../architecture/command-surface.md`](../architecture/command-surface.md)
