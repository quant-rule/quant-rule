# Execution Plan: The Docs Directory Layout

- Issue: [#7](https://github.com/quant-rule/quant-rule/issues/7) — docs: reorganize the docs directory layout
- Branch: `docs/7-docs-directory-layout`
- Status: Approved
- Date: 2026-10-21 (UTC)

## Intent

The documentation tree grows downward from the date, which buries the kind of document it holds and forces a reader to know the date before knowing what they are looking for. The layout becomes `docs/<kind>/<UTC-date>/<file>`, with the kind first, so the tree is browsable by what a document is. The `exec-plans` name goes, because it reads as a working abbreviation rather than a name, and the AI transparency log becomes a folder of one file per work item instead of one file per date.

Affected users: the human developer, who navigates the repository; and the agents, whose rules in `AGENTS.md` point at these paths.

## Current state

Evidence gathered on 2026-10-21 UTC, `origin/main` at commit `7519c7c` (the merge of pull request #6).

- The tree is `docs/2026-10-20/ai-transparency-log.md` plus `docs/2026-10-20/exec-plans/` holding three plans. Four files in total, under one date directory.
- The date directory is named after the local date. The work items are not in the UTC date they belong to: the shell-application work item ran at `2026-10-21T00:31:44Z` through `2026-10-21T00:35:04Z`, which is 2026-10-20 in the repository's local time (`UTC-3`). `AGENTS.md` section 12 states that all timestamps are UTC, so the current folder name contradicts it.
- `AGENTS.md` references the old layout in five places. Section 5 line 78 opens "Repository layout under `docs/`" and its three sub-bullets name `docs/<YYYY-MM-DD>/exec-plans/<issue>-<slug>.md`, `docs/<YYYY-MM-DD>/ai-transparency-log.md`, and `docs/<YYYY-MM-DD>/` for architecture decision records. Section 6 line 89 names `docs/<YYYY-MM-DD>/ai-transparency-log.md`. Section 7 line 158 names the same path in the pull request requirements. Section 11 line 433 names `docs/<YYYY-MM-DD>/exec-plans/<issue>-<slug>.md`. Section 13 line 537 names the transparency log path.
- The three execution plans contain four old-layout path references each, in their acceptance criteria and work-breakdown tables. Plan 1 line 21 also records the observation "No `docs/<date>/` directory has ever been created", which uses the old notation.
- The transparency log is one file with a single H1 and three `## Entry N — ...` sections. Its entries carry a `Date and tool` field, an `Intent and boundary` field, a `Verification and evidence` field, and a `Learning` field, which are the four items `AGENTS.md` section 6 requires.
- The architecture decision record location does not exist and is explicitly marked in `AGENTS.md` as a future location. An empty directory cannot be tracked by Git, so this path is created with the first decision rather than now.
- The repository contains no Rust source, no `Cargo.toml`, no `tests/` directory, and no CI configuration. `cargo --version` returns `cargo 1.99.0`. No build, test, lint, or doc run applies to this change, and none is claimed.
- Issue #7 is the only open issue. It was created at `2026-10-21T00:42:22Z` with the `documentation` label and was moved to `In Progress` on the project board before this change was made to the working tree.

## Scope

Included:

- Move the three execution plans to `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md`, dated by the UTC date of the work item.
- Split the AI transparency log into `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md`, one file per work item, each base name matching its execution plan.
- Update the five path references in `AGENTS.md`: sections 5, 6, 7, 11, and 13. State the layout rule, that the date segment is UTC, and that a kind folder is created only when it is first used. Note in section 5 that `docs/README.md` is the index and sits outside the layout, and in section 6 that the log file is named to match its execution plan where the work item has one.
- Update the old path references inside the three moved plans, and mark the recorded dates as UTC. One historical observation in plan 1 keeps its meaning but its notation is refreshed: "No `docs/<date>/` directory has ever been created" becomes "No `docs/<kind>/<date>/` directory has ever been created", because the notation it used no longer describes anything. The claim is unchanged.
- Add `docs/README.md` describing the layout.
- Record this change in an execution plan and an AI transparency log entry, both written in the new structure.

Explicit non-goals:

- No Rust source, no `Cargo.toml`, no `tests/` directory, no CI configuration.
- No change to the substance of any plan or log entry beyond the header block, the date field, and the path references. The entry bodies are preserved verbatim.
- No change to the fourteen numbered sections of `AGENTS.md`, their numbering, or their titles. Only the five path references and the layout rule change. No existing cross-reference is removed or altered; two references to section 12 are added.
- No change to the issue, commit, or pull request formats, the orchestration protocol, the role model, or the approval gates.
- No creation of `docs/adr/`. Git cannot track an empty directory, and the location stays a documented future one until the first decision is recorded.
- No change to any other repository.
- No merge of the pull request. That remains with the human.

## Assumptions and uncertainty

- **"Content first, then date, then file" means `docs/<kind>/<date>/<file>`.** The human gave the shape as `docs/execution-plans/<date>/file`, and the same shape is applied to every kind. The kind is a name for what the document is, not for who wrote it.
- **The date segment is UTC.** The human chose this on 2026-10-21 when the alternative, the local date, was presented. The consequence is that the shell-application work item moves from 2026-10-20 to 2026-10-21, because it ran at `00:31Z` to `00:35Z` on the 21st. The two earlier work items stay in 2026-10-20, at `23:12Z` and `23:26Z`. This also removes a standing contradiction with section 12, which requires UTC.
- **The transparency log becomes one file per work item.** The human chose this over keeping one file per date. The consequence is that section 6 no longer describes appending to a shared file, and instead describes creating the entry for the work item in hand, named to match its execution plan.
- **Old path references inside the moved documents are updated.** The human chose this over leaving them as written. The reasoning is that the acceptance criteria and work-breakdown tables are instructions a reader follows, and a path that no longer exists cannot be followed. The historical narrative and the quoted evidence are left as they are.
- **The `docs/README.md` index is wanted.** The human quoted the line offering it before approving, which is read as a request for it. It is a short description of the layout and adds no new rule.
- **The recorded dates become UTC-marked rather than rewritten wholesale.** The date fields that key a file become `(UTC)`-marked, and the one shell-application work item changes value to 2026-10-21. The UTC instants quoted in the bodies, such as `2026-10-21T00:20:15Z`, are untouched, because they were already UTC and already exact.

## Decisions

| Decision | Options considered | Choice | Rationale | Approval |
|---|---|---|---|---|
| Shape of the layout | `docs/<date>/<kind>/<file>`; `docs/<kind>/<date>/<file>` | `docs/<kind>/<date>/<file>` | The kind is what a reader wants to know first. A tree that starts with the date makes the reader already know the date. | Human, 2026-10-21 |
| Name of the plan kind | `exec-plans`; `execution-plans`; `plans` | `execution-plans` | The human rejected `exec-plans` as unprofessional. `execution-plans` is the full word and matches the term used in the body of the document. | Human, 2026-10-21 |
| Granularity of the transparency log | One file per work item; one file per date | One file per work item | A log file that grows without bound is hard to navigate, and one file per work item matches the plan folder. | Human, 2026-10-21 |
| Basis of the date segment | Local date; UTC date | UTC date | Section 12 already requires UTC for all timestamps, and the local date contradicts it. The cost is that one work item changes folder. | Human, 2026-10-21 |
| Old path references in moved documents | Update them; leave them as written | Update them | They are instructions a reader follows, and the "no stale documentation" rule in section 5 applies. The narrative is not touched. | Human, 2026-10-21 |
| Name of the ADR kind | `architecture-decision-records`; `decisions`; `adr` | `adr` | The human chose the conventional abbreviation, against the recommendation. | Human, 2026-10-21 |
| The ADR folder | Create it now; create it with the first decision | Create it with the first decision | Git does not track an empty directory, so an empty `docs/adr/` would not exist in the repository anyway. `AGENTS.md` keeps marking it as a future location. | Agent, within the approved scope |
| Git operations | Branch only; branch and local commit; full commit, push, PR | Full commit, push, PR | The human approved the plan in-session on 2026-10-21, which states the work breakdown that includes the commit, the push, and the pull request. | Human, 2026-10-21 |

## Acceptance criteria

Observable conditions for completion:

- `docs/` contains no directory other than `execution-plans/` and `ai-transparency-log/`, and the old `docs/<date>/` directory is gone.
- Every execution plan is at `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md`.
- Every transparency log entry is at `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md`, with a base name matching its execution plan.
- The shell-application work item sits under 2026-10-21 and the two earlier ones under 2026-10-20.
- `AGENTS.md` names the new paths in all five places and states the layout rule, the UTC basis of the date, and the rule that a kind folder appears when first used.
- No reference to the old layout remains in any tracked file.
- The entry bodies of the three split log files are byte-identical to the blocks they came from, apart from the header block and the date field.
- `docs/README.md` describes the layout.
- The fourteen numbered sections of `AGENTS.md` and their numbering are unchanged, every existing cross-reference still resolves, and the two added references resolve as well.
- This plan exists at `docs/execution-plans/2026-10-21/7-docs-directory-layout.md`, and the transparency log entry for this change exists at `docs/ai-transparency-log/2026-10-21/7-docs-directory-layout.md`.
- A pull request exists against `main` that closes #7, states the verification performed, and states the risks.

## Work breakdown

| Step | Action | Role | Paths affected | Depends on |
|---|---|---|---|---|
| 1 | File issue #7, add it to the board, move it to `In Progress` | Orchestrator | GitHub issue and board | — |
| 2 | Create the task branch from the latest `origin/main` | Implementation | git state | 1 |
| 3 | Write this execution plan at its new location, mark `Approved` | Orchestrator | `docs/execution-plans/2026-10-21/7-docs-directory-layout.md` | 2 |
| 4 | Move the three execution plans with `git mv` | Implementation | `docs/execution-plans/` | 3 |
| 5 | Split the transparency log into three files with the new naming and headers | Implementation | `docs/ai-transparency-log/` | 4 |
| 6 | Update the old path references and the date fields in the moved documents | Implementation | the moved documents | 5 |
| 7 | Update `AGENTS.md` sections 5, 6, 7, 11, and 13 | Implementation | `AGENTS.md` | 4 |
| 8 | Write `docs/README.md` | Implementation | `docs/README.md` | 7 |
| 9 | Write the transparency log entry for this change | Orchestrator | `docs/ai-transparency-log/2026-10-21/7-docs-directory-layout.md` | 7 |
| 10 | Verification pass: tree, rename detection, split integrity, stale-path sweep, `AGENTS.md` structure, whitespace | Test Engineer, Code Reviewer | all changed files | 4, 5, 6, 7, 8, 9 |
| 11 | Mark this plan `Verified` with observed evidence | Orchestrator | this plan | 10 |
| 12 | Commit, push, open pull request | Orchestrator | git state, PR | 11 |
| 13 | Move issue #7 to `Done` on the board | Orchestrator | GitHub project board | 12 |
| 14 | Stop for the human's merge authorization | Orchestrator | — | 13 |

## Verification

### Verification evidence

Commands actually executed on 2026-10-21 UTC, with observed results:

- `git fetch origin --prune`, `git log --oneline -1 main`, `git log --oneline -1 origin/main`, `git rev-list --left-right --count main...origin/main` — the local `main` and `origin/main` were both at `7519c7c`, with a count of `0 0`, so the local branch was not stale.
- `git checkout -b docs/7-docs-directory-layout origin/main` followed by `git merge-base --is-ancestor origin/main HEAD` — exits zero. `git diff --stat origin/main..HEAD` is empty, so the branch started from an identical tree.
- `git log --all --format='%h | %aI | %s'` and `gh pr view 2,4,6` and `gh api repos/quant-rule/quant-rule/issues/1,3,5` — the UTC instants that place each work item: item 1 at `2026-10-20T23:12:32Z` through `23:19:00Z`, item 2 at `2026-10-20T23:26:56Z` through `23:32:16Z`, item 3 at `2026-10-21T00:31:44Z` through `00:35:04Z`. The date segments follow from these.
- `find docs -type f | sort` — after the change, the tree holds `docs/README.md`, four execution plans under `docs/execution-plans/2026-10-20/` and `2026-10-21/`, and four log entries under `docs/ai-transparency-log/2026-10-20/` and `2026-10-21/`. The old `docs/2026-10-20/` directory is gone.
- `git diff --cached -M --stat` — the three plan moves are recorded as renames, and the transparency log is one deletion plus four additions: the three split entries and this change's own entry. Git cannot record a file that becomes several files as a rename, so the history of the original log does not follow the first entry. This is a known cost of the split, recorded rather than worked around.
- A script reads the original log from `git show HEAD:docs/2026-10-20/ai-transparency-log.md`, extracts each `## Entry N` block, drops the entry heading, and asserts that the remaining body appears verbatim in the corresponding new file. All three pass. The only difference inside the bodies is the `Date and tool` field, which was changed on purpose, and the check isolates that field rather than ignoring it.
- `grep -rn "exec-plans\|docs/<YYYY-MM-DD>/\|docs/2026-10-20/ai-transparency-log.md\|docs/<date>/" --include="*.md" .` — the only matches are in this plan and in this change's own transparency log entry, which quote the old layout because documenting the old layout is what they do. No other file references it.
- A script extracts every navigational path of the form `docs/<kind>/<date>/<file>.md` from every tracked file and checks it against the tree. Every one exists. References that are not navigational, such as the branch name `docs/1-establish-ai-agent-guidelines`, the rejected alternative `docs/agent-roles.md`, and the `docs/adr/` location that is created with the first decision, are excluded by the pattern and named here so the exclusion is visible.
- `grep -c "^## " AGENTS.md` — 14 top-level headings, numbered 1 to 14, contiguous, no duplicates. A script parses every `## N.` heading and every `section N` reference: the referenced set is {2, 3, 4, 6, 8, 9, 11, 12, 14}, and each reference resolves to exactly one heading. `AGENTS.md` is 565 lines, against 563 before the change.
- `git diff --cached --check` — no whitespace errors. `tail -c 1` over every Markdown file in the repository returns a newline, so each ends with one.
- The plan is checked against the ten required contents in `AGENTS.md` section 11, and the transparency log entry against the four required items in section 6.
- The complete diff was reviewed line by line before staging.

Acceptance criteria are evaluated against the evidence above. All are met except the one that depends on the pull request existing, which is satisfied when the pull request is opened. This plan is marked `Approved` rather than `Verified` at that point, because that criterion is not yet met; the status changes to `Verified` in the follow-up commit that records the pull request, following the precedent set by the earlier entries in this repository.

Limitations, stated explicitly:

- The repository contains no Rust source, no `Cargo.toml`, and no `tests/` directory, so `cargo build`, `cargo test`, `cargo clippy`, and `cargo doc` do not apply to this change. `cargo --version` was run and establishes toolchain availability only. No build, test, lint, or doc result is claimed.
- No automated markdown linter or link checker exists in the repository. Verification of a prose change is a manual read against the acceptance criteria, not a machine-checked result.
- Git cannot track an empty directory, so `docs/adr/` does not exist after this change even though `AGENTS.md` names it. It is created with the first decision recorded.

## Risks and recovery

- **The split loses the single-file chronology of the log.** Mitigation: each file carries a header naming the issue, the UTC date, and the matching execution plan, and the base names match, so a plan and its log entry sit at the same path in two sibling folders.
- **The date normalization reads as falsification.** Mitigation: only the date fields that key a file are changed, and they are marked `(UTC)`. The UTC instants quoted in the bodies are already exact and are untouched. The transparency log entry for this change records the change and its local-time correspondence.
- **A stale path reference survives in a document that was not inspected.** Mitigation: the verification sweeps the whole tree for the old notation and for any `docs/` path that does not exist on disk.
- **A reference to the old layout survives in `git` history.** Mitigation: accepted deliberately. The history records where the files were, and rewriting it is not in scope.
- **The `docs/adr/` folder is named but absent.** Mitigation: `AGENTS.md` continues to mark it as a future location, and the wording is checked against the tree by the path-existence check.
- **Recovery.** The change is a single documentation commit on a short-lived branch merged through a pull request, so it can be reverted with a new commit on `main`.

## Stop conditions

Stop and request human direction if:

- The layout cannot be applied without changing an existing governance rule in another section.
- Splitting the log turns out to lose information that the entry bodies do not carry.
- The push, the pull request, or the merge is rejected by repository protection settings.
- Any action would modify a repository other than `quant-rule/quant-rule`.
