# Execution Plan: Establish AI Agent Guidelines

- Issue: [#1](https://github.com/quant-rule/quant-rule/issues/1) — docs: establish AI agent guidelines
- Branch: `docs/1-establish-ai-agent-guidelines`
- Status: Approved — verification evidence recorded; the commit, push, pull request, and final board transition remain pending.
- Date: 2026-10-20

## Intent

QuantRule has no repository-wide instructions for AI coding agents. A draft `AGENTS.md` exists locally but was never committed, is internally inconsistent, and mixes in text that does not belong to this project. The intended outcome is a single root `AGENTS.md` that every agent reads first, states how the system is structured, and encodes the conventions the project will hold itself to.

Affected users: AI agents operating in this repository, and the human developer reviewing their output.

## Current state

Evidence gathered on 2026-10-20, branch `main` at commit `5d24888`:

- Repository `quant-rule/quant-rule`, public, default branch `main`.
- History contains one commit, `chore: initialize QuantRule`, which added `README.md` (3 lines).
- `AGENTS.md` exists on disk as an untracked file, 383 lines, and has never been committed or pushed.
- `docs/` exists and is empty. No `docs/<date>/` directory has ever been created.
- No Rust source, no `Cargo.toml`, no `tests/`, no `.github/` workflows, no `.gitignore`.
- Installed toolchain: `cargo 1.99.0`, `rustc 1.99.0`, `rustfmt 1.10.0`, `clippy 0.1.99`, `rustdoc 1.99.0` (stable `x86_64-unknown-linux-gnu`).
- `gh` authenticated as `zantgo` with `repo` and `admin:org` scopes. No repository rulesets, milestones, or PR templates exist.
- The organization project board `QuantRule` has a `Status` field with the options `Todo`, `In Progress`, and `Done`. Issue #1 was in `Todo` when this work began.
- Issue #1 is the only issue. Its body did not follow the `Goal` / `Scope` / `Done when` format that `AGENTS.md` itself prescribes, and it carried no labels.

Observed defects in the draft `AGENTS.md` (line numbers refer to the pre-change draft):

1. **Missing sections 1 through 12.** The document ran through unnumbered sections, then jumped directly to `13. Orchestration and Execution Protocol` and `14. Human Approval Gates`. The sections the issue's acceptance criteria require — coding conventions, testing, documentation — were among those absent.
2. **Unnecessary text carried over from an unrelated template.** Several passages described a product, a domain, and an approval gate that have no counterpart in QuantRule. They were eliminated. This plan does not restate them, and neither does the transparency log: the removal is the fact of record, not the content.
3. **Grammar and spelling.** "QuantRule is quantitative trading system", "its focuses on performance", "the indicators that are categorizies", "Programming Lenguage: Rust", "this data can later be user for making strategies".
4. **Numbering inconsistency.** Sections 1 through 12 were referenced implicitly by the surviving 13 and 14 headings but did not exist, so the document read as truncated.

## Scope

Included:

- Rewrite the root `AGENTS.md` so that all fourteen sections exist and are numbered consistently.
- Add the missing sections covering project architecture, coding conventions, testing, documentation, and determinism.
- Eliminate the irrelevant passages identified under current state, and replace the commit-message examples with QuantRule-relevant ones.
- Add an issue status lifecycle covering the project board's `Todo`, `In Progress`, and `Done` states, and apply it to issue #1.
- Correct the grammar and spelling defects listed above.
- Create this execution plan and the AI transparency log required by the project's own policy.
- Reformat issue #1 to the prescribed issue format and apply the `documentation` label.

Explicit non-goals:

- No Rust source code, `Cargo.toml`, workspace layout, or test suite is created.
- No CI workflow, `.gitignore`, or dependency manifest.
- No change to the architecture described in `AGENTS.md` beyond making it explicit and consistent.
- No change to the agent role model, the approval gates, or the execution-plan protocol, other than renumbering and removing irrelevant references.
- No modification of any repository other than `quant-rule/quant-rule`. Other repositories under the same organization are out of scope and are not touched by this change.

## Assumptions and uncertainty

- **"Technology-agnostic" is read as "no mandated libraries or frameworks", not "no language".** The language remains Rust, which is already stated in the document and is confirmed by the installed toolchain. Stating a language is information; mandating crates would be prescription. This assumption is recorded here so a reviewer can challenge it, and it is reflected in the document by the toolchain declaration in section 3 and by the rule that adding a dependency requires approval.
- **"Concise" is interpreted as tight, non-padded prose per section, not a hard line count.** The agent role definitions are roughly 180 lines of the document and the approval gates add about 30 more, and together they encode the human-approval model the project depends on. Removing them would shorten the file at the cost of the governance model. The file is therefore expected to land near 530 lines, which is recorded here as a known tension with the acceptance criterion rather than silently ignored.
- **Determinism rules are derived from the stated project purpose**, specifically "deterministic, rule-based" and "supports backtesting": identical input must yield identical output, the signal path must not depend on wall-clock time or unseeded randomness, and timestamps are UTC.
- **Causality and lookahead prevention are treated as coding conventions, not domain rules.** The rule that an indicator may not read beyond the current bar is stated in the document as guidance for authors. Whether it is enforced by a type-level or runtime mechanism is a later architectural decision, deliberately not made here.
- The human approved the correction-and-restore scope, the issue reformat, the board lifecycle, and full commit/push/PR authorization in-session on 2026-10-20.

## Decisions

| Decision | Options considered | Choice | Rationale | Approval |
|---|---|---|---|---|
| Treatment of existing `AGENTS.md` | Correct in place; full condensation rewrite; minimal edits; commit as-is | Correct and restore missing sections | Preserves the deliberately authored role and approval model while fixing objective defects and adding the AC-required sections. A rewrite would discard governance content the project depends on. | Human, 2026-10-20 |
| Issue #1 handling | Reformat body and label; label only; leave untouched | Reformat body and add `documentation` label | The document prescribes a single issue format; the open issue should not violate the rules it mandates. | Human, 2026-10-20 |
| Board lifecycle | Encode in `AGENTS.md` and apply to #1; apply to #1 only; leave the board alone | Encode in `AGENTS.md` and apply to #1 | The human asked for the lifecycle to be a standing rule rather than a one-off. Encoding it makes it durable and keeps future agents consistent. | Human, 2026-10-20 |
| Git operations | Branch only; branch and local commit; full commit, push, PR | Full commit, push, PR | Explicitly authorized in-session. | Human, 2026-10-20 |
| Scope of the cleanup | Leave the removed text documented in the plan and log; remove all references to it | Remove all references | The human directed that the removed text be described only as eliminated, with no restatement of what it was, so that neither the repository, the history, nor the logs carry it forward. | Human, 2026-10-20 |
| Section layout | Preserve unnumbered headings; renumber 1 through 14 | Renumber 1 through 14 | The surviving 13 and 14 headings prove the intended structure was numbered throughout. Consistency removes the appearance of truncation. | Within the approved correction scope |
| Role definitions in one file versus split | Keep in `AGENTS.md`; split to `docs/agent-roles.md` | Keep in `AGENTS.md` | `AGENTS.md` is the file agents load automatically; a split risks the governance model going unread. | Agent, flagged for human |

## Acceptance criteria

Observable conditions for completion:

- `AGENTS.md` exists at the repository root, is tracked by git, and contains all fourteen sections with no gaps in numbering.
- Sections exist covering project purpose, architecture and data flow, coding conventions, testing, documentation, determinism, AI use policy, Git workflow, issue status lifecycle, issue/commit/PR text, approval gates, agent roles, execution plans, the orchestration protocol, and human approval gates.
- No passage describing a product, domain, or approval gate that has no counterpart in QuantRule remains anywhere in the repository, in the working tree or in history.
- The grammar and spelling defects listed under current state are corrected.
- Rust is named as the implementation language, and every `cargo` command named in the document exists in the installed toolchain.
- Every path referenced by `AGENTS.md` either exists in the repository or is explicitly marked as a future location.
- The issue status lifecycle is documented in `AGENTS.md` section 9, and issue #1 is in `In Progress` on the board while the work is in flight.
- `docs/2026-10-20/ai-transparency-log.md` exists and records actual usage for this change.
- `docs/2026-10-20/exec-plans/1-establish-ai-agent-guidelines.md` exists and is marked `Verified` once evidence is recorded.
- Issue #1 body follows the `Goal` / `Scope` / `Done when` format and carries the `documentation` label.
- A pull request exists against `main` that closes #1, lists the changes, states the verification actually performed, and states the risks.
- No commit, branch, tag, or file in the local or remote repository contains the eliminated text, in the working tree or in history.

## Work breakdown

| Step | Action | Role | Paths affected | Depends on |
|---|---|---|---|---|
| 1 | Create task branch from `main` | Implementation | git state | — |
| 2 | Move issue #1 to `In Progress` on the board | Orchestrator | GitHub project board | 1 |
| 3 | Write this execution plan, mark `Approved` | Orchestrator | `docs/2026-10-20/exec-plans/1-establish-ai-agent-guidelines.md` | 1 |
| 4 | Rewrite `AGENTS.md`: restore sections 1 through 12, renumber, remove irrelevant text, correct grammar, add the issue status lifecycle | Implementation | `AGENTS.md` | 3 |
| 5 | Write the AI transparency log | Orchestrator | `docs/2026-10-20/ai-transparency-log.md` | 3 |
| 6 | Reformat issue #1 and apply the label | Orchestrator | GitHub issue #1 | 4 |
| 7 | Verification pass: read the full diff, cross-check every referenced path and command, scan the repository and history for eliminated text | Test Engineer, Code Reviewer | all changed files | 4, 5, 6 |
| 8 | Mark this plan `Verified` with observed evidence | Orchestrator | this plan | 7 |
| 9 | Commit, push, open pull request | Orchestrator | git state, PR | 8 |
| 10 | Move issue #1 to `Done` on the board | Orchestrator | GitHub project board | 9 |

## Verification

### Verification evidence

Commands actually executed on 2026-10-20, with observed results:

- `git log --all --oneline` — one commit, `5d24888 chore: initialize QuantRule`. The draft `AGENTS.md` was never committed, so the removed text was never part of the local history.
- `git log --all -p | grep -i -E "<eliminated-term-list>"` — no matches. The placeholder stands for the actual term list, substituted at execution time.
- `git cat-file --batch-all-objects` followed by a per-blob content scan — no blob in the object store contains the eliminated text. Edits made during this change leave superseded blobs behind as unreachable objects, so a garbage collection was run to remove them; `git fsck --unreachable --dangling` afterwards reports nothing, and a repeat of the per-blob scan returns no matches.
- `git reflog expire --expire=now --all` followed by `git gc --prune=now` — removed the superseded pre-rewrite blobs of the plan and the log, which had been staged but never committed and were therefore unreachable.
- `grep -rn -i -E "<eliminated-term-list>" .` over the working tree — no matches. The placeholder in this record stands for the actual term list, which was substituted at execution time and is deliberately not written down.
- `grep -n "^## " AGENTS.md` — exactly fourteen top-level headings, numbered 1 through 14 in order, no gaps and no duplicates.
- `cargo --version`, `cargo fmt --version`, `cargo clippy --version`, `rustdoc --version` — returned `cargo 1.99.0`, `rustfmt 1.10.0-stable`, `clippy 0.1.99`, `rustdoc 1.99.0`. `cargo test --help` and `cargo doc --help` were also run and both subcommands are present. This confirms only that the commands named in `AGENTS.md` exist in the installed toolchain. It is **not** evidence that a build, test, lint, or doc run passes: the repository contains no Rust workspace, so no such command was or could be run against it.
- `gh project item-list` against the organization project — the `Status` field options are `Todo`, `In Progress`, and `Done`; issue #1 was observed in `Todo` and then moved to `In Progress`.
- `gh api repos/quant-rule/quant-rule/issues/1` — body is 16 lines in the prescribed format, labels are `["documentation"]`.
- The complete diff was reviewed line by line before committing.

Limitations, stated explicitly:

- The repository contains no Rust source, so `cargo build`, `cargo test`, and `cargo clippy` cannot be run against a workspace and are not evidence of anything. The document's command guidance is validated only for toolchain availability, not for a passing build.
- No automated markdown linter, link checker, or documentation test suite exists in the repository. Verification of this change is a manual review, not a machine-checked result.
- The historical line count recorded under current state refers to the pre-change draft, which was untracked and has since been overwritten. It is agent-reported and is not independently reproducible.
- This change is documentation only. It makes no behavioral claim about the trading system, and none can be inferred from it.

## Risks and recovery

- **Loss of governance content during the rewrite.** Mitigation: the role definitions, approval gates, and execution-plan protocol are carried over apart from renumbering and the removal of irrelevant passages. Verified by reading the complete diff.
- **A section is dropped without being noticed.** Mitigation: the acceptance criteria enumerate all fourteen sections; the verification pass checks the headings against that list.
- The document grows past the "concise" acceptance criterion. Accepted: the file lands at roughly 530 lines, of which about 210 are the role definitions and approval gates that carry the governance model. The size is recorded under Assumptions and uncertainty, and the reversal path is recorded here — splitting the role definitions into a second file in a follow-up change.
- **A referenced path or command does not exist.** Mitigation: each reference is checked during the verification pass; paths that do not exist yet are marked as future locations in the document itself.
- **The board status drifts from the real state of the work.** Mitigation: the lifecycle is defined in `AGENTS.md` section 9, and the verification pass confirms the live board state against the work performed.
- **Branch protection blocks the push or the merge.** No rulesets are configured on the repository, so a direct push to the branch is expected to succeed. If it is rejected, the human is informed rather than bypassing the protection.
- **Recovery.** The change is a single documentation commit on a short-lived branch. Reverting the commit on the branch and force-pushing would restore the previous state, but per project rules a new commit is preferred over rewriting shared history.

## Stop conditions

Stop and request human direction if:

- Restoring the missing sections requires a business, risk, or architecture decision not settled by the approved requirements.
- A passage cannot be removed without changing the meaning of the surrounding governance rule.
- The verification pass finds that a path or command named in the document contradicts observed repository state in a way that implies a different intended architecture.
- The push or pull request is rejected by repository protection settings.
- Any action would modify a repository other than `quant-rule/quant-rule`.
