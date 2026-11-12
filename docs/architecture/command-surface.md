# Command Surface — Stage-Isolated Pipeline Access

- Status: Current; approved 2026-11-06
- Owner: Human
- Part of: [`docs/architecture/README.md`](README.md)
- Plan: [`docs/execution-plans/2026-11-06/s3-command-surface.md`](../execution-plans/2026-11-06/s3-command-surface.md)
- Claim type: command and artifact contract; no model, strategy, or edge claim.

## 1. Why the surface is command-first

QuantRule is a local application with two first-class surfaces: the command
shell and the desktop UI. They are the same program; both call the shared
application service, and feature parity is required. The shell is primarily for
agents and remains the stable command interface; the desktop UI is the human
workstation. There is no deployed service, account, or cloud runtime. The
pipeline stages are reachable through the shell today; the desktop pipeline
parity is tracked by the [audit remediation plan](../execution-plans/2026-11-11/audit-app-service-native-remediation.md).
A capability that only one surface can reach is a parity gap, not a completed
capability.

The pipeline is a chain of independently useful stages. A human or an agent must
be able to stop after any stage, inspect the artifact that stage produced, and
decide whether the next stage is worth running. Treating the backtest as one
opaque command hides the data, model, strategy, execution, and accounting errors
that the stages exist to expose.

The surface therefore follows six rules:

1. **Command-first.** Every pipeline stage has a command with a stable name, its
   arguments, its output shape, and its error behavior.
2. **Artifact-first.** A stage command consumes an explicit input artifact and
   writes an explicit output artifact. The artifact is the handoff; no stage
   reaches behind another stage to recompute its input.
3. **Stage-isolated.** A stage command runs on its own. `backtest run` is an
   orchestration convenience over the same stage engines and artifacts, never
   the only way to reach a stage.
4. **No stubs.** A command is listed in `help`, the README, and a tutorial only
   when it is implemented and tested. A command that parses but does nothing is
   a defect because it misleads both humans and agents.
5. **Deterministic output.** The same input, configuration, and data version
   produce the same artifact bytes and the same printed text. No stage reads the
   wall clock, uses unseeded randomness, or depends on unordered iteration.
6. **Human acceptance.** The engine measures, decides, fills, accounts, and
   evaluates. The human owns the acceptance criteria and makes the final
   acceptance decision. No command declares an edge or accepts a strategy.

## 2. Stage contract

The run composition is validated by `run check`; it is not a pipeline stage and
does not decide a trade.

| Stage | Engine | Input | Output | Command surface | Status |
|---:|---|---|---|---|---|
| 1 | Data Engine | Raw OHLCV | Normalized candles, timeframes, gaps, quality | `candles`, `candles-tolerant`, `timeframes`, `fetch` | Implemented |
| 2 | Model Engine | Candles/timeframes and model config | Readings, discrete states, confirmed levels, events, progression, context | `events`, `report model` | Implemented |
| 3 | Strategy Engine | Model measurements, strategy config, position state | Decisions/signals and their as-of rationale | `strategy explain`, `strategy run` | Implemented |
| 4 | Execution Engine | Decisions, execution config, portfolio sizing config, candles | Orders, fills, and rejections | `execution run` | Implemented |
| 5 | Portfolio Engine | Fills, portfolio config, marks | Positions, equity, PnL, exposure, drawdown | `portfolio replay` | Implemented |
| 6 | **Qualification Engine** | Evidence bundle and human-owned acceptance criteria | Qualification record and verdict | `qualify` | Implemented |
| 7 | Analytics Engine | Result artifacts | Descriptive metrics and evaluation tables | `analytics evaluate` | Implemented |
| — | Backtest orchestration | Run config and strategy config | Stage bundles and a run manifest | `backtest run` | Implemented |

### Terminology

The stage names are fixed, and so are three distinctions that the commands must
not blur:

- **Model "predictions" means measurements.** The model publishes readings,
  discrete states, confirmed levels, events, progression, and context. It does
  not publish a probabilistic forecast, a confidence, a composite score, or an
  edge claim.
- **Strategy "edge" means decisions/signals.** The Strategy Engine publishes a
  decision and the as-of evidence that justified it. "Edge" is a research verdict
  about evidence, not an engine output.
- **Qualification "verdict" means evidence status.** The Qualification Engine
  reports whether the evidence satisfies the human-owned criteria. It does not
  accept the strategy, declare profitability, or replace the human decision.

