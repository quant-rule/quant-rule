# AI Transparency Log — User Manual Kind and Baseline Status Index

- Date: 2026-11-16 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #849, `849-manual-kind`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)

## Intent and boundary

Establish `docs/manual/` as the living user-manual kind with a 12-chapter
skeleton, and remove the baseline status contradiction between `docs/README.md`
and `docs/improvement/baselines.md`. The AI wrote the manual index, the docs-kind
table and status row, the `AGENTS.md` layout entry, the changelog entry, and
this log. It did not write the chapter contents; those land with their
capabilities in #859.

## Verification and evidence

- The manual index names the 12 chapters and the evidence labels.
- The status index now says three model baselines are accepted and no strategy
  or portfolio baseline is accepted, matching the registry.
- The manual kind is listed in the docs index and in the `AGENTS.md` repository
  layout.
- The checks are recorded in the pull request.

## Learning

- The baseline row was stale: it said "no baseline accepted" while the registry
  recorded three accepted model baselines. The correction names the exact
  baselines instead of replacing one vague statement with another.
- A manual chapter is not `Current` until its commands and screen exist. The
  skeleton records that rule so the manual does not claim a capability early.
