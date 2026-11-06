# S3.x Command Surface — Stage-Isolated Pipeline Access

- Date: 2026-11-06 (UTC)
- Status: Approved; Phases 1–7 merged; Phase 8 verified locally; publication pending; human acceptance pending
- Issue: none (human-approved slug; no issue number is invented)
- Branch: one task branch per phase; current `test/s3-closure-verification`
- Worktree: `s3-closure-verification` (OpenCode task worktree)
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

## 14. Phase 2 verification record

Observed in the Phase 2 worktree on 2026-11-06:

| Check | Result |
|---|---|
| `cargo test --workspace` | 1,196 passed, 0 failed, 8 ignored. |
| `cargo clippy --all-targets -- -D warnings` | Pass. |
| `cargo fmt --all -- --check` | Pass. |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | Pass. |
| `python3 tools/check_command_surface.py` | Pass; 10 implemented, 5 planned, 10 README rows, 2 tutorial files. |
| `python3 tools/check_registry.py` | Pass; 46 indicators, 143 events, 101 parameters. |
| `python3 tools/check_validation.py` | Pass; 189 claims. |
| `python3 tools/check_verification.py` | Pass; row=1091 family=329 tool=332 n/a=327. |
| `python3 tools/check_verification_matrix.py` | Pass; 46 indicators, 143 events. |
| `python3 tools/check_event_emitters.py` | Pass; 143 registered codes. |
| `python3 tools/check_links.py` | Pass; 574 Markdown files. |
| `python3 tools/check_terms.py` | Pass; 344 files. |
| `python3 tools/check_legacy.py` | Pass; 640 current files. |
| `python3 tools/check_file_size.py` | Pass. |
| `git diff --check` | Pass (run before publication). |

Behavior evidence:

- `strategy explain` on `target/strategy-lab/model` at bar 21 prints an `enter`
  decision with `side: long`, an ATR stop intent, and one event evidence record.
- `strategy run` writes `decisions.ndjson`, `summary.txt`, and `provenance.json`;
  two runs into separate roots are byte-identical.
- The decision record carries no `fill`, `quantity`, `fee`, `cash`, `equity`, or
  `pnl` key.
- A non-empty strategy target is refused; a compression-bracket strategy is
  refused by `strategy run` with the documented execution-feedback reason, while
  `strategy explain` supports it for one decision.
- The strategy bundle reader reads `readings.csv`, `events.csv`, and
  `provenance.json`; it does not re-run the model or read a candle.

The model bundle gained the already-published `macd_histogram_state` per-bar
column so the strategy can read a state the model contract already publishes.
The model values and event definitions are unchanged.

No independent Code Reviewer or Quant Validator pass has been performed. The
agent performed the implementation and the self-review in one session; no
independent review is claimed. The change adds a stage command and an additive
artifact column; it does not change a signal formula or event definition. The
Quant Validator record is not claimed, and the PR states the reason it is not
applicable to a model or strategy behavior change.

Phase 2 is verified locally. Publication (commit, push, and pull request) is
pending human authorization.

## 15. Phase 3 verification record

Observed in the Phase 3 worktree on 2026-11-06:

| Check | Result |
|---|---|
| `cargo test --workspace` | 1,208 passed, 0 failed, 8 ignored. |
| `cargo clippy --all-targets -- -D warnings` | Pass. |
| `cargo fmt --all -- --check` | Pass. |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | Pass. |
| `python3 tools/check_command_surface.py` | Pass; 11 implemented, 4 planned, 11 README rows, 3 tutorial files. |
| `python3 tools/check_registry.py` | Pass; 46 indicators, 143 events, 101 parameters. |
| `python3 tools/check_validation.py` | Pass; 189 claims. |
| `python3 tools/check_verification.py` | Pass; row=1091 family=329 tool=332 n/a=327. |
| `python3 tools/check_verification_matrix.py` | Pass; 46 indicators, 143 events. |
| `python3 tools/check_event_emitters.py` | Pass; 143 registered codes. |
| `python3 tools/check_links.py` | Pass; 575 Markdown files. |
| `python3 tools/check_terms.py` | Pass; 345 files. |
| `python3 tools/check_legacy.py` | Pass; 645 current files. |
| `python3 tools/check_file_size.py` | Pass. |
| `git diff --check` | Pass (run before publication). |

