# Execution Plan: Closing the three loose ends from the state review

- Issue: [#44](https://github.com/quant-rule/quant-rule/issues/44) — fix: close the three loose ends found in the state review
- Branch: `fix/44-loose-ends`
- Worktree: `/home/zantgo/Projects/quant-rule--44-loose-ends`
- Status: Approved
- Date: 2026-10-21 (UTC)
- Transparency log: [`docs/ai-transparency-log/2026-10-21/44-loose-ends.md`](../../ai-transparency-log/2026-10-21/44-loose-ends.md)

## Intent

A state review on 2026-10-21 read the repository, the documentation, the board, and the issues, and found three things left open by the work that has already been merged. Each is small, each is a fact that the repository states somewhere and contradicts somewhere else, and each one misleads a reader today:

1. The model engine's architecture decision record is marked `Proposed` although its pull request merged and the record itself says the human's confirmation is what moves it to `Accepted`.
2. `AGENTS.md` section 4 puts integration tests in `tests/` at the workspace root, a directory a virtual workspace root never compiles, while the data engine's own plan already decides that the tests live in the crates' `tests/` directories.
3. The board's sync script treats a merged pull request as an open one, so eleven merged pull requests read `Status` = `Todo` and `State` = `Open` on the board while the rule in `AGENTS.md` section 9 says a merged pull request is closed.

The intended outcome is a repository whose records match each other: the decision record carries its status, the testing rule names a layout that compiles, and the board says what actually happened to every item on it.

Affected users: the human developer, who reads the board and the records; and the agents, who will implement the model engine and the data engine against them.

## Current state

Evidence gathered on 2026-10-21 UTC, `origin/main` at commit `3ecb434`.

- The repository holds `AGENTS.md` (625 lines, fourteen numbered sections), `README.md`, `docs/`, and `.github/`. There is no `Cargo.toml`, no Rust source, and no `tests/` directory. `cargo --version` and `rustc --version` return `cargo 1.99.0` and `rustc 1.99.0`, which establishes toolchain availability only; no build, test, lint, or doc result applies to this change and none is claimed.
- `docs/adr/2026-10-21/four-axis-analytical-model.md` line 4 reads `Status: Proposed — accepted for implementation once the human confirms it`, and its last section reads: "The human's confirmation of this record is what moves the status from Proposed to Accepted." Pull request #31, which introduced the record, merged on 2026-10-21.
- `AGENTS.md` section 4 reads: "**Unit tests** live next to the code they cover, in the same module. Integration tests live in `tests/` at the workspace root." The next bullet reads: "**Future paths.** `tests/` and the `cargo` commands above describe the layout the Rust workspace will use."
- `docs/execution-plans/2026-10-21/9-data-engine.md` decision B3 reads: "Per-crate `tests/`, and section 4's sentence corrected in this change", with the rationale "A virtual workspace root has no package, so a root `tests/` directory is never compiled." The same plan's acceptance criteria read "integration tests in the crates' `tests/` directories", and its current-state section quotes section 4's sentence as the thing that "cannot hold as written once the workspace exists".
- `.github/scripts/project-sync.sh` line 307 tests `content_state` against `CLOSED` to write the `State` field, and line 327 does the same to write a leaf's `Status`. A merged pull request reports its GraphQL state as `MERGED`, not `CLOSED`, so a merged pull request is written as `State` = `Open` and `Status` = `Todo`. The script's own header states the board's rule: "a pull request merged takes `Done`", and `AGENTS.md` section 9 states: "The `State` field carries `Open` or `Closed` on every item, and a merged pull request is closed."
- A read-only audit of the board against the state of the content each item holds reports 44 items and 11 disagreements, and every one of the 11 is a merged pull request: #2, #4, #6, #8, #16, #31, #33, #35, #38, #39, and #41, each reading `Status` = `Todo` and `State` = `Open`. The two other pull requests are correct: #40 was closed without merging and reads `Done` and `Closed`, and #43 was merged after the script was corrected for the option-id defect and reads `Done` and `Closed`.
- Issue #44 was opened on 2026-10-21, added to the board with `Item type` = Issue and `State` = Open, given a `Start date` of 2026-10-21, and moved to `In Progress` before this change was made to the working tree.

## Scope

Included in this change:

- `docs/adr/2026-10-21/four-axis-analytical-model.md`: the status moves to `Accepted`, and the closing section records what the confirmation rests on.
- `AGENTS.md` section 4: the integration-test location names the crate that owns the code, and the future-paths bullet follows.
- `docs/execution-plans/2026-10-21/9-data-engine.md`: decision B3, the current-state note, and the acceptance criteria agree with the corrected rule.
- `.github/scripts/project-sync.sh`: a merged pull request is closed, for the `State` field and for a leaf's `Status`.
- The board: the fixed script is run over the eleven merged pull requests, and the result is read back.
- This plan and the AI transparency log that matches it.

Explicit non-goals:

- No Rust source, no `Cargo.toml`, no `tests/` directory, and no CI configuration for the workspace. The data engine remains unimplemented.
- No change to the fourteen numbered sections of `AGENTS.md`, their numbering, their titles, or their cross-references.
- No change to the model's contract, the four axes, the event model, the aggregation rules, or the code registries. The eleven refinements A1 to A11 stand as they are.
- No change to the shell command vocabulary, which `AGENTS.md` section 3 reserves to the human.
- No merge of any pull request.

## Assumptions and uncertainty

- **The human's instruction to close the loose ends is the confirmation the ADR asks for.** The ADR names the confirmation as the thing that moves it to `Accepted`, and the instruction to fix the loose ends that were listed, with the ADR among them, is that confirmation. The record says so, with the date, so a later reader can see what it rested on.
- **The integration-test rule is corrected now rather than in the data engine's change.** The data engine's plan commits the workspace to three crates, and a virtual workspace root has no package, so a root `tests/` directory is never compiled. The sentence is therefore already false against the layout the plan decides, and it is corrected in the change that removes the contradiction rather than left standing for a later one. The cost is one line in a file the data engine's change will touch anyway.
- **The board is corrected by running the fixed script, not by hand.** The script is idempotent and computes the state the board should be in, and the earlier work item ran it against the live board in exactly this way. Running it is the same act the workflow performs, so the history and the future go through one code path.
- **The script is not extended to sweep the whole board.** The workflow triggers on every issue and pull request event, so with the defect fixed the board converges on its own from now on, and the eleven historical items are a one-off correction. A sweep would be a larger change to a script that works, and it is not needed to close the loose end.
- The corrections touch a workflow script, a governance file, and three documents. None of them changes the product's behaviour, because the product does not exist yet.

## Decisions

| # | Open point | Options | Choice | Rationale |
|---|---|---|---|---|
| C1 | How a merged pull request is recognised | Test for `CLOSED` only; test for `CLOSED` and `MERGED`; test for anything that is not `OPEN` | Test for `CLOSED` and `MERGED` | A pull request's GraphQL state is one of `OPEN`, `CLOSED`, or `MERGED`, and an issue's is `OPEN` or `CLOSED`. `CLOSED` and `MERGED` are the two closed cases, and naming them states the rule that `AGENTS.md` section 9 already states. "Anything that is not `OPEN`" would also be correct and would be broader than the evidence supports |
| C2 | Which field the closed rule drives | The `State` field only; the `State` field and a leaf's `Status`; the `Status` field only | The `State` field and a leaf's `Status` | `State` says whether the item is still open, and `Status` says where it sits in the lifecycle. A merged pull request is finished, so it is `Done`. Leaving `Status` at `Todo` is what made the board read as though eleven merges were still in flight |
| C3 | Where the integration-test rule is corrected | In this change; in the data engine's change; leave it and record the contradiction | In this change, and the data engine's plan is updated to match | The rule is already false against the layout the plan commits to, and a rule that says the tests live somewhere they cannot live is a rule the next implementer will trip over. The correction is one line, and the plan's own decision B3 named the same correction |
| C4 | How the board's history is corrected | By hand, item by item; by running the fixed script over the affected items; by extending the script to sweep the board | By running the fixed script over the affected items | One code path for the history and the future, and the script is idempotent, so a second run writes nothing. A sweep is a larger change to a script that works |
| C5 | Whether the ADR's `Accepted` status needs the human's explicit word | Ask again; treat the instruction to fix the listed loose ends as the confirmation | Treat the instruction as the confirmation, and record it in the ADR with the date | The instruction named the ADR's stale status as a loose end to fix, and the record asks for a confirmation rather than for a particular form of words. The ADR now states what the status rests on, so a reader can judge it |

## Acceptance criteria

Observable conditions for completion:

- The ADR's status reads `Accepted`, and its closing section records the confirmation and the date it was given.
- `AGENTS.md` section 4 names a location for integration tests that a virtual workspace root compiles, and its future-paths bullet follows the same rule.
- The data engine's plan states the same rule in its decision table, its current-state section, and its acceptance criteria.
- The sync script writes `State` = `Closed` and `Status` = `Done` for a merged pull request, and leaves an open pull request at `State` = `Open`.
- The board audit reports zero disagreements between the board and the state of the content each item holds.
- The fixed script is idempotent: a second run over the same item changes nothing and exits zero.
- `bash -n` is clean on the script.
- The fourteen numbered sections of `AGENTS.md`, their numbering, their titles, and every cross-reference in the file are unchanged, and every cross-reference still resolves.
- This plan and the transparency log that matches it exist and carry the contents `AGENTS.md` sections 11 and 6 require.
- A pull request exists against `main` that closes #44, states the verification performed, and states the risks.

## Work breakdown

| Step | Action | Role | Paths affected | Depends on |
|---|---|---|---|---|
| 1 | File issue #44, add it to the board, move it to `In Progress` | Orchestrator | GitHub issue, board | — |
| 2 | Create the worktree and the branch from the latest `origin/main` | Implementation | git state | 1 |
| 3 | Write this plan and the transparency log | Orchestrator | `docs/` | 2 |
| 4 | Correct the ADR's status and its closing section | Implementation | `docs/adr/2026-10-21/` | 3 |
| 5 | Correct `AGENTS.md` section 4 and the data engine's plan | Implementation | `AGENTS.md`, `docs/execution-plans/2026-10-21/` | 3 |
| 6 | Correct the sync script's handling of a merged pull request | Implementation | `.github/scripts/project-sync.sh` | 3 |
| 7 | Run the fixed script over the merged and closed items; read the board back | Implementation | board | 6 |
| 8 | Verification pass: script syntax, idempotence, board audit, link check, heading check, diff review | Test Engineer, Code Reviewer | all changed files | 4–7 |
| 9 | Commit, push, open the pull request | Orchestrator | git state, PR | 8 |
| 10 | Move issue #44 to `Done` on the board | Orchestrator | board | 9 |
| 11 | Stop for the human's merge authorization | Orchestrator | — | 10 |

## Verification

The commands that produce the evidence, and what they are expected to show:

- `bash -n .github/scripts/project-sync.sh` — no syntax error.
- The script run against a merged pull request, an open pull request, a closed issue, and an open issue, with the board read back after each — a merged pull request reads `State` = `Closed` and `Status` = `Done`; an open pull request reads `Open` and `Todo`; a closed issue reads `Closed` and `Done`; an open issue reads `Open` and `Todo`.
- The same script run a second time over the same items — no field changes, exit zero.
- The board audit — zero disagreements, against the 11 recorded before the change.
- A script that parses every `## N.` heading and every `section N` reference in `AGENTS.md` — the headings are contiguous 1 through 14, and every reference resolves to exactly one heading.
- A link check over the changed documents — no broken relative link.
- `git diff --check` — no whitespace error, and every changed and added file ends with a newline.
- A read of the complete diff.

No `cargo` command applies: the repository holds no Rust source, and this change adds none.

## Risks and recovery

- **The script is the workflow's writer, and a defect in it moves the board.** Mitigation: the change is one condition on the content's own state, the script's own header and `AGENTS.md` section 9 both already state the rule it now implements, and the change is verified against four content states rather than one.
- **Running the script against the live board writes to it.** Mitigation: the script is idempotent and writes only what differs, the run is over the eleven items that disagree, and the board is read back after the run.
- **`AGENTS.md` grows.** The section 4 change replaces one sentence with one sentence, and the future-paths bullet is edited rather than added to, so the file does not grow.
- **The ADR's `Accepted` status rests on an instruction rather than a quotation.** Mitigation: the ADR states what the status rests on and the date, so the record is honest about its own basis and a reader can challenge it.
- **Recovery.** The change is one branch merged through a pull request, so it can be reverted with a new commit on `main`. The board corrections are idempotent and are reversed by running the previous script over the same items, which sets the eleven merged pull requests back to `Todo` and `Open`.

## Stop conditions

Stop and request human direction if:

- The board's fields cannot be written by the script under the permissions the workflow's app token carries.
- Correcting section 4 turns out to require changing another section, or a cross-reference stops resolving.
- The ADR's status would have to be changed in a way that says more than the instruction supports.
