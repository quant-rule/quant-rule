# Execution Plan: A container's board status follows its sub-issues

- Issue: [#36](https://github.com/quant-rule/quant-rule/issues/36) — docs: make a container's board status follow its sub-issues
- Branch: `docs/36-container-status`
- Worktree: `/home/zantgo/Projects/quant-rule--36-container-status`
- Status: Verified — issue closed; historical record
- Date: 2026-10-21 (UTC; 2026-10-20 local time)
- Transparency log: [`docs/ai-transparency-log/2026-10-21/36-container-status.md`](../../ai-transparency-log/2026-10-21/36-container-status.md)

## Intent

The board read as though the model engine were built while its fourteen sub-issues sat in `Todo`. The cause was a rule: a container was `Done` when its own deliverable was merged, and it was never `In Progress` while one of its children was.

The human replaced that rule with a simpler one — a parent stays in progress unless all its sub-issues are done — and asked for it to be the rule always. This work item states it, brings the documents that describe the old rule into line, and repairs the state the old rule produced.

Affected users: the human developer, who reads the board to see what is in flight; and the agents, who are told to follow the new rule.

## Current state

Evidence gathered on 2026-10-21 UTC, `origin/main` at commit `f2ec7ee` (the merge of pull request #35).

- `AGENTS.md` section 9's lifecycle rules held the container rule as "A container is `Done` when its own deliverable is delivered, and it is not `In Progress` while one of its sub-issues is, because one issue is `In Progress` at a time", and held "One issue is `In Progress` at a time unless the human explicitly authorizes parallel work."
- The human moved issue #14 on the board from `Done` to `In Progress` and stated that a parent must be in progress unless all its sub-issues are done.
- The state that rule produced was read directly: issue #14 was `CLOSED` on GitHub with all fourteen of its sub-issues `OPEN`, and its board item read `Status` = In Progress with `State` = Closed. The board's own "Auto-close issue" workflow had closed the issue when the item was marked `Done`, so the two states were coupled by the board's behaviour rather than by a choice.
- Four documents described the old rule: the execution plans for issues #14 and #32, and their transparency logs.
- The project's seven built-in workflows were read through the API, and GitHub's documentation for the built-in automations was consulted. The built-ins write the `Status` field only: an item added takes `Todo`, an issue closed takes `Done`, a pull request merged takes `Done`, and an item marked `Done` has its issue closed. No built-in recomputes a container's status from its children, and none touches the custom `State` or `Item type` fields.

## Scope

Included:

- `AGENTS.md` section 9: the container rule, the scope of the single `In Progress` slot, the exception for a container's move to `Done`, and a new rule on who maintains the board.
- Issue #14's body: the umbrella line and its acceptance criteria.
- The four documents that describe the old rule: the two execution plans are amended, and the two transparency logs gain an entry recording the change.
- The repair: issue #14 is reopened on GitHub, and its board `State` returns to `Open`.
- This plan and the transparency log for this work item.

Explicit non-goals:

- No workflow, no App, and no credential. The automation that will maintain the board unattended is issue #37, and it lands after the human registers a GitHub App.
- No change to the fourteen numbered sections, their numbering, their titles, or their cross-references.
- No change to the built-in project workflows, which are the board's own and are configured in the browser.
- No change to the issue bodies of the fourteen sub-issues, whose criteria are already the build criteria.

## Assumptions and uncertainty

- **The rule is the human's, not the agent's.** The previous rule was the agent's reading of a single-slot constraint, and the human has replaced it. The work item records that, so the next agent does not reason its way back.
- **A container mirrors its children and is not the work in flight.** The single `In Progress` slot is scoped to work items, so the board may show a container and one child in progress at once. That is the intended reading, and the scoping states it.
- **The coupling between the board's `Status` and the GitHub issue's own state is the board's behaviour.** Marking an item `Done` closes its issue, and the rule now keeps a container out of `Done` until its children are done, so the container's issue stays open while its work is in flight. The automation in issue #37 reopens a container whose child reopens.
- **The custom fields stay the agent's until the workflow exists.** The new rule says a repository workflow owns the `State` and `Item type` fields and recomputes a container's status, and that until it is in place the agent does that work. The clause is removed when issue #37 lands.
- **The rules that govern agent behaviour cannot be tested in this repository.** The verification is a read against the acceptance criteria.

## Decisions

| # | Open point | Options | Choice | Rationale |
|---|---|---|---|---|
| H1 | What a container's status is while a child is open | In Progress; Todo; leave it to the agent | `In Progress` | A container whose title says what it delivers reads as active work while its children are open, which is what the board is for. `Todo` would say the work has not begun, which is false |
| H2 | Whether a container occupies the single `In Progress` slot | It does; it does not; the rule is dropped | It does not; the slot is scoped to work items | The slot exists so that one thing is being worked at a time. A container is not being worked, it is reporting |
| H3 | Whether a container's move to `Done` needs a pull request of its own | Yes; no | No | Its own deliverable was merged when it was delivered. The last child's merge is what completes it, and requiring a second pull request for a container would be work without a deliverable |
| H4 | Who maintains the board in the meantime | The agent; the built-in workflows; a repository workflow | The agent now, a repository workflow once issue #37 lands | The built-in workflows write the `Status` field and cannot recompute a container, so nothing covers the rule until the workflow exists. The rule says so, so the state of the world is stated rather than implied |
| H5 | Whether the old rule's damage is repaired | Reopen the issue and fix the field; leave it | Reopen and fix | The issue was closed by the board's own workflow as a consequence of the old rule, and the state contradicted the rule the human has just stated. Leaving it would leave the board showing a closed container above fourteen open children |
| H6 | What happens to the documents that state the old rule | Amend them; append to them | The plans are amended, the logs are appended to | A plan states what was decided and is corrected when the decision changes. A transparency log is a record of what happened, so the change is a new entry and the earlier entry stands |

## Acceptance criteria

Observable conditions for completion:

- `AGENTS.md` section 9 states that a container's status follows its children, scopes the single `In Progress` slot to work items, and states the exception for a container's move to `Done`.
- `AGENTS.md` section 9 states who maintains the board: the built-in workflows for the `Status` field, a repository workflow for the custom fields and a container's status once it exists, and the agent in the meantime.
- Issue #14's body states that the issue stays `In Progress` until every one of its fourteen sub-issues is done.
- No document in the repository states the old rule.
- Issue #14 is open on GitHub, its board `Status` is `In Progress` and its `State` is `Open`, and its fourteen sub-issues are open.
- The fourteen sections are unchanged in number, numbering, and title, and every `section N` cross-reference still resolves.
- `git diff --check` reports no whitespace errors, every changed file ends with a newline, and a link check reports no broken relative links.
- A pull request exists against `main` that states the verification performed and the risks.

## Work breakdown

| Step | Action | Role | Paths affected | Depends on |
|---|---|---|---|---|
| 1 | File issue #36, add it to the board, move it to `In Progress` | Orchestrator | GitHub issue, board | — |
| 2 | Create the worktree and the branch from the latest `origin/main` | Implementation | git state | 1 |
| 3 | Write this plan and the transparency log | Orchestrator | `docs/` | 2 |
| 4 | Amend `AGENTS.md` section 9 | Implementation | `AGENTS.md` | 3 |
| 5 | Amend the two execution plans; append to the two transparency logs | Implementation | `docs/` | 3 |
| 6 | Update issue #14's body; reopen it; set its board `State` to `Open` | Implementation | GitHub issue, board | 4 |
| 7 | Verification pass | Test Engineer, Code Reviewer | all changed files | 4, 5, 6 |
| 8 | Commit, push, open the pull request | Orchestrator | git state, PR | 7 |
| 9 | Merge, move issue #36 to `Done`, remove the worktree and delete the branch | Orchestrator | board, git state | 8 |

## Verification

Commands actually executed on 2026-10-21 UTC, with observed results:

- The board was read before the change: the seven built-in workflows, their enabled state, the project's `Status`, `State`, and `Item type` fields with their option ids, issue #14's closed state with fourteen open sub-issues, and the `parent`, `subIssues`, and `reopenIssue` API shapes the automation in issue #37 relies on.
- A script parsed every `## N.` heading and every `section N` reference in the finished `AGENTS.md`: the headings are contiguous 1 through 14, and each reference resolves to exactly one heading.
- A search over the repository finds no document that still states the old rule.
- `git diff --check` — no whitespace errors. A trailing-newline check on every changed and added file returns a newline for each. A link check over `docs/` reports no broken relative links.
- A board read-back after the repair shows issue #14 open on GitHub with `Status` = In Progress and `State` = Open, its fourteen sub-issues open in `Todo`, and issue #36 in `In Progress`.

Limitations, stated explicitly:

- The rules in this repository that govern agent behaviour cannot be established by a test in this repository.
- The board's built-in workflow configurations are not readable through the API, so the action behind "Item closed" is confirmed from GitHub's documentation rather than from the project itself.
- No Rust code exists, so no `cargo` command applies to this change and no result for one is claimed.

## Risks and recovery

- **The board shows two items in progress.** A container and the child being worked. Mitigation: the rule states the slot is scoped to work items, and the container's title says what it delivers rather than what it is waiting for.
- **The rule is enforced by an agent until issue #37 lands.** A sub-issue closed by the human, in the browser, leaves the parent stale. Mitigation: the rule says the board is kept in step by workflows, and the clause names the agent's part until the workflow exists, so the gap is stated rather than hidden.
- **The container's issue and its board item are coupled by the board's own workflow.** Reopening a container's issue is therefore not a free action. Mitigation: the repair reopens the issue and sets the field explicitly, and the automation in issue #37 does the same whenever a child reopens.
- **Recovery.** The change is one branch, merged through a pull request, so it can be reverted with a new commit on `main`. The reopened issue and the board fields are one API call each and can be put back.

## Stop conditions

Stop and request human direction if:

- The rule cannot be stated without contradicting another section of `AGENTS.md`.
- The repair would need a change to the board's built-in workflows, which are configured in the browser.
- The human's rule turns out to mean something other than a parent's status following its children.
