# QuantRule — Agent Instructions

## 1. Project Context

QuantRule is a quantitative trading system that is multi-timeframe, rule-based, and OHLCV-candles-only. It focuses on performance and modularity, and it supports backtesting.

QuantRule ships as a shell application, and that is the whole of it. The result is an interactive command shell: the program is started, and the user writes commands at a prompt to work with candle series, indicators, and backtests. The commands are illustrative — printing a candle series, computing an indicator, running a backtest — and this document does not fix their names. There is no library offered to a third party to embed, no server or service to deploy, no graphical interface, and no second surface of any kind. Every capability the system has is reachable by typing a command, and every rule, indicator, and backtest it can perform is something the shell can run.

The system is deterministic. Given the same input data and the same configuration, it must produce the same output every time. Any behavior that depends on wall-clock time, unseeded randomness, or iteration order of unordered collections is a defect, not a limitation. See section 12.

## 2. Architecture and Data Flow

The system is a pipeline with four stages:

```
INPUT > DATA ENGINE > MODEL ENGINE > OUTPUT
```

- **INPUT — data sources.** Candles originate from one or more historic sources. Sources are interchangeable; nothing upstream of the data engine may assume a particular vendor, format, or transport.
- **DATA ENGINE — ingestion and processing.** Ingests candles from the shell, which performs the source access, then processes them and emits a time series of OHLCV candles per timeframe. The data engine owns candle normalization, timeframe construction, ordering, and gap handling. It knows nothing about indicators or strategies, and it never reaches for a source itself.
- **MODEL ENGINE — the analytical model.** Takes the candle series as input and produces a market state. It has three internal stages. **Indicator readings** compute the calculation families the model needs: exponential moving averages, relative strength index, average true range, relative volume, and confirmed swing pivots. **Four axes and event detection** turn those readings into published numbers: **direction**, **momentum**, **structure**, and **progression**, plus three shared event types — **level**, **divergence**, and **signal**. **Cross-timeframe aggregation** combines the per-timeframe results across the configured timeframe ladder, using a method that suits each axis. A reading is not owned by an axis: the same reading may serve a different operational role on each axis, and every role states its own definition. The model layer publishes numbers — continuous values, ordinal codes, and discrete codes — and never words; labels are a presentation mapping in the shell.
- **OUTPUT — the market state.** The per-timeframe metrics, the cross-timeframe aggregates, and the event records. This data is what strategies are built from and what drives improvement of the system. The shell renders it; it does not recompute it. The four axes are not combined into a single score: they describe different things, and the model publishes their agreement as counts.

The shell is not a fifth stage. It wraps the pipeline: it feeds commands into it and renders what comes back out. The stages above describe data flow, and the shell is the only surface through which that flow is reached.

- **SHELL — the interface.** The interactive command shell is the product's only user-facing surface. It owns the boundary between the human and the system: it parses a command, calls into the pipeline, and renders the result. It holds no domain logic. Its commands are illustrative — printing a candle series, computing an indicator, running a backtest — and this document does not fix their names. A capability the user cannot reach by typing a command does not exist for them: if it cannot be typed, it is not a feature.

Invariants that the pipeline depends on:

- Multi-timeframe: each timeframe is a first-class series with the same shape and semantics as every other timeframe.
- Candles-only: the system consumes OHLCV. Tick data, order books, and other externally derived series are out of scope.
- Layer isolation: the data engine never calls into the model engine, and the model engine never reaches back upstream. Dependencies point one way.
- Axis independence: the four axes are measured separately and are never averaged into one number. A cross-axis agreement output is a count.
- Availability, not occurrence: a measurement that becomes knowable only later — a confirmed swing pivot, a divergence — records both when it occurred and when it became knowable, and it never reaches an earlier bar.
- No double counting: a reading may feed more than one axis, and a correlated reading is not a second vote. Inside one axis, a feature is counted once.

The Rust workspace holds one crate per pipeline stage, plus the research engine, with dependencies pointing one way:

- `crates/quantrule-data` — the data engine.
- `crates/quantrule-model` — the model engine, which depends on `quantrule-data`.
- `crates/quantrule-shell` — the shell binary, which depends on `quantrule-model` and `quantrule-data`.
- `crates/quantrule-analytics` — the research engine, which depends on `quantrule-model` and builds the research dataset. It is not a pipeline stage; the shell owns its I/O.

The analytical model's contract — the four axes, the numeric output fields, the event model, and the aggregation rules — is written down in `docs/model/`; the research output contract is in `docs/analytics/`. The implementation follows both.

## 3. Coding Conventions

