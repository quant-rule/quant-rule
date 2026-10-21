# Execution Plan: The Data Engine

- Issue: [#9](https://github.com/quant-rule/quant-rule/issues/9) — feat: build the data engine
- Branch: `feat/9-data-engine`
- Worktree: created when the work starts, per `AGENTS.md` section 7
- Status: Proposed — prepared in the environment-setup work item, not yet approved, and no implementation has started
- Date: 2026-10-21 (UTC; 2026-10-20 local time)
- Transparency log: [`docs/ai-transparency-log/2026-10-21/9-data-engine.md`](../../ai-transparency-log/2026-10-21/9-data-engine.md)

This plan was written on 2026-10-21 during the environment-setup work item, so that the data engine can be started without re-deriving its design. No Rust code was written: the human stopped the implementation at that point and asked for the environment to be set up instead. The plan therefore states a design and a decision list, and it awaits the human's approval before any of it is built.

## Intent

QuantRule has no code. The pipeline described in `AGENTS.md` section 2 starts at a data engine that does not exist, and the model engine behind it is blocked on it. This work item creates the Rust workspace, the candle and timeframe types, the normalization and timeframe construction the model engine consumes, and the shell surface that reaches them.

Affected users: the human developer, who owns the product's only interface; and the model-engine agents, which need a normalized series per timeframe to do anything at all.

## Current state

Evidence gathered on 2026-10-21 UTC, `origin/main` at commit `5523837` (the merge of pull request #31).

- The repository contains `AGENTS.md` (592 lines), `README.md`, and `docs/`. There is no `Cargo.toml`, no Rust source, no `tests/` directory, and no CI configuration. `cargo --version` returns `cargo 1.99.0` and `rustc --version` returns `rustc 1.99.0`, which establishes toolchain availability only.
- `AGENTS.md` section 2 states that the workspace "is expected to hold one crate per pipeline stage — a data crate, a model crate, and the shell binary — with dependencies pointing from the shell to the model to the data", and that "the concrete crate names and paths are recorded in this section in the same change that creates the workspace". No crate name is assumed before that.
- `AGENTS.md` section 3 makes the shell the only layer permitted to perform I/O, to read the clock, or to write to a terminal, and section 4 requires the engine to be reachable from the shell by a command whose output is asserted in a test.
- `AGENTS.md` section 4 stated that "integration tests live in `tests/` at the workspace root". A virtual workspace root has no package and therefore no `tests/` target, so that sentence could not hold as written once the workspace exists. The sentence was corrected on 2026-10-21 by issue [#44](https://github.com/quant-rule/quant-rule/issues/44), which now names the `tests/` directory of the crate that owns the code.
- Issue #9 is open on the board in `Todo`. Its scope is the workspace, the candle and timeframe types, normalization, ordering, duplicate resolution, timeframe construction, gap handling, and shell reachability. Its done-when requires purity, passing tests, causality and determinism coverage, five named edge cases, a recorded crate layout, and a shell command whose output is asserted in a test.

## Scope

Included:

- The Rust workspace: a data crate, a model crate, and the shell binary, with the dependency direction section 2 names, and the layout recorded in section 2 in this change.
- The data engine: the candle and timeframe types, the series, normalization with validation, ordering, and duplicate resolution, higher-timeframe construction aligned to boundaries, and gap detection.
- The shell skeleton: a command parser, a help listing, a renderer, and a CSV source, enough to reach the engine from a command.
- The tests named by issue #9 and by `AGENTS.md` section 4, including the correction to section 4's integration-test location.
- An execution plan and an AI transparency log for this work item.

Explicit non-goals:

- No indicator, no axis, no event, and no model output. The model engine is issue #14, and the model crate is created here only as the declared layout, empty of behaviour.
- No strategy, no portfolio, no execution, and no backtest.
- No network source, no live market, and no clock value in any output.
- No third-party dependency. The workspace uses the standard library only.
- No new documentation kind and no change to the command contract of an existing command, because there are no commands yet.

## Assumptions and uncertainty

- **The base series is the shell's input.** The shell reads candles; the data engine never reaches for a source. The first source is a CSV file whose path the user names, because a file is deterministic, testable, and needs no network.
- **A candle's volume is optional.** The model contract says volume may be absent, so `volume` is an `Option<f64>` rather than a zero, and a missing volume is never turned into a zero.
- **Timestamps are unix seconds in UTC.** One explicit format, compared as integers, and rendered as RFC 3339 in the shell.
- **Timeframes are those that divide a day.** One minute through one day, all of which align to epoch boundaries and to UTC midnight for the daily bar. A weekly or monthly bar needs a calendar rule that is not in scope, so it is not offered.
- **Duplicates resolve to the last record.** A repeated timestamp is a re-sent candle, and the last record wins. The alternative, first wins, is equally deterministic; the choice is stated so that the fixture and the tests are unambiguous.
- **Invalid candles are dropped and counted, not fatal.** A shell user pointing at a messy file gets a usable series and a report of what was dropped, rather than an error for the whole file.
- **Only complete buckets become candles.** A higher-timeframe bar is emitted only when its bucket is aligned and closed, so a partial leading or trailing bucket is dropped and counted.
- **The model crate is created empty.** Section 2 declares three crates, so the workspace has three. An empty crate is not speculative: the layout is already decided, and the shell's dependency direction is real only if the model crate exists.

## Decisions

| # | Open point | Options | Choice | Rationale |
|---|---|---|---|---|
| B1 | Where the crates live | `crates/<name>`; top-level directories | `crates/quantrule-data`, `crates/quantrule-model`, `crates/quantrule-shell` | One place for the packages, so the workspace root holds only workspace configuration, and the names read as what they are |
| B2 | The crate names | `quantrule-data` and so on; `data`, `model`, `shell` | `quantrule-data`, `quantrule-model`, `quantrule-shell` | A workspace with more than one package needs names that cannot collide with a dependency name. The prefix makes the package obvious in a dependency list and in an error message |
| B3 | The integration-test location | Keep `tests/` at the workspace root; per-crate `tests/`; a root package | Per-crate `tests/`; section 4's sentence was corrected on 2026-10-21 by issue #44, ahead of this change | A virtual workspace root has no package, so a root `tests/` directory is never compiled. A root package exists only to host tests, which is indirection for its own sake. The rule is corrected to describe the layout that actually runs |
| B4 | The model's crate in this change | Create it empty; wait for #14 | Create it empty | Section 2 declares the layout and this change is where the layout is recorded. An empty crate is the smallest thing that makes the declared dependency direction true |
| B5 | The first source | A CSV file; a built-in synthetic series | A CSV file named by the user | A file is the only source that is deterministic, needs no network, and can be asserted on byte for byte in a test. A built-in series would be a second, hidden source |
| B6 | The CSV format | A header with named columns; positional columns | A header with named columns, in any order | A named header documents itself and lets a column be missing. Positional columns are shorter to parse and opaque to a reader |
| B7 | The shell's command set | One command; a small set | `help`, `timeframes`, `candles` | The issue requires at least one command whose output is asserted. A help listing is required by section 3's discoverability rule, `timeframes` names what the engine supports, and `candles` reaches the engine |
| B8 | Duplicates | Last wins; first wins; merged | Last wins | A repeated timestamp is a re-sent candle and the later record is the more recent statement. The rule is stated, and the fixture covers it |
| B9 | Invalid candles | Drop and count; fail the whole file | Drop and count | A shell user pointing at a real file gets a usable series and a report. Failing the whole file would make the engine unusable on imperfect data and would move validation policy into the shell |
| B10 | Partial buckets | Emit them; drop them | Drop them and count | A bar built from half a bucket is not comparable with a bar built from a whole one, and mixing them would corrupt every downstream average. Dropping is recorded in the report so nothing is silent |

## Acceptance criteria

Observable conditions for completion:

- The workspace holds a data crate, a model crate, and the shell binary, and the dependency direction is shell to model to data.
- `AGENTS.md` section 2 records the crate names and paths, and section 4's integration-test sentence matches the layout that runs.
- The data engine is a pure function of the candles passed in: no I/O, no clock, no randomness, and a test that proves it.
- Candles are validated, ordered by timestamp, and deduplicated, and the report says what was dropped.
- Higher timeframes are built from a base series on epoch-aligned boundaries, and partial buckets are dropped and counted.
- Gaps are detected and reported.
- `cargo test` passes, with unit tests beside the code and integration tests in the crates' `tests/` directories.
- Causality and determinism tests cover the engine, per `AGENTS.md` sections 4 and 12.
- Edge cases are covered: empty series, single candle, unaligned boundary, gap, and zero volume.
- The engine is reachable from the shell by at least one command whose output is asserted in a test, and the same command prints the same bytes interactively and non-interactively.
- The shell's help listing names every command, its arguments, and the shape of its output.
- `cargo fmt --check`, `cargo clippy --all-targets`, and `cargo doc` are clean.

## Work breakdown

| Step | Action | Role | Paths affected | Depends on |
|---|---|---|---|---|
| 1 | Create the worktree and the branch from the latest `origin/main` | Implementation | git state | — |
| 2 | Write this plan and the transparency log | Orchestrator | `docs/` | 1 |
| 3 | Create the workspace and the three crates; record the layout in `AGENTS.md` sections 2 and 4 | Implementation | `Cargo.toml`, `crates/`, `AGENTS.md` | 2 |
| 4 | Implement the candle, timeframe, and series types | Implementation | `crates/quantrule-data/src/` | 3 |
| 5 | Implement normalization, timeframe construction, and gap detection | Implementation | `crates/quantrule-data/src/` | 4 |
| 6 | Implement the shell's command parser, help listing, renderer, and CSV source | Implementation | `crates/quantrule-shell/src/` | 5 |
| 7 | Write the unit and integration tests | Test Engineer | `crates/*/tests/` | 4, 5, 6 |
| 8 | Verification pass: `fmt`, `clippy`, `test`, `doc`, and the edge cases | Test Engineer, Code Reviewer | all | 7 |
| 9 | Commit, push, open the pull request | Orchestrator | git state, PR | 8 |
| 10 | Move issue #9 to `Done` on the board, and remove the worktree and the branch | Orchestrator | board, git state | 9 |

## Verification

No implementation has started, so no verification has been performed and no result is claimed. The commands that will produce the evidence are:

- `cargo fmt --check` — no formatting differences.
- `cargo clippy --all-targets -- -D warnings` — no warnings.
- `cargo test` — every test passes.
- `cargo doc --no-deps` — builds without warnings.

The engine's own properties are proven by tests rather than asserted: purity, causality, determinism across repeated runs and reordered input, and the five edge cases.

Limitations, stated explicitly:

- The workspace has no CI configuration, and none is added here; the checks are run locally and recorded.
- The CSV source is the only source. A network source, a vendor format, and any clock-derived value are out of scope and are not implemented.
- The model crate is empty by design; the model engine is issue #14.
- No build or test result is claimed for any crate that does not yet exist.

## Risks and recovery

- **The candle and timeframe types become the contract the model engine consumes.** Mitigation: they are the smallest types that carry OHLCV, a UTC timestamp, and a timeframe; the model engine's needs are recorded in `docs/model/`, and a change to a field is a change to a contract the model engine reads.
- **The base-step inference drives timeframe construction and gap detection.** Mitigation: the step is the minimum positive difference between consecutive timestamps, it is documented, and it is tested against a series with a gap in it.
- **The shell becomes a second source of truth for the series.** Mitigation: the shell parses a file and hands the parsed candles to the data engine, which owns every rule about them; the shell's own rules are only about parsing and rendering.
- **A virtual workspace cannot host `tests/` at its root, so section 4's rule changes.** Mitigation: the rule was corrected on 2026-10-21 by issue [#44](https://github.com/quant-rule/quant-rule/issues/44), ahead of this change, because it was already false against the layout this plan commits to. The correction says where the tests are.
- **The crate names are now fixed in `AGENTS.md` section 2.** They are chosen once, and a later rename touches the layout the whole workspace depends on.
- **Recovery.** The change is one branch, merged through a pull request, so it can be reverted with a new commit on `main`. The branch and its worktree are removed only after the merge, and only after the working tree is verified clean.

## Stop conditions

Stop and request human direction if:

- The crate layout cannot be recorded in `AGENTS.md` section 2 without changing another section.
- A third-party dependency turns out to be needed, which is a decision reserved to the human by section 14.
- The CSV format cannot be defined without a decision the human has not delegated, such as the canonical timestamp format for a source.
- The verification cannot be completed without a network, a live market, or a clock value in the output.
