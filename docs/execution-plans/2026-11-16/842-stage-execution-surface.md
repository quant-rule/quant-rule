# Stage Execution Surface — Execution Plan

- Date: 2026-11-16 (UTC)
- Status: In Progress
- Claim type: Behaviour Claim
- Scope: diagnostic stage execution for the data, model, strategy, and portfolio engines; one coupled backtest evidence path; research ledger; operations verification; CLI and GUI parity; user manual
- Owner: Human
- Contract: [`docs/workflows/execution-plan.md`](../../workflows/execution-plan.md), [`AGENTS.md`](../../../AGENTS.md)
- Epic: #842
- Related: [`docs/workflows/improvement-workflow.md`](../../workflows/improvement-workflow.md), [`docs/workflows/run-contract.md`](../../workflows/run-contract.md), [`docs/architecture/command-surface.md`](../../architecture/command-surface.md)

## 1. Intent

Give the trader a first-class way to run the data, model, strategy, and portfolio engines separately for diagnosis and iteration, without weakening the single evidence path. The product becomes a Workbench for iteration, a Backtests inbox for evidence, a Research ledger for governed improvement, and an Operations page for verification — mirrored in the CLI and the GUI.

The affected users are the single operator and the agents that use the command shell. The improvement is measured by faster engine iteration, an explicit diagnostic/evidence boundary, and parity between the browser and the shell.

## 2. Current state

| Area | Observed state |
|---|---|
| Release | `v1.2.0`; branch `main` at `6a9ba1f8` |
| Command surface | 27 shell commands in `help.rs` and `command-surface.md` |
| GUI | 8 navigation sections, 10 screen rows, 27-command parity table |
| Stage execution | No endpoint executes a single pipeline stage; `http-api.md` §8 prohibits it |
| Model artifact reuse | `SubmitRunRequest` carries only `data_version` and `config_id` |
| Stage storage | No `stage` schema, table, lifecycle, or retention |
| Research surface | Improvement runs are file-based under `data/improvement`; no CLI/GUI research surface |
| Workflows | No Engine Execution, Backtest, or End-to-End Verification workflow |
| Manual | `docs/manual/` does not exist |
| Tests | 1,727 `#[test]`, 344 `#[tokio::test]`, 97 integration test files, 17 web test files |
| Gates | `tools/gate.sh`, composed tier, parity tier, 24 checkers |

The current architecture deliberately kept strategy, execution, and portfolio together and prohibited partial-pipeline endpoints. This plan reverses that prohibition only for diagnostic stage runs and preserves the coupled run as the only evidence path.

## 3. Scope

Included:
- Server-side stage runs for model, strategy, and portfolio; the data stage remains `dataset upload`/`inspect`.
- Stage artifacts, lifecycle, TTL, profile scoping, cancellation, and no partial publish.
- Model artifact reuse by `run submit`; strategy and portfolio recomputation.
- CLI `stage`, `research`, `baseline`, `invalidation`; `run submit --model-artifact`.
- Workbench, Backtests, Research, and Operations GUI.
- Engine Execution, Backtest, and End-to-End Verification workflows; one ADR.
- `docs/manual/` user manual and checker integration.
- Migrations, tests, parity, changelog, AI log, and release note.

Excluded:
- New indicators, events, strategy rules, execution rules, or portfolio rules.
- A change to the model, strategy, execution, or portfolio contracts.
- A seventh service.
- Live trading, scheduling, accounts, multi-tenancy, or a new data source.
- Rewriting the Improvement Workflow's stages, roles, gates, or green criteria.

## 4. Assumptions and uncertainty

| Assumption | Risk if wrong |
|---|---|
| The app stage use cases are stable | Stage routes may need engine changes |
| The model bundle is reusable byte-for-byte | Promotion would not match a full run |
| The backtest worker can handle stage kinds | A new service or worker would be required |
| Stage artifacts can be content-addressed and deduplicated | Profile scoping or retention semantics change |
| The config bundle identifies stage subsets | Stage runs would be invalidated by unrelated files |
| The browser can absorb a new information architecture | Parity and client-doc tests need a larger rewrite |
| `docs/manual` is accepted as a documentation kind | The manual would live under tutorials |

## 5. Decisions

