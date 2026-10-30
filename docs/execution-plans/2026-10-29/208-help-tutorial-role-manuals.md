# Help, Tutorial, and Quant Role Manuals

- Date: 2026-10-29 (UTC)
- Status: **Approved — 2026-10-29.** The human authorized the full implementation
  with "proceed with the full implementation following the plan". Verification
  evidence is recorded as it is observed.
- Container: [#208](https://github.com/quant-rule/quant-rule/issues/208)
- Sub-issues:
  [#209](https://github.com/quant-rule/quant-rule/issues/209) (lab summary),
  [#210](https://github.com/quant-rule/quant-rule/issues/210) (tutorial),
  [#211](https://github.com/quant-rule/quant-rule/issues/211) (role manuals),
  [#212](https://github.com/quant-rule/quant-rule/issues/212) (help topics)
- Baseline: `main` at `263c984`
- Transparency log:
  [`docs/ai-transparency-log/2026-10-29/208-help-tutorial-role-manuals.md`](../../ai-transparency-log/2026-10-29/208-help-tutorial-role-manuals.md)
- Model/strategy behavior: unchanged

## Intent

Deliver, in order:

1. A `lab summary` shell command that writes a deterministic `summary.txt` for
   a lab result folder.
2. A full-pipeline tutorial that runs entirely through shell commands.
3. Three professional operating manuals: Quant Researcher, Quant Developer,
   Quant Validator.
4. A redesigned help system: short `help`, `help <command>`, `help all`,
   `help glossary`, `help tutorial`, `help roles`.
5. README and docs-index updates, including the
   `data/lab-result-YYYY-MM-DDTHH-MM-SSZ` convention.

The tutorial and manuals must be usable by a human or by an agent acting in the
named role. Every product step is a shell command. Code editing and `cargo`
checks remain outside the shell and are marked as developer gates.

## Current state

- `main` is at `263c984`; the working tree is clean; only the main worktree
  existed before this program.
- `AGENTS.md` section 10 defines the three quant roles.
- `docs/roles/2026-10-29/quant-roles.md` is the overview; there are no role
  manuals.
- `help` is one 150-line constant with no topics; `help now` is rejected.
- `backtest --out` writes five bundles: `analytical`, `execution`, `model`,
  `strategy`, `portfolio`.
- README uses `data/first-run`; there is no root `summary.txt`.
- No tutorial exists.
- The full-year SOLUSD file has 522,633 M1 candles. The model, research, and
  store commands cap at 100,000; `market-report`, `strategy-run`, and
  `backtest` accept up to 600,000.
- `data/` is local and gitignored.

## Scope

Included:

- `lab summary` command and `summary.txt` contract.
- Full-pipeline tutorial with three role labs.
- Three role manuals.
- Help redesign with topics.
- README, docs index, `AGENTS.md` docs-kind line, and transparency log.
- The `data/lab-result-YYYY-MM-DDTHH-MM-SSZ` convention.

Excluded:

- Changes to indicators, model formulas, strategy rules, execution, portfolio
  accounting, configuration schemas, fixtures, the oracle, or CI behavior.
- New statistical or machine-learning signal generation.
- Profitability claims.
- Deletion or renaming of the local `data/first-run` folder.
- A tutorial-execution engine that runs commands for the user.

## Decisions

| ID | Decision | Status |
|---|---|---|
| D1 | Help has three levels: `help` map, `help <command>` contract, `help all` complete reference. | Approved |
| D2 | `help glossary`, `help tutorial`, and `help roles` are additional topics. | Approved |
| D3 | Keep precise terms such as `NDJSON`, `report bundle`, and `data version`; define them in the glossary rather than replacing them. | Approved |
| D4 | Add a `lab summary` shell command. It reads the five bundles and writes `summary.txt`; it does not recompute the model. | Approved |
| D5 | A lab result folder is `data/lab-result-YYYY-MM-DDTHH-MM-SSZ`, containing the five bundles plus `summary.txt`. | Approved |
| D6 | The tutorial uses a two-month SOLUSD slice for the 100k-capped commands and the full-year example config for the backtest. | Approved |
| D7 | The three role manuals live under `docs/roles/2026-10-29/` and are linked from the roles overview. | Approved |
| D8 | The tutorial and manuals use role cards, actor labels, checkpoints, stop conditions, and handoff packets so humans and agents follow the same procedure. | Approved |
| D9 | `data/first-run` is replaced in documentation by the lab result convention; the local folder is left untouched. | Approved |
| D10 | PR order: lab summary, tutorial, role manuals, help redesign. Each depends on the previous one where noted. | Approved |
| D11 | No Quant Validator is required for the implementation because model and strategy behavior do not change. The Validator lab in the tutorial is an exercise, not a production validation. | Approved |

## Issue tree

| Item | Title | Label | Branch | PR title | Depends on |
|---|---|---|---|---|---|
| Container | Help, tutorial, and quant role manuals | `enhancement` | — | — | — |
| Sub 1 | Add the lab result summary command | `enhancement` | `feat/209-lab-summary` | `feat: add the lab result summary command` | — |
| Sub 2 | Publish the full-pipeline tutorial | `documentation` | `docs/210-lab-tutorial` | `docs: publish the full-pipeline tutorial` | Sub 1 |
| Sub 3 | Publish the quant role manuals | `documentation` | `docs/211-quant-role-manuals` | `docs: publish the quant role manuals` | Sub 2 |
| Sub 4 | Redesign shell help with command topics | `enhancement` | `feat/212-help-topics` | `feat: redesign shell help with command topics` | Sub 2, Sub 3 |

## Work breakdown

### 1. Lab summary command (PR 1)

Owner: Implementation Agent, with Test Engineer and Code Reviewer passes.

Files:

- `crates/quantrule-shell/src/lab_summary.rs` — bundle reader, summary
  renderer, atomic writer.
- `crates/quantrule-shell/src/command/lab.rs` — argument parser and command
  runner.
- `crates/quantrule-shell/src/command/types.rs` — command variant, arguments,
  role and operator types, errors.
- `crates/quantrule-shell/src/command/parse.rs`,
  `crates/quantrule-shell/src/command/dispatch.rs`,
  `crates/quantrule-shell/src/command.rs`, and
  `crates/quantrule-shell/src/main.rs` — wiring.
- `crates/quantrule-shell/src/command/help.rs` and
  `crates/quantrule-shell/tests/fixtures/help.txt` — temporary help entry.
- `crates/quantrule-shell/tests/lab_summary.rs` and
  `crates/quantrule-shell/tests/fixtures/lab/` — integration tests and golden
  fixture.
- `crates/quantrule-shell/src/command/tests.rs` — parser tests.
- `README.md` — command reference and lab result convention.
- This plan and the transparency log.

Contract:

```text
lab summary <lab-dir>
  --role <researcher|developer|validator>
  --data-version <id>
  --config <run.json>
  --created-utc <YYYY-MM-DDTHH:MM:SSZ>
  [--operator <human|agent>]
  [--notes <path>]
  [--force]
```

Behavior:

- Require the five bundle directories and their `report.json` files.
- Read `provenance.json` from each bundle where present.
- Validate that every report's data version, and every present provenance data
  version, matches `--data-version`.
- Write `<lab-dir>/summary.txt` atomically.
- Refuse an existing `summary.txt` unless `--force`.
- Print `wrote <path>` on success.
- Do not read the clock; `--created-utc` is explicit.
- Do not recompute the model.

### 2. Full-pipeline tutorial (PR 2)

Owner: Implementation Agent for the document; Quant Researcher, Quant
Developer, and Quant Validator passes for the role labs.

Files:

- `docs/tutorials/2026-10-29/full-pipeline.md`.
- `docs/README.md` — add the `tutorials` kind and link.
- `README.md` — add the tutorial link; keep the lab result convention.
- `AGENTS.md` section 5 — add
  `docs/tutorials/<YYYY-MM-DD>/<file>.md`.
- `docs/roles/2026-10-29/quant-roles.md` — add a tutorial link.
- This plan and the transparency log.

Structure:

1. What the lab produces.
2. Before you start: binary, repository root, data, config, determinism rules.
3. Data engine: `fetch`, `candles`, `candles-tolerant`, `timeframes`.
4. Model engine: `market-dashboard`, `market-state`, `axis`, `events`, `legs`,
   `model-series`, `market-chart`, `market-report`.
5. Research engine: `research export`, `research summary`.
6. Strategy: `strategy-run`.
7. Full run: `backtest`, `report ...`, `lab summary`.
8. Store: `store init`, `store ingest`, `store verify`, `store list`,
   `store show`, `db ...`.
9. Role labs: Researcher, Developer, Validator.
10. What the lab proves and does not prove.
11. Troubleshooting.
12. Command index.

### 3. Quant role manuals (PR 3)

Owner: Implementation Agent for the documents; Quant Researcher, Quant
Developer, and Quant Validator passes for their own procedures.

Files:

- `docs/roles/2026-10-29/quant-researcher-manual.md`.
- `docs/roles/2026-10-29/quant-developer-manual.md`.
- `docs/roles/2026-10-29/quant-validator-manual.md`.
- `docs/roles/2026-10-29/quant-roles.md` — manual table and handoff chain.
- `docs/README.md` — role manuals table.
- `README.md` — role manuals link.
- This plan and the transparency log.

Each manual contains:

1. Role card: purpose, inputs, outputs, stop conditions, handoff.
2. When the role is required.
3. Operating procedure: numbered steps and exact shell commands.
4. Artifacts and lab result folder.
5. Checks before handoff.
6. Agent instructions: ordered steps, no scope expansion, stop at gates.
7. Human instructions: what to review and decide.
8. Handoff packet template.
9. Common failure modes.
10. Normative links: `AGENTS.md` section 10 and the roles overview.

### 4. Help redesign (PR 4)

Owner: Implementation Agent, with Test Engineer and Code Reviewer passes.

Files:

- `crates/quantrule-shell/src/command/help.rs` — rewrite as a metadata-driven
  help module.
- `crates/quantrule-shell/src/command/types.rs`,
  `crates/quantrule-shell/src/command/parse.rs`, and
  `crates/quantrule-shell/src/command/dispatch.rs` — help topics.
- `crates/quantrule-shell/tests/cli.rs` and
  `crates/quantrule-shell/tests/fixtures/help*.txt` — golden output.
- `README.md` — help contract and topics.
- Tutorial and manuals — add the one-line shell entry points.
- This plan and the transparency log.

Help levels:

1. `help` — one-screen map of the pipeline, commands, roles, and reference
   topics.
2. `help <command>` — full contract for one command.
3. `help all` — every command and option.
4. `help glossary` — precise terms and definitions.
5. `help tutorial` — one-screen tutorial map pointing to the tutorial doc.
6. `help roles` — one-screen role map pointing to the three manuals.

Behavior:

- `help` with no arguments remains the short map.
- `help <topic>` accepts one- or two-word topics, including `store ingest`,
  `research export`, and `lab summary`.
- `help <unknown>` prints an error and points to `help all`.
- `help all` is generated from the same metadata as `help` and
  `help <command>` so they cannot drift.
- Output is fixed-width, plain text, with no terminal-width dependence and no
  color.
- The glossary keeps the exact term and defines it; it does not replace it.

## Acceptance criteria

- `lab summary` works on a fixture and on the example run.
- Missing bundle, data-version mismatch, invalid role, invalid datetime, and
  existing summary are covered by tests.
- The summary is byte-identical for the same inputs.
- The tutorial runs end-to-end and produces five bundles plus `summary.txt`.
- Every tutorial command is copy-pasteable and its expected output is stated.
- The tutorial states what the lab proves and does not prove.
- Each manual is self-contained and names `AGENTS.md` section 10 as normative.
- The three handoffs form a complete chain.
- `help` fits one screen; every command has a `help <command>` page;
  `help all` lists every command and option.
- `help glossary`, `help tutorial`, and `help roles` point to existing docs.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and
  `cargo doc --no-deps` pass.
- `git diff --check` is clean and every relative link resolves.
- No model, strategy, execution, portfolio, configuration, fixture, oracle, or
  CI behavior changes.

## Verification

| Layer | Check | Command | Expected |
|---|---|---|---|
| Shell | Format | `cargo fmt --all -- --check` | No diff |
| Shell | Lints | `cargo clippy --all-targets -- -D warnings` | No warnings |
| Shell | Tests | `cargo test` | All pass |
| Shell | Docs | `cargo doc --no-deps` | No warnings |
| Lab summary | Golden | `cargo test -p quantrule-shell --test lab_summary` | Byte-identical summary |
| Lab summary | Errors | `cargo test -p quantrule-shell --test lab_summary` | Missing bundle, mismatch, existing summary fail clearly |
| Help | Golden | `cargo test -p quantrule-shell --test cli` | Short map, detail, all, glossary, tutorial, roles |
| Help | Parity | `cargo test -p quantrule-shell --test cli` | Interactive and non-interactive identical |
| Tutorial | End-to-end | `backtest` + `lab summary` | Five bundles plus `summary.txt` |
| Docs | Links | Manual check | Every relative link resolves |
| Repo | Whitespace | `git diff --check` | Clean |

Limitations:

- The tutorial's example backtest is illustrative. It is not out-of-sample
  validation and makes no profitability claim.
- The Validator lab is an exercise. It is not a production validation record.
- The local `data/first-run` folder is not migrated or deleted.

## Risks and recovery

| Risk | Mitigation |
|---|---|
| `help` output contract changes | Explicit approval, golden fixtures, README update, `help all` preserves completeness. |
| `lab summary` parses reports loosely | Validate bundle presence and data version, fail with explicit errors, never recompute the model. |
| Docs drift from commands | "Last verified" line, link to `help <command>` as the contract, update on release. |
| 100k/600k cap confusion | Two-month tutorial slice, caps table, `--window latest` where appropriate. |
| Folder timestamp ambiguity | Explicit `--created-utc`; the engine never reads the clock. |
| Existing `data/first-run` loss | Do not delete or rename it; docs move to the new convention. |
| Agent skips a human gate | Role cards, stop conditions, actor labels, summary handoff. |

## Stop conditions

Stop and ask if:

- `lab summary` cannot read the existing report JSON without changing the
  report contract.
- The help redesign requires changing a command's behavior beyond `help`.
- The tutorial reveals a contradiction with the model or command contract that
  requires a model or strategy change.
- The two-month fetch is unavailable and the existing full-year file cannot be
  used with `--window latest`.
- A PR would change model, strategy, execution, portfolio, configuration,
  fixtures, the oracle, or CI behavior.
- Issue creation, board status, or Git state cannot be established safely.
- A merge is not explicitly authorized.

## Final acceptance

- All four sub-issues are Done and closed.
- The container is Done.
- `main` is clean and up to date.
- The tutorial runs end-to-end on `main` and produces five bundles plus
  `summary.txt`.
- `help`, `help <command>`, `help all`, `help glossary`, `help tutorial`, and
  `help roles` all work.
- The three manuals are linked and consistent with `AGENTS.md` section 10.
- `cargo fmt`, `clippy`, `test`, and `doc` pass.
- No model or strategy behavior changed.
- The execution plan is marked `Verified`; the transparency log records the
  evidence.
- All task worktrees and branches are removed after merge.

## Evidence

Evidence is recorded per PR in the transparency log and in this section as it is
observed.

### PR 1 — lab summary

Implemented on `feat/209-lab-summary` in `quant-rule--209-lab-summary`:

- `lab summary` parser, bundle reader, deterministic renderer, and atomic writer.
- Interactive and non-interactive dispatch, help entry, README command
  reference, and the `data/lab-result-YYYY-MM-DDTHH-MM-SSZ` convention.
- Fixture lab folder, golden `summary.txt`, 15 integration tests, and parser
  unit tests.

Observed verification (2026-10-29 UTC):

| Check | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | Clean |
| Lints | `cargo clippy --all-targets -- -D warnings` | No warnings |
| Tests | `cargo test` | 1,374 passed, 0 failed |
| Docs | `cargo doc --no-deps` | Built without warnings |
| Help golden | `target/debug/quantrule help > /tmp/opencode/help.out` then `diff -u crates/quantrule-shell/tests/fixtures/help.txt /tmp/opencode/help.out` | Identical |
| Example run | `target/release/quantrule backtest --config config/examples/solusd-2025/run.json --out data/lab-result-2026-10-29T12-00-00Z` then `lab summary ...` | Five bundles plus `summary.txt`; 48 trades, final equity 8016.143910 |
| Determinism | Second `lab summary --force` on the example run | Same SHA-256 (`2b8019...`) |
| Existing summary | Second run without `--force` | Refused with `already exists; use --force` |
| Whitespace | `git diff --check` | Clean |

Pull request: [#213](https://github.com/quant-rule/quant-rule/pull/213), merged
at `2026-10-30T00:59:55Z` (merge commit `a9fb070`).

### PR 2 — tutorial

Implemented on `docs/210-lab-tutorial` in `quant-rule--210-lab-tutorial`:

- `docs/tutorials/2026-10-29/full-pipeline.md` — the full pipeline, the store,
  three role labs, checkpoints, stop conditions, handoff packets, limitations,
  troubleshooting, and a command index.
- `docs/README.md` — the `tutorials` kind and the current-tutorial table.
- `README.md` — the "Learn the pipeline" link.
- `AGENTS.md` section 5 — the `docs/tutorials` kind.
- `docs/roles/2026-10-29/quant-roles.md` — the tutorial link.

Observed verification (2026-10-30 UTC):

| Check | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | Clean |
| Lints | `cargo clippy --all-targets -- -D warnings` | No warnings |
| Tests | `cargo test` | 1,374 passed, 0 failed, 6 ignored |
| Docs | `cargo doc --no-deps` | Built without warnings |
| Whitespace | `git diff --check` | Clean |
| Links | Relative-link check over the changed docs | 24 links; all resolve |
| Data engine | `candles`, `candles-tolerant`, `timeframes` on the two-month slice | 1,000 candles; 84,108 read, 0 rejected; 4,000 timeframe lines |
| Model | Dashboard, state, axis, events, legs, model-series, chart, market-report | 45 dashboard lines; four state keys; 4 axis lines; 7,095 events; 8 legs; 2 model bars; 21,922-byte PNG; four report artifacts |
| Research | `research export`, `research summary` | 9 files; 107,918 layer rows; 41 summary lines |
| Strategy | `strategy-run` on the two-month slice | 363 records: 182 entries, 181 exits |
| Store | `store init`, `ingest`, `verify`, `list`, `show`, `db` | `schema=2`; 107,918 bars; `verified ... tables=7`; 74/61/285 CSV columns |
| Full run | Three full-year `backtest` runs plus `lab summary` | 22.1 s, 22.6 s, 23.9 s; all five bundles byte-identical across the three runs; 48 trades, final equity 8016.143910, commission 885.610499, halted true |
| Role labs | Researcher, developer, and validator notes plus `lab summary --force` | Notes appear under Role notes; `Next` hands off to the next role; validator records the halt and the in-sample split |

Pull request: [#214](https://github.com/quant-rule/quant-rule/pull/214),
open. Merge requires explicit human authorization.

Limitations:

- The example backtest halts at the 20% drawdown limit. The tutorial reports
  that as an observed result and makes no profitability claim.
- The Validator lab is an exercise, not a production validation record.
- The role manuals are the next work item; the tutorial's role labs are
  self-contained and link to the roles overview and `AGENTS.md` section 10.

### PR 3 — role manuals

Implemented on `docs/211-quant-role-manuals` in
`quant-rule--211-quant-role-manuals`:

- `docs/roles/2026-10-29/quant-researcher-manual.md`,
  `quant-developer-manual.md`, and `quant-validator-manual.md` — each with a
  role card, when-required table, numbered procedure with commands, artifacts,
  handoff checks, agent and human instructions, a packet template, failure
  modes, and normative links.
- `docs/roles/2026-10-29/quant-roles.md` — the role-manual table and the
  three-packet handoff chain.
- `docs/README.md` — the role manuals table.
- `README.md` — the three manual links.

Observed verification (2026-10-30 UTC):

| Check | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | Clean |
| Lints | `cargo clippy --all-targets -- -D warnings` | No warnings |
| Tests | `cargo test` | 1,374 passed, 0 failed, 6 ignored |
| Docs | `cargo doc --no-deps` | Built without warnings |
| Whitespace | `git diff --check` | Clean |
| Links | Relative-link check over the changed docs | 60 links; all resolve |
| Manual audit | `cargo test -p quantrule-model`, `cargo test -p quantrule-backtest` | All pass |
| Availability sample | `events ... --window latest` plus the record check | 0 events before availability; 7,095 events in the sample |
| Example values | `grep` over the validator lab summary | 48 trades, 8016.143910, 0.203150, halted true, commission 885.610499 |

Pull request: [#215](https://github.com/quant-rule/quant-rule/pull/215),
open. Merge requires explicit human authorization.

Limitations:

- The manuals are procedures. The example lab is illustrative and is not a
  production validation record.
- The Validator manual is not an independent model judgment: the same engine
  and machine produce the reproduction.
- No model, strategy, execution, portfolio, configuration, fixture, oracle, or
  CI behavior changed.

### PR 4 — help redesign

Implemented on `feat/212-help-topics` in `quant-rule--212-help-topics`:

- `crates/quantrule-shell/src/command/help.rs` — the command metadata table
  and the short-map, per-command, and complete-reference renderers.
- `crates/quantrule-shell/src/command/help/topics.rs` — the glossary,
  tutorial, and roles maps.
- `crates/quantrule-shell/src/command/types.rs` — `HelpTopic`,
  `ParseError::UnknownHelpTopic`, and `RunError::Help`.
- `crates/quantrule-shell/src/command/parse.rs` — one- and two-word topic
  parsing; `dispatch.rs` and `command.rs` wiring.
- `crates/quantrule-shell/src/command/tests.rs` — parser coverage for topics
  and the unknown-topic error.
- `crates/quantrule-shell/tests/cli.rs` and
  `crates/quantrule-shell/tests/fixtures/help*.txt` — golden output for the
  short map, one command page, `help all`, glossary, tutorial, roles, and the
  `store ingest` two-word topic.
- `README.md` — the help contract and topics; the tutorial, the three manuals,
  and the roles overview — one-line shell entry points.

Observed verification (2026-10-30 UTC):

| Check | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | Clean |
| Lints | `cargo clippy --all-targets -- -D warnings` | No warnings |
| Tests | `cargo test` | 1,391 passed, 0 failed, 6 ignored |
| Docs | `cargo doc --no-deps` | Built without warnings |
| Whitespace | `git diff --check` | Clean |
| Links | Relative-link check over the changed docs | 63 links; all resolve |
| Short map | `quantrule help` | 43 lines; every command name present |
| Complete reference | `quantrule help all` | 504 lines; every command and option present |
| Command page | `quantrule help lab summary` | 21 lines; the full contract |
| Two-word topic | `quantrule help store ingest` | 16 lines; the full contract |
| Glossary | `quantrule help glossary` | 118 lines; precise terms and contract paths |
| Tutorial map | `quantrule help tutorial` | 30 lines; points at the tutorial |
| Role map | `quantrule help roles` | 23 lines; points at the three manuals |
| Unknown topic | `quantrule help bogus` | Exit 1; `unknown help topic 'bogus'; type 'help all' ...` |
| Parity | `help` and `help store ingest` interactive vs non-interactive | Byte-identical |
| Drift | Unit test over the metadata | Every command page and option is in `help all`; the short map uses the same metadata |

Pull request: [#216](https://github.com/quant-rule/quant-rule/pull/216),
open. Merge requires explicit human authorization.

Limitations:

- The help text is static. It names the tutorial and manuals but does not read
  them; a missing file is not detected by the shell.
- The glossary defines terms; it does not replace the model, research, or report
  contracts.
- No model, strategy, execution, portfolio, configuration, fixture, oracle, or
  CI behavior changed.
