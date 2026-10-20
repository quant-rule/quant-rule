# Execution Plan: Closing Action Line in Agent Responses

- Issue: [#3](https://github.com/quant-rule/quant-rule/issues/3) — docs: require a closing action line in agent responses
- Branch: `docs/3-closing-action-line`
- Status: Verified
- Date: 2026-10-20

## Intent

An agent that finishes a task and leaves the human guessing what to do next has not finished. The intended outcome is a documented rule requiring every final response to the human to close with a single bold line naming the action required from the human, or stating that no action is needed.

Affected users: the human developer reading agent output, and the agents producing it.

## Current state

Evidence gathered on 2026-10-20, `origin/main` at commit `6867f8d` (the merge of pull request #2). The local `main` was stale at `5d24888` at the time of the survey.

- `AGENTS.md` exists at the repository root with fourteen numbered sections and no numbering gaps.
- Section 8, "Issue, Commit and Pull Request Text", prescribes formats for issue bodies, commit messages, and pull request descriptions. It says to keep them "brief, human-readable and in the same format every time" but does not cover the agent's direct response to the human.
- Section 13, "Orchestration and Execution Protocol", Stage 5 lists what an agent must report before completing: changed files, tests, results, limitations, and any decisions still required. It does not require a closing call to action.
- No instruction anywhere in the repository states that the response must end with a specific line, and no instruction covers the case where nothing is required of the human.
- Issue #3 is the only open issue. It was created on 2026-10-20 and carries the `documentation` label.
- The organization project board exposes the `Status` field with the options `Todo`, `In Progress`, and `Done`. Issue #3 was moved to `In Progress` before this change was made.

## Scope

Included:

- Add a "Response format" rule to `AGENTS.md` requiring the agent's response to end with a single bold line.
- Specify that the line states either the action required from the human or that no action is required.
- Specify that the line is at most one phrase long, and that it is not a sentence.
- Require the agent to apply the rule to its own final response, not only to responses it produces in other tools.
- Retitle section 8 to "Issue, Commit, Pull Request and Response Text" and extend its opening sentence to cover responses.
- Add a cross-reference in section 13, Stage 5, pointing at the new response format.
- Record the change in an execution plan and an AI transparency log.

Explicit non-goals:

- No change to the fourteen numbered sections, their numbering, or their content beyond the three `AGENTS.md` edits named above.
- No change to the issue, commit, or pull request formats in section 8.
- No change to the orchestration protocol beyond the single cross-reference in Stage 5, and none to the role model or the approval gates.
- No Rust source, no `Cargo.toml`, no test suite, no CI configuration.
- No change to any other repository.

## Assumptions and uncertainty

- **The line is required at the end of every final response to the human.** It is not required of intermediate messages inside a longer session, of a tool's own output, or of an artifact written into the repository. A response that asks the human a blocking question is still a final response to the human, so the rule applies to it: the question is the action, and the closing line states what the human needs to decide.
- **"One phrase" means one short imperative or noun phrase, not one sentence.** A sentence carries a subject and a predicate and can hide a second instruction. The distinction is what keeps the line scannable and makes it safe to parse.
- **Bold is required.** The rule states the intent — that the line is visually distinct from the body of the response — so an agent does not get the form wrong, but the form itself is bold text and not a heading.
- **The "no action" case is mandatory.** A rule that only fires when something is pending forces an agent to invent a pending action. An agent that has genuinely finished must be able to say so, and must not manufacture a task for the human to look busy.
- The human set this requirement directly on 2026-10-20, in the same instruction that authorized the commit, the pull request, and the merge for this change.

## Decisions

| Decision | Options considered | Choice | Rationale | Approval |
|---|---|---|---|---|
| Where the rule lives | New numbered section; subsection of section 8; subsection of section 13 | A subsection of section 8, retitled to cover response text | Section 8 is the document's home for the formats agents produce. Adding a numbered section would renumber sections 9 through 14 and every cross-reference to them, which is a large diff for a small rule. | Agent, within the approved scope |
| Wording of the closing line | Fixed vocabulary; free text | Free text, constrained to one phrase | Fixed vocabulary would be easier to parse but would not describe an arbitrary action, and the human asked for the action the human needs, not a code. | Human, 2026-10-20 |
| Whether the rule is mandatory | Mandatory; recommended | Mandatory | A recommended rule is not a rule. The human asked for a requirement. | Human, 2026-10-20 |
| Placement in the response | Final line of the response | Final line | A closing line is only closing if nothing follows it. | Agent, within the approved scope |

## Acceptance criteria

Observable conditions for completion:

- `AGENTS.md` contains a rule requiring the agent's final response to end with a single bold line.
- The rule states that the line names the action required from the human, or that no action is required.
- The rule states that the line is at most one phrase long.
- The rule explicitly covers the case where the work is complete and nothing is required of the human.
- The rule states that the agent applies it to its own final response.
- The existing fourteen sections and their numbering are unchanged, and every cross-reference still resolves.
- `docs/2026-10-20/ai-transparency-log.md` records this change.
- `docs/2026-10-20/exec-plans/3-closing-action-line.md` exists and is marked `Verified` once evidence is recorded.
- A pull request exists against `main` that closes #3, states the verification performed, and states the risks.

## Work breakdown

| Step | Action | Role | Paths affected | Depends on |
|---|---|---|---|---|
| 1 | Move issue #3 to `In Progress` on the board | Orchestrator | GitHub project board | — |
| 2 | Create the task branch from the latest `main` | Implementation | git state | 1 |
| 3 | Write this execution plan, mark `Approved` | Orchestrator | `docs/2026-10-20/exec-plans/3-closing-action-line.md` | 2 |
| 4 | Add the response format rule to `AGENTS.md` | Implementation | `AGENTS.md` | 3 |
| 5 | Append an entry to the AI transparency log | Orchestrator | `docs/2026-10-20/ai-transparency-log.md` | 3 |
| 6 | Verification pass: read the full diff, check numbering and cross-references, confirm the rule is unambiguous | Test Engineer, Code Reviewer | all changed files | 4, 5 |
| 7 | Mark this plan `Verified` with observed evidence | Orchestrator | this plan | 6 |
| 8 | Commit, push, open pull request | Orchestrator | git state, PR | 7 |
| 9 | Review the pull request, then merge it with the human's authorization | Code Reviewer, Orchestrator | PR | 8 |
| 10 | Move issue #3 to `Done` on the board | Orchestrator | GitHub project board | 9 |

## Verification

### Verification evidence

Commands actually executed on 2026-10-20, with observed results:

- `git log --oneline -1 main` and `git log --oneline -1 origin/main` — the local `main` was stale at `5d24888` while `origin/main` was at `6867f8d` after the merge of pull request #2. The task branch was created from the local `main`, and `git merge-base --is-ancestor origin/main HEAD` confirmed it did not contain `origin/main`, so it was reset onto `origin/main`. Before the change, `git rev-parse HEAD` was `6867f8d` and `git diff --stat origin/main HEAD` was empty. On the branch, `git merge-base --is-ancestor origin/main HEAD` exits zero and `git diff --stat origin/main..HEAD` lists only the three files of this change.
- `grep -n "^## " AGENTS.md` — the fourteen top-level headings are unchanged from the merged state, and the new rule is added as a `###` subsection, so no renumbering occurs.
- `grep -n "section [0-9]*" AGENTS.md` — every cross-reference still resolves to an existing section.
- `git diff --cached --stat` over the change — the change modifies `AGENTS.md` and the AI transparency log and adds the execution plan, three files in total.
- `grep -c "^## " AGENTS.md` — 14 top-level headings, unchanged from the merged state, so no renumbering occurred.
- Every `section N` cross-reference in `AGENTS.md` was checked against the headings: sections 2, 3, 4, 6, 8, 9, 11, 12, and 14 each resolve to exactly one heading.
- The plan was checked against the ten required contents in `AGENTS.md` section 11: all ten are present by name, plus the Verification evidence subsection.
- The transparency log was checked against the four required items in `AGENTS.md` section 6: Date and tool, Intent and boundary, Verification and evidence, and Learning with the rejected and corrected suggestions nested under it.

Acceptance criteria were evaluated against the evidence above. All are met, except the one that depends on the pull request existing, which is satisfied when the pull request is opened.

Limitations, stated explicitly:

- The repository contains no Rust source, so no build, test, lint, or doc run applies to this change, and none is claimed.
- No automated markdown linter or link checker exists in the repository. Verification of a prose rule is a manual read against the acceptance criteria, not a machine-checked result.
- The rule governs agent behaviour. Its correctness cannot be established by a test in this repository; it is verifiable only by observing that an agent can follow it unambiguously, which is what the verification pass checks.

## Risks and recovery

- **The rule is too vague to follow.** Mitigation: the rule states the format, the maximum length, the required case where no action is needed, and the prohibition on inventing a pending action. The verification pass reads it against each acceptance criterion.
- **The rule is misread as applying to every intermediate message.** Mitigation: the rule states that it applies to the final response to the human, that it does not apply to intermediate messages inside a longer session, and that a blocking question is itself the action.
- **Renumbering damage.** Mitigation: the rule is a `###` subsection of an existing section, so the fourteen numbered sections and their cross-references are untouched. Confirmed by the heading check.
- **The rule conflicts with the existing pull request format in section 8.** Mitigation: section 8 governs artifacts written into the repository; this rule governs the conversational response to the human. The rule states this boundary explicitly.
- **Recovery.** The change is a single documentation commit on a short-lived branch and is merged through a pull request, so it can be reverted with a new commit on `main`.

## Stop conditions

Stop and request human direction if:

- The rule cannot be written without changing an existing governance rule in another section.
- Applying the rule requires deciding something the human has not delegated, such as which of several pending actions takes priority beyond the rule's stated default of naming the single next one.
- The push, the pull request, or the merge is rejected by repository protection settings.
- Any action would modify a repository other than `quant-rule/quant-rule`.
