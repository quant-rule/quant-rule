# Execution Plan: The Shell Application Surface

- Issue: [#5](https://github.com/quant-rule/quant-rule/issues/5) — docs: define the shell application surface
- Branch: `docs/5-shell-application-surface`
- Status: Verified
- Date: 2026-10-20

## Intent

QuantRule's product is a shell application. The result the user gets is an interactive command shell: the program is started, and commands are typed at a prompt to work with candle series, indicators, and backtests. There is no library to embed, no service to deploy, no graphical interface, and no second surface.

`AGENTS.md` does not say this anywhere. Section 1 describes the system as "a quantitative trading system that is multi-timeframe, rule-based, and OHLCV-candles-only" without ever naming the surface the user touches, and section 2 ends the data flow at OUTPUT, leaving the reader with a pipeline and no way in. The intended outcome is a document that states the shell as the product's only interface and makes every existing rule consistent with it.

Affected users: the human developer, who owns the product shape; and the agents, who build against a description that currently omits the thing they are building.

## Current state

Evidence gathered on 2026-10-20, `origin/main` at commit `13ddfd6` (the merge of pull request #4).

- `AGENTS.md` exists at the repository root with fourteen numbered sections, contiguous, no duplicates, 549 lines.
- Section 1, "Project Context", has two paragraphs: the product description and the determinism statement. A read of the file at `13ddfd6` finds no occurrence of "shell" or "prompt" in it. "repl" and "cli" do occur elsewhere in the file, but only as substrings of "replace" and "clippy". The words "command" and "Command" occur eight times, as part of "commands" and "Command format".
- Section 2, "Architecture and Data Flow", presents a four-stage pipeline — INPUT, DATA ENGINE, MODEL ENGINE, OUTPUT — and three invariants: multi-timeframe, candles-only, layer isolation. The OUTPUT bullet describes a combined time series and says it "is what strategies are built from". No stage or boundary represents a user interface.
- Section 3, "Coding Conventions", has a "Purity" rule forbidding I/O, clock, randomness, and global mutable state in indicator and rule computation. It does not say where I/O is permitted, because no layer has been designated to hold it.
- Section 4, "Testing", covers unit, integration, causality, determinism, golden, property, and edge-case testing. Nothing covers a command's rendered output.
- Section 5, "Documentation", requires rustdoc, module docs, a README update on user-facing change, and a `docs/` layout. It does not require a reference for the command set.
- Section 12, "Determinism and Reproducibility", covers clock, randomness, unordered iteration, scheduling, UTC, numeric stability, and provenance. Its scope is the computation. What the user is shown is not addressed.
- `README.md` is three lines: a title and "A deterministic, rule-based trading signal engine." It names no interface.
- The repository contains no Rust source, no `Cargo.toml`, no `tests/` directory, and no CI configuration. `cargo --version` returns `cargo 1.99.0` and `rustc --version` returns `rustc 1.99.0`, which establishes toolchain availability only. No build, test, lint, or doc run applies to this change, and none is claimed.
- Issue #5 is the only open issue. It was created at `2026-10-21T00:20:15Z`, which is 2026-10-20 in the repository's local time, and it carries the `documentation` label. It was moved to `In Progress` on the project board before this change was made to the working tree.

## Scope

Included:

- Add a product statement to `AGENTS.md` section 1: the deliverable is an interactive command shell, and no other interface exists.
- Add a `SHELL — the interface` bullet to section 2 and a note that the shell wraps the pipeline rather than being a stage of it.
- Edit the section 2 `DATA ENGINE` bullet so that the source access belongs to the shell and the engine never reaches for a source itself.
- Add four rules to section 3: shell boundary, command contract, discoverability, and time at the boundary. Edit the `Purity` rule to state that it binds the engine, and edit `Public API` to distinguish the internal Rust API from the product's command set.
- Add two requirements to section 4: shell command tests and deterministic rendering.
- Add a command reference requirement to section 5.
- Add a deterministic output requirement to section 12.
- Update `README.md` to name the shell as the interface.
- Record the change in an execution plan and an AI transparency log.

Explicit non-goals:

- No Rust source, no `Cargo.toml`, no `tests/` directory, no CI configuration.
- No specification of the command set. The command names that appear in the document are explicitly illustrative and are not a contract; fixing a command vocabulary is a separate decision that belongs to the human (section 14).
- No change to the fourteen numbered sections, their numbering, their titles, or their cross-references.
- No change to the issue, commit, or pull request formats, the orchestration protocol, the role model, or the approval gates.
- No change to any other repository.
- No merge of the pull request. That remains with the human.

## Assumptions and uncertainty

- **"Shell application" means an interactive command shell inside the Rust binary.** It does not mean a POSIX shell script, and it does not replace Rust as the implementation language. The human confirmed this choice on 2026-10-20 when the alternative was presented. The consequence is that section 3's toolchain rule and the whole Rust toolchain in section 4 stand unchanged.
- **The shell is an interface, not a pipeline stage.** The existing four-stage diagram describes data flow from a source to a combined series. The shell consumes that output and renders it, and it also feeds commands back into the pipeline. Treating the shell as a fifth stage would misdescribe the data flow, so the document states that the shell wraps the pipeline.
- **The shell is where impurity is permitted, and nowhere else.** Section 3's purity rule currently forbids I/O, the clock, randomness, and global mutable state without saying where such things may live. The shell is the answer, and the answer is what keeps the engines testable. This reading is an interpretation of an existing rule, not a new rule, and it is stated in the document so a reviewer can challenge it.
- **Determinism extends to what the user sees.** Section 12 governs computation. A trading system whose printed output varies between runs is not reproducible in the sense the section intends, so the visible output is brought under the same regime. This is an extension of an existing principle, and the human confirmed the scope of the change on 2026-10-20.
- **Illustrative command names are not a commitment.** The document names examples — printing a candle series, computing an indicator, running a backtest — because a product statement with no example is hard to act on. They are marked illustrative, and the command contract rule forbids changing an existing command's meaning without a human decision, so the examples cannot harden into an accidental interface.
- The human set the requirement directly on 2026-10-20 and approved this plan on the same date, authorizing the commit, the push, and the pull request that the procedure requires.

## Decisions

| Decision | Options considered | Choice | Rationale | Approval |
|---|---|---|---|---|
| What "shell application" means | Interactive command shell in the Rust binary; POSIX shell script; deliberately undecided | Interactive command shell in the Rust binary | A POSIX script reading would contradict section 3's Rust toolchain rule and require rewriting the toolchain, testing, and documentation rules. The interactive shell reading keeps the whole document coherent. | Human, 2026-10-20 |
| Width of the change | Sections 1 and 2 only; sections 1, 2 and README; the complete set plus README | Complete set plus README | A product statement that contradicts the purity, testing, and determinism rules elsewhere in the same file is a defect. Fixing the consequence in the same change is cheaper than fixing it later and cheaper than leaving it. | Human, 2026-10-20 |
| Whether to name example commands | Freeze a concrete command list; give no examples; illustrative and non-binding | Illustrative and non-binding | Section 3 forbids speculative abstraction, and a fixed vocabulary would be a product decision taken by an agent. Examples convey the shape of the product without committing to it. | Human, 2026-10-20 |
| Where the shell sits in section 2 | A fifth pipeline stage; a wrapper around the pipeline; a separate section | A wrapper around the pipeline | The pipeline describes data flow. The shell both consumes that flow and injects commands into it, so it is not a stage. | Agent, within the approved scope |
| How the command set is protected | No protection; a rule requiring a human decision to change a command's meaning; freeze the vocabulary now | A rule requiring a human decision | The command set becomes the product's public interface the moment it exists. The rule makes changing it a gated decision, which is the minimum needed to keep an agent from quietly redefining it. | Agent, within the approved scope |

## Acceptance criteria

Observable conditions for completion:

- `AGENTS.md` section 1 states that the deliverable is an interactive command shell and that no other interface exists.
- Section 2 names the shell as the interface, states that it wraps rather than extends the pipeline, and carries the shell-only-surface statement.
- Section 3 states where I/O, the clock, and terminal output are permitted, and that the shell holds no domain logic.
- Section 3 protects the command set: a command's meaning is not changed without a human decision, and every command is discoverable from a help listing.
- Section 4 requires a command's rendered output to be asserted in tests, and requires rendering to be byte-stable.
- Section 5 requires a command reference.
- Section 12 requires the shell's printed output to be deterministic.
- Command names that appear in `AGENTS.md` are marked illustrative and are not stated as a contract.
- `README.md` names the shell as the interface.
- The fourteen numbered sections, their numbering, and their cross-references are unchanged, and every cross-reference still resolves.
- `docs/2026-10-20/ai-transparency-log.md` records this change.
- `docs/2026-10-20/exec-plans/5-shell-application-surface.md` exists and is marked `Verified` once evidence is recorded.
- A pull request exists against `main` that closes #5, states the verification performed, and states the risks.

## Work breakdown

| Step | Action | Role | Paths affected | Depends on |
|---|---|---|---|---|
| 1 | File issue #5, add it to the board, move it to `In Progress` | Orchestrator | GitHub issue and board | — |
| 2 | Create the task branch from the latest `origin/main` | Implementation | git state | 1 |
| 3 | Write this execution plan, mark `Approved` | Orchestrator | `docs/2026-10-20/exec-plans/5-shell-application-surface.md` | 2 |
| 4 | Edit `AGENTS.md`: sections 1, 2, 3, 4, 5, and 12 | Implementation | `AGENTS.md` | 3 |
| 5 | Edit `README.md` to name the shell as the interface | Implementation | `README.md` | 3 |
| 6 | Append an entry to the AI transparency log | Orchestrator | `docs/2026-10-20/ai-transparency-log.md` | 3 |
| 7 | Verification pass: read the full diff, check numbering and cross-references, check the required plan and log contents, review the document against the acceptance criteria | Test Engineer, Code Reviewer | all changed files | 4, 5, 6 |
| 8 | Mark this plan `Approved` with observed evidence; the pull request criterion is still open | Orchestrator | this plan | 7 |
| 9 | Commit, push, open pull request | Orchestrator | git state, PR | 8 |
| 10 | Move issue #5 to `Done` on the board | Orchestrator | GitHub project board | 9 |
| 11 | Record the commit, the push, and the pull request in this plan and in the transparency log; mark this plan `Verified` | Orchestrator | this plan, the transparency log | 10 |
| 12 | Stop for the human's merge authorization | Orchestrator | — | 11 |

## Verification

### Verification evidence

Commands actually executed on 2026-10-20, with observed results:

- `git fetch origin --prune`, `git log --oneline -1 main`, `git log --oneline -1 origin/main`, `git rev-list --left-right --count main...origin/main` — the local `main` and `origin/main` were both at `13ddfd6`, with a count of `0 0`, so the local branch was not stale this time.
- `git checkout -b docs/5-shell-application-surface origin/main` followed by `git merge-base --is-ancestor origin/main HEAD` — exits zero, so the branch contains the current `origin/main`. `git diff --stat origin/main..HEAD` is empty, so the branch started from an identical tree.
- `grep -c "^## " AGENTS.md` — 14 top-level headings, numbered 1 to 14, contiguous, no duplicates. The change adds prose inside existing sections and adds no numbered section, so this count is unchanged. `grep -n "^## " AGENTS.md` places them at lines 3, 11, 37, 56, 73, 86, 101, 163, 200, 233, 413, 465, 480, and 546.
- A script parsed every `## N.` heading and every `section N` reference in the file. The headings are contiguous 1 through 14, the referenced set is {2, 3, 4, 6, 8, 9, 11, 12, 14}, and each reference resolves to exactly one heading. No unresolved reference.
- `grep -n -i -E "library|api|interface|server|service|deploy|embed|publish|crate|graphical|repl" AGENTS.md` over the finished file — the remaining hits are section 2's crate layout note, section 3's references to library code and dependencies, the "Public API" clarification, and the deployment and secret-handling language in the governance sections. None contradicts a shell-only product, because the new statement denies a library offered to a third party rather than denying internal crates.
- `git diff --check` — no whitespace errors. `tail -c 1` on `AGENTS.md`, `README.md`, and this plan returns a newline for each, so all three end with one.
- `wc -l` on `AGENTS.md` — 563 lines after the change, against 549 before it, a growth of 14 lines.
- `git diff --cached --numstat` — the change modifies `AGENTS.md` and `README.md`, and adds the execution plan and the AI transparency log entry. `git diff --numstat origin/main..HEAD` is empty before the commit, because `HEAD` is still `13ddfd6`; the staged diff is the correct form of that observation at this point in the procedure.
- An independent review pass was run by a separate agent reading the staged diff, the full file, and the acceptance criteria. It confirmed that no command name, syntax, or argument list is fixed by the change, that the fourteen headings and every cross-reference are intact, and that the log entry's structure matches section 6. It also raised defects that were corrected before staging: the data engine's source access was left in conflict with the shell boundary rule, the purity rule's prohibition on randomness and global state was left unqualified for the shell, the shell-only-surface statement sat inside the list of pipeline invariants, the clock permission could collide with the deterministic output rule, and the section 1 product sentence asserted an ordering and an argument shape that the illustrative marker did not cover. The corrections are recorded in the AI transparency log.
- The plan is checked against the ten required contents in `AGENTS.md` section 11: Intent, Current state, Scope, Assumptions and uncertainty, Decisions, Acceptance criteria, Work breakdown, Verification, Risks and recovery, and Stop conditions are all present by name, plus the Verification evidence subsection.
- The transparency log entry is checked against the four required items in `AGENTS.md` section 6: Date and tool, Intent and boundary, Verification and evidence, and Learning with the rejected and corrected suggestions nested under it.
- The complete diff was reviewed line by line before staging.
- The change was committed as `a328318` ("docs: define the shell application surface"), pushed to `origin/docs/5-shell-application-surface`, and opened as pull request #6 against `main`, closing issue #5. `git ls-remote --heads origin docs/5-shell-application-surface` returns `a3283188c921cc46546d35777cee6d1ce46bc8ec`, and `gh pr view 6` reports the pull request as OPEN from `docs/5-shell-application-surface` to `main`. Issue #5 was moved to `Done` on the project board, and its board status was confirmed by the mutation's response. The GitHub issue itself stays open until the pull request is merged, because the board status and the issue's own state are independent (section 9).

Acceptance criteria are evaluated against the evidence above. All are met, including the criterion that depends on the pull request existing, which is satisfied by pull request #6. The status of this plan was `Approved` in the commit that carried the change, because that criterion was not yet met at that point; it is changed to `Verified` in the follow-up commit that records the commit, the push, and the pull request, following the precedent set by the earlier entries in this repository. The merge itself is not part of the criteria and remains with the human.

Limitations, stated explicitly:

- The repository contains no Rust source, no `Cargo.toml`, and no `tests/` directory, so `cargo build`, `cargo test`, `cargo clippy`, and `cargo doc` do not apply to this change. `cargo --version` and `rustc --version` were run and establish toolchain availability only. No build, test, lint, or doc result is claimed.
- No automated markdown linter or link checker exists in the repository. Verification of a prose change is a manual read against the acceptance criteria, not a machine-checked result.
- The document describes a shell that does not exist yet. The statement is a product statement, and no claim is made that any command has been implemented.
- The rules added here govern agent behaviour and cannot be established by a test in this repository.

## Risks and recovery

- **The document grows further past its concision budget.** `AGENTS.md` was 549 lines and is now 563, a growth of 14 lines for the whole change, and earlier entries recorded that the length is already in tension with the "concise" criterion. Mitigation: the additions are confined to the six sections the shell actually affects, the new bullets are single sentences wherever the meaning survives compression, and no section is added. The growth is reported in the pull request rather than left implicit.
- **The shell statement contradicts an existing rule somewhere in the file.** Mitigation: the verification pass reads all fourteen sections against the new statement, and every section the shell touches — 1, 2, 3, 4, 5, and 12 — is edited in the same change rather than left to conflict. The independent review pass is what established that this was necessary: it found the data engine's source access and the purity rule both in conflict with the first draft of the shell boundary rule.
- **The shell statement is read as forbidding a library.** Mitigation: the wording states that no library is offered *to a third party*, and section 2's crate layout note, which records the internal crate layout when the workspace is created, is left intact. The internal crate layout is explicitly still a future recording, per section 2.
- **The illustrative commands harden into an accidental contract.** Mitigation: the document marks them as illustrative and section 3 requires a human decision before an existing command's meaning changes.
- **The purity rule becomes unusable.** Mitigation: the purity rule now states that it binds the engine, and the shell boundary rule designates the shell as the only layer permitted to perform I/O, read the clock, or write to a terminal, and it states explicitly that the shell may hold session state and may use seeded randomness. The prohibition in section 3 now has a boundary instead of an unqualified ban.
- **Recovery.** The change is a single documentation commit on a short-lived branch merged through a pull request, so it can be reverted with a new commit on `main`.

## Stop conditions

Stop and request human direction if:

- The shell statement cannot be written without changing an existing governance rule in another section.
- Applying the statement requires deciding something the human has not delegated, such as the actual command vocabulary, the command syntax, or whether the shell replaces or supplements any existing entry point.
- The push, the pull request, or the merge is rejected by repository protection settings.
- Any action would modify a repository other than `quant-rule/quant-rule`.