Behavior evidence:

- `execution run` on the fixture decisions writes `orders.csv`, `fills.csv`,
  `rejections.csv`, `summary.txt`, and `provenance.json`.
- The fixture produces two fills: an entry at bar 19 and a stop-loss exit at bar
  30; the rejection table is empty.
- Two runs into separate roots are byte-identical.
- A non-empty execution target is refused.
- A decisions bundle whose `data_version` differs from the run's is refused.
- A synthetic `enter_bracket` decision is refused with the side-aware sizing
  reason.
- The execution bundle carries no account cash balance, equity curve, or P&L;
  the summary records the initial cash and notional cap used for sizing.

The execution stage reads the strategy decisions bundle and the run config. It
does not read a strategy config, a model bundle, a statistic, or a policy
decision. A standalone run sizes entries from the run's initial cash; the full
backtest runner supplies the ledger's current equity.

No independent Code Reviewer or Quant Validator pass has been performed. The
agent performed the implementation and the self-review in one session; no
independent review is claimed. The change adds a stage command and writes the
existing execution engine's records; it does not change a signal formula, event
definition, or strategy rule. The Quant Validator record is not claimed, and the
PR states the reason it is not applicable to a model or strategy behavior change.

Phase 3 is verified locally. Publication (commit, push, and pull request) is
pending human authorization.

## 16. Phase 4 verification record

Observed in the Phase 4 worktree on 2026-11-06:

| Check | Result |
|---|---|
| `cargo test --workspace` | 1,220 passed, 0 failed, 8 ignored. |
| `cargo clippy --all-targets -- -D warnings` | Pass. |
| `cargo fmt --all -- --check` | Pass. |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | Pass. |
| `python3 tools/check_command_surface.py` | Pass; 12 implemented, 3 planned, 12 README rows, 4 tutorial files. |
| `python3 tools/check_registry.py` | Pass; 46 indicators, 143 events, 101 parameters. |
| `python3 tools/check_validation.py` | Pass; 189 claims. |
| `python3 tools/check_verification.py` | Pass; row=1091 family=329 tool=332 n/a=327. |
| `python3 tools/check_verification_matrix.py` | Pass; 46 indicators, 143 events. |
| `python3 tools/check_event_emitters.py` | Pass; 143 registered codes. |
| `python3 tools/check_links.py` | Pass; 576 Markdown files. |
| `python3 tools/check_terms.py` | Pass; 346 files. |
| `python3 tools/check_legacy.py` | Pass; 650 current files. |
| `python3 tools/check_file_size.py` | Pass. |
| `git diff --check` | Pass (run before publication). |

Behavior evidence:

- `portfolio replay` on the fixture fills writes `trades.csv`, `equity.csv`,
  `funding.csv`, `risk_events.csv`, `metrics.json`, `summary.txt`, and
  `provenance.json`.
- The fixture produces one closed trade with a stop exit and no funding charge;
  `final_equity = initial_cash + net_pnl` holds.
- An empty fills bundle produces zero trades and `win_rate`, `profit_factor`, and
  `average_trade` as `null`, never zero.
- Two runs into separate roots are byte-identical.
- A non-empty portfolio target is refused.
- A fills bundle whose `data_version` differs from the run's is refused.
- The portfolio bundle carries account cash, equity, P&L, exposure, and drawdown;
  the metrics are accounting measurements, not a profitability claim.

No independent Code Reviewer or Quant Validator pass has been performed. The
agent performed the implementation and the self-review in one session; no
independent review is claimed. The change adds a stage command and writes the
existing portfolio engine's records; it does not change a signal formula, event
definition, or strategy rule. The Quant Validator record is not claimed, and the
PR states the reason it is not applicable to a model or strategy behavior change.

Phase 4 is verified locally. Publication (commit, push, and pull request) is
pending human authorization.

## 17. Phase 5 verification record

Observed in the Phase 5 worktree on 2026-11-06:

