# Checkpoint B — Operating Model and Feedback

## Claim

A solo developer can deliver the Tauri Platform through a short, iterative
lifecycle with a visible review gate and a runnable increment at each phase.

## Lifecycle

```text
issue -> plan -> implement -> test -> PR -> In Review -> accept/merge -> Done
```

- **Unit:** one issue with one claim type and one PR.
- **Review:** the human gate is `In Review`; `verify` and `sync` must be green.
- **Done:** after acceptance, merge, verification, and worktree cleanup.
- **Feedback:** tests, CI, the course checkpoints, and user review of the app.

## Flow and waiting points

| Step | Owner | Wait |
|---|---|---|
| Plan approval | Human | Scope and verification table |
| Implementation | Implementer | Dependency or decision |
| PR checks | CI | Build and test time |
| Human review | Human | Review queue |
| Installer test | Test Engineer | Clean Linux environment |

## Feedback loop

- Signal: a failed check, a review finding, or a user cannot complete the first
  run.
- Authority: the human accepts or rejects.
- Decision: fix, revise the plan, or stop.
- Artifact: the PR, the verification table row, and the plan update.
- Revisit: the next checkpoint.

## Evidence

The Phase 1 workflow pilot and the four-stage board are recorded in issue #590.
Phase 3 ran through issues #605–#611 with a plan, PR, green checks, `In Review`,
and cleanup. The parent plan's P0–P3E rows are `Pass`; the verification report is
[`docs/reports/2026-11-11/implementation-verification.md`](../../../reports/2026-11-11/implementation-verification.md).

## Limit

The lifecycle is exercised on one solo developer and one Linux machine. It does
not yet cover a team, a multi-user support queue, or a native WebDriver flow.

## Decision

Keep the iterative/incremental lifecycle; defer team coordination until the solo
workflow is validated.
