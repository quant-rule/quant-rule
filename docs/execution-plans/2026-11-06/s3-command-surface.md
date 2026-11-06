# S3.x Command Surface — Stage-Isolated Pipeline Access

- Date: 2026-11-06 (UTC)
- Status: Approved; Phase 1 implemented and verified locally; publication pending
- Issue: none (human-approved slug; no issue number is invented)
- Branch: `docs/s3-command-contracts`
- Worktree: `s3-command-contracts` (OpenCode task worktree)
- Claim type: command, artifact, and qualification contract; no model or strategy
  behavior change in Phase 1; no edge claim
- Approval: human instruction "PROCEED WITH THE FULL IMPLEMENTATION" after the
  stage contract and the Qualification Engine boundary were approved

## 1. Intent

QuantRule is a shell application, and the shell is the only product surface. The
current shell exposes eight commands and stops at the model. The execution,
portfolio, strategy, and benchmark engines exist in `quantrule-backtest`, but no
runner reaches them, so a human or an agent cannot inspect a strategy decision, a
fill, an equity curve, or the evidence behind a verdict.

This plan makes every pipeline stage independently runnable, inspectable, and
documented. Each stage gets a command with a stable contract, an artifact with a
schema and provenance, a help entry, a README row, and a tutorial example.
`backtest run` orchestrates the stages but does not hide them.

The final stage is the **Qualification Engine**, invoked as `qualify`. It
evaluates the complete evidence bundle against human-owned acceptance criteria
and emits a verdict of `pass`, `fail`, or `insufficient evidence`. The human makes
the final acceptance decision.

## 2. Current state

- `crates/quantrule-shell/src/command/help.rs` is the command registry. It lists
  eight commands: `candles`, `candles-tolerant`, `timeframes`, `events`,
  `report model`, `policy check`, `fetch`, and `help`.
- `crates/quantrule-shell/src/command/parse.rs` and `dispatch.rs` route the same
  eight commands; `quit` is a shell meta-command.
- `README.md` repeats the command table manually. The M1.0 lab tutorial is the
  only current tutorial.
- `crates/quantrule-backtest` contains the strategy, execution, portfolio, and
  benchmark engines, but no runner and no shell command.
- `crates/quantrule-shell/src/config/run.rs` loads the policy, data, model,
  execution, and portfolio files. It has no strategy or analytics file.
- `docs/architecture/`, `docs/config/`, and `docs/README.md` describe strategy,
  execution, portfolio, and backtest runs as S3.x target work. There is no
  command-surface contract and no drift gate.
- The model verification campaign is closed at 1,091 row cells and 329 accepted
  family cells. It did not open `model_validation` and did not claim an edge.

## 3. Scope

### 3.1 Included

- A command-surface contract naming every stage, command, input, output, status,
  and discovery surface.
- An artifact contract for every stage bundle: file list, record fields,
  provenance, ordering, and refusal rules.
- A qualification contract: human-owned criteria, required evidence dimensions,
  verdict semantics, and the human acceptance boundary.
- A checker that fails when `help.rs`, the README table, the command registry, or
  a tutorial disagree, and that refuses to list a planned command as available.
- The strategy commands and decisions artifact.
- The execution command and fills artifact.
- The portfolio command and performance artifact.
- The `backtest run` orchestration command.
- The `qualify` command and qualification record.
- The `analytics evaluate` command and metrics bundle.
- Tests, tutorials, README updates, and the AI transparency log for each phase.
- An independent verification phase that runs every stage on its own and records
  residual risks.

### 3.2 Excluded

- Changes to model formulas, event definitions, progression arithmetic, or
  strategy rules. A model or strategy behavior change needs a separate approved
  hypothesis.
- A new strategy hypothesis, a profitability claim, or an edge claim.
- Opening `model_validation` or any one-shot validation partition.
- A live-trading, broker, order-book, tick, machine-learning, or statistical
  signal path.
- A server, a service, a graphical interface, or an embedded library.
- Changes to an existing command's meaning. A new capability arrives as a new
  command; an existing command's contract change is reserved to the human.
- Adding `strategy` or `analytics` to `run.json`. Stage commands take those files
  explicitly; integrating them into the run composition root is a separate
  decision.
- Releases, tags, deployments, and merges. Every Git gate is separate.

## 4. Assumptions and uncertainty

- The human approved the stage table, the name **Qualification Engine**, the
  command `qualify`, and the human-final-acceptance boundary.
- The existing `run.json` schema stays `quantrule.run/v2`. Strategy and
  qualification criteria are explicit command arguments, not new run fields.
- The existing model bundle is the model stage artifact. Its
  `quantrule.provenance/v1` file is not rewritten by Phase 1; new stage bundles
  use the common provenance contract.
