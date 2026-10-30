# Continuous Improvement Roles

- Date: 2026-10-30
- Work item: `audit-improvement-workflow`

## Mission

Find problems, turn them into actionable plans, and independently verify that
fixes close the findings without regressions.

## Audit Agent

Read-only. Produces `docs/audits/<date>/<scope>-audit.md`. Records reproducible
evidence, expected invariants, observed behavior, impact, and at least two
remediation options for Critical/Major findings. Does not modify code, create
issues, or approve work.

## Planner Agent

Reads the audit and the current codebase. Reproduces or code-reads every finding,
deduplicates, searches for the root cause, compares remediation options against
simplicity/modularity/regression risk, and writes
`docs/execution-plans/<date>/<issue>-<slug>.md`. After human approval it creates
fix issues/sub-issues. It does not implement or approve.

## Improvement Verifier

Read-only. Independently checks that a merged fix closes the finding, has a
regression test, contains no unrelated change, does not weaken tests, and has
the required Quant Validator record. It updates the finding to `Verified` or
returns it for remediation. It cannot verify a finding or plan it authored.

## Separation of duties

- Audit cannot plan, implement, or verify its own finding.
- Planner cannot implement.
- Verifier cannot verify its own work.
- Execution cannot mark a finding `Verified`.
- Human owns `Won't fix` and residual risk.

## Finding status

`Open` → `Triaged` → `Planned` → `In progress` → `Fixed` → `Verified`
plus `Won't fix`, `Duplicate`, and `Stale`.

Quant findings additionally require Quant Researcher/Developer/Validator
evidence; Continuous Improvement cannot close them alone.
