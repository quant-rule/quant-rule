# AI Transparency Log — S3.x Command Surface

- Date: 2026-11-06 (UTC)
- Plan: [`docs/execution-plans/2026-11-06/s3-command-surface.md`](../../execution-plans/2026-11-06/s3-command-surface.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash), single session
- Intent and boundary: read the current shell, backtest, and documentation
  surfaces; draft the command, artifact, and qualification contracts; add the
  command-surface checker. No model or strategy behavior change. Publication is a
  separate human gate.

## 1. Preflight

The preflight read the current command registry, parser, dispatcher, README,
M1.0 tutorial, CI gate, model contract, policy contract, and the backtest
strategy, execution, and portfolio types. Observed state:

| Item | Observation |
|---|---|
| Implemented commands | `candles`, `candles-tolerant`, `timeframes`, `events`, `report model`, `policy check`, `fetch`, `help`. |
| Shell meta-command | `quit`; it is not in `help.rs`. |
| Backtest crate | Strategy, execution, portfolio, and benchmark engines exist; no runner and no shell command. |
| Run config | `quantrule.run/v2` loads policy, data, model, execution, and portfolio; no strategy or analytics file. |
| Current tutorial | `docs/tutorials/2026-11-06/m1.0-model-lab.md`, eight commands. |
| Command drift gate | None. The README table and `help.rs` were maintained by hand. |
| Model verification | Closed at 1,091 row cells and 329 family cells; no validation partition opened. |

## 2. Drafted work

- Added `docs/architecture/command-surface.md`: the stage table, the command
  registry with implemented/planned status, the discovery contract, the artifact
  rule, and the Qualification Engine boundary.
- Added `docs/config/2026-11-06/engine-artifacts.md`: the common provenance
  envelope, the data and model artifacts, and the planned strategy, execution,
  portfolio, backtest, qualification, and analytics bundles.
- Added `docs/config/2026-11-06/qualification-criteria.md`: the human-owned
  criteria, the required evidence dimensions, the verdict semantics, the human
  acceptance record, and the statistics boundary.
- Added `docs/execution-plans/2026-11-06/s3-command-surface.md`: the full S3.x
  plan, the issue hierarchy without invented numbers, the PR sequence, the roles,
  the verification table, and the stop conditions.
- Added `tools/check_command_surface.py`: it reads the implemented commands from
  `help.rs`, the README command table, the command registry table, and the
  tutorial code blocks; it fails on a missing, extra, or mislabelled command.
- Wired the checker into `.github/workflows/ci.yml`.
- Updated `README.md`, `docs/README.md`, and `docs/architecture/README.md` to link
  the contracts without listing a planned command as available.

## 3. Corrections and rejections

- The source stage table used "Predictions" for the model output. The contract
  now names readings, states, levels, events, progression, and context, and states
  explicitly that a prediction is a measurement, not a forecast.
- The source stage table used "Trading Signals / Edge" for the strategy output.
  The contract now names decisions/signals and their as-of rationale, and states
  that an edge is a research verdict, not an engine output.
- The initial stage 6 name and command used "policy". The approved name is the
  **Qualification Engine** and the command is `qualify`; the criteria are
  human-owned and the human makes the final acceptance decision.
- The first artifact draft would have required `code_commit` on the existing
  model bundle. The contract instead applies the common provenance envelope to
  new stage bundles and leaves the existing `quantrule.provenance/v1` model
  bundle unchanged.
- The first command plan would have added `strategy` to `run.json`. The contract
  keeps `run.json` at `quantrule.run/v2` and passes the strategy and criteria
  files explicitly, so no config schema change is hidden in an implementation
  phase.
- The first qualification draft would have let the engine write the acceptance
  field. The contract fixes `human_decision: null` and puts the decision in a
  separate human-authored `decision.md`.

## 4. Verification

Observed on 2026-11-06 in this worktree:

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

The checker was also exercised against two deliberate drifts: a removed
`candles-tolerant` README row and an added `strategy run` README row. Both made it
exit 1 with the expected messages; restoring the README returned it to 0.

No independent Code Reviewer or Quant Validator pass has been performed. The
agent performed the drafting and the self-review in one session; no independent
review is claimed. Phase 1 changes no model or strategy behavior, so no Quant
Validator reproduction is applicable.

