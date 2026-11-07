# QuantRule Workflows

## Purpose

This folder defines the operating contracts for QuantRule's engineering and
quantitative research work. The workflows are complementary: some decide what to
change, some decide whether a change is correct, some decide whether a change is
an improvement, and some preserve the evidence.

The human remains the sole decision owner. An AI recommendation is not an
approval, and a completed agent task is not accepted until the required human
decisions and evidence are recorded.

## Repository integration

The root [`AGENTS.md`](../../AGENTS.md) is the normative contract and the entry
point. This index routes procedural detail to the workflow documents. Do not
create a second root agent-instructions file.

The current status of the pipeline, model, strategy, portfolio, and tooling is
in [`docs/README.md`](../README.md). This index records workflow status only.

The shell's `help workflows` topic is the in-shell selection guide. It maps an
intention to a workflow and its first command; the full contracts are here.

Historical records are indexed by [`docs/README.md`](../README.md). The change
history is in [`CHANGELOG.md`](../../CHANGELOG.md).

## Workflow index

| Workflow | Question | Type | Status | Use now? |
|---|---|---|---|---|
| [`audit-workflow.md`](audit-workflow.md) | Is the program correct and consistent with its contracts? | Engineering assessment | Current | Yes |
| [`refactor-workflow.md`](refactor-workflow.md) | Is the structure cheap to change and provable? | Engineering improvement | Current | Yes |
| [`improvement-workflow.md`](improvement-workflow.md) | Does a change improve the trading system? | Quant spine | Current | Yes; no tiered run yet |
| [`run-contract.md`](run-contract.md) | What makes a run valid evidence? | Quant record | Current | Yes; v2 current |
| [`research-swarm.md`](research-swarm.md) | How do pre-registered cells run concurrently? | Quant execution | Current | Yes; no active campaign |
| [`model-research.md`](model-research.md) | Which model measurement deserves promotion? | Quant tier | Current | Yes |
| [`model-change.md`](model-change.md) | How is an indicator or event implemented and registered? | Quant implementation | Current | Yes |
| [`strategy-research.md`](strategy-research.md) | Does a rule produce a cost-inclusive edge? | Quant tier | Current | Gated by a model baseline |
| [`portfolio-research.md`](portfolio-research.md) | Do execution and accounting improve the account outcome? | Quant tier | Current | Gated by a strategy baseline |
| [`policy-change.md`](policy-change.md) | Does the human-owned envelope change? | Governance | Current | Yes |
| [`data-lifecycle.md`](data-lifecycle.md) | Is the input valid, versioned, and admissible? | Data | Current | Yes |
| [`requirements-design.md`](requirements-design.md) | What should the new capability be? | Engineering upstream | Current | Yes |
| [`release-operations.md`](release-operations.md) | Is this safe to release and recover? | Engineering downstream | Current | Yes |
| [`deprecation-retirement.md`](deprecation-retirement.md) | How is a surface removed without losing history? | Engineering lifecycle | Current | Yes |
| [`recordkeeping.md`](recordkeeping.md) | Where is the evidence and what is its status? | Cross-cutting | Current | Yes |

## How to choose a workflow

| If the change is… | Use |
|---|---|
| A defect, contract drift, test gap, or documentation defect | [Audit Workflow](audit-workflow.md) |
| A structural problem, coupling, file size, or duplication | [Refactor Workflow](refactor-workflow.md) |
| A model measurement, event, or family hypothesis | [Model Research](model-research.md) |
| An indicator, event, or model implementation change | [Model Change](model-change.md) |
| A strategy hypothesis | [Strategy Research](strategy-research.md) |
| An execution or portfolio mechanics hypothesis | [Portfolio Research](portfolio-research.md) |
| A policy envelope change | [Policy Change](policy-change.md) |
| A data source, version, or partition change | [Data Lifecycle](data-lifecycle.md) |
| A new command, report, or capability | [Requirements & Design](requirements-design.md) |
| A release, tag, rollback, or incident | [Release & Operations](release-operations.md) |
| Retiring a model, event, command, or data source | [Deprecation & Retirement](deprecation-retirement.md) |
| A record, ID, status, or changelog issue | [Recordkeeping](recordkeeping.md) |
| An improvement run, baseline, or invalidation | [Improvement Workflow](improvement-workflow.md) + [Run Contract](run-contract.md) |
| Parallel pre-registered cells | [Research Swarm](research-swarm.md) |

## How they fit together

```text
POLICY (human-owned, outside the tiers)
   |
   v
DATA (versioned, validated) --> MODEL RESEARCH --> MODEL CHANGE (if needed) --> MODEL BASELINE (human)
                                      |
                                      v
                                 STRATEGY RESEARCH --> STRATEGY BASELINE (human)
                                      |
                                      v
                                 PORTFOLIO RESEARCH (execution + portfolio) --> PORTFOLIO BASELINE (human)
                                      |
                                      v
                                 QUALIFICATION / ANALYTICS (evidence, no verdict)

Cross-cutting:
  Audit (defects) | Refactor (structure) | Requirements & Design (new capability)
  Release & Operations (release/incident) | Deprecation & Retirement (remove surface)
  Recordkeeping (records, IDs, changelog, AI log)
```

The Improvement Workflow is the tier spine. The Research Swarm is the parallel
execution mechanism. The Run Contract is the unit of evidence. The tier runbooks
add the questions, commands, and gates specific to model, strategy, and
portfolio work. Policy and data are outside the tiers but can invalidate a
baseline.

## Shared rules

