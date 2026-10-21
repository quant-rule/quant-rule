# Execution Plan: Date the board so the timeline can show each item's life

- Issue: [#42](https://github.com/quant-rule/quant-rule/issues/42) — feat: date the board so the timeline can show each item's life
- Branch: `feat/38-timeline-dates`
- Worktree: `/home/zantgo/Projects/quant-rule--38-timeline-dates`
- Status: Approved
- Date: 2026-10-21 (UTC; 2026-10-20 local time)
- Transparency log: [`docs/ai-transparency-log/2026-10-21/42-timeline-dates.md`](../../ai-transparency-log/2026-10-21/42-timeline-dates.md)

## Intent

The project's Timeline view says that it "needs at least one date or iteration field to get started", and it has none it can draw. The board holds the built-in `Created`, `Updated`, and `Closed` fields, but they carry no values on this board and the view does not offer them.

This work item gives the board two date fields the view can use, keeps them current, and fills them in for the work that is already done.

Affected users: the human developer, who reads the Timeline to see what is open and when things finished.

## Current state

Evidence gathered on 2026-10-21 UTC, `origin/main` at commit `777c5d2` (the merge of pull request #41).

- The project's fields were read through the API. It holds fifteen: `Title`, `Assignees`, `Status`, `Labels`, `Linked pull requests`, `Milestone`, `Repository`, `Reviewers`, `Parent issue`, `Sub-issues progress`, `Created`, `Updated`, `Closed`, `Item type`, and `State`. The three built-in date fields carry no values, which the API confirms for every item.
- The `createProjectV2Field` mutation exists and its `dataType` accepts `DATE` from `ProjectV2CustomFieldType`. Two date fields were created: `Start date`, `PVTF_lADOFJhfMs4Bn-OgzhmjWZU`, and `Closed date`, `PVTF_lADOFJhfMs4Bn-OgzhmjWaA`.
- `Issue.closedAt` and `PullRequest.closedAt` are readable, so a close can be dated from the event's own record rather than from a clock the workflow reads.
- The Timeline view's date field is not settable through the API, which `AGENTS.md` section 9 already records: the API sets the view's name and layout, and the human picks the axis in the browser.
- A merged pull request reports its state as `MERGED` rather than `CLOSED`, which the script's first version of the date path did not account for.

## Scope

Included:

- The two date fields on the project, and the script that writes the `Closed date`.
- `AGENTS.md` section 9: the rule that says who writes which field, and the note on the Timeline view.
- A backfill of the `Closed date` of the twenty-one items that are already closed or merged.
- This plan and the transparency log for this work item.

Explicit non-goals:

- No `Target date` field. A target date is a commitment, and the repository has no record of the human's commitments; inventing one would put a date on the board that nobody chose.
- No `Start date` for the work already done. The date the work began is known only to the session that did it, and the closed items are already dated by their close.
- No change to the Timeline view's configuration, which is the human's in the browser.
- No change to the rule that a container's status follows its children.

## Assumptions and uncertainty

- **The closed date comes from the issue's own record.** The script reads `closedAt` and writes its date, so the board's date is the date of the event rather than the date the workflow happened to run.
- **The start date is the agent's.** The agent moves a leaf to `In Progress` and sets the date with it, because the agent is the only party that knows the work has begun. A board write is not an issue event, so no workflow can learn it.
- **An open item carries no closed date.** The script clears the field when an item is open, so a reopened issue does not keep the date it closed on.
- **The fields are the board's, not the repository's.** They are created through the API and live in the project's configuration. The repository records who writes them.

## Decisions

| # | Open point | Options | Choice | Rationale |
|---|---|---|---|---|
| K1 | Which date fields to add | Start and Closed; Target and Closed; one date field | `Start date` and `Closed date` | The question is when an item was open and when it closed, and those are exactly the two. A target date is a commitment nobody has made, and a single field cannot show a life |
| K2 | Who writes the closed date | The workflow, from `closedAt`; the workflow, from its own clock | The workflow, from `closedAt` | The issue's own record is the event's date, and a workflow that reads a clock would date a close by when it noticed |
| K3 | Who writes the start date | The agent; the workflow | The agent | Moving a leaf to `In Progress` is a board write and raises no issue event, so no workflow can learn that work has begun |
| K4 | Whether the history is backfilled | Yes; no | Yes | The closed items are the history the timeline is for, and the closed date is exact rather than estimated |
| K5 | Whether the start date is backfilled too | Yes, from the issue's creation; no | No | The issue's creation is when it was filed, not when the work began, and a field that means one thing going forward and another in the history is a field that misleads |
| K6 | How a merged pull request is recognised | By its state being `CLOSED`; by anything that is not open | Anything that is not open | A merged pull request reports `MERGED`, which the first version of the date path did not match, so its date was left empty |

## Acceptance criteria

Observable conditions for completion:

- The project holds a `Start date` and a `Closed date` field, both of type date.
- Closing an issue sets its item's `Closed date` to that day, and reopening the issue clears it.
- Every closed or merged item on the board carries its `Closed date`, and no open item carries one.
- `AGENTS.md` section 9 names the writer of each field.
- `bash -n` is clean on the script, and the date path is exercised against the live board.
- A pull request exists against `main` that states the verification performed and the risks, and the branch and its worktree are removed after the merge.

## Work breakdown

| Step | Action | Role | Paths affected | Depends on |
|---|---|---|---|---|
| 1 | File issue #42, add it to the board, move it to `In Progress` | Orchestrator | GitHub issue, board | — |
| 2 | Create the worktree and the branch from the latest `origin/main` | Implementation | git state | 1 |
| 3 | Write this plan and the transparency log | Orchestrator | `docs/` | 2 |
| 4 | Create the two date fields on the project | Implementation | project configuration | 2 |
| 5 | Extend the script to write and clear the closed date | Implementation | `.github/scripts/project-sync.sh` | 4 |
| 6 | Amend `AGENTS.md` section 9 | Implementation | `AGENTS.md` | 3 |
| 7 | Backfill the closed date of every closed and merged item | Implementation | project configuration | 5 |
| 8 | Commit, push, open the pull request | Orchestrator | git state, PR | 6, 7 |
| 9 | Merge, move issue #42 to `Done`, remove the worktree and delete the branch | Orchestrator | board, git state | 8 |

## Verification

Commands actually executed on 2026-10-21 UTC, with observed results:

- The project's fields were read before and after: fifteen fields before, seventeen after, the two new ones of type `DATE` with the ids `PVTF_lADOFJhfMs4Bn-OgzhmjWZU` and `PVTF_lADOFJhfMs4Bn-OgzhmjWaA`.
- The `createProjectV2Field` mutation was called for each name, and the response returned the field's id, name, and type.
- `bash -n .github/scripts/project-sync.sh` — no syntax errors.
- The script was run against a closed issue, a merged pull request, and an open issue. Issue #1 took the closed date `2026-10-20`, which is the day its own `closedAt` records. Pull requests #2 and #4 took the same treatment, after the state test was changed to match anything that is not open, because a merged pull request reports `MERGED`.
- Every closed and merged item was then run through the script, and a read-back reports twenty-one items with a closed date, none missing, and no open item carrying one.
- `git diff --check` — no whitespace errors; the script ends with a newline and carries the executable bit.

Limitations, stated explicitly:

- The Timeline view's date field is chosen by the human in the browser, because the API does not expose it. The fields exist and carry values; the axis is the human's to pick.
- The `Start date` is empty for every item on the board today, because the work has not been started in the new arrangement. It is filled when the agent begins an item.
- The history's start dates are absent, by the decision in K5.

## Risks and recovery

- **A date that means two things.** Mitigation: the closed date is exact and the start date is not backfilled, so neither field carries a value it cannot support.
- **The workflow writes a date it should not.** It writes only the closed date, and only from the issue's own record, and it clears the field when the item is open.
- **The board's dates and its status disagree.** They are written from the same event, in the same run, so they move together.
- **Recovery.** The fields are removed through the API, the change is one branch merged through a pull request, and the backfill is re-run by the script. A wrong date is corrected by re-running it.

## Stop conditions

Stop and request human direction if:

- The date fields cannot be created on the project, or the Timeline still refuses them.
- The human wants a `Target date` field, which is a commitment the repository cannot infer.
- The backfill would put a date on an item whose close the board does not record.
