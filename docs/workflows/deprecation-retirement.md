# Deprecation and Retirement Workflow

- Date: 2026-11-07 (UTC)
- Status: Current
- Scope: retiring a model family, indicator, event, command, data source, configuration, tool, or record
- Owner: Planner (scope), Implementer (removal), Refactor Verifier or Improvement Verifier (closure)
- Contract: [`docs/registry/history.md`](../registry/history.md), [`docs/registry/deletion-manifest.md`](../registry/deletion-manifest.md)
- Claim type: retirement process; no model, strategy, or edge claim.

## 1. Purpose

The Deprecation and Retirement Workflow answers one question: **how is a surface
removed without losing history, evidence, or a stable identity?**

QuantRule has retired several surfaces. A retirement is not a delete button. It
is a reference check, a history record, an invalidation decision, and a
changelog entry.

## 2. When to use

Use it when:

- retiring a model family, indicator, event, state, or reading;
- retiring a command, report, or artifact;
- retiring a data source, data version, or partition;
- retiring a configuration schema or field;
- retiring a tool or workflow;
- removing a legacy path that no current surface uses.

Do not use it when:

- the change is a defect fix: use the Audit Workflow;
- the change is behavior-neutral: use the Refactor Workflow;
- the change is a new capability: use Requirements & Design;
- the change is a model or strategy hypothesis: use the Quant workflows.

## 3. Stages

```text
Proposal -> Reference Check -> Approval -> Deprecate -> Remove -> Verify -> Record
```

| Stage | Question | Owner | Output | Gate |
|---|---|---|---|---|
| 1. Proposal | What is retired, and why? | Planner | Retirement proposal | The reason is specific |
| 2. Reference check | What still references it? | Implementer | Reference inventory | Every reference is classified |
| 3. Approval | Is the retirement authorized? | Human | Approval | Explicit human authorization |
| 4. Deprecate | Is the surface marked deprecated? | Implementer | Status change | No new work uses it |
| 5. Remove | Is the surface removed? | Implementer | Diff | No current reference remains |
| 6. Verify | Is the removal complete? | Verifier | Verification record | History and evidence preserved |
| 7. Record | Is the retirement traceable? | Orchestrator | History record, changelog entry, AI log | Links resolve |

## 4. Reference check

The reference inventory classifies every reference as:

- a current caller that must be removed or migrated;
- a test or fixture that must be removed or updated;
- a historical record that must be kept;
- a documentation link that must be updated;
- a configuration or data file that must be migrated or retired;
- an external reference outside the repository.

A reference check that finds an unclassified current caller stops the removal.

## 5. History and identity

- A retired ID is never reused.
- A retired event code is never reused.
- A retired model family keeps its history record.
- A retired command keeps its release note and its tutorial history.
- A retired data version keeps its version identity and invalidation tombstone.
- The history boundary is [`docs/registry/history.md`](../registry/history.md).
- The deletion inventory is
  [`docs/registry/deletion-manifest.md`](../registry/deletion-manifest.md).

## 6. Roles and boundaries

| Role | Responsibility | Boundary |
|---|---|---|
| Planner | Scope, reason, and reference plan | No implementation; no approval |
| Implementer | Deprecate and remove | No scope expansion |
| Verifier | Check completeness and history | Read-only; no repair |
| Human | Authorize the retirement | Sole final authority |

## 7. Human gates

| Gate | Human action |
|---|---|
| Proposal | Approve the retirement scope |
| Destructive removal | Authorize the removal |
| History | Approve the history record |
| Residual risk | Accept any remaining reference |

## 8. Green criteria

A retirement is green when:

- the reason is specific;
- every reference is classified;
- no current caller remains;
- the history record and deletion inventory are updated;
- the affected baseline is retired or invalidated;
- the changelog release entry and AI log are present;
- the full gate passes;
- the human authorizes the removal.

## 9. Artifacts and records

| Artifact | Path |
|---|---|
| Retirement record | `docs/registry/history.md` and the owning workflow record |
| Deletion inventory | `docs/registry/deletion-manifest.md` |
| Invalidation | `docs/improvement/INVALIDATIONS.md` |
| Release note | `docs/releases/<date>/<tag>.md` if released |
| Changelog | `CHANGELOG.md` |
| AI log | `docs/ai-transparency-log/<date>/<issue>-<slug>.md` |

## 10. Stop conditions

Stop and request human direction when:

- a current caller cannot be removed or migrated;
- a historical record would be lost;
- an ID or code would be reused;
- an affected baseline cannot be identified;
- the removal is destructive and the human has not authorized it;
- the change is actually a refactor or a defect fix.

## 11. Handoff

| To | Packet |
|---|---|
| Audit Workflow | A defect found during the reference check |
| Refactor Workflow | A behavior-neutral move that is not a retirement |
| Recordkeeping | The retirement ID, history record, and invalidation |
| Release & Operations | The release note for the removal |

## 12. Related documents

- [`refactor-workflow.md`](refactor-workflow.md)
- [`audit-workflow.md`](audit-workflow.md)
- [`release-operations.md`](release-operations.md)
- [`recordkeeping.md`](recordkeeping.md)
- [`../registry/history.md`](../registry/history.md)
- [`../registry/deletion-manifest.md`](../registry/deletion-manifest.md)
- [`../../AGENTS.md`](../../AGENTS.md)
