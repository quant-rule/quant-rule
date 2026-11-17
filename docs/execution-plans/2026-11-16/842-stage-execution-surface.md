# Stage Execution Surface — Execution Plan

- Date: 2026-11-16 (UTC)
- Status: In Review
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
| Release | `v1.3.0` (authorized) | Human |

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
| P3-07 | Strategy parity | Controls and client docs updated | `cargo test -p quantrule-shell --test client_parity`; `bun test src/lib/clientParity.test.ts`; `python3 tools/check_client_docs.py` | Controls and client docs updated | Both clients built the 36-command view at #853; shell 29 requests and browser 27 agreed with the fixture; client-doc checker passed | Pass | AI (implementation) | 2026-11-17 | Shared fixture carries strategy submit, detail, decisions, and explain |
| P4-01 | Portfolio stage compute | `stage run portfolio` runs execution + portfolio | `cargo test -p quantrule-backtest-service --test worker portfolio_stage`; `cargo test -p quantrule-shell --test stage_portfolio_commands` | Executor runs execution then portfolio; shell submits the decisions input | 5 worker portfolio tests pass; shell prints the submission and fetch contract | Pass | AI (implementation) | 2026-11-17 | Strategy artifact lookup, derived data/model identity, dataset verification |
| P4-02 | Fills/metrics bundle | Orders, fills, rejections, metrics, summary, provenance | `cargo test -p quantrule-backtest-service --test worker portfolio_stage::the_portfolio_stage_publishes_execution_and_portfolio_bundles` | Prefixed `execution/` and `portfolio/` files | 12 prefixed files; second run is byte-identical | Pass | AI (implementation) | 2026-11-17 | No flat merge of the two `summary.txt`/`provenance.json` files |
| P4-03 | Portfolio CLI/GUI | Commands and Portfolio tab | `cargo test -p quantrule-shell`; `bun test`; `bunx playwright test e2e/portfolio-stage.e2e.ts` | Commands and Portfolio tab reach the use cases | Shell portfolio command passes; Bun passes; 2 portfolio Playwright flows pass | Pass | AI (implementation) | 2026-11-17 | Submit, tables, metrics, summary, nested download, cancel, delete |
| P4-04 | Portfolio invariants | Fees, slippage, funding, final equity, drawdown halt | `cargo test -p quantrule-backtest-service --test worker portfolio_stage`; `cargo test -p quantrule-backtest` | Metrics satisfy `final_equity = initial_cash + net_pnl`; a halt publishes a risk event | Published metrics and execution summary pass; tight drawdown publishes one risk event and `halted=true`; the existing ledger test blocks later entries | Pass | AI (implementation) | 2026-11-17 | Fees, slippage, funding, and final equity present; no metric recomputation |
| P4-05 | Portfolio boundary | Strategy decisions unchanged; not evidence | `cargo test -p quantrule-backtest-service --test worker portfolio_stage`; `cargo test -p quantrule-evidence-service --test stages` | No canonical digest or run row; stage cannot be a run | The executor returns only prefixed execution and portfolio files; stage/run stores and routes remain separate | Pass | AI (implementation) | 2026-11-17 | Baseline surface is deferred to the research ledger |
| P4-06 | Portfolio parity | Controls and client docs updated | `cargo test -p quantrule-shell --test client_parity`; `bun test src/lib/clientParity.test.ts`; `python3 tools/check_client_docs.py` | Controls and client docs updated | Both clients build the 37-command view and the portfolio detail; the client-doc checker passes | Pass | AI (implementation) | 2026-11-17 | Shared fixture carries the portfolio submission and detail |
| P5-01 | `model_artifact_id` contract | Submission accepts optional model artifact; old body valid | `cargo test -p quantrule-api`; `cargo test -p quantrule-evidence-service --test promoted_runs` | Old and new bodies deserialize; the promoted submission creates a run | Both bodies pass; promoted run is created; old full-run bodies remain valid | Pass | AI (implementation) | 2026-11-17 | Field is nullable and omitted when absent |
| P5-02 | Model artifact validation | Kind, profile, config hash match | `cargo test -p quantrule-evidence-service --test promoted_runs` | Stable `run.model_artifact_invalid` errors | Missing, wrong kind, another profile, dataset, config, and model version are rejected with the stable code | Pass | AI (implementation) | 2026-11-17 | Succeeded model stage only |
| P5-03 | Worker uses model artifact | No model recomputation | `cargo test -p quantrule-backtest-service --test worker promotion` | No model-service call; stage bytes loaded | `MemoryModel.calls()` stays zero; stage manifest and blobs are verified | Pass | AI (implementation) | 2026-11-17 | Missing/corrupt stage artifact fails or retries |
| P5-04 | Provenance | Run records model artifact ID/hash | `cargo test -p quantrule-evidence-service --test promoted_runs`; `cargo test -p quantrule-shell` | Detail and `run status` show the artifact id | Evidence detail carries the id; shell prints it or `none`; browser RunDetail shows it | Pass | AI (implementation) | 2026-11-17 | Top-level detail field, not provenance |
| P5-05 | Byte-identical model | Model files equal a full run | `cargo test -p quantrule-backtest-service --test worker promotion` | Published model files equal the stage bytes | `model/...` artifacts match the stage hashes and sizes | Pass | AI (implementation) | 2026-11-17 | No model recomputation |
| P5-06 | Canonical digest | Full run and promoted run share the digest | `cargo test -p quantrule-backtest-service --test worker promotion` | Equal canonical digest | The promoted run digest equals the coupled baseline digest | Pass | AI (implementation) | 2026-11-17 | Distinct run ids, same data digest |
| P5-07 | Deterministic rerun | Same inputs → same digest | `cargo test -p quantrule-backtest-service --test worker promotion`; `cargo test -p quantrule-evidence-service --test promoted_runs` | Existing promoted run returned | Resubmission returns the existing id; a processed rerun is skipped | Pass | AI (implementation) | 2026-11-17 | Full run id unchanged |
| P5-08 | Promote UX | CLI flag and GUI selector; recompute warning | `cargo test -p quantrule-shell`; `bunx playwright test e2e/configs.e2e.ts e2e/run-detail.e2e.ts` | Flag and optional field reach the contract | Shell sends the flag; Configs sends the field; Run detail shows the diagnostic reference | Pass | AI (implementation) | 2026-11-17 | No new command |
| P5-09 | Mismatch error | Wrong model artifact → stable error | `cargo test -p quantrule-evidence-service --test promoted_runs` | `run.model_artifact_invalid` | All invalid cases return the stable 400 code | Pass | AI (implementation) | 2026-11-17 | Includes profile mismatch |
| P5-10 | Run Contract docs | Model reuse and provenance documented | `python3 tools/check_links.py`; `python3 tools/check_doc_length.py` | Documents agree | Run Contract, Backtest, control-plane, stage/client/command docs updated | Pass | AI (implementation) | 2026-11-17 | Identity formula and recomputation rule recorded |
| P6-01 | Research v3 manifest | `quantrule.improvement-run/v3`; validator | `cargo test -p quantrule-api improvement` | Pass | The v3 manifest, round-trip, and rejection tests pass; v2 is refused; the frozen set is exact and `created_utc` matches the id date | Pass | AI (implementation) | 2026-11-17 | Gateway references only; unknown fields denied |
| P6-02 | Research create | `research create` records a valid manifest | `cargo test -p quantrule-evidence-service --test research_routes`; `cargo test -p quantrule-shell --test research_commands` | Pass | The server generates the next id and UTC timestamp, derives changed/frozen/holdout, rejects a model-only config and invalid references, and the CLI sends the gateway-reference body | Pass | AI (implementation) | 2026-11-17 | Model tier is a coupled full run |
| P6-03 | Research submit | Linked pipeline run created; record stores run/digest | `cargo test -p quantrule-evidence-service --test research_ledger` | Pass | The store links the pipeline run id first, records the digest once when available, and refuses a duplicate link or digest; an accepted decision needs the digest, while a rejected decision may omit it | Pass | AI (implementation) | 2026-11-17 | Store-level; the `research submit` command is #857 |
| P6-04 | Research compare | Candidate vs baseline table | `cargo test -p quantrule-evidence-service --test research_routes`; `bun test src/lib/research.test.ts` | Pass | Model comparison reads report coverage; strategy/portfolio reads portfolio metrics; historical baselines return null values with a note; failed/queued candidates conflict | Pass | AI (implementation) | 2026-11-17 | Published numbers only |
| P6-05 | Research decision | Human accept/reject/no-edge; baseline update | `cargo test -p quantrule-evidence-service --test research_routes`; `cargo test -p quantrule-evidence-service --test research_ledger`; `cargo test -p quantrule-shell --test research_commands` | Pass | Accepted appends a baseline in the decision transaction; invalidated appends a tombstone; duplicate decision or registry row rolls back; accepted without digest conflicts; rejected failed run succeeds | Pass | AI (implementation) | 2026-11-17 | Explicit human action only |
| P6-06 | Baselines/invalidations | List/show; append-only | `cargo test -p quantrule-evidence-service --test research_ledger`; `python3 tools/check_links.py` | Pass | Three model baselines and eight tombstones are seeded for `default`; list is profile-scoped; append methods refuse duplicate ids and SQLite and Postgres refuse update/delete | Pass | AI (implementation) | 2026-11-17 | Seeded from the committed registry |
| P6-07 | Stage cannot be baseline | Server refuses stage artifact | `cargo test -p quantrule-evidence-service --test research_routes a_stage_artifact_is_not_a_pipeline_run_for_acceptance` | Stable error | A stage artifact id is not found in the run store; an accepted decision returns `research.conflict` and writes no baseline | Pass | AI (implementation) | 2026-11-17 | Run store is the only pipeline link |
| P6-08 | Research GUI | List, record, compare, decision, baselines | `bun test`; `bunx playwright test e2e/research.e2e.ts` | Pass | The Research screen has all nine controls and test ids; unit and Playwright flows cover create, submit, compare, decision, baseline, and invalidation | Pass | AI (implementation) | 2026-11-17 | No auto-submit or auto-accept |
| P6-09 | Human authority | Decision is explicit; no auto-accept | `cargo test -p quantrule-shell --test research_commands`; `bunx playwright test e2e/research.e2e.ts`; docs | Pass | The decision is a separate explicit command/control with a note; accepted requires a succeeded run and digest; nothing auto-submits or auto-accepts | Pass | AI (implementation) | 2026-11-17 | The human owns the decision |
| P7-01 | Operations GUI | Health, profiles, connection, verification, retention | `bun test`; Playwright | Pass | The Operations screen carries `operations-health`, `operations-profile`, `operations-retention`, and `operations-verify`; 4 unit tests pass; 2 Playwright flows pass; the full browser suite is 53 pass | Pass | AI (implementation) | 2026-11-17 | Profile/token presence is read-only; Connection owns the token controls |
| P7-02 | End-to-end runbook | Clean-stack steps and digest comparison | `check_workflows.py`; `check_links.py`; tutorial | Pass | `docs/tutorials/2026-11-17/end-to-end-verification.md` records `make up`, `make verify`, the recorded digest, `tools/test_tiers.sh composed`, and failure actions; the workflow and docs index link it; both checkers pass | Pass | AI (implementation) | 2026-11-17 | 178 lines, within the tutorial cap |
| P7-03 | Composed tier | Two equivalent runs → one digest | `tools/test_tiers.sh composed` | Pass | `make up` printed `QuantRule v1.2.0 ready`; `make verify` printed `canonical digest sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa`; the composed tier printed `composed: gateway healthy, product 1.2.0, run parity verified` | Pass | AI (implementation) | 2026-11-17 | Two runs, 12 canonical artifacts and 3,559,171 bytes each |
| P7-04 | TTL isolation | Expired stage does not affect evidence | evidence test | Pass | `cargo test -p quantrule-evidence-service --test stage_retention` is 6 pass, including a run that survives expiry; `--test operations` is 5 pass for no cleanup, success, failure, and no stage store | Pass | AI (implementation) | 2026-11-17 | Only stage rows and stage objects are named |
| P7-05 | Operations remote/token | Operations route requires profile and token | `cargo test -p quantrule-gateway --test evidence_proxy --test surface` | Pass | `/operations` is forwarded with the injected profile; traversal is refused before upstream; a missing profile is `profile.not_found`; a gated call without the token is `auth.unauthorized` | Pass | AI (implementation) | 2026-11-17 | The gateway holds no operations state |
| P8-01 | Manual | 12 chapters present and linked | `check_links.py`; `check_doc_length.py`; `check_manual.py` | Pass | 12 chapters exist, are linked, and carry a status; chapters 1–8 are 86–119 lines; `check_manual.py` reports 12 chapters, 46 commands, 13 screens, 4 tabs; links and doc length pass | Pass | AI (implementation) | 2026-11-17 | Reference chapters 9–12 are tables, not padded procedures |
| P8-02 | Command reference | Every command, args, output, errors | `check_command_surface.py`; `check_manual.py` | Pass | Chapter 9 has one row for each of the 46 implemented commands; `check_command_surface.py` reports 46 implemented, 0 planned | Pass | AI (implementation) | 2026-11-17 | Rows are checked against `help.rs`, not copied blindly |
| P8-03 | GUI reference | Every screen/control | `check_client_docs.py`; `check_manual.py` | Pass | Chapter 10 covers all 13 client screens and all 4 Workbench tabs with controls, test ids, badges, and mirrored commands; the client-doc checker passes | Pass | AI (implementation) | 2026-11-17 | Test ids match `capabilities.ts` and the components |
| P8-04 | Tutorials | Stage, backtest, end-to-end invocations | `check_command_surface.py` | Pass | `stage-to-backtest.md` is 150 lines and invokes only implemented commands; the checker sees 17 tutorial files and every implemented command still has a tutorial invocation | Pass | AI (implementation) | 2026-11-17 | Links to the shell and end-to-end tutorials instead of duplicating them |
| P8-05 | README/status | New command count and journey | `check_command_surface.py`; `check_client_docs.py` | Pass | The root README names the user manual and the new tutorial; the docs index shows the manual Current with 12 chapters and lists the new tutorial; both checkers pass | Pass | AI (implementation) | 2026-11-17 | Count stays 46 commands |
| P8-06 | Changelog/AI log | Unreleased entry; AI transparency file | `check_changelog.py`; `check_links.py` | Pass | `CHANGELOG.md` has one Unreleased Added entry for the manual, checker, and tutorial; `docs/ai-transparency-log/2026-11-17/859-manual.md` exists and links resolve | Pass | AI (implementation) | 2026-11-17 | No release or tag work |
| P8-07 | Manual checker | Manual is a discovery surface | `check_manual.py`; `test_check_manual.py` | Pass | The checker requires linked chapters, status lines, all 46 command rows, all screens and tabs, the six section headings, and an evidence label; 9 tests cover the pass, missing file, missing command, usage mismatch, missing screen, missing section, invalid label, and missing tab | Pass | AI (implementation) | 2026-11-17 | Added to `tools/checkers.txt` and `tools/checker_tests.txt` |
| P8-08 | Parity suite | Every command has a browser control | `bun test`; shell parity | Pass | Browser suite is 146 pass; Playwright is 53 pass; every one of the 46 commands still has a browser control in the shared parity fixture | Pass | AI (implementation) | 2026-11-17 | No command or screen was removed |
| P8-09 | Structure/docs | Module docs, file size, doc caps, links, terms | `check_structure.py`; doc checkers | Pass | `check_structure.py`, `check_doc_length.py`, `check_terms.py`, `check_file_size.py`, and `check_links.py` pass; the checker gate runs 25 checkers | Pass | AI (implementation) | 2026-11-17 | The manual is a living document, not a historical record |
| P9-01 | Full gate | `tools/gate.sh` | `tools/gate.sh` | Pass | `workspace gate passed`; 2,321 passed, 0 failed, 16 ignored; 25 checkers | Pass | AI (implementation) | 2026-11-17 | Run on the v1.3.0 release commit |
| P9-02 | Composed | `tools/test_tiers.sh composed` | composed | Pass | `make up` printed `QuantRule v1.3.0 ready`; `make verify` printed the unchanged `sha256:453c90b4…` digest; each run verified 12 artifacts and 3,559,171 bytes; composed printed `composed: gateway healthy, product 1.3.0, run parity verified` | Pass | AI (implementation) | 2026-11-17 | Stack stopped with `make down` |
| P9-03 | Parity | `tools/test_tiers.sh parity` | parity | Pass | `port parity ok: 1 registered port(s)` | Pass | AI (implementation) | 2026-11-17 | Registry check passed |
| P9-04 | Release | `v1.3.0` note, changelog, tag authorized | `check_releases.py --note v1.3.0`; human gate | Pass | Human authorized v1.3.0; the v1.3.0 note, changelog section, current-tag map, and release check resolve to `docs/releases/2026-11-17/v1.3.0.md`; the tag is pending the tag run | Pass | AI (implementation) | 2026-11-17 | Tag creation remains the human-owned release action |
| P9-05 | Board/PRs | Parent/sub-issues, claim type, statuses, dates | GitHub Project | All Done | PR open, merge and board closure pending | Pending | AI (implementation) | 2026-11-17 | Human merges and closes the board items |
| P9-06 | Cleanup | Worktree removed; local/remote branches deleted | `tools/worktree_cleanup.py check`; `git worktree list`; `git branch -a` | Clean | Worktree and branches remain until merge | Pending | AI (implementation) | 2026-11-17 | Post-merge cleanup |
| P9-07 | Plan verified | Every row Pass or Accepted risk | This table | Verified | Not yet; P9-05, P9-06, the tag run, and human acceptance remain | Pending | AI (implementation) | 2026-11-17 | Do not mark Verified before the tag and cleanup |

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