- The backtest crate already owns the strategy, execution, and portfolio
  calculations. The work is the runner, the artifact writer, the command, and the
  tests; the engine logic is not duplicated in the shell.
- The exact CSV/NDJSON column order is fixed by the phase that first writes the
  artifact, then treated as immutable. Additional fields may be added; an
  existing field's name and meaning do not change.
- The qualification thresholds are human-owned. Phase 1 defines the structure and
  verdict semantics; it does not choose a threshold or a pass rule.
- A command that cannot be run on its own is not done, even if its engine library
  works.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Stage 6 name | **Qualification Engine**; command `qualify` | The name avoids "policy" and "strategy"; the engine evaluates evidence. |
| Human boundary | The engine emits a verdict; the human accepts or rejects in a separate record | No command declares an edge or writes `human_decision`. |
| Model output term | Model "predictions" are published measurements | Documentation names readings, states, levels, events, progression, and context. |
| Strategy output term | Strategy "edge" is a decision/signal plus its as-of rationale | "Edge" remains a research verdict, never an engine output. |
| Command listing | A command is listed only when implemented and tested | No stub commands; planned work is documented in the contract only. |
| Stage isolation | Each stage reads an explicit artifact and writes its own bundle | A stage can be run and inspected without the later stages. |
| Orchestration | `backtest run` uses the same stage engines and preserves their bundles | The full run is convenient, not opaque; it interleaves strategy, execution, and portfolio so the strategy sees the actual position. |
| Statistics | Confined to qualification and analytics evaluation | No statistic enters a strategy decision or a model reading. |
| Existing commands | Their names, arguments, and meanings are unchanged | A new capability arrives as a new command. |
| Run composition | Strategy and criteria are explicit arguments | `run.json` stays v2; no new config field is introduced. |
| Artifact provenance | New bundles use `quantrule.stage-provenance/v1` | The existing model bundle keeps its provenance schema. |

## 6. Acceptance criteria

### 6.1 Phase 1 — command and artifact contracts

- `docs/architecture/command-surface.md` names every stage, command, input,
  output, status, and discovery surface.
- `docs/config/2026-11-06/engine-artifacts.md` defines the artifact and
  provenance contract for every stage.
- `docs/config/2026-11-06/qualification-criteria.md` defines the criteria
  dimensions, verdict semantics, and human acceptance boundary.
- `tools/check_command_surface.py` fails when an implemented command is missing
  from `help.rs`, the README, or a tutorial, and when a planned command is listed
  as available.
- The README, `docs/README.md`, and `docs/architecture/README.md` link the new
  contracts.
- No planned command appears in `help.rs`, the README command table, or a
  tutorial.
- Every existing gate passes.

### 6.2 Phase 2 — Strategy

- `strategy explain` and `strategy run` parse, dispatch, and run.
- The decisions bundle matches the artifact contract.
- The strategy input boundary test proves the strategy cannot read a diagnostic,
  a statistic, a fill, a position, cash, or P&L.
- Help, README, tutorial, and checker pass.

### 6.3 Phase 3 — Execution

- `execution run` consumes a decisions bundle, sizes entries with the portfolio
  sizing rule, and writes orders, fills, and rejections.
- The fill policy is unchanged and the artifact is deterministic.
- Help, README, tutorial, and checker pass.

### 6.4 Phase 4 — Portfolio

- `portfolio replay` consumes a fills bundle and writes trades, equity, funding,
  risk events, and metrics.
- A missing metric is `null`, never zero.
- Help, README, tutorial, and checker pass.

### 6.5 Phase 5 — Backtest orchestration

- `backtest run` runs the five stages and preserves every bundle.
- It interleaves strategy, execution, and portfolio so the strategy sees the
  position execution actually holds.
- The manifest names every stage artifact and its hash.
- The command adds no domain logic.
- Help, README, tutorial, and checker pass.

### 6.6 Phase 6 — Qualification

- `qualify` reads a backtest bundle and a human-owned criteria file.
- It emits `pass`, `fail`, or `insufficient_evidence` with per-criterion evidence.
- It writes `human_decision: null` and never writes `decision.md`.
- Help, README, tutorial, and checker pass.

### 6.7 Phase 7 — Analytics

- `analytics evaluate` writes descriptive metrics and evaluation tables.
- It writes no verdict.
- Help, README, tutorial, and checker pass.

### 6.8 Phase 8 — Verification

- Every stage command runs independently on fixture data.
- Every artifact has the documented schema, provenance, and deterministic order.
- A rerun is byte-identical.
- The command surface checker passes.
- The full gate passes.
- Residual risks and the human acceptance boundary are recorded.

## 7. Work breakdown

