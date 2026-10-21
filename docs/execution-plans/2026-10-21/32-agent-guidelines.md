# Execution Plan: The agent guidelines for sub-issues, file size, and worktrees

- Issue: [#32](https://github.com/quant-rule/quant-rule/issues/32) — docs: add the worktree, sub-issue, and conciseness guidelines
- Branch: `docs/32-agent-guidelines`
- Worktree: `/home/zantgo/Projects/quant-rule--32-agent-guidelines`
- Status: Approved
- Date: 2026-10-21 (UTC; 2026-10-20 local time)
- Transparency log: [`docs/ai-transparency-log/2026-10-21/32-agent-guidelines.md`](../../ai-transparency-log/2026-10-21/32-agent-guidelines.md)

## Intent

The human asked for three additions to the agent instructions, and asked for the environment to be ready for the issues to start tomorrow.

- How an issue is decomposed into sub-issues.
- How long a source file and a document should be.
- How a task's work is isolated in a Git worktree, and how the task is cleaned up once its pull request is merged.

Affected users: the human developer, who owns the workflow; and the agents, who follow these rules on every task.

## Current state

Evidence gathered on 2026-10-21 UTC, `origin/main` at commit `5523837` (the merge of pull request #31).

- `AGENTS.md` held 592 lines in fourteen numbered sections. Section 7's "Branch Preparation" was six bullets, all of which concerned inspection and the choice of a base branch; it said nothing about where a task's files live.
- Section 9's "Git Approval Gates" listed "Deleting branches or performing destructive Git operations" as gated, with no exception for the routine cleanup that follows a merge.
- Section 9's lifecycle rules held the container rule for an issue with sub-issues, added on 2026-10-21 with the model-engine contract, and nothing about how a split is decided.
- Section 3's "Modularity" bullet spoke of cohesive modules but set no size; section 5 had no rule on conciseness.
- The repository had one branch, `main`, and one worktree, both clean. Six merged branches and their remotes had been deleted earlier the same day, after their pull requests were confirmed merged and their content confirmed present on `main`.
- The data engine's plan had been written and its implementation was then stopped at the human's instruction. The half-written Rust code, its worktree, and its branch were removed, and the plan was kept in a corrected form for the next session.

## Scope

Included:

- `AGENTS.md` section 7: the worktree rules — a dedicated worktree and branch per issue, the checks before acting, the sibling directory named `<repository>--<task-slug>`, the project directory left on `main`, the rules about other sessions' directories, and the rule that a session which cannot be started in the right worktree asks the human for one.
- `AGENTS.md` section 9: the sub-issue decomposition rules, and the authorization for the routine post-merge cleanup of a worktree and a task branch.
- `AGENTS.md` section 3: the file-size guideline, under 1,000 lines unless a justification is written down, split along a module's own seams, without fragmenting a cohesive module.
- `AGENTS.md` section 5: the conciseness guideline for documentation.
- `AGENTS.md` sections 10 and 13: the two places that state how a session starts and who may write to a file, adjusted to name the worktree.
- The data engine's plan and its transparency log, kept for the next session and marked as proposed and unstarted.
- This plan and the transparency log for this work item.

Explicit non-goals:

- No change to the fourteen numbered sections, their numbering, their titles, or their cross-references.
- No change to the shell, the model, the command set, or any product behaviour.
- No change to the approval gates for anything other than the routine post-merge cleanup of a task's own worktree and branch.
- No implementation of the data engine. Its plan is kept, not built.

## Assumptions and uncertainty

- **The worktree is the default, and it applies to every issue.** The human asked for the default explicitly, and for it to hold "regardless of whether OpenCode starts in the project root or another worktree". The rule is written as a default with a stated exception path, not as an option.
- **The cleanup is a routine, not a gate.** The human asked for it to be authorized in section 9 without a separate approval, and to keep the existing gates for everything else. The rule therefore names the conditions under which the cleanup happens: the pull request merged, the acceptance criteria verified, the issue marked `Done`, and nothing uncommitted that would be lost.
- **The three guidelines go where their subject is already governed.** The sub-issue rules belong with the issue lifecycle in section 9, the file size with the code conventions in section 3, the conciseness with the documentation rules in section 5, and the worktree rules with the branch preparation in section 7. No new numbered section is created, and no rule is stated twice.
- **The line count is a prompt, not a quota.** The human's wording says "preferably remain under 1,000 lines ... unless there is a documented justification" and warns against arbitrary fragmentation. The rule is written to keep both halves of that.
- **The data engine's plan is kept as a proposal.** It was written on the human's instruction to start the work, and it is kept so the next session does not have to re-derive ten decisions. It is marked `Proposed`, because the human never approved it, and its verification section no longer claims a run that never happened.

## Decisions

| # | Open point | Options | Choice | Rationale |
|---|---|---|---|---|
| G1 | Where the sub-issue rules go | A new numbered section; a subsection of section 9; a bullet in the lifecycle rules | A subsection of section 9, named "Decomposing an issue into sub-issues" | The subject is the issue lifecycle, which section 9 already owns, and the container rule added with the model-engine contract sits directly above it. A new numbered section would renumber sections 4 through 14 and every cross-reference to them |
| G2 | How the sub-issue rules relate to the container rule | Replace it; extend it; state both | Extend: the container rule stays as the lifecycle statement, and the new subsection states how a split is decided | The container rule says what a parent is; the new subsection says how to get there. They answer different questions, and one does not contain the other. The container rule's own wording was later replaced by issue #36, at the human's direction, so that a parent's status follows its children |
| G3 | Where the file-size rule goes | Section 3; section 5; a new section | Section 3, beside "Modularity" | A file-size rule is a code convention, and the rule it qualifies is the modularity rule it sits next to |
| G4 | Where the conciseness rule goes | Section 3; section 5 | Section 5 | It is about documents rather than code, and section 5 owns the documentation rules |
| G5 | Whether the file-size rule is a hard limit | A hard limit with exceptions; a preference with a documented justification | A preference with a written justification | The human's wording says "preferably ... unless there is a documented justification", and a hard limit would force splits that make the code worse |
| G6 | Whether the worktree rules replace or extend "Branch Preparation" | Replace; extend | Extend, and restructure the subsection into the three stages the work happens in — before acting, creating the worktree, working in it | The existing bullets are the "before acting" stage and are kept. The new material is the other two stages, and one list of mixed rules would not be readable |
| G7 | Where the worktree lives | A subdirectory of the repository; a sibling directory; a system temporary directory | A sibling directory named `<repository>--<task-slug>` | The human specified it. A sibling keeps the task's files out of the repository, and the name states which task owns the directory |
| G8 | How the post-merge cleanup is authorized | A blanket exception; an exception with conditions; leave it gated | An exception with conditions, in section 9 | The human asked for the routine cleanup to be authorized without a separate approval and for the other gates to stand. The conditions are the human's own: merged, criteria verified, issue marked `Done`, nothing lost |
| G9 | Whether the cleanup applies to the remote branch | Yes; no | Yes, "locally and on the remote", with the remote deletion following repository policy | The human said so, and a local-only deletion leaves a branch that the next task's branch list has to skip |
| G10 | Whether the data engine's plan is kept | Keep it on `main`; keep it in the worktree; discard it | Keep it on `main`, marked `Proposed`, with a transparency log of its own | The next session starts from a design rather than from a blank page, and the plan says plainly that nothing has been built |
| G11 | The branch and worktree for this work item | `docs/32-agent-guidelines` in `quant-rule--32-agent-guidelines` | That pair | The naming conventions in section 7, applied to this task, and the rule is followed from the first task it applies to |

## Acceptance criteria

Observable conditions for completion:

- `AGENTS.md` section 9 states how an issue is split into sub-issues, and states the conditions under which a task's worktree and branch are removed after the merge. The container rule it also states was later replaced by issue #36, so that a parent's status follows its children.
- `AGENTS.md` section 3 states the file-size guideline, and section 5 states the conciseness guideline.
- `AGENTS.md` section 7 states the worktree rules, including the sibling directory, the project directory on `main`, the checks before acting, the rule about another session's directory, and the request to the human when a session cannot be started in the right worktree.
- No rule is stated in two places, and no new numbered section is created.
- The fourteen sections are unchanged in number, numbering, and title, and every `section N` cross-reference still resolves.
- The data engine's plan and log exist and claim no work that was not done.
- `git diff --check` reports no whitespace errors, and every changed file ends with a newline.
- A pull request exists against `main` that states the verification performed and the risks, and after it is merged the repository has one branch, `main`, and one worktree.

## Work breakdown

| Step | Action | Role | Paths affected | Depends on |
|---|---|---|---|---|
| 1 | Confirm the merged branches are contained in `main`, then delete them locally and on the remote | Orchestrator | git state | — |
| 2 | Create the worktree and the branch for this task from the latest `origin/main` | Implementation | git state | 1 |
| 3 | File issue #32 and add it to the board | Orchestrator | GitHub issue, board | 2 |
| 4 | Write this plan and the transparency log | Orchestrator | `docs/` | 3 |
| 5 | Amend `AGENTS.md` sections 7, 9, 3, 5, 10, and 13 | Implementation | `AGENTS.md` | 4 |
| 6 | Keep the data engine's plan and its log, corrected | Implementation | `docs/` | 4 |
| 7 | Verification pass: structure, cross-references, whitespace, newlines, and the diff read in full | Test Engineer, Code Reviewer | all changed files | 5, 6 |
| 8 | Commit, push, open the pull request | Orchestrator | git state, PR | 7 |
| 9 | Merge, mark issue #32 `Done`, remove the worktree and delete the branch | Orchestrator | board, git state | 8 |

## Verification

Commands actually executed on 2026-10-21 UTC, with observed results:

- `git fetch origin --prune`, `git branch --merged origin/main`, and `git diff --name-status origin/<branch> origin/main` for each of the six merged branches. Every branch's pull request was confirmed `MERGED` through the API, and the only file reported as deleted between a branch and `main` was `docs/2026-10-20/ai-transparency-log.md`, which pull request #8 moved to `docs/ai-transparency-log/2026-10-21/5-shell-application-surface.md`. The content is present at the new path, so no work was lost.
- `git branch -D` and `git push origin --delete` for the six branches. The repository now holds `main` only, locally and on the remote, with one worktree.
- A script parsed every `## N.` heading and every `section N` reference in the finished `AGENTS.md`: the headings are contiguous 1 through 14, the referenced set is {2, 3, 4, 6, 7, 8, 9, 11, 12, 14}, and each reference resolves to exactly one heading. No unresolved reference.
- `git diff --check` — no whitespace errors. A trailing-newline check on every changed and added file returns a newline for each.
- `git worktree list` and `git branch -a` after the cleanup of this task's worktree, confirming one branch and one worktree.

Limitations, stated explicitly:

- The guidelines are rules, and the rules in this repository that govern agent behaviour cannot be established by a test in this repository. The verification is a read against the acceptance criteria.
- No Rust code exists, so `cargo build`, `cargo test`, `cargo clippy`, and `cargo doc` do not apply to this change and no result for them is claimed.

## Risks and recovery

- **A worktree rule that agents misread as permission to work anywhere.** Mitigation: the rule names the directory, the branch, the checks before acting, and the case where a session cannot be started in the right place.
- **A cleanup rule that deletes work.** Mitigation: the cleanup is conditioned on a merged pull request, verified acceptance criteria, an issue marked `Done`, and an explicit check that nothing uncommitted or unmerged would be lost, with a blocker reported instead of a deletion when that cannot be established.
- **The file-size rule produces mechanical splits.** Mitigation: the rule says the count is a prompt rather than a target, and that a cohesive module is not broken up for the sake of it.
- **The rules contradict each other.** Mitigation: every section the change touches was read against every other, the two consistency edits in sections 10 and 13 are recorded, and the verification pass re-reads the whole file.
- **Recovery.** The change is one branch, merged through a pull request, so it can be reverted with a new commit on `main`.

## Stop conditions

Stop and request human direction if:

- A guideline cannot be stated without contradicting an existing rule in another section.
- The cleanup authorization cannot be conditioned in a way that preserves the existing gates for other destructive Git operations.
- The worktree rules cannot be applied to this task without breaking a rule in section 7 or section 9.