## 3. Command registry

The registry is the single list of command status. A command is **Implemented**
only when it has a `CommandHelp` entry in
[`crates/quantrule-shell/src/command/help.rs`](../../crates/quantrule-shell/src/command/help.rs),
a row in the README command table, and a runnable example in a tutorial. A
command is **Planned** until all three exist. The checker
[`tools/check_command_surface.py`](../../tools/check_command_surface.py) enforces
the distinction.

| Command | Stage | Status | Input | Output |
|---|---|---|---|---|
| `candles` | Data | Implemented | `<csv-path>` | NDJSON candles |
| `candles-tolerant` | Data | Implemented | `<csv-path>` | NDJSON candles and a quality report |
| `timeframes` | Data | Implemented | `<csv-path>` | NDJSON candles per timeframe and an aggregation report |
| `fetch` | Data | Implemented | `<symbol> <start> <end>` and options | CSV path and accepted/rejected counts |
| `memory` | Memory | Implemented | `status`, `record`, lists, metric/claim/note CRUD, export/import, backup/restore | Memory records and artifact paths |
| `events` | Model | Implemented | `<csv-path> [--window full\|latest]` | Event records |
| `report model` | Model | Implemented | `--config <run.json>` and options | Model bundle |
| `run check` | Configuration | Implemented | `--config <run.json>` | Validation line |
| `help` | Shell | Implemented | `[<command>\|all\|glossary\|workflows]` | Plain text |
| `strategy explain` | Strategy | Implemented | Model bundle, strategy config, bar index | One decision and its as-of evidence |
| `strategy run` | Strategy | Implemented | Model bundle, strategy config | Decisions bundle |
| `execution run` | Execution | Implemented | Decisions bundle, run config with execution and sizing | Fills bundle |
| `portfolio replay` | Portfolio | Implemented | Fills bundle, run config | Portfolio bundle |
| `backtest run` | Orchestration | Implemented | Run config, strategy config | Stage bundles and a run manifest |
| `qualify` | Qualification | Implemented | Evidence bundle, qualification criteria | Qualification record and verdict |
| `analytics evaluate` | Analytics | Implemented | Result artifacts | Metrics and benchmark bundle |

The current shell also accepts `quit` as a shell meta-command. It is not a
pipeline command and is not part of the stage contract.

`strategy run` supports the event-composite engine standalone. The
compression-bracket engine needs the fill side that execution supplies, so it is
available through `strategy explain` for one decision until the backtest runner
reaches the stage.

`execution run` consumes the strategy decisions bundle and the run config. A
standalone run sizes entries from the run's initial cash; it does not track
equity. A bracket decision needs the runner's side-aware sizing and is refused
with a clear error until the backtest runner supplies it.

`portfolio replay` consumes the execution fills bundle and the run config. It
marks the open position at each execution-timeframe close and writes the
portfolio bundle. A metric that is undefined is `null`, never zero.

`backtest run` is the coupled runner: it computes the model once, then
interleaves the strategy, execution, and portfolio engines so the strategy sees
the position execution actually holds. It writes the same stage bundles as the
standalone commands and a manifest with every stage provenance hash.

`qualify` consumes the backtest bundle and the human-owned criteria. It checks
the manifest and every stage provenance hash, resolves each criterion against the
named artifact field, applies the fixed verdict precedence, and writes the
qualification bundle. It writes `human_decision: null` and never writes
`decision.md`; acceptance remains a separate human act.

`analytics evaluate` consumes the same backtest bundle and publishes descriptive
metrics and evaluation tables. It aggregates the strategy decisions by timeframe
and copies or derives the portfolio tables. It writes no verdict, no acceptance,
and no strategy decision. A metric or table the evidence does not support is
`null` or unsupported with a reason code.

## 4. Artifact and provenance rule

Every stage command that writes a bundle writes one directory containing the
stage's data files and a `provenance.json`. The data files carry records; the
provenance file carries identity. The common fields are:

| Field | Meaning |
|---|---|
| `schema` | The artifact or provenance schema version. |
| `stage` | The producing stage: `data`, `model`, `strategy`, `execution`, `portfolio`, `backtest`, `qualification`, or `analytics`. |
| `data_version` | The identity of the input candle series, supplied by the caller. |
| `config_id` | A stable fingerprint of the configuration that produced the artifact. The existing model bundle calls this `config_hash`. |
| `code_commit` | The source revision of the executable, when the build supplies one. |
| `source_artifacts` | The stage artifacts this run consumed, with their stage and schema identity. |
| `seed` | The explicit seed when a run uses randomness; `null` otherwise. |