| Check | Result |
|---|---|
| `cargo test --workspace` | 1,229 passed, 0 failed, 8 ignored. |
| `cargo clippy --all-targets -- -D warnings` | Pass. |
| `cargo fmt --all -- --check` | Pass. |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | Pass. |
| `python3 tools/check_command_surface.py` | Pass; 13 implemented, 2 planned, 13 README rows, 5 tutorial files. |
| `python3 tools/check_registry.py` | Pass; 46 indicators, 143 events, 101 parameters. |
| `python3 tools/check_validation.py` | Pass; 189 claims. |
| `python3 tools/check_verification.py` | Pass; row=1091 family=329 tool=332 n/a=327. |
| `python3 tools/check_verification_matrix.py` | Pass; 46 indicators, 143 events. |
| `python3 tools/check_event_emitters.py` | Pass; 143 registered codes. |
| `python3 tools/check_links.py` | Pass; 577 Markdown files. |
| `python3 tools/check_terms.py` | Pass; 347 files. |
| `python3 tools/check_legacy.py` | Pass; 655 current files. |
| `python3 tools/check_file_size.py` | Pass. |
| `git diff --check` | Pass (run before publication). |

Behavior evidence:

- `backtest run` writes `manifest.json`, `provenance.json`, and the model,
  strategy, execution, and portfolio bundles under `<out>/backtest/`.
- The manifest lists four stages, their relative paths, their provenance paths,
  and four non-empty provenance hashes; the backtest provenance lists four
  source artifacts.
- The coupled fixture produces one closed stop trade; the accounting invariant
  `final_equity = initial_cash + net_pnl` holds.
- The execution bundle records `sizing_basis: ledger_current` because the
  coupled runner sizes each entry from the ledger's current equity.
- Two runs into separate roots produce byte-identical backtest trees.
- A non-empty backtest target is refused.
- A missing strategy config is refused before a bundle is written.

The command composes the existing stage engines and writes their bundles; it
does not add a signal formula, event definition, or strategy rule. The model is
computed once and the same `ModelRun` feeds both the model bundle and the
strategy input.

No independent Code Reviewer or Quant Validator pass has been performed. The
agent performed the implementation and the self-review in one session; no
independent review is claimed. The Quant Validator record is not claimed, and the
PR states the reason it is not applicable to a model or strategy behavior change.

Phase 5 is verified locally. Publication (commit, push, and pull request) is
pending human authorization.

## 18. Phase 6 verification record

Observed in the Phase 6 worktree on 2026-11-06:

| Check | Result |
|---|---|
| `cargo test --workspace` | 1,279 passed, 0 failed, 8 ignored. |
| `cargo clippy --all-targets -- -D warnings` | Pass. |
| `cargo fmt --all -- --check` | Pass. |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | Pass. |
| `python3 tools/check_command_surface.py` | Pass; 14 implemented, 1 planned, 14 README rows, 6 tutorial files. |
| `python3 tools/check_registry.py` | Pass; 46 indicators, 143 events, 101 parameters. |
| `python3 tools/check_validation.py` | Pass; 189 claims. |
| `python3 tools/check_verification.py` | Pass; row=1091 family=329 tool=332 n/a=327. |
| `python3 tools/check_verification_matrix.py` | Pass; 46 indicators, 143 events. |
| `python3 tools/check_event_emitters.py` | Pass; 143 registered codes. |
| `python3 tools/check_links.py` | Pass; 579 Markdown files. |
| `python3 tools/check_terms.py` | Pass; 348 files. |
| `python3 tools/check_legacy.py` | Pass; 676 current files. |
| `python3 tools/check_file_size.py` | Pass. |
| `git diff --check` | Pass. |

Behavior evidence:

- `qualify` writes `qualification.json`, `criteria.csv`, `qualification.md`, and
  `provenance.json` under `<out>/qualification/`.
- The pass fixture records three mandatory passes and one diagnostic failure;
  the verdict is `pass`, `human_decision` is `null`, and `decision.md` is absent.
- The fail fixture records one mandatory `fail` and one mandatory
  `insufficient_evidence`; the verdict is `fail`.