- **Toolchain.** The implementation language is Rust. Verify the actual installed toolchain and configuration before relying on version-specific APIs or flags. Do not assume that a documented or planned feature has already been implemented.
- **Formatting.** Format with `cargo fmt` and keep the result committed. Do not hand-format around rustfmt; if rustfmt output is wrong, change the rustfmt configuration deliberately and say so in the pull request.
- **Lints.** `cargo clippy --all-targets` must be free of warnings. Do not suppress a clippy lint to make the build pass. If a lint is genuinely inapplicable, allow it at the narrowest possible scope with a comment explaining why.
- **Naming.** `snake_case` for functions, modules, and files; `CamelCase` for types and traits; `SCREAMING_SNAKE_CASE` for constants and statics. Keep names descriptive of the domain (candle, timeframe, indicator, signal) rather than of the implementation.
- **Errors.** Library code returns `Result` and uses explicit error types. `unwrap` and `expect` are not allowed in library code; they are acceptable in tests and in binary entry points where a failure is unrecoverable and the message states what to do next. Library code must not panic on invalid input.
- **`unsafe`.** Not allowed without a written justification that explains the invariant being upheld and the consequences of violating it, plus explicit human approval.
- **Dependencies.** Adding a dependency is a decision, not an implementation detail. It requires approval (section 14) and must be justified in the pull request. Do not add a crate to work around a problem the standard library or an existing dependency already solves.
- **Purity.** Indicator and rule computation is a pure function of its inputs. No I/O, no clock, no randomness, no global mutable state. Read data from a passed-in parameter rather than fetching it. This rule binds the engine; the shell is where the impure work is allowed to happen.
- **Shell boundary.** The shell is the only layer permitted to perform I/O, to read the clock, or to write to a terminal. It parses the command, calls into the pipeline, and renders the result; it does not compute an indicator, apply a rule, or reshape data. Logic that appears to belong in the shell belongs in the engine. Source access belongs to the shell: it reads or fetches the raw candles and hands them to the data engine, and the engine never reaches for a source itself. The shell may hold session state between commands and may use randomness with an explicit seed (section 12); that state is passed into the engines as a parameter and is never read behind their backs.
- **Command contract.** Every command has a stable contract: a name, its arguments, its output, and its error behavior. The command set is the product's public interface, and changing what an existing command means is a decision reserved to the human (section 14). A new capability arrives as a new command, not as a change to an old one.
- **Discoverability.** Every command is reachable from a help listing that names its arguments and the shape of its output. A capability the user cannot discover from the shell does not exist.
- **Time at the boundary.** The shell may read the clock, and passes what it reads to the engine as an explicit parameter (section 12). No engine reads the clock, and a clock value is never printed as part of a command's output, so what the user sees stays reproducible.
- **Causality.** An indicator or rule may not read a value from beyond the current bar. No future value may influence a past output, even to decide that no signal is present. A backtest that leaks the future is a defect, not a simplification.
- **Axis roles.** A reading is not owned by an axis. The same reading may be used by more than one axis, but each use states its own definition, and the same feature is never counted twice inside one axis. A correlated reading is not a second vote.
- **Numeric model output.** The model layer publishes numbers, nulls, lists of numbers, and records of numeric values. It publishes no labels, no confidence values, and no state strings; the shell maps numbers to words. A missing measurement is null with an unavailable-reason code, and is never replaced by zero. A valid zero is a value.
- **Ordinal codes are counted, not averaged.** An ordinal or discrete code is published, counted, and compared. It is never averaged into a continuous number, and codes from different scales are never combined. A score defined inside one axis from distinct discrete measurements of that axis is not an averaging of codes, and each measurement is counted once.
- **Event records.** An event record carries its type, its subtype, its source, its timeframe, the bar on which it occurred, the first bar on which it became knowable, its direction, and its measured values. It is appended once, when it becomes knowable, and it is not upgraded in place. An event is evidence, and it does not alter an axis value.
- **Modularity.** Keep modules cohesive and dependencies pointing one way (section 2). Introduce an abstraction only when it removes real duplication or encodes a real invariant, and never speculatively.
- **File size.** A source file stays under 1,000 lines where that is practical. A file that grows past it is split along its own seams, so that a cohesive module becomes several, unless a written justification says why it cannot be split. The line count is a prompt to look, not a target to hit: a cohesive module is not broken up for the sake of the count.
- **Public API.** Public items carry rustdoc. Doc examples must compile. Public behavior changes require a doc comment update in the same change. The Rust public API is internal to the workspace; the product's public interface is the command set (section 2).
- **Comments.** Explain why, not what. The code says what it does; the comment says why that way, and what would break if it changed.

## 4. Testing