The existing model bundle keeps its `quantrule.provenance/v1` file and its
`config_hash` field. New stage bundles use the common fields and record the model
bundle as a source artifact. A later stage never infers an input from the
filesystem when the input artifact can name it.

The artifact contract, field details, and handoff rules are in
[`docs/config/2026-11-06/engine-artifacts.md`](../config/2026-11-06/engine-artifacts.md).

## 5. Discovery contract

A command exists only when all five surfaces agree:

| Surface | What it carries |
|---|---|
| `help.rs` | The command name, usage, summary, output shape, and contract notes. `help`, `help <command>`, and `help all` render from this registry; the glossary and workflow topics are static text. |
| README command table | The current command set and a one-line output description. |
| Tutorial | At least one runnable invocation of the command, with the expected output shape. |
| Command registry table | The stage, status, input, and output of the command. |
| Checker | A failing gate when any of the other four disagree. |

The checker reads the implemented commands from `help.rs`, the README table, the
registry table above, and the fenced code blocks under `docs/tutorials/`. It
fails when:

- an implemented command is missing from `help.rs`, the README, or a tutorial;
- a README row or an `Implemented` registry row names a command that `help.rs`
  does not have;
- a command marked `Planned` appears in `help.rs` or the README;
- a tutorial code block invokes a command that is not implemented;
- a `CommandHelp` entry has an empty usage, summary, or output contract.

A planned command is documented here and nowhere else until it is implemented.
It is not added to `help`, the README command table, or a tutorial early.

## 6. Qualification Engine boundary

The Qualification Engine is stage 6. It answers one question: **does the
evidence satisfy the human-owned acceptance criteria?** It does not answer
whether the strategy is profitable in general, and it does not accept the
strategy.

- The **human owns** the account objective, risk envelope, universe, thresholds,
  partitions, costs, and pass/fail rules. They are recorded before the run.
- The engine **evaluates** the evidence bundle against those criteria and emits a
  qualification record with a verdict of `pass`, `fail`, or
  `insufficient evidence`, with reasons and evidence references.
- The engine **never** declares an edge, chooses a threshold after seeing the
  result, drops a failing criterion, or writes the human's acceptance.
- The **human makes the final acceptance decision** in a separate record. A
  `pass` may still be rejected for residual risk; a `fail` cannot be accepted as
  an edge.

The engine evaluates the whole evidence set, not profitability alone:
out-of-sample and walk-forward evidence; robustness across folds, regimes, and
instruments; drawdown, duration, and recovery; trade count, profit factor, win
rate, and return distribution; exposure, concentration, tail risk, and portfolio
limits; cost and slippage sensitivity; parameter stability; multiple-testing
correction; data coverage and quality; and the matched baseline controls.

The criteria contract is in
[`docs/config/2026-11-06/qualification-criteria.md`](../config/2026-11-06/qualification-criteria.md).

## 7. Adding a command

A new command lands as one coherent change:

1. Add the stage engine logic to the owning crate, not to the shell.
2. Add the `CommandHelp` entry with usage, summary, output shape, and notes.
3. Add the README row and the tutorial invocation.
4. Move the registry row from `Planned` to `Implemented`.
5. Add the command test: run it and assert on what it printed or wrote.
6. Add the artifact schema and provenance fields when the command writes a
   bundle.
7. Run `python3 tools/check_command_surface.py` and the full gate.

A change to an existing command's meaning is not part of this checklist. It is a
contract change reserved to the human.

## 8. Sequence

The command surface was delivered in phases, one stage at a time:

| Phase | Deliverable | Status |
|---:|---|---|
| 1 | Command and artifact contracts, qualification criteria, drift checker | Merged |
| 2 | Strategy commands and decisions artifact | Merged |
| 3 | Execution command and fills artifact | Merged |
| 4 | Portfolio command and performance artifact | Merged |
| 5 | Backtest orchestration command | Merged |
| 6 | Qualification command and record | Merged |
| 7 | Analytics evaluation command | Merged |
| 8 | Independent stage verification and closure | Merged and verified locally; human acceptance pending |

Each phase adds only implemented commands to the discovery surfaces. The plan
and traceability are in
[`docs/execution-plans/2026-11-06/s3-command-surface.md`](../execution-plans/2026-11-06/s3-command-surface.md).
