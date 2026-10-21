# Execution Plan: Issues and Pull Requests on the Board

- Issue: [#15](https://github.com/quant-rule/quant-rule/issues/15) — docs: make issues and pull requests visible on the board
- Branch: `docs/15-board-issue-and-pr-visibility`
- Status: Approved
- Date: 2026-10-21 (UTC)

## Intent

The project board is where the issue lifecycle is tracked, and it must let the human see every work item's issue and the pull request that closes it. Today it holds both, but nothing on the board says which is which: an issue and its pull request carry the same title, and the visible card fields do not include anything that distinguishes them. The Roadmap view is worse: it has no visible fields and no grouping, so it cannot lay out a timeline at all.

The intended outcome is a board where both kinds of item are visible and labelled, and a Roadmap view with the fields a timeline needs.

Affected users: the human, who reads the board to see what is in flight and what is done.

## Current state

Evidence gathered on 2026-10-21 UTC, `origin/main` at commit `a07db15` (the merge of pull request #8).

- The organization project `QuantRule` is `PVT_kwDOFJhfMs4Bn-Og`. It held eight items when the survey began: issues #1, #3, #5, and #7, and pull requests #2, #4, #6, and #8, every one with `Status` = `Done`, none archived, every `Title` populated. During this work the human filed six engine issues, #9 through #14, and put them on the board, and this change's own issue #15 was put on the board by the procedure. The board therefore holds fifteen items: eleven issues and four pull requests. The pull request for this change will be added to the board by GitHub's automation when it is opened, making sixteen, and it is labelled in the follow-up commit so that no item is left without a value.
- The Board view has layout `BOARD_LAYOUT`, an empty filter, and the visible fields `Title`, `Assignees`, `Status`, `Linked pull requests`, and `Sub-issues progress`. It groups vertically by `Status`.
- The Roadmap view has layout `ROADMAP_LAYOUT`, an empty filter, no visible fields, and no grouping. The project has three date fields, `Created`, `Updated`, and `Closed`, and none of them is configured on that view. The API refuses visible fields on that view with the message "Roadmap views do not support visible fields", so its configuration cannot be set the way the Board view's can. Reading the field values showed that all three date fields are null on every item, whether the item is open or closed, so no date field on this board distinguishes an open item from a closed one.
- The project has thirteen fields: `Title`, `Assignees`, `Status`, `Labels`, `Linked pull requests`, `Milestone`, `Repository`, `Reviewers`, `Parent issue`, `Sub-issues progress`, `Created`, `Updated`, and `Closed`. There is no field that records whether an item is an issue or a pull request.
- The pull request items were not added by the procedure. The timeline of every pull request shows an `added_to_project_v2` event within a second or two of the pull request being opened, because its body closes an issue that was already tracked, and no add call was made for any of them.
- An issue and its pull request carry the same title. Issue #1 and pull request #2 are both "docs: establish AI agent guidelines", and the same holds for #3 and #4, #5 and #6, and #7 and #8.
- The `Labels` field is a free discriminator that is already in the data: the issues carry the `documentation` label and the pull requests carry none.
- The API cannot express everything a view needs. The whole mutation set was enumerated: `updateProjectV2View` accepts `name`, `layout`, `filter`, and `configuration`, and `ProjectV2ViewConfigurationInput` accepts only `visibleFieldIds`. No mutation accepts a grouping, and no mutation accepts a filter at view creation. A view reads back `filter`, `configuration`, `groupByFields`, and `verticalGroupByFields`, and the visible fields are read through `configuration.visibleFields` on `ProjectV2ViewConfiguration`.
- The repository contains no Rust source, no `Cargo.toml`, no `tests/` directory, and no CI configuration. `cargo --version` returns `cargo 1.99.0`. No build, test, lint, or doc run applies to this change, and none is claimed.
- Issue #15 was created at `2026-10-21T01:13:58Z` with the `documentation` label and was moved to `In Progress` on the project board before this change was made to the working tree.

## Scope

Included:

- Add an `Item type` single-select field to the project, with the options `Issue` and `Pull request`.
- Add a `State` single-select field to the project, with the options `Open` and `Closed`.
- Populate `Item type` and `State` on all fifteen items, so that every issue reads `Issue` and every pull request reads `Pull request`, and so that an open item reads `Open` and a closed or merged one reads `Closed`.
- Add `Item type`, `State`, and `Labels` to the visible fields of the Board view, additively.
- Add an `All items` view with a table layout, listing every item — open issues, closed issues, and pull requests — in one list.
- Rename the Roadmap view to `Timeline` and leave its layout and filter as they are, because the API rejects visible fields for that layout. The date field its timeline is drawn from is chosen by the human in the browser.
- Add a `Board Contents` subsection to `AGENTS.md` section 9 recording what the board holds and how the two kinds of item are told apart.
- Record this change in an execution plan and an AI transparency log entry.

Explicit non-goals:

- No change to any issue or pull request, and none to any title. The board's item count moves on its own during this work, because the human filed six engine issues and this change's own issue is on the board, and because GitHub's automation adds each pull request as it is opened; that is recorded rather than presented as a choice. The four pairs of duplicated titles are left as they are: distinguishing them is the job of a field, not of a rewrite of history.
- No change to the `AGENTS.md` issue status lifecycle, the approval gates, the orchestration protocol, or the role model.
- No deletion of the Roadmap view, and no attempt to make it a Board layout. The human asked for it to be made to work, not removed.
- No new filtered views. The human chose the `Item type` column and grouping by it, over creating separate `Issues` and `Pull requests` views, because the API cannot report which items a view displays and a filter written blind cannot be verified.
- No Rust source, no `Cargo.toml`, no `tests/` directory, no CI configuration.
- No change to any other repository.
- No merge of the pull request. That remains with the human.

## Assumptions and uncertainty

- **The human's report that the issues are missing from the board is not reproduced by the data.** Every issue is an item, every view has an empty filter, and no item is archived. At the time of the survey the four completed issues and their four pull requests all read `Status` = `Done`; the six engine issues the human filed afterwards read `Todo`. The most likely explanations are that the Roadmap view renders nothing, that the duplicated titles make an issue card and its pull request card indistinguishable, or that the card fields on offer do not include a discriminator. This change makes the board unambiguous under all three explanations, and it cannot rule out a browser-side condition that the API cannot see.
- **The field is named `Item type` rather than `Type`.** GitHub's new project interface has a built-in `Type` column, and a custom field with the same name would be confusing next to it. `Item type` is unambiguous and cannot collide.
- **A pull request is added to the board by GitHub's automation, not by the procedure.** The timeline of every pull request shows an `added_to_project_v2` event within a second or two of the pull request being opened, and no add call was ever made for one. The attribution of that event does not distinguish an issue from a pull request, because fourteen of the fifteen items carry it, including the issues that were put on the board deliberately; only issue #1 carries the account that added it. The rule is therefore about what the automation does, not about who the event names.
- **The `Labels` field is a second signal that costs nothing.** The issues carry the `documentation` label and the pull requests do not, which is a property of how issues are labelled rather than a rule, so nothing depends on it. Adding it to the visible fields gives the human a second reading without any new data.
- **The Roadmap view's timeline axis needs one click from the human.** No mutation accepts a grouping. The API can give the view the date fields to choose from; the human chooses the axis. This is the only manual step in the change.
- The human confirmed the goal, the `Item type` field over filtered views, and the full procedure on 2026-10-21.

## Decisions

| Decision | Options considered | Choice | Rationale | Approval |
|---|---|---|---|---|
| How the two kinds are told apart | A new `Item type` field; the built-in `Labels` field; separate filtered views; renaming the issues | A new `Item type` field, with `Labels` also made visible | A field is data the board renders, sorts, and groups by. `Labels` alone is an accident of how issues are labelled, and it is a free second signal. Renaming closed issues would rewrite the record of work already done. | Human, 2026-10-21 |
| What the board holds | Issues only; issues and pull requests | Issues and pull requests | The human asked for both to be visible. The pull request carries the merge, which the board does not otherwise show. | Human, 2026-10-21 |
| The Roadmap view | Delete it; leave it broken; make it work | Make it work | The human asked for it to work. It has no visible fields and no grouping, both of which can be improved, and the date fields to draw a timeline from already exist on the project. | Human, 2026-10-21 |
| Extra views for issues and pull requests | Create them; use the `Item type` column and group by it | Use the `Item type` column | A view filter written through an API that cannot report which items a view displays cannot be verified before the human looks at it. | Human, 2026-10-21 |
| Name of the new field | `Type`; `Item type` | `Item type` | GitHub's interface has a built-in `Type` column. A custom field named `Type` would sit next to it and be indistinguishable. | Agent, within the approved scope |
| Effect on the existing visible fields | Replace the set; add to it | Add to it | The visible fields are additive, so nothing that works today disappears. | Agent, within the approved scope |
| How open and closed items are shown | A `State` field; the built-in `Closed` date field; the `Status` field alone | A `State` field | Every built-in date field is null on every item of this board, so none of them says whether an item is still open. `Status` describes the work item's place in the lifecycle, which is not the same question. | Agent, within the approved scope |
| Where every item is listed at once | The `Board` view alone; a table view named `All items` | A table view named `All items` | The board view separates the same items across the columns of the lifecycle, so open and closed items are never in one place. | Agent, within the approved scope |

## Acceptance criteria

Observable conditions for completion:

- The project has a field named `Item type` of type single-select, with an `Issue` option and a `Pull request` option.
- All eleven issues carry the value `Issue`, and all four pull requests carry the value `Pull request`.
- The project has a field named `State` of type single-select, with an `Open` option and a `Closed` option, and every item carries the value that matches its GitHub state, a merged pull request being closed.
- The Board view's visible fields include `Item type`, `State`, and `Labels`, and its filter is empty.
- An `All items` view exists with a table layout, and its columns include `Item type` and `State`.
- The Roadmap view is named `Timeline`, and it is recorded that the API cannot configure its fields or its grouping.
- The board holds fifteen items plus the pull request that GitHub's automation adds for this change, every issue and pull request with the `Status` its lifecycle gives it, none archived, and no issue or pull request renamed. The sixteenth item, that pull request, is labelled with `Item type` = `Pull request` and `State` = `Closed` in the follow-up commit, so that no item on the board is left without a value.
- `AGENTS.md` section 9 states what the board holds, that a closing pull request is added by GitHub's automation, and how an issue and its pull request are told apart.
- The fourteen numbered sections of `AGENTS.md`, their numbering, and their cross-references are unchanged, and every cross-reference still resolves.
- This plan exists at `docs/execution-plans/2026-10-21/15-board-issue-and-pr-visibility.md`, and the transparency log entry exists at `docs/ai-transparency-log/2026-10-21/15-board-issue-and-pr-visibility.md`.
- A pull request exists against `main` that closes #15, states the verification performed, and states the risks.

## Work breakdown

| Step | Action | Role | Paths affected | Depends on |
|---|---|---|---|---|
| 1 | File issue #15, add it to the board, move it to `In Progress` | Orchestrator | GitHub issue and board | — |
| 2 | Create the task branch from the latest `origin/main` | Implementation | git state | 1 |
| 3 | Write this execution plan at its new location, mark `Approved` | Orchestrator | `docs/execution-plans/2026-10-21/15-board-issue-and-pr-visibility.md` | 2 |
| 4 | Create the `Item type` field on the project | Implementation | project board | 3 |
| 5 | Populate `Item type` and `State` on the fifteen items | Implementation | project board | 4 |
| 6 | Add `Item type`, `State`, and `Labels` to the Board view's visible fields; rename the Roadmap view to `Timeline`; create the `All items` table view | Implementation | project board | 4 |
| 7 | Add the `Board Contents` subsection to `AGENTS.md` section 9 | Implementation | `AGENTS.md` | 3 |
| 8 | Write the transparency log entry for this change | Orchestrator | `docs/ai-transparency-log/2026-10-21/15-board-issue-and-pr-visibility.md` | 7 |
| 9 | Verification pass: read the board back, check every item's value, check both views, check the `AGENTS.md` structure, check the diff | Test Engineer, Code Reviewer | project board, all changed files | 5, 6, 7, 8 |
| 10 | Mark this plan `Verified` with observed evidence | Orchestrator | this plan | 9 |
| 11 | Commit, push, open pull request | Orchestrator | git state, PR | 10 |
| 12 | Move issue #15 to `Done` on the board | Orchestrator | GitHub project board | 11 |
| 13 | Stop for the human's merge authorization | Orchestrator | — | 12 |

## Verification

### Verification evidence

Commands actually executed on 2026-10-21 UTC, with observed results:

- `git fetch origin --prune`, `git log --oneline -1 main`, `git log --oneline -1 origin/main`, `git rev-list --left-right --count main...origin/main` — the local `main` and `origin/main` were both at `a07db15`, with a count of `0 0`, so the local branch was not stale.
- `git checkout -b docs/15-board-issue-and-pr-visibility origin/main` followed by `git merge-base --is-ancestor origin/main HEAD` — exits zero, and `git diff --stat origin/main..HEAD` is empty, so the branch started from an identical tree.
- A GraphQL query over `projectV2.items` at the time of the survey — eight items, of which four are `ISSUE` and four are `PULL_REQUEST`, every one with `Status` = `Done`, `isArchived` = false, and a populated `Title`. The same query after the change returns fifteen items, and a read-back after the pull request is opened returns sixteen.
- A GraphQL query over `projectV2.views` — two views. `Board` has layout `BOARD_LAYOUT`, filter `''`, visible fields `Title`, `Assignees`, `Status`, `Linked pull requests`, `Sub-issues progress`, and groups vertically by `Status`. `Roadmap` has layout `ROADMAP_LAYOUT`, filter `''`, no visible fields, and no grouping.
- A GraphQL query over `projectV2.fields` — thirteen fields, listed in the current state above, none of which records the kind of item.
- Introspection of `__schema.mutationType` — the complete list of `ProjectV2` mutations was enumerated, and `UpdateProjectV2ViewInput` was inspected: it accepts `viewId`, `name`, `layout`, `filter`, and `configuration`, and `ProjectV2ViewConfigurationInput` accepts only `visibleFieldIds`. No mutation accepts a grouping, and `CreateProjectV2ViewInput` accepts no filter.
- `gh api repos/quant-rule/quant-rule/issues/N/timeline` for every issue and pull request — the `added_to_project_v2` event for each pull request occurs within a second or two of the pull request being opened, and no add call was made for any pull request. The attribution of that event does not distinguish an issue from a pull request: fourteen of the fifteen items carry `github-project-automation[bot]`, including the issues that were put on the board deliberately, and only issue #1 carries the account that added it.
- `gh project item-list 1 --owner quant-rule --format json` and a GraphQL query over `projectV2.items` — the item ids and the kind of every item, used to populate the field. The board held fifteen items at the time of the change: eleven issues and four pull requests.
- `gh project field-list 1 --owner quant-rule --format json` — the thirteen field ids and the three `Status` option ids, used to set the visible fields.
- `updateProjectV2View` with `configuration.visibleFieldIds` on the Roadmap view returns the error "Roadmap views do not support visible fields". The view was therefore renamed with `name` and its `layout` and `filter` left as they were, and the date field its timeline uses is left to the human.
- `gh project field-create 1 --owner quant-rule --name "State" --data-type SINGLE_SELECT --single-select-options "Open,Closed"` — the field is `PVTSSF_lADOFJhfMs4Bn-Ogzhmhkwo`, with the options `a7b14cd3` = `Open` and `ea1738d6` = `Closed`. A second pass of fifteen `gh project item-edit` calls set the value on every item from the item's GitHub state, mapping `OPEN` to `Open` and `CLOSED` and `MERGED` to `Closed`.
- A GraphQL query over `fieldValueByName` for `Created`, `Updated`, and `Closed` returned null for every one of the fifteen items, which is what established that a `State` field was needed at all.
- `createProjectV2View` with layout `TABLE_LAYOUT` created the `All items` view, `PVTV_lADOFJhfMs4Bn-OgzgMKRYQ`, and `updateProjectV2View` set its columns to `Title`, `Status`, `Item type`, `State`, `Labels`, and `Linked pull requests`. Reading all three views back confirms their names, layouts, filters, and visible fields.
- After the change: `gh project field-list` shows the `Item type` and `State` fields and their options; a script reads each item's `Item type` and `State` through the GraphQL API and asserts that they match the item's kind and its GitHub state; the archived flag is unchanged on every item; all three views' `visibleFields` and `filter` are read back.
- `grep -c "^## " AGENTS.md` — 14 top-level headings, contiguous 1 to 14. A script parses every `## N.` heading and every `section N` reference: each reference resolves to exactly one heading.
- `git diff --cached --check` — no whitespace errors. `tail -c 1` over every Markdown file returns a newline.
- The plan is checked against the ten required contents in `AGENTS.md` section 11, and the transparency log entry against the four required items in section 6.
- The complete diff was reviewed line by line before staging.

Acceptance criteria are evaluated against the evidence above. All are met except the one that depends on the pull request existing, which is satisfied when the pull request is opened. This plan is marked `Approved` rather than `Verified` at that point, because that criterion is not yet met; the status changes to `Verified` in the follow-up commit that records the pull request, following the precedent set by the earlier entries in this repository.

Limitations, stated explicitly:

- The repository contains no Rust source, no `Cargo.toml`, and no `tests/` directory, so `cargo build`, `cargo test`, `cargo clippy`, and `cargo doc` do not apply to this change. `cargo --version` was run and establishes toolchain availability only. No build, test, lint, or doc result is claimed.
- The API reports a project's items and a view's configuration, but it does not report which items a view displays, and it cannot render a view. The visual result is therefore verified by the human, and this change cannot prove that the issue cards are now visible on their screen. It can only prove that the data and the view configuration now say they should be.
- No automated markdown linter or link checker exists in the repository. Verification of a prose change is a manual read against the acceptance criteria, not a machine-checked result.

## Risks and recovery

- **The Roadmap view still needs a click.** Its timeline axis is a grouping, and no mutation sets one; the API goes further and refuses visible fields for that layout with "Roadmap views do not support visible fields". Mitigation: the view is renamed `Timeline`, which says what it is for, the project already carries the three date fields the axis can be drawn from, and the one manual step is stated in the report and in the pull request.
- **The human's report is not reproduced by the data.** Mitigation: the change is aimed at the observable causes, which are the indistinguishable titles and the absence of a discriminator field. If the issues still do not appear after it, the remaining causes are in the browser, and the report says so rather than claiming the problem is fixed.
- **A custom field duplicates what the card badge shows.** Mitigation: `gh project field-delete` removes it in one command if the human finds it redundant, and the field carries no other responsibility.
- **Two custom fields now carry state the interface can show itself.** Mitigation: both are populated from the API on every item, both are visible on the board and in the table, and `gh project field-delete` removes either in one command. A field that is never populated would be a defect, so the verification pass asserts a value on every item rather than checking that the field exists.
- **The visible fields of the Board view grow.** Mitigation: the change is additive, so no field that works today disappears, and `Assignees` and `Sub-issues progress` are both empty for this repository, so the extra column costs nothing.
- **The board's item count moves while this change is in flight.** Six engine issues and this change's own issue were added to the board during the work, and this change's pull request is added by the automation when it is opened. Mitigation: the field is populated on every item the board holds at the time of the change, and the verification pass reads the count and the values back rather than assuming them.
- **A board write goes to the wrong item.** Mitigation: the item ids are read from the API and the kind of each item is read from the same API, and the verification pass asserts the value on every item individually.
- **Recovery.** The two fields and their values are removed with `gh project field-delete` and `gh project item-edit --clear`, the view configuration is restored with one `updateProjectV2View` per view, the `All items` view is removed with `deleteProjectV2View`, and the documentation change is reverted with a new commit on `main`.

## Stop conditions

Stop and request human direction if:

- The `Item type` field cannot be created or populated, and the field is the only way to tell the two kinds apart.
- Setting the visible fields would remove a field that is in use, since this plan commits to an additive change only.
- The push, the pull request, or the merge is rejected by repository protection settings.
- Any action would modify a repository other than `quant-rule/quant-rule`.