| Phase | Work | Affected paths | Roles | PR |
|---:|---|---|---|---|
| 1 | Command surface, artifact contract, qualification criteria, checker, README and index updates | `docs/architecture/command-surface.md`, `docs/config/2026-11-06/`, `docs/execution-plans/2026-11-06/s3-command-surface.md`, `docs/ai-transparency-log/2026-11-06/s3-command-surface.md`, `tools/check_command_surface.py`, `.github/workflows/ci.yml`, `README.md`, `docs/README.md`, `docs/architecture/README.md` | Architect, Test Engineer, Code Reviewer | PR 1 |
| 2 | Strategy commands, decisions artifact, strategy config parser, boundary and determinism tests | `crates/quantrule-shell/src/command/`, `crates/quantrule-backtest/src/strategy/`, tests | Quant Researcher, Quant Developer, Quant Validator, Implementation, Test | PR 2 |
| 3 | Execution command, fills artifact, order/fill tests | `crates/quantrule-shell/src/command/`, `crates/quantrule-backtest/src/execution/`, tests | Implementation, Test | PR 3 |
| 4 | Portfolio command, performance artifact, metric tests | `crates/quantrule-shell/src/command/`, `crates/quantrule-backtest/src/portfolio/`, tests | Implementation, Test | PR 4 |
| 5 | Backtest orchestration and manifest | `crates/quantrule-shell/src/command/`, `crates/quantrule-backtest/`, tests | Orchestrator, Implementation, Test | PR 5 |
| 6 | Qualification engine, criteria parser, record, verdict tests | `crates/quantrule-shell/src/command/`, `crates/quantrule-backtest/`, tests | Quant Researcher, Quant Developer, Quant Validator | PR 6 |
| 7 | Analytics evaluation command and metrics | `crates/quantrule-shell/src/command/`, `crates/quantrule-backtest/src/benchmark/`, tests | Implementation, Test, Quant Validator | PR 7 |
| 8 | Independent stage verification, tutorial completion, closure record | `docs/`, tests | Orchestrator, Test Engineer, Code Reviewer | PR 8 |

Only one implementation agent writes to a given file or workstream at a time.
Parallel investigation is allowed when the scopes do not overlap.

## 8. Issue hierarchy

No GitHub issue numbers are invented. Numbers are assigned when the issues are
opened.

### Parent — `Complete command-first S3.x pipeline surface`

Goal: make every pipeline stage independently runnable and inspectable from the
shell, with documented commands and artifacts.

Done when:

- every stage has a command, help entry, README row, and tutorial;
- every stage artifact has a documented schema and provenance;
- the Qualification Engine emits a verdict without replacing human acceptance;
- `backtest run` orchestrates the stages but does not hide them;
- all gates pass.

### Sub-issues

| Sub-issue | Scope | Depends on |
|---|---|---|
| 1. Define S3.x command and artifact contracts | Command surface, artifact schemas, qualification criteria, drift checker, README/help/tutorial expectations. No stage logic. | — |
| 2. Add strategy engine and commands | `strategy explain`, `strategy run`; decisions artifact; as-of rationale; position-state input; help, README, tutorial. | 1 |
| 3. Add execution engine and command | `execution run`; fills artifact; cost/slippage model; deterministic order; help, README, tutorial. | 2 |
| 4. Add portfolio engine and command | `portfolio replay`; trades, equity, PnL, exposure, drawdown; help, README, tutorial. | 3 |
| 5. Add backtest orchestration command | `backtest run`; stage invocation; manifest; no hidden stage logic; help, README, tutorial. | 3, 4 |
| 6. Add Qualification Engine and command | `qualify`; human-owned criteria; qualification record with per-criterion evidence and verdict; help, README, tutorial. | 5 |
| 7. Add analytics evaluation command | `analytics evaluate`; descriptive metrics over result artifacts; help, README, tutorial. | 5 |
| 8. Verify S3.x command surface | Run every stage independently; verify help, README, tutorials, artifacts, determinism; record residual risks. | 1–7 |

## 9. Verification

| Check | Expected |
|---|---|
| `cargo test --workspace` | All tests pass, 0 failures. |
| `cargo clippy --all-targets -- -D warnings` | Pass. |
| `cargo fmt --all -- --check` | Pass. |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | Pass. |
| `python3 tools/check_command_surface.py` | Pass; implemented commands match help, README, registry, and tutorials; planned commands are not listed as available. |
| `python3 tools/check_registry.py` | Pass. |
| `python3 tools/check_validation.py` | Pass. |
| `python3 tools/check_verification.py` | Pass. |
| `python3 tools/check_verification_matrix.py` | Pass. |
| `python3 tools/check_event_emitters.py` | Pass. |
| `python3 tools/check_links.py` | Pass. |
| `python3 tools/check_terms.py` | Pass. |
| `python3 tools/check_legacy.py` | Pass. |
| `python3 tools/check_file_size.py` | Pass. |
| `git diff --check` | Pass. |
| Every stage command runs independently | Pass on fixture data. |
| Every stage artifact has schema, provenance, and deterministic order | Pass by test. |
| `qualify` emits `pass`, `fail`, or `insufficient_evidence` with reasons | Pass by test. |
| The verdict does not replace human acceptance | Pass by boundary test. |
| No hidden library-only stage | Pass by command coverage. |

