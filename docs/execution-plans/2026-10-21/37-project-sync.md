# Execution Plan: Keep the board's status in step with the issues

- Issue: [#37](https://github.com/quant-rule/quant-rule/issues/37) — ci: keep the board's status in step with the issues
- Branch: `feat/37-project-sync`
- Worktree: `/home/zantgo/Projects/quant-rule--37-project-sync`
- Status: Approved
- Date: 2026-10-21 (UTC; 2026-10-20 local time)
- Transparency log: [`docs/ai-transparency-log/2026-10-21/37-project-sync.md`](../../ai-transparency-log/2026-10-21/37-project-sync.md)

## Intent

The board's built-in workflows write the `Status` field: an item added takes `Todo`, an issue closed takes `Done`, a pull request merged takes `Done`, and an item marked `Done` has its issue closed. Two things are left over. The custom `State` and `Item type` fields are never written, so a pull request added by the board's own automation arrives with no `Item type`, and an item's `State` stays `Open` after its issue is closed. And nothing recomputes a container's status from its sub-issues, which is the rule issue #36 put in place.

This work item adds the workflow that covers both gaps, so the board is right whoever made the change.

Affected users: the human developer, who reads the board to see what is in flight; and the agents, whose board updates are then no longer their work.

## Current state

Evidence gathered on 2026-10-21 UTC, `origin/main` at commit `e304145` (the merge of pull request #38).

- The project's seven built-in workflows were read through the API and GitHub's documentation for them was consulted. They write the `Status` field only, and none of them has a trigger for a container's sub-issues.
- GitHub's documentation states that "`GITHUB_TOKEN` is scoped to the repository level and cannot access projects", and recommends a GitHub App for organization projects. The permission list for `GITHUB_TOKEN` was read and holds no `projects` permission.
- The human registered the GitHub App, stored its private key as the organization secret `PROJECT_APP_PRIVATE_KEY` with visibility "all", and stored its id `5406299` as the organization variable `PROJECT_APP_ID`. Both were read back through the API: the secret's name and the variable's value.
- The private key was pasted into a conversation before it was rotated. It was deleted from the app and replaced, and the replacement was stored directly in the organization secret without passing through a conversation.
- The API shapes the script needs were read directly: `Issue.parent`, `Issue.subIssues` with each sub-issue's state, the `closeIssue` and `reopenIssue` mutations, the `addProjectV2ItemById` and `updateProjectV2ItemFieldValue` mutations, and the project's three single-select fields with their option ids.

## Scope

Included:

- `.github/workflows/project-sync.yml`, triggered by issue and pull request events, which mints the app's installation token and runs the script.
- `.github/scripts/project-sync.sh`, which writes the `State` and `Item type` fields, sets a leaf's `Status` from its own state, and recomputes a container's `Status` from its sub-issues, keeping a container's issue open or closed to match.
- `AGENTS.md` section 9: the clause that says the agent does the workflow's work until the workflow is in place is removed.
- This plan and the transparency log for this work item.

Explicit non-goals:

- No change to the board's built-in workflows, which are the board's own and are configured in the browser.
- No change to the rule itself, which issue #36 put in place.
- No second project, and no change to the project's fields or views.
- No work on the model's own code, and no change to the shell.

## Assumptions and uncertainty

- **The app's token is the only writer of the fields it owns.** The script sets `State`, `Item type`, a container's `Status`, and a leaf's `Status` on a close or a reopen. The agent sets a leaf's `Todo` and `In Progress` while it works, which is the one thing no workflow can know.
- **A container's issue follows its board item.** The board closes an issue whose item reaches `Done`, and nothing reopens it. The script closes and reopens a container's issue to match the status it computes, so the two never disagree.
- **A leaf's status follows its own state.** A leaf is `Done` when its issue is closed and `Todo` otherwise, which is the same rule the board's built-in applies on a close.
- **The script writes what differs, not what it computes.** Every write is idempotent, so a run that changes nothing leaves nothing behind, and the workflow can be re-run.
- **The organization holds the credentials, not the repository.** The variable and the secret are set at the organization level, and the secret's visibility is "all", so every repository in the organization could use it. The workflow is the only consumer today.
- **A workflow in this repository only sees this repository's issues.** Every issue on the board is in this repository, so the container rule is covered. A sub-issue in another repository would not be, and the agent's fallback covers that case.

## Decisions

| # | Open point | Options | Choice | Rationale |
|---|---|---|---|---|
| J1 | How the script authenticates | The `GITHUB_TOKEN`; a personal access token; the app's installation token | The app's installation token | `GITHUB_TOKEN` cannot access projects, and GitHub's documentation recommends an app for organization projects. A personal access token would be a user credential that expires; the installation token is scoped to the app and the repository |
| J2 | The language of the script | Bash with `gh` and `jq`; a JavaScript script; a Rust binary | Bash with `gh` and `jq` | The repository is a Rust project and the runner's Rust toolchain is not guaranteed, and a JavaScript script would introduce a second runtime into a Rust repository. `gh` and `jq` are on the runner, and the pattern is the one GitHub's own documentation uses |
| J3 | How the actions are referenced | A version tag; a commit SHA | A commit SHA, with the version in a comment | The workflow holds a credential, so the code that runs with it should not move under it. The repository's policy does not require pinning, and the pin is cheap |
| J4 | What a leaf's status is | `Todo` unless it is closed; `In Progress` while the agent works; `Done` when closed | `Done` when closed, `Todo` otherwise | The agent owns `In Progress`, because only the agent knows that work has started. A workflow that set it would undo the agent's own mark |
| J5 | Whether a container's issue is closed by the script | Yes; leave it to the board | The script closes and reopens it to match | The board closes an issue whose item reaches `Done`, and nothing reopens it when a child comes back. Leaving it alone would leave a closed container above open children, which is the state issue #36 repaired |
| J6 | What the script does when the item is not on the board | Nothing; add it | Add it | The board's own auto-add and the script can race, and a content that is not on the board has no fields to keep in step |

## Acceptance criteria

Observable conditions for completion:

- The workflow runs on an issue or pull request event and mints the app's installation token.
- The script writes the `State` and `Item type` fields, and recomputes a container's `Status` from its sub-issues.
- A container's issue is closed when its last sub-issue is closed, and reopened when one of them reopens.
- Running the script twice leaves the board unchanged.
- `AGENTS.md` section 9 no longer says the agent maintains the custom fields, and the rule it states is the one the workflow enforces.
- `bash -n` is clean on the script, and the script's read paths and its container decision are exercised against the live board before it is merged.
- The script is executable and the workflow references it by path.

## Work breakdown

| Step | Action | Role | Paths affected | Depends on |
|---|---|---|---|---|
| 1 | File issue #37, add it to the board, move it to `In Progress` | Orchestrator | GitHub issue, board | — |
| 2 | Create the worktree and the branch from the latest `origin/main` | Implementation | git state | 1 |
| 3 | Write this plan and the transparency log | Orchestrator | `docs/` | 2 |
| 4 | Write the script and the workflow | Implementation | `.github/` | 3 |
| 5 | Amend `AGENTS.md` section 9 | Implementation | `AGENTS.md` | 3 |
| 6 | Exercise the script's read paths and its container decision against the live board | Test Engineer | `.github/scripts/` | 4 |
| 7 | Commit, push, open the pull request | Orchestrator | git state, PR | 6 |
| 8 | Merge, and let the workflow run on a real event | Orchestrator | board, git state | 7 |
| 9 | Move issue #37 to `Done`, remove the worktree, delete the branch | Orchestrator | board, git state | 8 |

## Verification

Commands actually executed on 2026-10-21 UTC, with observed results:

- `gh api /orgs/quant-rule/actions/secrets` and `.../actions/variables` — the secret `PROJECT_APP_PRIVATE_KEY` is present at the organization level with visibility "all", and the variable `PROJECT_APP_ID` is present with the value `5406299`. The repository's own secret and variable lists were empty before the change and are not used.
- `bash -n .github/scripts/project-sync.sh` — no syntax errors.
- The script's read paths were run against the live board with the author's own token: issue #17 is read as an issue with state `OPEN`, the parent `#14`, no sub-issues, and its board item `PVTI_lADOFJhfMs4Bn-OgzhCS5tQ`; issue #14 is read with fourteen `OPEN` sub-issues and its board item `PVTI_lADOFJhfMs4Bn-OgzhCRlKk`.
- The container decision was exercised over five cases: all sub-issues closed gives `Done`, one open gives `In Progress` whichever position it holds, none closed gives `In Progress`, and no sub-issues gives `Todo`. Issue #14 with its fourteen open sub-issues computes `In Progress`, which is the status its board item carries.
- The workflow file was parsed and its triggers, permissions, and steps read back, and both actions were confirmed to be pinned to a commit SHA.
- `git diff --check` — no whitespace errors, and the script ends with a newline and carries the executable bit.

Limitations, stated explicitly:

- The end-to-end run is observed after the merge, because a workflow file only runs from the default branch.
- The script is exercised with the author's token rather than the app's installation token, because the app's token is only available inside the workflow. The first workflow run is the test of the app's own permissions.
- `shellcheck` is not installed on the machine, so the script was checked with `bash -n` and by running its paths, not by a linter.

## Risks and recovery

- **The app's token can write to issues.** The script reopens a container's issue when a child reopens, which is a write to an issue. Mitigation: the app's repository permission is `Issues: read and write` and nothing else, it is installed on this repository alone, and the script writes an issue's state only to match a status it computed.
- **The script and the board's built-in workflow both write `Status`.** Mitigation: the script writes `Done` on a close, which is what the built-in writes, and `Todo` on a reopen, which the built-in does not write at all. The two agree wherever they overlap.
- **The script fights the agent.** It would, if it set a leaf's `In Progress`. It does not: the agent owns that value, and the script only writes it on a close or a reopen.
- **A script that fails leaves the board wrong.** Mitigation: the workflow fails visibly, and the script writes what differs, so a second run finishes the job.
- **Recovery.** The change is one branch, merged through a pull request, so it can be reverted with a new commit on `main`. The workflow is then inert and the agent's rule in section 9 applies.

## Stop conditions

Stop and request human direction if:

- The app's installation token cannot be minted, which would mean the app is not installed or its permissions are wrong.
- The script's write turns out to fight the board's built-in workflow rather than agree with it.
- Keeping the agent's authority over a leaf's `In Progress` turns out to leave the board ambiguous in a way the human does not accept.