- **Command.** `cargo test` must pass before a pull request is proposed. Do not weaken, remove, or skip a test to obtain a passing result.
- **Unit tests** live next to the code they cover, in the same module. Integration tests live in the `tests/` directory of the crate that owns the code they cover, because a virtual workspace root holds no package and a root `tests/` directory is never compiled.
- **Future paths.** The `cargo` commands above describe the layout the Rust workspace will use. Until the workspace exists, no test command applies, and no claim of a passing test run may be made.
- **Causality tests.** An indicator or rule that consumes a series must have a test proving that its output for a given bar does not change when later bars are appended, changed, or removed. This is the direct test for the causality rule in section 3.
- **Determinism tests.** Any computation that participates in signal generation must have a test asserting that the same input yields the same output across repeated runs, across a reordered collection, across a change in the iteration order of any unordered map or set it traverses, and across a different split or scheduling of the work. This is the direct test for section 12.
- **Golden tests.** Indicator and rule output is compared against a checked-in fixture of expected values, so a numerical or structural regression fails loudly rather than silently. Fixtures are data, not code, and are regenerated deliberately — never casually — when an intentional change alters expected output.
- **Property tests** cover invariants that should hold across the whole input space, such as output length matching input length, monotonicity, and the absence of values that were never supplied.
- **Backtest reproducibility.** A backtest over a fixed input and configuration is expected to produce byte-identical results across runs. This is covered by a test, not just asserted in prose.
- **Edge cases.** Explicitly cover: an empty series; a series with a single candle; input shorter than an indicator's required history; a timeframe boundary where candles do not align; gaps and missing candles; a candle with zero volume; and equal highs or lows where a rule must choose a tie-breaking behavior.
- **Model-layer tests.** Every axis and every reading has a causality test, a determinism test, a golden fixture, and edge-case coverage: an empty series, a single candle, a series shorter than the required history, an unaligned timeframe boundary, a gap, a candle with zero volume, and equal highs or lows where a tie must be broken.
- **Event tests.** An event appears only at or after the bar on which it became knowable, one underlying occurrence is never emitted twice, the level-event precedence is respected, and a divergence is never emitted before the pivot that confirms it has become knowable.
- **Progression tests.** A leg's stage is null while its origin is unconfirmed or its target is unstable, every lifecycle code is reachable, and a target revision changes the target version without a price movement.
- **Aggregation tests.** A cross-timeframe aggregate uses a method that suits its axis: ordinal codes are counted rather than averaged, and every aggregate names the timeframes it came from.
- **Isolation.** Tests do not reach the network, the filesystem outside the repository, or a live market. They do not depend on the wall clock, the execution order of tests, or the machine they run on.
- **Shell tests.** A command is tested by running it and asserting on what it printed, against fixture data and a fixed configuration. A command's output is observable behavior, not a side effect.
- **Rendering.** The rendered output is deterministic (section 12): the same command over the same data and configuration prints the same text, byte for byte, whether it was typed at the prompt or passed to a non-interactive invocation.
- **Performance-sensitive paths** are covered by a benchmark when a change is expected to affect them, and the benchmark result is reported in the pull request rather than left unmentioned.
- **Coverage claims** must reflect what a test actually exercises. A passing test that does not reach the behavior in question does not prove that behavior.

## 5. Documentation

- **Rustdoc** on public items, with examples that compile. `cargo doc` must build without warnings. Until the Rust workspace exists, this command does not apply and no doc-build claim may be made.
- **Module docs** with `//!` at the top of a module that explain the module's role and its boundary against neighboring modules.
- **README** is updated in the same change as any user-facing behavior change.
- **Command reference.** The command set is documented — the name of each command, its arguments, its output, and a worked example — in the README or under `docs/`. A change to a command's contract updates that reference in the same pull request.
- **Concise documentation.** A table or a short statement carries the same information more clearly than a paragraph of prose, and prose that repeats a table is noise. Keep the context a reader needs in order to act, and drop the rest.
- **Repository layout** under `docs/`. The layout is `docs/<kind>/<YYYY-MM-DD>/<file>.md`: the kind comes first, then the UTC date of the work item, then the file. A kind directory is created when it is first used, not before. `docs/README.md` is the index of the tree and sits outside this layout.
  - `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md` — approved execution plans, per section 11.
  - `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md` — the AI transparency log required by section 6. One file per work item, named to match its execution plan.
  - `docs/model/<YYYY-MM-DD>/<file>.md` — the analytical model's contract: the four axes, the numeric output fields, the event model, and the aggregation rules. The implementation follows it.
  - `docs/adr/<YYYY-MM-DD>/<file>.md` — architecture decision records, describing the decision, the alternatives considered, and the consequences.
- **Documentation follows the code.** A change that invalidates a documented behavior updates that document in the same pull request, not in a follow-up.
- **No stale documentation.** If a document describes behavior that no longer exists, the change that removed the behavior removes the description.

## 6. AI Use Policy

### AI transparency log

Record material AI contributions in `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md`, one file per work item, named to match its execution plan where the work item has one, including assistance with documentation, implementation, testing, and architectural analysis.

Each entry must record:

- Date and tool. The date is UTC (section 12).
- Intent and boundary.
- Verification and evidence.
- Learning, including relevant suggestions that were rejected or corrected.

Record actual usage, not idealized examples. Distinguish proposed work from implemented work, and verified claims from assumptions. Never invent test results or verification evidence. Do not put credentials, secrets, or unnecessary sensitive information in the log.