## 10. Risks and recovery

- **A stage is only a library.** Stop; the stage is not done until a command
  reaches it.
- **A command is a stub.** Stop; implement the stage or do not list the command.
- **The Qualification Engine starts accepting a strategy.** Stop; the human
  boundary is violated.
- **A statistical or fitted value enters a strategy decision.** Stop; the rule
  purity boundary is violated.
- **An existing command's meaning changes.** Stop and request human approval.
- **A new dependency or architecture boundary is required.** Stop and request
  approval.
- **A stage artifact is not deterministic.** Stop; a rerun that differs is a
  defect, not a formatting issue.
- **A model or strategy behavior change is discovered.** Stop; it needs a
  separate approved hypothesis and Quant Validator record.

Rollback is the phase branch. No history is rewritten, no release is tagged, and
no remote state is changed without a separate authorization.

## 11. Stop conditions

- A required command name or contract is not approved.
- A stage cannot read an explicit input without recomputing an upstream stage.
- A stage artifact would carry a wall-clock timestamp, unseeded randomness, or
  unordered iteration.
- A qualification criterion would be chosen or changed after the result is known.
- A command would be listed in `help`, the README, or a tutorial before it works.
- A gate fails for a reason outside the phase's scope.
- A change would require modifying an existing command's meaning.

## 12. Traceability

| Requirement | Contract | Verification |
|---|---|---|
| Every stage is typed | `docs/architecture/command-surface.md` §2–3 | `tools/check_command_surface.py` |
| Every command is discoverable | `docs/architecture/command-surface.md` §5 | Checker; help tests; tutorial |
| Every stage artifact has provenance | `docs/config/2026-11-06/engine-artifacts.md` §2–11 | Artifact tests; determinism tests |
| Qualification is evidence-based | `docs/config/2026-11-06/qualification-criteria.md` §3–4 | Verdict tests; golden fixture |
| The human accepts | `docs/config/2026-11-06/qualification-criteria.md` §7 | Boundary test; `human_decision: null` |
| Model output is measurement | `docs/architecture/command-surface.md` §2 | Model contract; no prediction field |
| Strategy output is decision | `docs/architecture/command-surface.md` §2 | Strategy boundary test |
| Statistics stays in evaluation | `docs/config/2026-11-06/qualification-criteria.md` §8 | Strategy boundary test |
| Backtest does not hide stages | `docs/config/2026-11-06/engine-artifacts.md` §8 | Stage bundle preservation test |
| Existing commands keep their meaning | `docs/architecture/command-surface.md` §3 | Help tests; README diff review |

## 13. Phase 1 verification record

Observed in this worktree on 2026-11-06:

| Check | Result |
|---|---|
| `python3 tools/check_command_surface.py` | Pass; 8 implemented, 7 planned, 8 README rows, 1 tutorial file. |
| `cargo test --workspace` | 1,173 passed, 0 failed, 8 ignored. |
| `cargo clippy --all-targets -- -D warnings` | Pass. |
| `cargo fmt --all -- --check` | Pass. |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | Pass. |
| `python3 tools/check_registry.py` | Pass; 46 indicators, 143 events, 101 parameters. |
| `python3 tools/check_validation.py` | Pass; 189 claims for 46 indicators and 143 active events. |
| `python3 tools/check_verification.py` | Pass; row=1091 family=329 tool=332 n/a=327. |
| `python3 tools/check_verification_matrix.py` | Pass; 46 indicators, 143 events. |
| `python3 tools/check_event_emitters.py` | Pass; 143 registered codes. |
| `python3 tools/check_links.py` | Pass; 572 Markdown files. |
| `python3 tools/check_terms.py` | Pass; 342 files. |
| `python3 tools/check_legacy.py` | Pass; 630 current files. |
| `python3 tools/check_file_size.py` | Pass. |
| `git diff --check` | Pass. |

Drift-guard evidence:

- Removing the `candles-tolerant` row from the README made the checker exit 1
  with `README.md is missing implemented command 'candles-tolerant'`; restoring
  the row returned the checker to 0.
- Adding a `strategy run` row to the README made the checker exit 1 with
  `README.md lists 'strategy run', which help.rs does not implement` and
  `README.md lists planned command 'strategy run' as available`; restoring the
  README returned the checker to 0.

Phase 1 is verified locally. Publication (commit, push, and pull request) is
pending human authorization.
