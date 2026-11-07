# Recordkeeping and Traceability

- Date: 2026-11-07 (UTC)
- Status: Current
- Scope: every workflow record, its identity, its status, and its links
- Contract: [`docs/workflows/README.md`](README.md)
- Claim type: record contract; no model, strategy, or edge claim.

## 1. Purpose

Recordkeeping makes each decision, finding, change, verification, and outcome
traceable. It prevents duplicate or conflicting work and stops repeated
fix-revert-refactor cycles.

Records are append-only. A historical record is never overwritten, renamed,
moved, or deleted. Corrections and supersessions are dated addenda that link back
to the original.

## 2. Record kinds

| Record | Path | ID | Owner | Statuses |
|---|---|---|---|---|
| Workflow | `docs/workflows/<name>.md` | — | Orchestrator | Current, Target, Historical, Deprecated, Proposed, Superseded |
| Audit | `docs/audits/<date>/<scope>-audit.md` | `AUD-<date>-NN` | Audit Agent | Open, Triaged, Planned, In progress, Fixed, Verified, Won't fix, Duplicate, Stale |
| Refactor | `docs/refactors/<date>/<scope>-refactor.md` | `REF-<date>-NN` | Structure Auditor | Open, Triaged, Planned, In progress, Refactored, Verified, Deferred, Won't refactor, Duplicate, Stale |
| Improvement run | `data/improvement/<date>/<run-id>/` | `MODEL-/STRATEGY-/PORTFOLIO-<date>-NN` | Loop Governor | pending, accepted, rejected, invalidated, no edge, new hypothesis, failed |
| Programme | `docs/governance/<date>/<PROGRAM-ID>.md` | `PROGRAM-<date>-NN` | Human + Coordinator | Proposed, Approved, Stopped, Closed; verdict `no_edge`, `development_only`, `candidate`, `accepted` |
| Policy change | `docs/config/` + policy-change record | `POL-<date>-NN` | Human + Planner | Proposed, Approved, Applied, Superseded, Rejected |
| Data change | `docs/config/` + `data/partitions.json` | `DATA-<date>-NN` | Data Steward | Proposed, Validated, Active, Superseded, Invalidated |
| Release | `docs/releases/<date>/<tag>.md` | tag | Release Verifier | Planned, Ready, Released, Rolled back, Superseded |
| Deprecation | `docs/registry/history.md` + deprecation record | `DEP-<date>-NN` | Planner | Proposed, Approved, Deprecated, Removed, Superseded |
| Execution plan | `docs/execution-plans/<date>/<issue>-<slug>.md` | issue/slug | Planner | Proposed, Approved, Verified, Superseded |
| AI transparency | `docs/ai-transparency-log/<date>/<issue>-<slug>.md` | work item | Orchestrator | append-only |
| Changelog | `CHANGELOG.md` | release version | Implementer | grouped by release; released sections immutable |
| Invalidation | `docs/improvement/INVALIDATIONS.md` | run/baseline/artifact | Reviewer + human | append-only |

## 3. Identity rules

- An ID is assigned once and never reused.
- A run ID is `MODEL-YYYY-MM-DD-NN`, `STRATEGY-YYYY-MM-DD-NN`, or
  `PORTFOLIO-YYYY-MM-DD-NN`; `NN` starts at `01`.
- A programme ID is `PROGRAM-YYYY-MM-DD-NN`.
- An audit finding is `AUD-YYYY-MM-DD-NN`; a refactor finding is
  `REF-YYYY-MM-DD-NN`.
- A policy change is `POL-YYYY-MM-DD-NN`; a data change is
  `DATA-YYYY-MM-DD-NN`; a deprecation is `DEP-YYYY-MM-DD-NN`.
- A retired ID is never reused. A failed cell keeps its ID and its failure
  record.

## 4. Status rules

- Only the owning role moves a record into its next status.
- `Fixed` is not `Verified`; a merge is evidence that a change landed, not that
  the finding is closed.
- `Refactored` is not `Verified`; equivalence and the named metric must be
  independently checked.
- `Accepted` is a human decision. An agent may recommend, never accept.
- `Won't fix`, `Deferred`, `Won't refactor`, and residual-risk acceptance are
  human-only decisions.
- A record with an open Critical or Major finding cannot be a baseline.
- A run whose `results.json` records `ok: false` is `failed`, not partially
  successful.

## 5. Append-only rules

- Never overwrite, rename, move, or delete a historical record.
- A correction is a dated addendum that links to the original.
- A superseded record keeps its content and gains a forward link.
- An invalidation is a tombstone row in
  [`docs/improvement/INVALIDATIONS.md`](../improvement/INVALIDATIONS.md).
  Deleting artifacts without a tombstone is a defect.
- The changelog is append-only per release. A released section is not rewritten;
  a correction is a new release or a dated addendum in the next release.

## 6. Cross-index rules

- `docs/README.md` is the global status index.
- `docs/workflows/README.md` is the workflow index.
- `CHANGELOG.md` is the grouped release history; the GitHub Project is the
  execution tracker. They are not duplicates.
- An audit or refactor finding links its issue, plan, PR, and verification.
- An improvement run links its manifest, review, decision, baseline, and
  invalidation.
- A programme links its charter, cells, results, and closeout.
- A policy or data change links its impact analysis, affected baselines, and
  re-baseline sequence.

## 7. AI transparency

Record material AI contributions in
`docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md`, one file per work
item. Each entry records the date and tool, the intent and boundary, the
verification and evidence, and the learning, including rejected or corrected
suggestions. Never invent test results or omit failed checks.

## 8. Changelog

[`CHANGELOG.md`](../../CHANGELOG.md) is the grouped release history. It records
meaningful user-facing or developer-facing changes under `## [Unreleased]` and
under each released version, grouped by `Added`, `Changed`, `Deprecated`,
`Removed`, `Fixed`, or `Security`. It is not a pull-request index and does not
duplicate the GitHub Project.

A pull request that changes user-facing or developer-facing behavior adds or
updates an entry. A pull request that does not records “no changelog entry
needed” and the reason. Released sections are immutable; a correction is a new
release or a dated addendum in the next release. The changelog starts at the
2026-11-07 workflow documentation reset; earlier release notes are in
[`docs/releases/`](../releases/).

## 9. Verification

- `python3 tools/check_workflows.py` checks the workflow index and status.
- `python3 tools/check_changelog.py` checks the grouped release structure, allowed
  types, and release order.
- `python3 tools/check_links.py` checks every relative link.
- `python3 tools/check_terms.py` and `python3 tools/check_legacy.py` reject
  retired terms and labels.

## 10. Related documents

- [`docs/workflows/README.md`](README.md)
- [`docs/README.md`](../README.md)
- [`CHANGELOG.md`](../../CHANGELOG.md)
- [`docs/audits/README.md`](../audits/README.md)
- [`docs/refactors/README.md`](../refactors/README.md)
- [`docs/improvement/README.md`](../improvement/README.md)
