# AI Transparency Log — Mandatory Merged-Task Cleanup

- Date: 2026-11-10 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash
- Work item: `merged-branch-cleanup-rule`
- Claim type: agent-process rule; no model, strategy, or code change.

## Intent and boundary

Make merged-task cleanup mandatory after the M1.2 audit remediation left its
merged worktrees and local/remote branches behind while reporting `No action`.
The change is limited to `AGENTS.md`, the changelog, and this log. It does not
change code, tests, model behavior, or workflows beyond the Git completion rule.

## Change

`AGENTS.md` now says merged-task cleanup is mandatory, not a default: remove the
task worktree and delete its local and remote task branches before reporting
completion. The rule is repeated in the branch-worktree instructions, the
evidence/handoff stage, the workflow-closure stage, and the response format so a
merged task cannot be reported as `No action` while its worktree or branch
remains. `CHANGELOG.md` records the process change.

## Verification

- `python3 tools/check_links.py` — pass after the change.
- `python3 tools/check_terms.py` — pass after the change.
- `git diff --check` — pass.
- The M1.2 audit worktrees and branches were removed and verified before this
  change: `git worktree list` showed only `main`, the local branch list showed
  only `main`, and both remote task branches were deleted.

## Learning

- Reporting completion is a Git-state claim, not only a code/test claim. A
  merged branch or worktree left behind is unfinished cleanup.
- Repeating the rule in the stage, branch, and response sections removes the
  ambiguity that allowed the previous omission.

## Rejected or corrected

- Did not leave the cleanup as a single "default cleanup" paragraph; it is now
  mandatory and repeated at the points where completion is reported.
- Did not treat the merged branch deletion as requiring a separate approval; the
  rule records it as the routine completion step.