- A required partition without overlap turns every would-be pass into
  `insufficient_evidence` with `required_partition_missing`.
- A missing measurement records `observed: null` and
  `insufficient_evidence`, never zero.
- Criteria that resolve `execution/provenance.json` and `model/result.json`
  fields pass on the fixture, so the evidence-field paths cover every supported
  artifact.
- Two runs over the same evidence and criteria produce byte-identical
  qualification bundles.
- A tampered stage provenance is refused before evaluation.
- An invalid criteria file is refused before a bundle is written.
- The golden fixture fixes the record, the criteria table, the Markdown
  rendering, and the provenance.
- The strategy boundary test forbids a qualification record, verdict, or
  analytics metric in the strategy input.
- A unit test compares every embedded partition window with
  `data/partitions.json`, so the workflow contract and the engine cannot drift.

The command evaluates the evidence and writes a verdict; it does not add a
signal formula, event definition, strategy rule, or statistical input to the
signal path. The criteria file is human-owned and is fingerprinted as the stage
`config_id`.

No independent Code Reviewer or Quant Validator pass has been performed. The
agent performed the implementation and the self-review in one session; no
independent review is claimed. The Quant Validator record is not claimed because
the change adds an evaluation stage and does not alter the model or strategy
signal path.

Phase 6 is verified locally. Publication (commit, push, and pull request) is
pending human authorization.

## 19. Phase 7 verification record

Observed in the Phase 7 worktree on 2026-11-06:

| Check | Result |
|---|---|
| `cargo test --workspace` | 1,303 passed, 0 failed, 8 ignored. |
| `cargo clippy --all-targets -- -D warnings` | Pass. |
| `cargo fmt --all -- --check` | Pass. |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | Pass. |
| `python3 tools/check_command_surface.py` | Pass; 15 implemented, 0 planned, 15 README rows, 7 tutorial files. |
| `python3 tools/check_registry.py` | Pass; 46 indicators, 143 events, 101 parameters. |
| `python3 tools/check_validation.py` | Pass; 189 claims. |
| `python3 tools/check_verification.py` | Pass; row=1091 family=329 tool=332 n/a=327. |
| `python3 tools/check_verification_matrix.py` | Pass; 46 indicators, 143 events. |
| `python3 tools/check_event_emitters.py` | Pass; 143 registered codes. |
| `python3 tools/check_links.py` | Pass; 580 Markdown files. |
| `python3 tools/check_terms.py` | Pass; 349 files. |
| `python3 tools/check_legacy.py` | Pass; 695 current files. |
| `python3 tools/check_file_size.py` | Pass. |
| `git diff --check` | Pass. |

Behavior evidence:

- `analytics evaluate` writes `metrics.json`, `summary.txt`,
  `provenance.json`, and the eight declared `tables/` CSVs under
  `<out>/analytics/`.
- The golden fixture fixes `metrics.json`, `summary.txt`, `provenance.json`, and
  every table.
- The fixture records one trade, 51 hold decisions, and one entry; the decision
  counts and `by_timeframe.csv` match the strategy bundle.
- The metric table and every table header and row count match the `tables`
  manifest.
- Two runs over the same evidence produce byte-identical analytics bundles.
- A tampered stage provenance, a missing portfolio table, and an unsupported
  portfolio-metrics schema are refused before a bundle is written.
- The bundle carries no `verdict` and no `human_decision`.
- `by_regime` and `by_instrument` are recorded as unsupported with stable reason
  codes and are not synthesized.
- The shell enables `serde_json`'s `float_roundtrip` feature, so the analytics
  copy of a portfolio metric does not drift in the last digit.

The shared backtest-bundle reader now owns the manifest and stage-hash check for
both qualification and analytics. The analytics stage adds only artifact readers,
the metric builder, the table builder, and the renderer. It writes no signal, no
verdict, and no acceptance.

No independent Code Reviewer or Quant Validator pass has been performed. The
agent performed the implementation and the self-review in one session; no
independent review is claimed. The Quant Validator record is not claimed because
the change adds a descriptive evaluation stage and does not alter the model or
strategy signal path.