## 5. Learning

- The command registry is only a source of truth if a gate reads it. The README
  table and `help.rs` were consistent at the start, but nothing compared them, so
  a future addition could drift without a failing check.
- A planned command is useful in a contract because it makes the stage order
  reviewable, but it must never appear in `help`, the README, or a tutorial before
  it runs.
- The model bundle's provenance schema is a real compatibility constraint. The
  new stage bundles can share a stronger envelope without rewriting the model
  bundle in a documentation phase.
- The human acceptance boundary is easier to enforce when the engine writes
  `human_decision: null` and the decision is a separate file the engine cannot
  produce.

## 6. Publication

Publication (push, pull request, merge, tag, or release) is pending human
authorization. The work is local to the task branch `docs/s3-command-contracts`.

## 7. Phase 2 — strategy commands

- Intent and boundary: add `strategy explain` and `strategy run`, the
  `quantrule.strategy/v1` parser, the model-bundle reader, and the decisions
  bundle. The stage reads the model bundle; it does not re-run the model or read
  a candle, fill, account, or P&L.
- Implemented: `crates/quantrule-shell/src/config/strategy.rs`,
  `crates/quantrule-shell/src/strategy/`, `crates/quantrule-shell/src/command/strategy.rs`,
  the command wiring, help entries, README rows, strategy lab, and strategy
  configuration contract. `EventView` now carries the stable `event_code`, and
  `readings.csv` now carries the already-published `macd_histogram_state` column.
- Verification: `cargo test --workspace` 1,196 passed, 0 failed, 8 ignored;
  clippy, fmt, doc, all model gates, links, terms, legacy, file size, and
  `git diff --check` pass. The command-surface checker reports 10 implemented and
  5 planned. The integration test generates a real model bundle with
  `report model`, then proves the explain output, the decisions bundle, byte
  determinism, the non-empty-target refusal, and the bracket-run refusal.
- Corrections: the first strategy test used an extended event subtype 10, which
  the bounded event-composite engine does not accept; the test and fixture use a
  base subtype instead. The first help fixtures predated the two commands and
  were regenerated. The first command parser test still treated
  `strategy explain` as a deleted command; the case was removed because the
  command is now implemented.
- Learning: the strategy stage could not honestly read a model bundle until the
  per-bar `macd_histogram_state` was present; the model result already published
  the state, so the fix is an additive artifact column, not a model behavior
  change. The standalone bracket engine needs the fill side that execution owns,
  so `strategy run` refuses it rather than inventing a side.
- Publication (push, pull request, merge, tag, or release) is pending human
  authorization. The work is local to the task branch.

## 8. Phase 3 — execution command

- Intent and boundary: add `execution run`, the strategy decisions reader, and
  the execution bundle. The stage reads the decisions bundle and the run config;
  it does not read a strategy config, a model bundle, a statistic, or a policy
  decision.
- Implemented: `crates/quantrule-shell/src/execution/decisions.rs`,
  `crates/quantrule-shell/src/execution/run.rs`,
  `crates/quantrule-shell/src/command/execution.rs`, command wiring, help,
  README, execution lab, and the Phase 3 plan record.
- Verification: `cargo test --workspace` 1,208 passed, 0 failed, 8 ignored;
  clippy, fmt, doc, all model gates, links, terms, legacy, file size, and
  `git diff --check` pass. The command-surface checker reports 11 implemented and
  4 planned. The integration test builds the model and strategy bundles, then
  proves the execution bundle shape, byte determinism, the non-empty-target
  refusal, the data-version handoff, and the bracket-decision refusal.
- Corrections: the first summary formatter mixed positional and named format
  arguments and was replaced with `writeln!`; the first `summary_bytes` had nine
  arguments and was grouped into `SummaryInput` to satisfy the lint. The first
  iteration tried to zip a `TimeframeSeries` directly; the command now collects
  the execution candles explicitly.
- Learning: execution cannot size a bracket decision from the decision alone
  because the side is unknown until a fill; the standalone command refuses it
  and leaves it to the full runner. The decisions bundle's data version and bar
  identity are both checked before any order is submitted, so a mismatched
  handoff fails before a fill.
- Publication (push, pull request, merge, tag, or release) is pending human
  authorization. The work is local to the task branch.
