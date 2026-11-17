# AI transparency log — Audit remediation status after merge (2026-11-17)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) execution and
  record closure.
- Claim type: Behaviour Claim.
- Intent: mark the findings whose fixes are merged as `Fixed`, leave `Verified`
  to the independent Improvement Verifier, and bring the plan evidence in line
  with `main`.
- Boundary: branch `docs/audit-remediation-status`; no code change.

## What changed

- 33 findings in `docs/audits/2026-11-17/whole-project-audit.md` are `Fixed`
  with the merged PR link; the remaining 20 are `Open` (0 Critical, 11 Major,
  7 Minor, 2 Info). The audit header and a dated remediation note state the
  split.
- The plan's V01, V03, V04, V05, V20, and V21 rows are `Pass`; V02, V06, and
  V07 are `Blocked` on the composed/Postgres tiers; V19 remains `Pending` on
  the independent Verifier, the release-gate change, and the composed suite.
- The `Fixed` statuses point at the merged PRs: #881–#894 for the fixes and
  #895–#896 for the record corrections.

## Verification observed

| Command | Result |
|---|---|
| `python3 tools/check_links.py` | 1,045 Markdown files; no broken relative links |
| `python3 tools/check_changelog.py` | 10 releases, 187 entries |
| `python3 tools/check_doc_length.py` | 40 files within cap |
| `python3 tools/check_execution_plans.py` | current plans carry a claim type, verification table, and AI log |
| Audit table count | 33 Fixed, 20 Open, 53 total |
| `tools/gate.sh` | run after this log; the result is recorded in the task report |

## Quant validation (sequential, not independent)

No model or strategy behavior changed in this record update. The fixes were
validated on their branches and are merged; the audit status is a record of the
merge, not a new validation. The independent Improvement Verifier and the human
remain the authority for `Verified` and acceptance.

## Merge note

The stack was merged sequentially. Three verify jobs failed after a merge
because the CHANGELOG conflict resolution left an empty heading or an
unformatted line (#889, #893, #890); #898, #899, and #900 repaired `main`. The
fix content had already passed its branch gate before the merge; the repaired
`main` gate is the relevant evidence.

## Learning

- A per-PR CHANGELOG entry conflicts on every merge when all branches touch
  `[Unreleased]`. The resolver must drop empty headings and the merged tree must
  run `check_changelog.py` before the PR is merged.
- `Fixed` is a merge fact; `Verified` is an independent review fact. Recording
  them separately keeps the audit honest.