- **Human authority.** The human approves requirements, scope, priorities,
  architecture, policy, strategy, validation access, baselines, releases, and
  residual risk. Agents investigate, recommend, and execute only within an
  approved scope.
- **No self-verification.** The agent that implements a change does not verify or
  close its own record. If separate agents are unavailable, sequential role
  passes are disclosed and are not called independent review.
- **Evidence over assertion.** A finding, run, or verification names the command,
  the observed result, and the limitation. An unrun check is not a pass.
- **Permanent records.** Records are append-only. Corrections are dated addenda.
  A superseded record keeps its original content and links forward.
- **One source of truth.** `docs/README.md` is the current status index; this file
  is the workflow index; `CHANGELOG.md` is the grouped release history. Do not
  duplicate those tables elsewhere. The GitHub Project is the execution tracker;
  the changelog is the release history; the AI log is the work-item record.
- **One workflow per decision boundary.** Do not add a workflow for every noun.
  Add one where the wrong route would contaminate evidence or violate a gate.
- **No behavior change by accident.** A workflow change that alters model or
  strategy behavior stops and routes to the Quant roles.
- **No hidden legacy.** A current document does not describe a retired command,
  path, or schema as current. Historical records stay historical.

## Status vocabulary

| Status | Meaning |
|---|---|
| `Current` | Approved contract; may be used now |
| `Target` | Approved direction; not yet usable |
| `Historical` | Retained for reproducibility; not used for new work |
| `Deprecated` | Still present; no new work; removal scheduled |
| `Proposed` | Draft awaiting human approval |
| `Superseded` | Replaced by another document; link forward |

## Record map

The detailed record contract is in [`recordkeeping.md`](recordkeeping.md). The
short map is:

| Record | Path | ID | Statuses |
|---|---|---|---|
| Workflow | `docs/workflows/<name>.md` | — | Current, Target, Historical, Deprecated, Proposed, Superseded |
| Audit | `docs/audits/<date>/<scope>-audit.md` | `AUD-<date>-NN` | Open, Triaged, Planned, In progress, Fixed, Verified, Won't fix, Duplicate, Stale |
| Refactor | `docs/refactors/<date>/<scope>-refactor.md` | `REF-<date>-NN` | Open, Triaged, Planned, In progress, Refactored, Verified, Deferred, Won't refactor, Duplicate, Stale |
| Improvement run | `data/improvement/<date>/<run-id>/` | `MODEL-/STRATEGY-/PORTFOLIO-<date>-NN` | pending, accepted, rejected, invalidated, no edge, new hypothesis, failed |
| Programme | `docs/governance/<date>/<PROGRAM-ID>.md` | `PROGRAM-<date>-NN` | Proposed, Approved, Stopped, Closed; verdict `no_edge`, `development_only`, `candidate`, `accepted` |
| Policy change | `docs/config/` + `docs/workflows/policy-change.md` record | `POL-<date>-NN` | Proposed, Approved, Applied, Superseded, Rejected |
| Data change | `docs/config/` + `data/partitions.json` | `DATA-<date>-NN` | Proposed, Validated, Active, Superseded, Invalidated |
| Release | `docs/releases/<date>/<tag>.md` | tag | Planned, Ready, Released, Rolled back, Superseded |
| Deprecation | `docs/registry/history.md` + workflow record | `DEP-<date>-NN` | Proposed, Approved, Deprecated, Removed, Superseded |
| Changelog | `CHANGELOG.md` | release version | release history, grouped by version |
| AI transparency | `docs/ai-transparency-log/<date>/<issue>-<slug>.md` | work item | append-only |

## Templates

| Template | Use |
|---|---|
| [`templates/workflow-template.md`](templates/workflow-template.md) | Write a new workflow document |
| [`templates/policy-change-record.md`](templates/policy-change-record.md) | Record a policy change |
| [`templates/data-change-record.md`](templates/data-change-record.md) | Record a data source/version/partition change |
| [`templates/release-record.md`](templates/release-record.md) | Record a release |
| [`templates/deprecation-record.md`](templates/deprecation-record.md) | Record a retirement |
| [`templates/requirements-record.md`](templates/requirements-record.md) | Record a requirements/design decision |

The audit, refactor, and improvement templates live in their own kind READMEs:
[`docs/audits/README.md`](../audits/README.md),
[`docs/refactors/README.md`](../refactors/README.md), and
[`docs/improvement/README.md`](../improvement/README.md).

## Operating notes

1. Read `AGENTS.md` and the relevant architecture/domain documentation before
   acting.
2. Choose the workflow appropriate to the request; not every task needs every
   role or document.
3. Prepare a formal execution plan for substantial or high-risk work and wait for
   human approval.
4. Keep one work item in progress at a time unless the human authorizes parallel
   work.
5. Record only observed evidence. Mark unavailable checks as unverified, not
   passed.
6. Add or update the changelog release entry when the change is meaningful, and add the AI transparency entry in the same pull request
   as the change.
7. Update the workflow status and the record map when a workflow changes.
8. Stop when a new decision, a new dependency, or a behavior change is required.

## Related documents

- [`AGENTS.md`](../../AGENTS.md)
- [`docs/README.md`](../README.md)
- [`CHANGELOG.md`](../../CHANGELOG.md)
- [`docs/architecture/README.md`](../architecture/README.md)
- [`docs/improvement/README.md`](../improvement/README.md)
- [`docs/audits/README.md`](../audits/README.md)
- [`docs/refactors/README.md`](../refactors/README.md)