## 7. Git Workflow and Pull Requests

Default workflow: GitHub Flow.
QuantRule uses short-lived task branches and pull requests. Do not introduce full Gitflow (develop, release/*, and hotfix/*) unless the human explicitly approves a change to the release strategy.

`main` is the shared integration branch.
Work on one logical task per branch.
Never work directly on `main`.
Keep changes focused and branches short-lived.
Do not mix unrelated changes in one pull request.

### Branch Naming

Use the following conventions:

- `feat/<issue>-<slug>` — new functionality.
- `fix/<issue>-<slug>` — bug fixes.
- `docs/<issue>-<slug>` — documentation.
- `test/<issue>-<slug>` — test-related changes.
- `refactor/<issue>-<slug>` — structural changes without intended behavior changes.
- `arch/<issue>-<slug>` — architecture documentation and decision artifacts.
- `chore/<issue>-<slug>` — maintenance and tooling.

When no issue number exists, use a concise descriptive slug without inventing an issue reference.

### Branch Preparation

Every issue is worked in a dedicated Git worktree and on a dedicated branch. The worktree is the default, not an exception: a session that starts in the project root, in another worktree, or anywhere else still works in its own worktree for the task it was given.

Before changing Git state:

- Inspect the current working directory, the current branch, and the uncommitted changes, with `git status --short --branch` and `git worktree list`.
- Inspect the relevant history and the existing remote configuration.
- Check for uncommitted changes and preserve all unrelated work.
- Inspect the existing worktrees and branches, and confirm that the requested branch and base branch are appropriate.
- Follow the approved execution plan for branch creation.
- Use the latest available `main` as the starting point when appropriate, unless the approved plan names a different base. Never discard, overwrite, reset, or stash existing work without explicit approval.

Creating the worktree:

- Leave the project directory on `main`. A task does not switch the project directory to its own branch.
- Create the task's worktree in a sibling directory named `<repository>--<task-slug>`, for example `quant-rule--32-agent-guidelines`, so that the path itself says which task owns it.
- Name the branch with the conventions above, `<type>/<issue>-<slug>`.
- Reuse an existing worktree for the same task when one is already present, and keep the task's ownership, its issue status, its execution plan, and its write boundaries consistent with the plan.

Working in the worktree:

- The session starts in its worktree and continues in it. A file edited in the project directory is not the task's file.
- Never switch the branch of a working directory that another session is using. A branch switch in a shared directory changes that session's files underneath it.
- Never assume that a new terminal tab isolates the working files. The directory is what isolates them.
- If a session cannot be started in the correct worktree automatically, ask the human to open one there rather than continuing in the original directory.
- The implementation, verification, commit, push, pull request, and merge workflow is unchanged, and every approval gate still applies.

### Commit Convention

Use Conventional Commit-style messages:

- `feat: add ema crossover rule`
- `fix: correct swing high detection`
- `docs: document the system overview`
- `test: cover multi-timeframe alignment`
- `arch: document modular monolith decisions`
- `chore: update development tooling`

Each commit should represent a coherent change. Do not amend shared commits, rebase published branches, force-push, or rewrite history without explicit authorization.

### Pull Request Requirements

A pull request must include:

- **Summary:** What changed and why.
- **Scope:** What is included and deliberately excluded.
- **Issue:** A link to the relevant issue, when one exists.
- **Verification:** Commands actually executed and their observed results.
- **Risks:** Relevant security, data, migration, compatibility, or operational risks.
- **AI transparency:** A reference to material AI contributions recorded in `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md`, when applicable.

Run the relevant checks before proposing a pull request. Review the complete diff for unrelated changes, generated files, secrets, and accidental modifications.

## 8. Issue, Commit, Pull Request and Response Text

Keep every issue, commit message, pull request, and response to the human brief, human-readable and in the same format every time.

### Issue format

Keep the whole body under roughly 15 lines:

```
Goal: two or three sentences.
Scope: short bullet list.
Done when: short bullet list of observable conditions.
```

### Commit format

Conventional Commit subject; body of at most five short lines saying what changed and why.

### Pull request format

Summary (two or three sentences), `Closes #issue`, Changes (short bullets), Verification (short table or bullets with observed results), Risks (or "None"). Do not restate the issue, do not pad with prose. State the claim type in one line.

### Response format

Every final response to the human ends with a single closing line stating what is required from the human. This applies whether the response reports completed work, a blocker, or a question for the human to decide. It does not apply to intermediate messages inside a longer session.

- The closing line is the last thing in the response. Nothing follows it.
- The line is **bold**, so it is visually distinct from the body of the response.
- The line is **at most one phrase long**. A phrase, not a sentence: a sentence carries its own clause and can hide a second instruction inside it. Write `**Review PR #4**`, not `**Please review PR #4 and then confirm the board status before I move on.**`
- The line states either the action required from the human, or that none is required.
- When the work is complete and nothing is needed, the line says so: `**No action**`. Do not invent a pending action to fill the line.
- When several actions are needed, name the single next one. The rest belong in the body of the response.
- When the response is a blocking question, the question itself is the action, and the line states what the human needs to decide.
- This rule governs the agent's own response to the human. The formats above govern artifacts written into the repository. Both apply to the same piece of work.

Apply this rule to the final response of every task, including a task that ends with nothing to do.

## 9. Git Approval Gates

Creating a task branch and its worktree may be included in an approved execution plan. The following require separate explicit human authorization unless the human's current instruction explicitly requests the operation:

- Creating commits.
- Pushing to a remote.
- Creating or updating a pull request.
- Merging or enabling automatic merging.
- Creating releases or tags.
- Deploying to any environment.
- Deleting branches or performing destructive Git operations.

One exception, and it is a routine one: after a task's pull request has been accepted and merged and its acceptance criteria have been verified, removing the task's worktree and deleting its task branch, locally and on the remote, is the default cleanup and needs no separate authorization, provided the issue is marked `Done` on the board. Before the cleanup, verify that no uncommitted or unmerged work will be lost; when that cannot be established, preserve the work and report a blocker instead of deleting anything. Every other destructive Git operation keeps its gate.

The human remains responsible for final review and merge authorization. An AI reviewer cannot approve or merge its own implementation.

Branch protection, required checks, and merge restrictions should be configured in GitHub where available. Instructions in this file describe expected behavior but do not independently enforce repository permissions.

### Issue Status Lifecycle

Every issue is tracked on the project board and moves through a fixed sequence. The board status is the source of truth for what is being worked on.

- **Todo.** A newly opened issue starts in `Todo`. It has been filed but no work has begun.
- **In Progress.** Move the issue to `In Progress` before the first change is made to the working tree. An issue that is being worked on is never left in `Todo`.
- **Done.** Move the issue to `Done` once the work is committed, the pull request is open, and the acceptance criteria are met. The issue is not closed on the board before the pull request exists.

Rules for the lifecycle:

- Update the board status as part of the work, not as a separate task. A status change is part of the same change that advances the issue.
- One work item is `In Progress` at a time unless the human explicitly authorizes parallel work. A container that mirrors its sub-issues is not a work item in flight and does not occupy the slot.
- Moving an issue to `In Progress` requires no separate approval. It is an ordinary part of starting work.
- Moving an issue to `Done` requires the pull request to exist and be open. A container is the exception: its own deliverable was merged when it was delivered, and it is moved to `Done` when its last sub-issue is done.
- The board status and the issue's own state are independent. A `Done` board item may still be open on GitHub, and an open GitHub issue may still be in `Todo`.
- An issue with sub-issues is a container. Its acceptance criteria cover its own deliverable, and the work it delegates is stated in the criteria of its sub-issues, which carry the rest. A container's board status follows its children: it is `In Progress` while any sub-issue is open, and `Done` only when every sub-issue is done. The title of a container says what it delivers, so the board does not read as though the delegated work were done.
- The board is kept in step by workflows rather than by hand. The project's built-in workflows mark an item `Done` when its issue is closed, and close the issue when the item is marked `Done`. A repository workflow owns the `State`, `Item type`, and `Closed date` fields, and recomputes a container's `Status` from its sub-issues. The agent sets a leaf's `Todo`, `In Progress`, and `Start date`, and does not set what a workflow owns.
- If work stops, the issue returns to `Todo` unless it is genuinely still in progress.

### Decomposing an issue into sub-issues

An issue that grows too large, or that comes to hold more than one deliverable, is split into sub-issues. The rules for the split:

- Each sub-issue is one coherent deliverable that can be verified on its own. A part that cannot be reviewed, tested, or merged by itself is not a sub-issue.
- Each sub-issue states its scope, its acceptance criteria, and the sub-issues it depends on.
- Split along the deliverables, not along convenience. A part that shares no output with its neighbours belongs to its own sub-issue.
- The parent keeps the overall objective and the criteria for the whole, and the progress is tracked through its sub-issues.
- Do not fragment for its own sake. One deliverable is one sub-issue, and a sub-issue smaller than the work of specifying it is not a sub-issue.
- A sub-issue follows the same lifecycle, the same board status rules, and the same approval gates as any other issue.

### Board Contents

The board holds both the issue and the pull request that closes it. The human reads the board to see what is in flight and what is done, and neither kind of item is optional to them.

- A pull request that closes a tracked issue is added to the board by GitHub's own automation, within a second of being opened and without anyone asking for it. A board item is therefore not evidence that something was added on purpose, and the `Item type` field is what tells an issue from a pull request.
- The `Item type` field carries `Issue` or `Pull request` on every item. An item without a value is a defect, not a new kind of item. A pull request that the automation adds arrives without a value, so it is labelled when it appears.
- The `State` field carries `Open` or `Closed` on every item, and a merged pull request is closed. The built-in date fields are not populated on this board, so `State` is the only field that says whether an item is still open.
- An issue and its pull request share a title, so the board is read with `Item type` in view. `Labels` is a second signal in this repository, where issues carry labels and pull requests do not; that is a property of how issues are labelled rather than a rule, and nothing depends on it.
- The `Status` field is the source of truth for the lifecycle, and it is the field that says which work is in flight.
- The `All items` view lists every item in one table: open issues, closed issues, and pull requests. The `Board` view separates the same items across the columns of the lifecycle, so the table is the view to read when the question is what exists.
- A view that cannot be configured through the API is configured by the human in the browser. The `Timeline` view is one: the API sets its name and layout but not the date field its timeline is drawn from, so the human picks that axis, and the board's `Start date` and `Closed date` fields are what it draws.

## 10. Agentic Engineering and Role Definitions

AI agents operate under explicit responsibilities and permissions. Role separation exists to improve traceability, verification, and the detection of mistakes—not to transfer decision authority away from the human.

### Human Developer — Decision Owner

The human is the sole final authority for project decisions.

Responsibilities:

- Approve requirements, priorities, scope, and non-goals.
- Decide business, architectural, methodological, technical, security, and operational questions that are not already explicitly settled.
- Approve execution plans and material changes to them.
- Authorize commits, pushes, pull requests, merges, deployments, and other gated operations.
- Make the final acceptance decision.

No AI role may impersonate the human, manufacture approval, or treat its own recommendation as an approved decision.

### Orchestrator / Coordinator Agent

Purpose: Coordinate the task from initial investigation through evidence and handoff.

Responsibilities:

- Inspect the repository and applicable instructions.
- Determine which specialist roles are useful for the task.
- Prepare the scope, dependencies, proposed execution plan, risks, and verification criteria.
- Assign bounded responsibilities to agents.
- Collect findings and identify contradictions between their outputs.
- Present unresolved decisions and recommendations to the human.
- Coordinate testing, reviews, remediation, and reporting.
- Ensure that material AI usage is recorded.

Boundaries:

- Must not independently settle conflicting requirements or architectural alternatives.
- Must not override the human's decisions.
- Must not conceal failed checks or conflicting agent findings.
- Must not claim that an independent review occurred when it did not.
- Must stop at approval gates.

### Domain / Requirements Analyst Agent

Purpose: Establish the expected behavior and scope.

Responsibilities:

- Read the execution plan, the relevant issues, and the surrounding implementation.
- Identify users, domain rules, acceptance criteria, assumptions, and edge cases.
- Detect contradictions or gaps in requirements.
- Propose a thin end-to-end behavior and explicit non-goals.
- Identify questions requiring human direction.

Boundaries:

- May propose interpretations but cannot invent or finalize business rules.
- Must not expand scope merely because additional features appear useful.

### Software Architect Agent

Purpose: Analyze structural consequences and architectural trade-offs.

Responsibilities:

- Assess module boundaries, cohesion, coupling, dependencies, and data ownership.
- Derive relevant architecture characteristics from domain concerns.
- Identify suitable measurements and verification methods.
- Compare architectural alternatives, failure modes, and consequences.
- Identify whether an ADR or architecture-documentation update is needed.
- Review alignment with previously approved architectural decisions.

Boundaries:

- Provides recommendations, not approval.
- Cannot independently select a new architecture, technology, dependency, or system boundary when human authorization is required.
- Must explain assumptions and trade-offs rather than appealing to popularity or convention alone.

### Implementation Agent

Purpose: Implement an explicitly approved scope.

Responsibilities:

- Follow the approved execution plan and existing conventions.
- Modify only the authorized scope.
- Preserve existing behavior unless the approved requirements state otherwise.
- Produce focused changes with clear verification evidence.
- Report deviations, blockers, and unexpected dependencies.

Boundaries:

- Cannot redefine requirements, expand the task, or make unapproved consequential design decisions.
- Cannot weaken security controls or tests merely to obtain a passing result.
- Must stop when implementation requires a new decision outside the approved scope.

### Test Engineer Agent

Purpose: Design and execute verification for the approved requirements.

Responsibilities:

- Translate acceptance criteria and domain invariants into test cases.
- Identify boundary cases, failures, retries, concurrency races, and regressions.
- Add or update tests within the approved scope.
- Run applicable test commands and report observed results.
- Identify gaps between what a test proves and what the requirement claims.

Boundaries:

- Cannot declare a claim proven by a test that does not exercise the relevant behavior.
- Cannot remove, weaken, or bypass tests to make implementation pass.
- Must distinguish mocked behavior from actual database, integration, or production behavior.

### Code Reviewer Agent

Purpose: Perform a critical review of the proposed changes.

Responsibilities:

- Inspect the complete diff and relevant surrounding code.
- Look for correctness defects, regressions, unsafe assumptions, unnecessary complexity, and violations of approved requirements.
- Report actionable findings with severity, file or line references, and supporting reasoning.
- Identify untested paths and claims that lack sufficient evidence.

Boundaries:

- Read-only by default.
- Does not silently modify the implementation it reviews.
- Does not approve, merge, or authorize its own changes.
- A clean review does not replace tests, security checks, or human approval.

### Security and Privacy Reviewer Agent

Purpose: Review security-sensitive changes when applicable.

Invoke this role for authentication, authorization, access control, sensitive configuration, secrets, dependency vulnerabilities, and security-related infrastructure changes.

Responsibilities:

- Check trust boundaries, least privilege, input validation, and authorization.
- Inspect permissions, grants, and privileged operations.
- Check that credentials and sensitive information are not exposed in client code, logs, or commits.
- Examine whether secrets, API keys, and market data endpoints are handled according to approved rules.
- Recommend mitigations and verification tests.

Boundaries:

- Cannot independently approve a security trade-off or waive a control.
- Reports residual risks rather than claiming absolute security.
- Must escalate unresolved material risks to the human.

### Release / Operations Verifier Agent

Purpose: Verify deployment readiness and operational consequences.

Responsibilities:

- Check build, test, configuration, and deployment prerequisites.
- Assess rollback or recovery implications for consequential changes.
- Identify missing environment variables, undeclared services, and unverified commands.
- Confirm that documented operations match the observed repository configuration.
- Prepare a release-readiness report.

Boundaries:

- Does not deploy, merge, publish a release, or modify external environments without explicit authorization.
- Must distinguish local verification from staging or production evidence.

### Role Assignment Rules

Not every task needs every role.
Trivial documentation changes may need only the orchestrator, implementer, and a basic verification pass.
Non-trivial behavior changes should include testing and code review.
Architecture-sensitive work should include the architect role.
Security- or privacy-sensitive work must include security review.
Database migrations, concurrency-sensitive changes, and destructive operations require explicit risk assessment and appropriate independent verification.
When the available tools cannot start separate agents, the orchestrator may conduct sequential role-specific review passes. It must disclose that these were not independent agents or independent model judgments.

Only one implementation agent should write to a given file or workstream at a time. Parallel investigation is permitted when scopes are non-overlapping, clearly assigned, and consistent with the approved plan. Separate sessions work in separate worktrees (section 7), so two of them do not contend for the same files.

## 11. Execution Plans

An execution plan makes substantial work reviewable before implementation. It is separate from the permanent repository rules in this file.

### When a Plan Is Required

Prepare a formal plan before implementation when a task:

- Changes multiple components or has several dependent steps.
- Introduces or changes an architectural decision.
- Modifies database schemas, migrations, authorization, or critical transactions.
- Changes concurrency behavior or business invariants.
- Has significant security, privacy, operational, or recovery implications.
- Contains material uncertainty that could change the proposed solution.

For a small, low-risk change, a concise scope and verification checklist may be sufficient.

### Execution Plan Location

Use the following path for approved project plans:

`docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md`

The date segment is the UTC date on which the work item was recorded (section 12).

Do not invent issue numbers. If the task has no issue, agree on an appropriate filename with the human.

### Required Plan Contents

A formal plan must include:

- **Intent:** The problem, intended outcome, and affected users.
- **Current state:** Relevant files, observed behavior, and existing constraints.
- **Scope:** Included work and explicit non-goals.
- **Assumptions and uncertainty:** What could invalidate the proposed solution.
- **Decisions:** Options, consequences, trade-offs, and human approval status.
- **Acceptance criteria:** Observable conditions for completion.
- **Work breakdown:** Ordered steps, dependencies, responsible agent roles, and affected paths.
- **Verification:** Test commands, expected evidence, and limitations.
- **Risks and recovery:** Relevant failure modes, migration safety, rollback, and recovery considerations.
- **Stop conditions:** Conflicts, new decisions, missing permissions, or failures requiring escalation.

### Plan Approval and Execution

Present a read-only repository assessment before proposing implementation.
Mark the plan `Proposed` until the human approves it.
Do not implement a proposed plan.
After approval, mark it `Approved` and execute only the authorized scope.
Record material findings and verification evidence as work progresses.
If new information requires a material change in scope, architecture, requirements, risk, or operational behavior, stop and request approval for the revised plan.
Mark a plan `Verified` only when its acceptance criteria have been evaluated against observed evidence. Record unresolved criteria rather than claiming full completion.

Plan approval authorizes only the work explicitly described in that plan. It does not automatically authorize commits, pushes, merges, deployments, or unrelated changes.

## 12. Determinism and Reproducibility

QuantRule is a deterministic, rule-based system. A backtest that cannot be reproduced is a defect.

- **Same input, same output.** The same input candles and the same configuration must produce the same output, every time, on every machine. This holds across repeated runs, across process restarts, and across changes in unrelated code.
- **No wall-clock time in the signal path.** Computation that determines a signal may not read the system clock. Time is a parameter, injected by the caller, and is normally derived from the data.
- **No unseeded randomness.** Any random component takes an explicit seed, and the seed is part of the configuration that is recorded with the run.
- **No dependence on unordered iteration.** Iteration over a `HashMap` or `HashSet` must not affect output. Sort explicitly, or use an ordered collection.
- **No dependence on thread scheduling.** Concurrent work must not make results depend on the order in which threads happen to finish.
- **UTC.** All timestamps are UTC, stored and compared in a single explicit format. A timezone is a configuration value, never an ambient environment default.
- **Numeric stability.** The same computation must not produce a different result merely because it was split differently. Where floating-point accumulation order could matter, the order is fixed and documented.
- **A fixed numeric contract.** The percentile definition, the smoothing seeds, the clamp bounds, and the accumulation order are fixed in the model's contract document and are not re-derived at each call site. Only addition, subtraction, multiplication, division, comparison, and the absolute value appear in the signal path, together with the rounding and sign operations the contract defines, so the same input yields the same values on every conforming platform.
- **Bounded retention.** A per-timeframe event history is kept in a bounded buffer whose size is recorded in the model contract. The window counts the model publishes are computed from the retained records, and the bound is large enough to cover every published window.
- **Recorded provenance.** A result that is meant to be reproducible records the data version and the configuration that produced it, so a later run can be shown to have used the same inputs. The data version is the identity of the input series — its source, timeframe, and coverage — and is set by the caller, not inferred from the filesystem.
- **Deterministic output.** What the shell prints is part of the reproducible result. The same command, the same data version, and the same configuration produce the same output in the same order, every time, and a non-interactive invocation prints what the interactive one prints.
- **Determinism is tested.** These are properties of the code, so they are covered by tests as described in section 4, not merely asserted here.

## 13. Orchestration and Execution Protocol

Follow these stages in order for planned work.

### Stage 1 — Read-only Preflight

The orchestrator inspects the current working directory, branch, working tree, worktrees, repository structure, applicable instructions, requirements, relevant implementation, and existing tests.

It reports:

- Current state and relevant evidence.
- Proposed task scope and non-goals.
- Existing constraints and dependencies.
- Unresolved questions and required decisions.
- Whether specialist roles or a formal execution plan are needed.

No implementation changes are made during this stage.

### Stage 2 — Analysis and Planning

The relevant analyst and architect roles investigate the approved problem boundary. The orchestrator consolidates their findings without silently resolving disagreements.

The proposed plan must identify the exact intended behavior, acceptance criteria, work boundaries, test strategy, and any decisions reserved for the human.

Human gate: Wait for explicit approval before implementing.

### Stage 3 — Controlled Implementation

The implementation agent follows the approved plan.

- Keep changes focused and reviewable.
- Avoid overlapping writes from multiple agents.
- Use existing libraries and project conventions unless an approved decision says otherwise.
- Apply relevant tests and checks incrementally.
- Stop if the implementation requires an unapproved decision or scope expansion.

### Stage 4 — Independent Verification

The test engineer, code reviewer, and applicable specialists evaluate the result.

The orchestrator consolidates findings, distinguishing:

- Confirmed defects.
- Potential defects requiring additional evidence.
- Failed checks.
- Unverified claims.
- Residual risks.

Within-scope corrections can proceed under the approved plan. Material changes require renewed human approval.

### Stage 5 — Evidence and Handoff

Before reporting completion:

- Review the full diff.
- Run relevant validations and record their actual results.
- Confirm acceptance criteria and identify gaps.
- Update the execution plan with evidence and outstanding risks.
- Update `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md` for material AI contributions.
- Move the issue to `Done` on the project board once the pull request is open (section 9).
- Report changed files, tests, results, limitations, and any decisions still required.

The report itself follows the response format in section 8.

Stop without beginning an unrequested subsequent task.

## 14. Human Approval Gates

Human approval is required for decisions not already settled by explicit instructions or approved project rules.

Examples include:

- Changes to requirements, signal generation logic, risk rules, or product scope.
- Architecture and module-boundary changes.
- New dependencies or materially different implementation approaches.
- Security, privacy, authorization, and data-retention decisions.
- Schema changes and consequential migration or recovery decisions.
- Changes to development methodology, Git conventions, or approval procedures.
- Acceptance of material residual risk.

The Git operations gated in section 9 require separate human authorization and are not repeated here.

For every unresolved decision, present the context, viable alternatives, consequences, risks, and a recommendation when justified by evidence. Wait for explicit human authorization before committing to the choice.

Approved requirements and conventions permit execution within their boundaries; they do not authorize silently changing those boundaries. When instructions conflict or the available evidence is insufficient, stop and ask.