| Decision | Choice | Approval |
|---|---|---|
| Workflow shape | Three new workflow documents + four stage runbooks; no per-engine research workflow duplicates | Human |
| Backtest command | Keep `run submit`; no `backtest run` alias | Human |
| Stage evidence | Diagnostic only; model artifact reusable; strategy/portfolio artifacts not reusable | Human + Quant Validator |
| End-to-end | Release & Operations verification workflow | Human |
| Data stage | Existing upload/inspect; no `data run` | Human |
| Build order | Model → strategy → portfolio → promotion | Human |
| Retention | Stage artifacts expire; evidence permanent | Human |
| GUI shape | Workbench tabs + Backtests + Research + Operations | Human |
| Stage lifecycle owner | Evidence owns the `stage` schema; the backtest worker computes all stage kinds | Architect + Human |
| Research manifest | `quantrule.improvement-run/v3` with `data_version` and `config_id` | Human |
| Release | `v2.0.0` (proposed) | Human |

## 6. Acceptance criteria

- A stage run for model, strategy, or portfolio is reachable from the CLI and the browser and produces a content-addressed diagnostic artifact.
- A stage artifact is never evidence, never a baseline, and never qualification input.
- A backtest may reuse a frozen model artifact; its model bytes and canonical digest equal a full run; strategy and portfolio are recomputed.
- The Workbench, Backtests, Research, and Operations surfaces match the CLI capability for capability.
- The three new workflows are indexed and the manual is a discovery surface.
- `tools/gate.sh`, the composed tier, and the parity tier pass.
- The release note and tag are published; worktrees and branches are removed.

## 7. Work breakdown

| Phase | Issue | Deliverable | Depends |
|---|---|---|---|
| P0 | #842.1 | ADR for stage execution and the evidence boundary | — |
| P0 | #842.2 | Engine Execution workflow + four runbooks | ADR |
| P0 | #842.3 | Backtest workflow | ADR |
| P0 | #842.4 | End-to-End Verification workflow | ADR |
| P0 | #842.5 | Architecture docs, command surface, AGENTS.md | P0 docs |
| P0 | #842.6 | Manual kind + baseline status correction | ADR |
| P1 | #842.7 | Stage API, internal contract, schema | P0 |
| P1 | #842.8 | Stage worker, queue, retention, gateway | Stage contract |
| P2 | #842.9 | Model stage vertical slice | Stage substrate |
| P3 | #842.10 | Strategy stage vertical slice | Model stage |
| P4 | #842.11 | Portfolio stage vertical slice | Strategy stage |
| P5 | #842.12 | Backtest promotion with frozen model artifact | Portfolio stage |
| P6 | #842.13 | Research ledger schema + manifest v3 | Promotion |
| P6 | #842.14 | Research CLI, GUI, comparisons, decisions | Ledger |
| P7 | #842.15 | Operations + end-to-end verification | Promotion + research |
| P8 | #842.16 | Manual, command/GUI reference, parity | All surfaces |
| P9 | #842.17 | Release and closure | All |

The detailed issue set, dependencies, labels, and board state are in the epic #842.

## 8. Verification

