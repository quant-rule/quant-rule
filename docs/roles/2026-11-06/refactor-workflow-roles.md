# Refactor Workflow Roles

- Date: 2026-11-06 (UTC)
- Work item: #516 `Establish the Refactor Workflow`
- Workflow: [`docs/workflows/refactor-workflow.md`](../../workflows/refactor-workflow.md)
- Normative source: [`AGENTS.md` section 10](../../../AGENTS.md)

## Mission

Improve the structure of the program without changing observable behavior, so
that the next correct change stays local, testable, and understandable. The
Refactor Workflow changes code, tests, docs, tools, and repository layout; it
never changes a model, strategy, execution, portfolio, or analytics result.

## Stage ownership

| Stage | Owner | Output |
|---|---|---|
| Refactor Assessment | Structure Auditor | Dated assessment document with findings |
| Refactor Plan | Refactor Planner | Approved execution plan and refactor issues |
| Execution | Refactor Implementer | Merged behavior-neutral seam, tests, docs, PR |
| Verification | Refactor Verifier | Finding `Verified`, or returned for remediation |
| Loop Decision | Structure Auditor and human | Next cycle, stop, or new campaign |

## Structure Auditor

Read-only. Produces `docs/refactors/<date>/<scope>-refactor.md`. Records the
baseline, the structural scorecard, reproducible evidence, the expected
invariant, the observed structure, impact, and at least two options for High and
Medium findings. Does not modify code, create issues, or approve work.

## Refactor Planner

Reads the assessment and the current codebase. Reproduces or code-reads every
finding, deduplicates, searches for the structural root cause, compares options
against simplicity, modularity, and regression risk, and writes
`docs/execution-plans/<date>/<issue>-<slug>.md`. After human approval it creates
refactor issues and sub-issues. It does not implement or approve.

## Refactor Implementer

Executes only the approved seam. Keeps behavior identical, moves tests and
module documentation with the code, runs the full gate incrementally, and
records observed results. It does not verify its own work, expand scope, or
change a fixture or expected value to obtain a pass.

## Refactor Verifier

Read-only. Independently checks that a merged refactor preserved behavior and
improved the named metric. Re-runs the baseline from a clean worktree, checks the
test inventory, golden fixtures, representative output hashes, the complete
diff, the absence of unrelated changes, and the required documentation updates.
It updates the finding to `Verified` or returns it for remediation. It cannot
verify a finding, plan, or implementation it authored.

## Separation of duties

- Structure Auditor cannot plan, implement, or verify its own finding.
- Refactor Planner cannot implement.
- Refactor Implementer cannot move a finding to `Verified`.
- Refactor Verifier cannot verify its own assessment, plan, or implementation.
- Human owns `Deferred`, `Won't refactor`, and residual structural debt.

When one agent performs several roles, the orchestrator records the sequential
passes and does not claim independent review.

## Finding status

`Open` -> `Triaged` -> `Planned` -> `In progress` -> `Refactored` -> `Verified`,
plus `Deferred`, `Won't refactor`, `Duplicate`, and `Stale`.

`Refactored` is not `Verified`: a merge is not closure. Only the Refactor
Verifier moves a finding to `Verified`; only the human decides `Deferred` or
`Won't refactor`.

## Role mapping

| Refactor role | Existing role it specializes |
|---|---|
| Structure Auditor | Software Architect |
| Refactor Planner | Planner |
| Refactor Implementer | Implementation |
| Refactor Verifier | Code Reviewer and Improvement Verifier |

The Refactor Workflow is an Engineering workflow. It does not replace the
Security and Privacy Reviewer, the Release / Operations Verifier, or the human
approval gates.

## Quant boundary

A refactor that changes model or strategy behavior is not a refactor. It stops
and goes to the service or model workflow that owns the behavior, with the Quant
Researcher, Quant Developer, and Quant Validator as required by `AGENTS.md`
section 10. A move that changes a numeric result, fixture value, event code, or
strategy decision is a behavior change even when the diff looks mechanical.