Phase 7 is verified locally. Publication (commit, push, and pull request) is
pending human authorization.

## 20. Phase 8 closure record

- Status: Verified locally on 2026-11-06; publication pending; human acceptance pending.
- Scope: every S3.x stage command, its artifact and provenance handoff, its deterministic output, and the human acceptance boundary.

### 20.1 Acceptance criteria

| Criterion | Evidence | Result |
|---|---|---|
| Every stage has a command, help entry, README row, and tutorial. | `tools/check_command_surface.py` reports 15 implemented, 0 planned, 15 README rows, 8 tutorial files. `pipeline_surface::the_surface_help_lists_every_stage_command` lists every stage command. | Met. |
| Every stage artifact has a documented schema and provenance. | `pipeline_surface::every_stage_runs_on_its_own_and_hands_off_its_artifact`; `the_coupled_run_reuses_the_stage_engines_and_artifacts`; the per-stage artifact tests; `docs/config/2026-11-06/engine-artifacts.md`. | Met. |
| The Qualification Engine emits a verdict without replacing human acceptance. | Qualification pass, fail, insufficient, and missing-measurement tests; the pipeline test asserts `human_decision: null` and no `decision.md`; `qualification.md` records the pending human decision. | Met. |
| `backtest run` orchestrates the stages but does not hide them. | The coupled bundle uses the four stage layouts; the coupled model is byte-identical to the standalone model; the manifest names four provenance hashes; the coupled provenance lists four source artifacts. | Met. |
| All gates pass. | Section 20.3. | Met locally. |
| Residual risks and the human acceptance boundary are recorded. | Sections 20.4 and 20.5. | Met. |

### 20.2 Stage verification

| Stage | Command | Independent run | Artifact and provenance |
|---|---|---|---|
| Data | `candles`, `candles-tolerant`, `timeframes`, `fetch` | `cli.rs`, `report_command.rs`, and the source tests; the fetch tests use no network. | Candle NDJSON and quality/aggregation reports; no bundle. |
| Model | `report model` | `pipeline_surface` and `report_command.rs`. | `quantrule.model-result/v4` and `quantrule.provenance/v1`. |
| Strategy | `strategy run` | `pipeline_surface` and `strategy_command.rs`. | `decisions.ndjson` and `quantrule.stage-provenance/v1`. |
| Execution | `execution run` | `pipeline_surface` and `execution_command.rs`. | `orders.csv`, `fills.csv`, `rejections.csv`, and `quantrule.stage-provenance/v1`. |
| Portfolio | `portfolio replay` | `pipeline_surface` and `portfolio_command.rs`. | Trades, equity, funding, risk events, metrics, and `quantrule.stage-provenance/v1`. |
| Backtest | `backtest run` | `pipeline_surface` and `backtest_command.rs`. | `quantrule.backtest-manifest/v1`, four stage bundles, four provenance hashes. |
| Qualification | `qualify` | `pipeline_surface` and `qualification_command.rs`. | `quantrule.qualification-result/v1`, `criteria.csv`, `qualification.md`, `human_decision: null`. |
| Analytics | `analytics evaluate` | `pipeline_surface` and `analytics_command.rs`. | `quantrule.analytics-metrics/v1`, eight tables, summary, provenance; no verdict. |

The handoff chain is explicit in the provenance:

- Strategy names the model bundle as its source.
- Execution names the decisions bundle as its source.
- Portfolio names the fills bundle as its source.
- The backtest manifest names all four stage provenances and their FNV hashes.
- Qualification and analytics name the backtest manifest as their source.

### 20.3 Verification record