The commands below are the reviewer checklist. `Observed` and `Status` are filled during execution.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| P0-01 | ADR `docs/adr/2026-11-16/01-stage-execution-and-evidence-boundary.md` | ADR records decision, alternatives, consequences, and supersedes the `http-api.md` §8 prohibition | `python3 tools/check_links.py`; `python3 tools/check_doc_length.py` | ADR present, linked, within cap | — | Pending | — | — | — |
| P0-02 | Engine Execution workflow | Four stage runbooks, diagnostic rule, promote path; indexed in both indexes | `python3 tools/check_workflows.py` | Workflow ok | — | Pending | — | — | — |
| P0-03 | Backtest workflow | Eight stages, entry/exit gates, Run Contract link; indexed | `python3 tools/check_workflows.py` | Workflow ok | — | Pending | — | — | — |
| P0-04 | End-to-End Verification workflow | Clean-stack procedure; Release & Operations ownership; not a quant tier | `python3 tools/check_workflows.py` | Workflow ok | — | Pending | — | — | — |
| P0-05 | Architecture docs | `http-api.md` §8, `data-plane.md` §8, `services.md` §4, `module-map.md`, `client.md` agree | `check_links.py`; `check_client_docs.py`; `check_doc_length.py` | All pass | — | Pending | — | — | — |
| P0-06 | Command surface registry | New commands in `command-surface.md`; no retired command reintroduced | `python3 tools/check_command_surface.py` | Command surface ok | — | Pending | — | — | — |
| P0-07 | `AGENTS.md` | §2 command list, §3 command contract, §5 manual kind, §11 workflow selection | `check_claims.py`; `check_command_surface.py` | All pass | — | Pending | — | — | — |
| P0-08 | Workflow/global indexes | New workflows in both indexes | `python3 tools/check_workflows.py` | Workflow ok | — | Pending | — | — | — |
| P0-09 | `docs/manual/` kind | Kind listed in docs index and AGENTS.md; skeleton exists | `python3 tools/check_links.py` | Link ok | — | Pending | — | — | — |
| P0-10 | Baseline drift | `docs/README.md` improvement row agrees with `docs/improvement/baselines.md` | `rg "no baseline accepted" docs/README.md` | No contradiction | — | Pending | — | — | — |
| P0-11 | Human approval | Plan Approved; ADR accepted; workflows Current | Plan status and approval note | Approved | — | Pending | — | — | — |
| P1-01 | API stage types | Stage types compile and round-trip | `cargo test -p quantrule-api` | Pass | — | Pending | — | — | — |
| P1-02 | Internal stage contract | Stage manifest/record types | `cargo test -p quantrule-service-api` | Pass | — | Pending | — | — | — |
| P1-03 | Stage schema, Postgres | `stage.runs`, `stage.submissions`, `stage.artifacts` migrate | `cargo test -p quantrule-evidence-service` | Pass | — | Pending | — | — | — |
| P1-04 | Stage schema, SQLite | Same tables and constraints on SQLite | `cargo test -p quantrule-evidence-service --test sqlite_*` | Pass | — | Pending | — | — | — |
| P1-05 | Queue routing | Stage messages carry IDs only | `cargo test -p quantrule-runtime` | Pass | — | Pending | — | — | — |
| P1-06 | Stage lifecycle | Claim/computed/failed/cancelled; retries; terminal states | evidence + backtest tests | Pass | — | Pending | — | — | — |
| P1-07 | No partial publish | Killed worker leaves no visible artifact | worker test | No artifact | — | Pending | — | — | — |
| P1-08 | Retention | TTL deletes only stage artifacts | evidence test | Pass | — | Pending | — | — | — |
| P1-09 | Gateway proxy | `/stages/*`, `/research/*`; profile/token/path | `cargo test -p quantrule-gateway` | Pass | — | Pending | — | — | — |
| P1-10 | Profile scoping | Stage artifact hidden in another profile | evidence test | Pass | — | Pending | — | — | — |
| P1-11 | Stage manifest | `quantrule.stage-manifest/v1`; content-addressed | service-api test | Pass | — | Pending | — | — | — |
| P1-12 | No engine in shell | Shell stage modules contain no engine calls | `cargo test -p quantrule-shell --test architecture`; `check_structure.py` | Pass | — | Pending | — | — | — |
| P2-01 | Model stage compute | `stage run model` produces a model artifact | `cargo test -p quantrule-backtest-service stage::model`; `cargo test -p quantrule-shell --test http_commands stage_` | Model executor copies the bundle and shell submits a stage | backtest 6 tests and shell stage tests pass | Pass | AI (implementation) | 2026-11-17 | Model executor and printed output |
| P2-02 | Model artifact identity | Stage model artifact is the content-addressed model bundle | `cargo test -p quantrule-model-service --test bundle`; model executor test | Stage model files equal the model bundle artifacts | Manifest fields and bytes match | Pass | AI (implementation) | 2026-11-17 | Byte-for-byte copy, no recompute |
| P2-03 | Model CLI | `stage run model`, `show`, `events`, `fetch`, `list`, `cancel` | `cargo test -p quantrule-shell --test http_commands stage_` | All seven model-stage commands print their contract | Tests pass for run model/list/show/events/fetch/cancel/delete | Pass | AI (implementation) | 2026-11-17 | Includes error cases |
| P2-04 | Model GUI | Data + Model tabs; event browser; badge | `bun test`; `bunx playwright test e2e/model-stage.e2e.ts` | Workbench Data/Model tabs, stage flow, events, coverage | Bun 130 pass; Playwright 2 pass | Pass | AI (implementation) | 2026-11-17 | No chart library |
| P2-05 | Model parity | Controls and client docs updated | `bun test`; `cargo test -p quantrule-shell --test client_parity`; `python3 tools/check_client_docs.py` | Controls and client docs updated | Parity and client-doc checks pass | Pass | AI (implementation) | 2026-11-17 | Shared fixture updated |
| P2-06 | Model determinism | Same input/config/image → same artifact hash | `cargo test -p quantrule-backtest-service stage::model::tests::the_executor_is_deterministic`; `cargo test -p quantrule-model-service --test bundle` | Same input and bundle produce the same bytes | Deterministic tests pass | Pass | AI (implementation) | 2026-11-17 | Content-addressed copy |
| P2-07 | Model causality | Later bars do not change an earlier output | `cargo test -p quantrule-model` | Later bars do not change an earlier output | Model suite passes | Pass | AI (implementation) | 2026-11-17 | Model contract unchanged |
| P2-08 | Model evidence boundary | Stage model artifact rejected by qualification/baseline | `cargo test -p quantrule-evidence-service --test stages a_stage_is_profile_scoped_and_never_readable_as_a_run`; `cargo test -p quantrule-evidence-service --test stages a_stage_artifact_is_downloaded_by_its_recorded_hash` | A stage is not readable through a run route | Stage and run stores/routes remain separate | Pass | AI (implementation) | 2026-11-17 | Baseline/qualification surface is P6; the existing evidence path rejects the stage id |
| P2-09 | Model boundary | No strategy, fill, cash, P&L in model artifact | `cargo test -p quantrule-model-service --test bundle`; `cargo test -p quantrule-evidence-service --test stages` | No strategy, fill, cash, or P&L in the model artifact | Model bundle names only model artifacts; stage is diagnostic | Pass | AI (implementation) | 2026-11-17 | No strategy or portfolio executor |
| P3-01 | Strategy stage compute | `stage run strategy` produces decisions | `cargo test -p quantrule-backtest-service --test worker strategy_stage`; `cargo test -p quantrule-shell --test http_commands stage_run_strategy` | Executor runs the app strategy use case; shell submits the contract body | 7 worker strategy tests pass; shell prints `stage_run_id`, state, and created and posts the model-artifact input | Pass | AI (implementation) | 2026-11-17 | Unknown artifact → `stage.not_found`; `data_version` or `model_version` mismatch → `stage.invalid`; upstream provenance derived; strategy path resolved from `run.json` |
| P3-02 | Decisions bundle | `decisions.ndjson`, `summary.txt`, `provenance.json` | `cargo test -p quantrule-backtest-service --test worker strategy_stage::the_strategy_stage_publishes_only_decisions_and_provenance`; `cargo test -p quantrule-backtest-service --test worker strategy_stage::the_executor_uses_the_strategy_file_named_by_the_run_config` | Exactly three files, no execution fields | File names match; forbidden-word scan passes; a second run is byte-identical; a nested run-config strategy path is followed | Pass | AI (implementation) | 2026-11-17 | No fills, cash, P&L, exposure, or drawdown |
| P3-03 | Strategy explain | `stage explain` returns conditions/context | `cargo test -p quantrule-shell --test http_commands stage_run_strategy_and_explain`; `bun test src/lib/stage.test.ts` | Explain selects the published record and prints its fields | Shell prints bar, decision, side, reason, positions, and evidence; `--json` is one record; the browser selects bar 1 and refuses an absent bar | Pass | AI (implementation) | 2026-11-17 | No decision is recomputed |
| P3-04 | Strategy CLI/GUI | Commands and Strategy tab | `cargo test -p quantrule-shell --test http_commands`; `bun test`; `bunx playwright test e2e/strategy-stage.e2e.ts` | Commands and Strategy tab reach the use cases | Shell 43 tests pass; Bun 133 pass; the strategy Playwright flow passes and the full suite is 45 | Pass | AI (implementation) | 2026-11-17 | Submit, decisions table, detail, download, explain, cancel, delete, and no-P&L copy |
| P3-05 | Stage-vs-coupled divergence | Rejected-entry scenario produces different decisions | `cargo test -p quantrule-backtest-service --test worker a_rejected_coupled_entry_changes_later_decisions_but_the_stage_cannot_see_it` | Divergence documented and proven | Coupled `rejections.csv` carries `non_contiguous_bar`; standalone bar 3 is `hold`/long, coupled bar 3 is `enter`/flat; decision records carry no execution feedback | Pass | AI (implementation) | 2026-11-17 | Coupled behavior unchanged; stage never feeds a backtest |
| P3-06 | Strategy boundary | No fill, cash, P&L; not evidence | `cargo test -p quantrule-backtest-service --test worker strategy_stage`; `cargo test -p quantrule-evidence-service --test stages` | No execution value and no evidence status | Stage bundle is only the three decision files; the stage record is `evidence: false`; stage and run stores/routes remain separate | Pass | AI (implementation) | 2026-11-17 | `UnsupportedEngine` maps to a clear failure |
| P3-07 | Strategy parity | Controls and client docs updated | `cargo test -p quantrule-shell --test client_parity`; `bun test src/lib/clientParity.test.ts`; `python3 tools/check_client_docs.py` | Controls and client docs updated | Both clients build the same 36-command view; shell 29 requests and browser 27 agree with the fixture; client-doc checker passes | Pass | AI (implementation) | 2026-11-17 | Shared fixture carries strategy submit, detail, decisions, and explain |
| P4-01 | Portfolio stage compute | `stage run portfolio` runs execution + portfolio | backtest test | Pass | — | Pending | — | — | — |
| P4-02 | Fills/metrics bundle | Orders, fills, rejections, metrics, summary, provenance | app test | Pass | — | Pending | — | — | — |
| P4-03 | Portfolio CLI/GUI | Commands and Portfolio tab | `cargo test`; `bun test` | Pass | — | Pending | — | — | — |
| P4-04 | Portfolio invariants | Fees, slippage, funding, final equity, drawdown halt | backtest test | Pass | — | Pending | — | — | — |
| P4-05 | Portfolio boundary | Strategy decisions unchanged; not evidence | boundary tests | Pass | — | Pending | — | — | — |
| P4-06 | Portfolio parity | Controls and client docs updated | parity tests; `check_client_docs.py` | Pass | — | Pending | — | — | — |
| P5-01 | `model_artifact_id` contract | Submission accepts optional model artifact; old body valid | api/evidence tests | Pass | — | Pending | — | — | — |
| P5-02 | Model artifact validation | Kind, profile, config hash match | evidence test | Stable errors | — | Pending | — | — | — |
| P5-03 | Worker uses model artifact | No model recomputation | worker test | Pass | — | Pending | — | — | — |
| P5-04 | Provenance | Run records model artifact ID/hash | evidence test; `run status` fixture | Pass | — | Pending | — | — | — |
| P5-05 | Byte-identical model | Model files equal a full run | backtest test | Byte-identical | — | Pending | — | — | — |
| P5-06 | Canonical digest | Full run and promoted run share the digest | composed/fixture | Equal | — | Pending | — | — | — |
| P5-07 | Deterministic rerun | Same inputs → same digest | test/composed | Equal | — | Pending | — | — | — |
| P5-08 | Promote UX | CLI flag and GUI selector; recompute warning | `cargo test`; `bun test` | Pass | — | Pending | — | — | — |
| P5-09 | Mismatch error | Wrong model artifact → stable error | evidence test | `run.model_artifact_invalid` | — | Pending | — | — | — |
| P5-10 | Run Contract docs | Model reuse and provenance documented | `check_links.py`; human read | Pass | — | Pending | — | — | — |
| P6-01 | Research v3 manifest | `quantrule.improvement-run/v3`; validator | cargo test; validate tool | Pass | — | Pending | — | — | — |
| P6-02 | Research create | `research create` records a valid manifest | evidence test; CLI test | Pass | — | Pending | — | — | — |
| P6-03 | Research submit | Linked pipeline run created; record stores run/digest | integration test | Pass | — | Pending | — | — | — |
| P6-04 | Research compare | Candidate vs baseline table | cargo test; GUI fixture | Pass | — | Pending | — | — | — |
| P6-05 | Research decision | Human accept/reject/no-edge; baseline update | evidence test; CLI/GUI test | Pass | — | Pending | — | — | — |
| P6-06 | Baselines/invalidations | List/show; append-only | cargo test; `check_links` | Pass | — | Pending | — | — | — |
| P6-07 | Stage cannot be baseline | Server refuses stage artifact | evidence test | Stable error | — | Pending | — | — | — |
| P6-08 | Research GUI | List, record, compare, decision, baselines | `bun test`; Playwright | Pass | — | Pending | — | — | — |
| P6-09 | Human authority | Decision is explicit; no auto-accept | CLI/GUI test; docs | Pass | — | Pending | — | — | — |
| P7-01 | Operations GUI | Health, profiles, connection, verification, retention | `bun test`; fixture | Pass | — | Pending | — | — | — |
| P7-02 | End-to-end runbook | Clean-stack steps and digest comparison | `check_workflows.py`; tutorial run | Pass | — | Pending | — | — | — |
| P7-03 | Composed tier | Two equivalent runs → one digest | `tools/test_tiers.sh composed` | Pass | — | Pending | — | — | — |
| P7-04 | TTL isolation | Expired stage does not affect evidence | evidence test | Pass | — | Pending | — | — | — |
| P7-05 | Remote/token | Stage/research routes require profile and token | gateway/security test | Pass | — | Pending | — | — | — |
| P8-01 | Manual | 12 chapters present and linked | `check_links.py`; `check_doc_length.py` | Pass | — | Pending | — | — | — |
| P8-02 | Command reference | Every command, args, output, errors | `check_command_surface.py` | Pass | — | Pending | — | — | — |
| P8-03 | GUI reference | Every screen/control | `check_client_docs.py` | Pass | — | Pending | — | — | — |
| P8-04 | Tutorials | Stage, backtest, end-to-end invocations | `check_command_surface.py` | Pass | — | Pending | — | — | — |
| P8-05 | README/status | New command count and journey | command/client docs checkers | Pass | — | Pending | — | — | — |
| P8-06 | Changelog/AI log | Unreleased entry; AI transparency file | `check_changelog.py`; `check_links.py` | Pass | — | Pending | — | — | — |
| P8-07 | Manual checker | Manual is a discovery surface | `check_command_surface.py` | Pass | — | Pending | — | — | — |
| P8-08 | Parity suite | Every command has a browser control | `bun test`; shell parity | Pass | — | Pending | — | — | — |
| P8-09 | Structure/docs | Module docs, file size, doc caps, links, terms | `check_structure.py`; doc checkers | Pass | — | Pending | — | — | — |
| P9-01 | Full gate | `tools/gate.sh` | `tools/gate.sh` | Pass | — | Pending | — | — | — |
| P9-02 | Composed | `tools/test_tiers.sh composed` | composed | Pass | — | Pending | — | — | — |
| P9-03 | Parity | `tools/test_tiers.sh parity` | parity | Pass | — | Pending | — | — | — |
| P9-04 | Release | `v2.0.0` note, changelog, tag authorized | `check_releases.py`; human gate | Pass | — | Pending | — | — | — |
| P9-05 | Board/PRs | Parent/sub-issues, claim type, statuses, dates | GitHub Project | All Done | — | Pending | — | — | — |
| P9-06 | Cleanup | Worktree removed; local/remote branches deleted | `tools/worktree_cleanup.py check`; `git worktree list`; `git branch -a` | Clean | — | Pending | — | — | — |
| P9-07 | Plan verified | Every row Pass or Accepted risk | This table | Verified | — | Pending | — | — | — |

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| Stage artifact mistaken for evidence | `evidence: false`, badges, no qualification/baseline, tests |
| Strategy stage differs from coupled strategy | Recompute rule, UI/CLI warning, divergence test |
| Model reuse identity ambiguity | Record model artifact hash; require model config hash match; digest test |
| Queue starvation | Stage priority lower than evidence; quotas |
| TTL data loss | Evidence copies inputs; only diagnostic artifacts expire |
| Command/manual drift | Manual added to the discovery gate |
| GUI parity gap | `capabilities.ts`, parity tests, client-doc checker |
| Migration failure | Additive migrations; Postgres/SQLite parity; rollback by disabling routes |
| Scope too large | Vertical slices; model first; each phase independently usable |

## 10. Stop conditions

Stop and request human direction when:
- the evidence rule cannot be enforced without changing a model, strategy, or portfolio contract;
- the strategy stage cannot be isolated from position feedback without an engine change;
- the backtest worker cannot handle a stage kind without a new service;
- a migration breaks the desktop SQLite profile or an existing run record;
- the command-surface checker or a workflow gate cannot reconcile the new surface;
- a gate failure is outside the approved scope;
- a worktree or branch cannot be cleaned safely;
- the human rejects a workflow, ADR, command contract, or release.

## 11. Related documents

- [`docs/workflows/execution-plan.md`](../../workflows/execution-plan.md)
- [`docs/workflows/improvement-workflow.md`](../../workflows/improvement-workflow.md)
- [`docs/workflows/run-contract.md`](../../workflows/run-contract.md)
- [`docs/architecture/command-surface.md`](../../architecture/command-surface.md)
- [`docs/architecture/http-api.md`](../../architecture/http-api.md)
- [`AGENTS.md`](../../../AGENTS.md)