| Check | Result |
|---|---|
| `cargo test --workspace` | 1,309 passed, 0 failed, 8 ignored. |
| `cargo clippy --all-targets -- -D warnings` | Pass. |
| `cargo fmt --all -- --check` | Pass. |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | Pass. |
| `python3 tools/check_command_surface.py` | Pass; 15 implemented, 0 planned, 15 README rows, 8 tutorial files. |
| `python3 tools/check_registry.py` | Pass; 46 indicators, 143 events, 101 parameters. |
| `python3 tools/check_validation.py` | Pass; 189 claims. |
| `python3 tools/check_verification.py` | Pass; row=1091 family=329 tool=332 n/a=327. |
| `python3 tools/check_verification_matrix.py` | Pass; 46 indicators, 143 events. |
| `python3 tools/check_event_emitters.py` | Pass; 143 registered codes. |
| `python3 tools/check_links.py` | Pass; 581 Markdown files. |
| `python3 tools/check_terms.py` | Pass; 350 files. |
| `python3 tools/check_legacy.py` | Pass; 696 current files. |
| `python3 tools/check_file_size.py` | Pass. |
| `git diff --check` | Pass. |

The eight ignored tests are deliberate: two release-only benchmarks
(`a_520k_m1_extended_range_benchmark`, `a_100k_history_benchmark`) and six
fixture-regeneration helpers that would otherwise rewrite checked-in golden
fixtures during a normal test run. The non-ignored golden assertions still run.

Behavior evidence:

- The standalone chain (`report model` -> `strategy run` -> `execution run` ->
  `portfolio replay`) writes every stage bundle from explicit inputs.
- The coupled chain (`backtest run` -> `qualify` -> `analytics evaluate`) writes
  the same stage layouts, the manifest, the qualification bundle, and the
  analytics bundle.
- The standalone and coupled model bundles are byte-identical.
- The standalone and coupled chains are each byte-identical across repeated
  runs.
- The standalone execution records `sizing_basis: initial_cash`; the coupled
  execution records `sizing_basis: ledger_current`.
- The qualification record writes `human_decision: null` and no `decision.md`.
- The analytics bundle carries no verdict and no human decision.
- The strategy boundary test forbids qualification, verdict, analytics, cash,
  equity, PnL, fill, and position fields in the strategy input.

### 20.4 Residual risks and known limitations

- Qualification protocol completeness is the human's responsibility. The engine
  validates the declared criteria and `not_applicable` entries but does not infer
  whether an omitted dimension applies.
- Partition coverage is an overlap, not proof that a whole partition was read.
  `strategy_validation` and `portfolio_dev` share the 2024 window, so the period
  alone cannot distinguish them; the run manifest and criteria must name the
  intended partition.
- Analytics does not synthesize `by_regime` or `by_instrument` because the
  backtest bundle publishes no regime label or instrument identity. Inferential
  metrics are not synthesized because the v1 evidence carries no fold or p-value
  structure; qualification owns the multiple-testing declaration.
- Standalone `strategy run` supports the event-composite engine only. The
  compression-bracket engine needs the fill side and is available through
  `strategy explain` for one decision or through `backtest run`.
- Standalone `execution run` sizes entries from the run's initial cash; the
  coupled runner sizes from the ledger's current equity. The two chains share
  schemas and layouts but are not expected to be byte-identical beyond the model.
- The qualification and analytics evaluators live in the shell's stage modules,
  following the existing stage-command layout. The architecture guidance that
  stage logic belongs in the owning crate is not fully applied; moving the pure
  evaluators into a lower-level evaluation crate is a follow-up if that boundary
  is required.
- The shell enables `serde_json`'s `float_roundtrip` feature. It fixes decimal
  round-trip for copied metrics but changes JSON number parsing behavior across
  the shell.
- No `model_validation` partition was opened and no edge claim is made. The
  backtest, qualification, and analytics results are descriptive evidence, not a
  profitability or live-trading claim.

### 20.5 Review disclosure and human acceptance

Two attempts to start a separate read-only AI review were interrupted by server
restarts. The verification was therefore performed as a sequential self-review of
the command surface, handoff provenance, determinism, boundary isolation,
documentation, and tests. No independent agent review, Code Reviewer pass, Quant
Validator pass, or human review is claimed.

The human remains the final authority. The Qualification Engine writes
`human_decision: null`; the human records acceptance in a separate `decision.md`.
No command accepts a strategy, declares an edge, or authorizes a release. A
`pass` may still be rejected for residual risk; a `fail` is a valid research
result.

Phase 8 is verified locally. Publication (commit, push, and pull request) and
human acceptance are pending.
