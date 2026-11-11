# QuantRule Documentation

Work-item documents use `docs/<kind>/<YYYY-MM-DD>/<file>.md`: the kind comes first, then the UTC date of the work item, then the file. Living system documents sit directly under their kind, as `docs/architecture/<file>.md`. A kind directory appears when it is first used.

## Kinds

| Kind | Holds | Path |
|---|---|---|
| Execution plans | The plan for a substantial work item, per `AGENTS.md` section 11. | `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md` |
| AI transparency log | The record of material AI contributions for a work item, per `AGENTS.md` section 6. | `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md` |
| Agent roles | The quant and engineering roles, their handoffs, and when each is required. | `docs/roles/<YYYY-MM-DD>/<file>.md` |
| Architecture | The living system documentation: overview, product kata, architecture characteristics, style/quantum, and cross-cutting views. | `docs/architecture/<file>.md` |
| Course deliverables | Submission sources and generated PDFs for the Software Architecture and Development Methodologies courses. | `docs/course/{architecture,methodology}/{src,deliverables}` |
| Model | The analytical model's contract and evidence. The implementation follows it. | `docs/model/<YYYY-MM-DD>/<file>.md` |
| Configuration | The engine JSON configuration: files, fields, validation, and cross-config rules. | `docs/config/<YYYY-MM-DD>/<file>.md` |
| Reports | Historical report bundles and verification records. | `docs/reports/<YYYY-MM-DD>/<file>.md` |
| Releases | The release notes, the release index, and the tag rule. | `docs/releases/<YYYY-MM-DD>/<tag>.md`; `docs/releases/README.md` |
| Architecture decision records | A decision, the alternatives considered, and the consequences. | `docs/adr/<YYYY-MM-DD>/<file>.md` |
| Audits | Read-only findings, evidence, and remediation options. | `docs/audits/<YYYY-MM-DD>/<scope>-audit.md` |
| Refactors | Read-only structural findings, evidence, and refactor options. | `docs/refactors/<YYYY-MM-DD>/<scope>-refactor.md` |
| Governance | The improvement charter, green criteria, autonomy, and stop conditions. | `docs/governance/<YYYY-MM-DD>/<file>.md` |
| Workflows | The living workflow contracts, indexed by `docs/workflows/README.md`. | `docs/workflows/<file>.md` |
| Workflow templates | The skeleton for a new workflow and the data, release, deprecation, and requirements records. | `docs/workflows/templates/<file>.md` |
| Improvement registry | The committed run index, baselines, invalidation tombstones, and run record template. | `docs/improvement/<file>.md` |
| Changelog | The grouped release history: meaningful user-facing or developer-facing changes by release and type. | `CHANGELOG.md` |

The tutorial kind is defined in `AGENTS.md`; the current M1.0 lab tutorial is
linked below.

## Current status

This section is the single status index. Other documents link here instead of
repeating a status.

| Area | Status |
|---|---|
| Pipeline | Every stage is implemented: data, model, strategy, execution, portfolio, backtest, qualification, analytics. |
| Command surface | 15 implemented commands, 0 planned; the discovery gate passes. |
| Model | `M1.3` current; 49 indicators, 150 active events, 199 validation claims; accepted compression baselines; `M2.x` no edge; `M3.x` underpowered; `M4.x` no edge; `MR5.x` no edge; `MR6.x` no edge; `MR7.x` no edge. |
| Strategy | No accepted strategy baseline; the tier is gated by an accepted model baseline. |
| Portfolio | No accepted portfolio baseline; the tier is gated by an accepted strategy baseline. |
| Data partitions | `model_dev` 2018–2020, `model_validation` 2021–2022, `strategy_dev` 2018–2022, `strategy_validation` 2024, `portfolio_dev` 2024, `portfolio_holdout` 2025; 2023 excluded. A dense Binance BTCUSDT cross-venue source exists outside the partitions. |
| Improvement workflow | Current; the S3 surface is implemented; the SR1/SR2 strategy runs, the SR3 feature programme, the SR4 multi-instrument fade, the SR5 absorption count gate, the SR6 VWAP rejection run, the SR7 BTC+ETH H3 cross-section, the SR8 2023 buffer test, the SR8-A 2023 archive data rejection, the SR9 15m structure campaign, and the Binance BTCUSDT cross-venue check are recorded; no baseline accepted; human acceptance pending. |
| Run contract | Current; `quantrule.improvement-run/v2` is current, v1 is historical. |
| Research swarm | Current contract; no active campaign. |
| Release tags | `v0.9.0` is the latest release, the first Tauri Platform release; `v0.8.0` is the last pre-Tauri release; historical tags are retained. |
| Changelog | Grouped release history; `0.8.0` is the current release; the SR2–SR13 entries and research tooling are in its section. |
| Tooling | The current shell commands and `tools/improvement/run_current.py` are the execution path; the historical research scripts and the old improvement adapter are retained for reproducibility. |

## Current architecture

| Document | Status | Path |
|---|---|---|
| System architecture entry point | Current | `docs/architecture/README.md` |
| The Trustworthy Backtest Kata | Current | `docs/architecture/kata.md` |
| Architecture characteristics | Current | `docs/architecture/characteristics.md` |
| Command surface | Current; implemented/planned stage contract and drift gate | `docs/architecture/command-surface.md` |
| S3.x command-surface plan | Approved 2026-11-06; all phases merged; human acceptance pending | `docs/execution-plans/2026-11-06/s3-command-surface.md` |

## Current course deliverables

| Deliverable | Status | Path |
|---|---|---|
| Course folder index | Current; final for the Tauri Platform | `docs/course/README.md` |
| Architecture kata | Final | `docs/course/architecture/src/01-kata.md` |
| Architecture characteristics | Final | `docs/course/architecture/src/02-architecture-characteristics.md` |
| Architecture description | Final | `docs/course/architecture/src/03-architecture-description.md` |
| Methodology overview and checkpoints A–E | Final | `docs/course/methodology/src/` |
| PDF build | Current | `tools/course/build_pdfs.sh` |

## Current model

| Contract | Status | Path |
|---|---|---|
| M1.0 flat model | Historical baseline; path gate no edge; compression baselines accepted | `docs/model/2026-11-05/model-m1.0.md` |
| M1.1 opening-range sweep | Historical model; adds the opening-range sweep and control events; MR7 no edge | `docs/model/2026-11-08/model-m1.1.md` |
| M1.2 structure extensions | Historical model; pivot sequence, Fibonacci retracements, and horizontal bands; corrected by M1.3 | `docs/model/2026-11-10/model-m1.2.md` |
| M1.3 audit corrections | Current model; corrected sequence occurrence, Fibonacci tests, and fixed tolerances; predictive evidence pending | `docs/model/2026-11-10/model-m1.3.md` |
| Indicator coverage | Implemented 2026-11-05; updated 2026-11-10: 49 indicators, 150 active events, stable event codes; 199 validation claims; predictive evidence pending | `docs/model/2026-11-05/indicator-coverage.md` |
| Model families | Approved 2026-11-05; amended 2026-11-10; `M1.3` current, `M1.0`–`M1.2` historical, `M2.x`–`M4.x` historical, `MR5.x`–`MR7.x` research, and the future `S3.x` map | `docs/model/2026-11-05/model-families.md` |
| Model spec v1 | Implemented; condition-only; includes the `rvol_regime`, `vol_regime`, and `structure_state` states | `docs/model/2026-11-05/model-spec-v1.md` |
| `rvol_regime` context state | Current; low below 1.4, normal 1.4–2.0, high at or above 2.0; no event changes | `docs/model/2026-11-08/rvol-regime.md` |
| `vol_regime` context state | Current; compressed at or below 0.7, normal below 1.3, expanded at or above 1.3; no event changes | `docs/model/2026-11-09/vol-regime.md` |
| `structure_state` context state | Current; confirmed major HH/HL, LH/LL, or mixed; no event changes | `docs/model/2026-11-09/structure-state.md` |
| `signal.breakout_retest_hold` | Current; compression breakout plus a 4-bar retest within 0.2 ATR that closes back on the breakout side | `docs/model/2026-11-09/breakout-retest-hold.md` |
| Model spec v2 | Implemented; reading overrides, active events, causal vote, panel/session | `docs/model/2026-11-05/model-spec-v2.md` |
| Indicator and parameter registry | Current; CI check | `docs/registry/README.md` |
| Current-architecture history index | Current; retired paths and their recovery commit | `docs/registry/history.md` |
| M1.0 model lab tutorial | Current for the base surface; M1.1 adds the opening-range sweep pair | `docs/tutorials/2026-11-06/m1.0-model-lab.md` |
| Workflow lab tutorial | Current; workflow selection, stage commands, and the current run adapter | `docs/tutorials/2026-11-07/workflow-lab.md` |
| Strategy lab tutorial | Current; `report model`, `strategy explain`, `strategy run` | `docs/tutorials/2026-11-06/strategy-lab.md` |
| Execution lab tutorial | Current; `report model`, `strategy run`, `execution run` | `docs/tutorials/2026-11-06/execution-lab.md` |
| Portfolio lab tutorial | Current; `report model`, `strategy run`, `execution run`, `portfolio replay` | `docs/tutorials/2026-11-06/portfolio-lab.md` |
| Backtest lab tutorial | Current; the coupled `backtest run` and its stage bundles | `docs/tutorials/2026-11-06/backtest-lab.md` |
| Qualification lab tutorial | Current; `qualify`, the criteria verdicts, and the human acceptance boundary | `docs/tutorials/2026-11-06/qualification-lab.md` |
| Analytics lab tutorial | Current; `analytics evaluate`, the descriptive metrics, and the evaluation tables | `docs/tutorials/2026-11-06/analytics-lab.md` |
| Pipeline lab tutorial | Current; every stage independently, the coupled run, qualification, analytics, and determinism | `docs/tutorials/2026-11-06/pipeline-lab.md` |
| Trade context lab tutorial | Current; 250-bar entry/exit context, annotations, and exit-quality evaluation | `docs/tutorials/2026-11-09/trade-context.md` |
| Chart annotator tutorial | Current; local click-to-annotate page for entry/exit marks | `docs/tutorials/2026-11-09/chart-annotator.md` |
| Indicator verification matrix | Current; 46 rows, generated from the evidence map | `docs/model/2026-11-06/indicator-verification.md` |
| Event verification matrix | Current; 143 rows, generated from the evidence map | `docs/model/2026-11-06/event-verification.md` |
| Verification evidence map | Current; row/family/tool/n-a scope per class | `docs/registry/verification.json` |

## Current run contracts

| Contract | Status | Path |
|---|---|---|
| Run v3 and portfolio constraints | Current; `run v3`, data schema 3, model schema 4, `execution v2`, `portfolio v2` | `docs/config/2026-11-11/run-v3-and-portfolio-constraints.md` |
| Local research memory | Current; `quantrule.research-memory/v1`, export v1, backup/restore | `docs/config/2026-11-11/research-memory-v1.md` |
| Desktop UI | Current; Tauri 2 + Svelte 5 over the application service | `apps/desktop/README.md` |
| Linux distribution | Current; AppImage/deb, checksums, static page | `site/README.md` |
| Policy blueprint | Historical; superseded by run v3 | `docs/config/2026-10-30/policy-and-strategy.md` |
| Config tree | Current; current, evidence, and historical directories | `config/README.md` |
| Available timeframes | `quantrule.data/v3` | `docs/config/2026-10-30/data-v3.md` |
| Session coverage | Current; optional weekly calendar and session-aware coverage | `docs/config/2026-11-09/data-change-SESSION-COVERAGE-01.md` |
| MACD reading and events | Current | `docs/config/2026-11-03/model-macd.md` |
| Engine artifacts | Current; stage bundle, provenance, and handoff contract | `docs/config/2026-11-06/engine-artifacts.md` |
| Strategy configuration | Current; `quantrule.strategy/v1`, both generic engines, and the decision record | `docs/config/2026-11-06/strategy-config.md` |
| Qualification criteria | Current; human-owned criteria, verdict semantics, and human acceptance | `docs/config/2026-11-06/qualification-criteria.md` |
| Analytics benchmarks | Current; buy-and-hold and matched random-timing evaluation output | `docs/config/2026-11-09/analytics-benchmarks.md` |
| Trade context | Current; per-trade 250-bar entry/exit context, annotation mapping, and exit-quality table | `docs/config/2026-11-09/trade-context.md` |
| Strategy position excursions | Current; entry price/ATR, unrealized, MFE/MAE, giveback exit | `docs/config/2026-11-10/strategy-position-excursions.md` |
| Improvement run contract | Current; `quantrule.improvement-run/v2` is current, v1 is historical | `docs/workflows/run-contract.md` |

The current run does not reference `strategy.json` or `analytics.json`. The loader
rejects them with an unknown-field error. The strategy file and the qualification
criteria are explicit command arguments; the run schema stays
`quantrule.run/v3`.

## Current evidence

| Evidence | Status | Path |
|---|---|---|
| M1.0 model path gate | Development gate no edge; validation not opened | [`docs/improvement/2026-11-04/MODEL-V4-GATE.md`](improvement/2026-11-04/MODEL-V4-GATE.md) |
| Compression-breakout 4 ATR baseline | Accepted model baseline (human, 2026-11-04) | [`docs/improvement/2026-11-04/COMPRESSION-BASELINE-CANDIDATE.md`](improvement/2026-11-04/COMPRESSION-BASELINE-CANDIDATE.md) |
| Compression-range next-bar resolution | Accepted model baseline (human, 2026-11-04) | [`docs/improvement/2026-11-04/COMPRESSION-RANGE-CANDIDATE.md`](improvement/2026-11-04/COMPRESSION-RANGE-CANDIDATE.md) |
| M2.x model programme | P5 `no_edge`; P7 closeout complete; validation not opened | [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](execution-plans/2026-11-05/410-m2-model-programme.md) |
| M3.x model programme | Preflight `underpowered`; no development run; validation not opened | [`docs/improvement/2026-11-05/M3-PREFLIGHT.md`](improvement/2026-11-05/M3-PREFLIGHT.md) |
| M4.x model programme | Development `no_edge`; five primary cells; validation not opened | [`docs/improvement/2026-11-07/M4-DEV-SWARM.md`](improvement/2026-11-07/M4-DEV-SWARM.md) |
| M4.x model charter | Approved 2026-11-07; strategy tier closed | [`docs/governance/2026-11-07/M4-CHARTER.md`](governance/2026-11-07/M4-CHARTER.md) |
| MR5.x platform | `MR`/`-Cnn` naming, per-cell model config, sequence/confluence/path evaluators, reference grid | [`docs/execution-plans/2026-11-08/mr5-platform.md`](execution-plans/2026-11-08/mr5-platform.md) |
| MR5.x model programme | Development `no_edge`; 30 cells; no primary passed; validation not opened | [`docs/improvement/2026-11-08/MR5-DEV-SWARM.md`](improvement/2026-11-08/MR5-DEV-SWARM.md) |
| MR5.x power worksheet | Five primary base cells recommended; ten needs fresh data | [`docs/improvement/2026-11-08/MR5-POWER.md`](improvement/2026-11-08/MR5-POWER.md) |
| MR5.x data review | Validation reuse disclosed; ETH optional; fresh instrument via Data Lifecycle | [`docs/improvement/2026-11-08/MR5-DATA-REVIEW.md`](improvement/2026-11-08/MR5-DATA-REVIEW.md) |
| MR5.x charter | Approved; 10 × 3 grid; five primary base cells; development no edge | [`docs/governance/2026-11-08/MR5-CHARTER.md`](governance/2026-11-08/MR5-CHARTER.md) |
| MR6.x model programme | Development `no_edge`; 15 cells; no primary passed; reverse controls did not pass; validation not opened | [`docs/improvement/2026-11-08/MR6-DEV-SWARM.md`](improvement/2026-11-08/MR6-DEV-SWARM.md) |
| MR6.x power worksheet | Five primary base cells; family-level pairing; count gate first | [`docs/improvement/2026-11-08/MR6-POWER.md`](improvement/2026-11-08/MR6-POWER.md) |
| MR6.x data review | Relative-volume participation admitted; cumulative OBV/AD excluded; ETH included | [`docs/improvement/2026-11-08/MR6-DATA-REVIEW.md`](improvement/2026-11-08/MR6-DATA-REVIEW.md) |
| MR6.x charter | Approved; 5 × 3 grid; five primary base cells; reverse-order controls | [`docs/governance/2026-11-08/MR6-CHARTER.md`](governance/2026-11-08/MR6-CHARTER.md) |
| MR7.x model programme | Development `no_edge`; 15 cells; control positive but not reversing; validation not opened | [`docs/improvement/2026-11-08/MR7-DEV-SWARM.md`](improvement/2026-11-08/MR7-DEV-SWARM.md) |
| MR7.x charter | Approved; 5 × 3 grid; five primary base cells; no-reclaim control; path gate | [`docs/governance/2026-11-08/MR7-CHARTER.md`](governance/2026-11-08/MR7-CHARTER.md) |
| MR7.x data review | COPPERCMDUSD and TSLAUSUSD price-only; volume excluded | [`docs/improvement/2026-11-08/MR7-DATA-REVIEW.md`](improvement/2026-11-08/MR7-DATA-REVIEW.md) |
| M2.x model charter | Approved 2026-11-05; strategy tier closed | [`docs/governance/2026-11-05/M2-CHARTER.md`](governance/2026-11-05/M2-CHARTER.md) |
| Current-surface audit verification | Merged in PR #496; per-row indicator/event matrix pending | [`docs/reports/2026-11-06/current-surface-audit-verification.md`](reports/2026-11-06/current-surface-audit-verification.md) |
| Indicator/event verification evidence | Scope map: 33 row-level, 1410 family-level, 332 tool gates, 304 N/A | [`docs/reports/2026-11-06/indicator-event-verification.md`](reports/2026-11-06/indicator-event-verification.md) |
| Workflow documentation reset verification | Released in `v0.6.0`; verification record current | [`docs/reports/2026-11-07/workflow-verification.md`](reports/2026-11-07/workflow-verification.md) |
| 2018–2019 annotation zone analysis | Descriptive; EMA/Fibonacci zone profile and 2018 vs 2019 comparison | [`docs/reports/2026-11-10/annotation-zone-analysis.md`](reports/2026-11-10/annotation-zone-analysis.md) |
| Simulation Batch 1 | SIM-01 no pattern, SIM-02 no pattern, SIM-03 no exit rescue; entry-first conclusion; no edge claim | [`docs/reports/2026-11-11/simulation-batch-1.md`](reports/2026-11-11/simulation-batch-1.md) |
| Simulation Batch 2 | SIM-04/05/06 no pattern; six runs complete, five no_pattern and one no_exit_rescue; no edge claim | [`docs/reports/2026-11-11/simulation-batch-2.md`](reports/2026-11-11/simulation-batch-2.md) |
| Simulation Batch 3 | SIM-07 no pattern, SIM-08 bullish divergence candidate screen, SIM-09 underpowered, SIM-10 blocked | [`docs/reports/2026-11-11/simulation-batch-3.md`](reports/2026-11-11/simulation-batch-3.md) |
| Simulation data loss audit | Local store lost in worktree cleanup; finding `Verified`; residual nested-target false positive | [`docs/audits/2026-11-11/simulation-data-loss-audit.md`](audits/2026-11-11/simulation-data-loss-audit.md) |
| Simulation store recovery | Store regenerated at `/home/zantgo/quant-rule-data/simulation`; committed numbers reproduced | [`docs/reports/2026-11-11/simulation-recovery.md`](reports/2026-11-11/simulation-recovery.md) |
| Tauri Platform implementation verification | Phase 3 gate pass; P0–P3E; clean deb, offline, determinism, memory | [`docs/reports/2026-11-11/implementation-verification.md`](reports/2026-11-11/implementation-verification.md) |
| Desktop 0.9.0 release readiness | Ready for human acceptance; tag pending; gate, artifacts, and risks recorded | [`docs/reports/2026-11-11/release-readiness.md`](reports/2026-11-11/release-readiness.md) |
| Simulation campaign synthesis | Closed — no edge; ten runs; one candidate screen retired under the variant rule | [`docs/reports/2026-11-11/simulation-synthesis.md`](reports/2026-11-11/simulation-synthesis.md) |
| SIM08-BULL-DIV hypothesis | Retired; variant of the failed divergence-at-pivot programme; no edge claim | [`docs/reports/2026-11-11/sim08-bull-divergence-hypothesis.md`](reports/2026-11-11/sim08-bull-divergence-hypothesis.md) |
| SR10 EMA/Fibonacci exit study | Stopped early; 2018 primary failed; no candidate; 2024 closed | [`docs/improvement/2026-11-10/SR10.md`](improvement/2026-11-10/SR10.md) |
| SR11 trend-aligned EMA/Fibonacci entry | Completed; no candidate; EMA/Fibonacci line stopped; 2024 closed | [`docs/improvement/2026-11-10/SR11.md`](improvement/2026-11-10/SR11.md) |
| SR11 raw-edge diagnostic | No positive raw edge; entry flat to negative before costs; controls not beaten | [`docs/reports/2026-11-10/sr11-raw-edge.md`](reports/2026-11-10/sr11-raw-edge.md) |
| SR12 failed break/reclaim and band fade | H1 engine-blocked; H2 no candidate; sizing confirms no edge; 2024 closed | [`docs/improvement/2026-11-10/SR12.md`](improvement/2026-11-10/SR12.md) |

Pre-reset experiment records under `docs/improvement/` are historical. They are not
improvement runs under the current run contract and are not imported as baselines.

## Current workflows

The workflow index, selection guide, status vocabulary, and record map are in
[`docs/workflows/README.md`](workflows/README.md). The index is the single source
of workflow status; this page does not repeat it.

| Workflow | Path |
|---|---|
| Audit | `docs/workflows/audit-workflow.md` |
| Refactor | `docs/workflows/refactor-workflow.md` |
| Improvement | `docs/workflows/improvement-workflow.md` |
| Run contract | `docs/workflows/run-contract.md` |
| Research swarm | `docs/workflows/research-swarm.md` |
| Model research | `docs/workflows/model-research.md` |
| Model change | `docs/workflows/model-change.md` |
| Simulation research | `docs/workflows/simulation-research.md` |
| Strategy research | `docs/workflows/strategy-research.md` |
| Portfolio research | `docs/workflows/portfolio-research.md` |
| Policy change (historical) | `docs/workflows/policy-change.md` |
| Data lifecycle | `docs/workflows/data-lifecycle.md` |
| Requirements and design | `docs/workflows/requirements-design.md` |
| Execution plan | `docs/workflows/execution-plan.md` |
| Release and operations | `docs/workflows/release-operations.md` |
| Deprecation and retirement | `docs/workflows/deprecation-retirement.md` |
| Recordkeeping | `docs/workflows/recordkeeping.md` |

## Current roles

| Role | Manual | Status |
|---|---|---|
| Quant Researcher | [Quant Researcher Manual](roles/2026-10-29/quant-researcher-manual.md) | Historical pre-M1.0 manual; the role definition in `AGENTS.md` section 10 remains normative |
| Quant Developer | [Quant Developer Manual](roles/2026-10-29/quant-developer-manual.md) | Historical pre-M1.0 manual; the role definition in `AGENTS.md` section 10 remains normative |
| Quant Validator | [Quant Validator Manual](roles/2026-10-29/quant-validator-manual.md) | Historical pre-M1.0 manual; the role definition in `AGENTS.md` section 10 remains normative |
| Statistical Methodologist | `AGENTS.md` section 10 (Quant Researcher specialization) | Current; MR7 power/MDE and multiple-testing worksheet |
| All three | [Quant roles overview](roles/2026-10-29/quant-roles.md) | Current role map and handoff chain |
| Improvement Workflow | [Improvement Workflow roles](roles/2026-10-30/improvement-workflow-roles.md) | Current; the S3 surface is implemented, no tiered run has used it yet |
| Audit Workflow | [Audit Workflow roles](roles/2026-10-30/audit-workflow-roles.md) | Current |
| Refactor Workflow | [Refactor Workflow roles](roles/2026-11-06/refactor-workflow-roles.md) | Current |
| Data Steward | `AGENTS.md` section 10 | Current; owns the data input boundary |

## Local workspace

`data/` is a local workspace. Only its README files and `partitions.json` are
committed; candles, reports, and run artifacts stay local. There is no database
store in the current surface.

## Developer tools

| Tool | Purpose | Path |
|---|---|---|
| Progression replay oracle | Regenerates and checks the per-bar Progression and retained-leg golden fixtures independently of the Rust implementation. | [`tools/oracle/README.md`](../tools/oracle/README.md) |
| Simulation harness | Selects M1.3 windows, renders clean charts, writes hindsight marks, and reviews them; exploration only. | [`tools/simulation/README.md`](../tools/simulation/README.md) |
| Worktree cleanup guard | Classifies, archives, and removes a merged task worktree; required before `git worktree remove`. | `tools/worktree_cleanup.py` |
| Verification manifest builder | Regenerates `verification.json` and both verification matrices from the registry and claims. | `tools/build_verification.py` |
| Legacy label scan | Rejects retired model, strategy, and architecture labels in the current surface. | `tools/check_legacy.py` |
| Registry, validation, matrix, evidence, emitter, link, and term checks | CI gates for the current contracts. | `tools/check_registry.py`, `tools/check_validation.py`, `tools/check_verification_matrix.py`, `tools/check_verification.py`, `tools/check_event_emitters.py`, `tools/check_links.py`, `tools/check_terms.py` |
| Workflow checker | Checks the workflow index, status, required sections, and links. | `tools/check_workflows.py` |
| Execution-plan checker | Checks current plans for a claim type and reviewer verification table. | `tools/check_execution_plans.py` |
| Claim-type checker | Checks issue/PR templates and the normative claim-type section. | `tools/check_claims.py` |
| Changelog checker | Checks the grouped release structure, allowed types, and release order. | `tools/check_changelog.py` |
| Release checker | Checks release notes, tag names, dates, and changelog alignment. | `tools/check_releases.py` |
| Partition checker | Checks canonical instruments, aliases, partitions, and config references. | `tools/check_partitions.py` |
| Command-surface checker | Fails when `help.rs`, the README command table, the command registry, or a tutorial disagree; keeps planned commands out of the available surface. | `tools/check_command_surface.py` |
| Structural fitness checker | Enforces module docs, engine purity, data allowlist, workflow indexing, dated docs directories, and naming rules; reports size, surface, and layout metrics. | `tools/check_structure.py` |

The historical experiment matrix runner and research model-cell runner are
retained for reproducibility and are not current tools. The improvement run store
has a current adapter (`run_current.py`) and historical scripts (`run.py` and the
earlier harnesses). The boundary is recorded in
[`tools/improvement/README.md`](../tools/improvement/README.md),
[`tools/experiment/README.md`](../tools/experiment/README.md), and
[`tools/research/README.md`](../tools/research/README.md).

## Releases

| Release | Tag | Notes |
|---|---|---|
| Release index and tag rule | — | [`docs/releases/README.md`](releases/README.md) |
| Last pre-Tauri release | `v0.8.0` | [`docs/releases/2026-11-11/v0.8.0.md`](releases/2026-11-11/v0.8.0.md) |
| Model states, session coverage, and SR9 | `v0.7.0` | [`docs/releases/2026-11-09/v0.7.0.md`](releases/2026-11-09/v0.7.0.md) |
| Workflow documentation reset | `v0.6.0` | [`docs/releases/2026-11-07/v0.6.0.md`](releases/2026-11-07/v0.6.0.md) |
| Workflow reset baseline | `v0.5.0` | [`docs/releases/2026-11-01/v0.5.0.md`](releases/2026-11-01/v0.5.0.md) |
| Lab Summary and Help | `v0.3.0` | [`docs/releases/2026-10-30/v0.3.0.md`](releases/2026-10-30/v0.3.0.md) |
| MVP v1 | `v0.2.0` | [`docs/releases/2026-10-28/v0.2.0.md`](releases/2026-10-28/v0.2.0.md) |

These releases describe surfaces that the P3 cutover later removed. They remain the
record of what was released. Historical tags `v0.4.0-exp-baseline` and
`audit-closure-2026-10-30` are retained as markers; they are not releases.

## Current programs

| Program | Status | Plan |
|---|---|---|
| Clean current architecture | P0.1–P8 complete; M3 preflight underpowered | `docs/execution-plans/2026-11-05/467-clean-current-architecture.md` |
| P2 M1.0 naming | Merged; M1.0 contract, config identities, retired model parsers, legacy scan | `docs/execution-plans/2026-11-05/470-m1-naming.md` |
| P3 retired-surface removal | Merged in PR #484 at `84ad934` | `docs/execution-plans/2026-11-05/471-four-axis-removal.md` |
| P4 config cleanup | Merged in PR #485 at `50549dd` | `docs/execution-plans/2026-11-05/472-config-cleanup.md` |
| P5 panel/session artifacts | Merged in PR #486 at `392ce80` | `docs/execution-plans/2026-11-05/473-panel-session-artifacts.md` |
| P6 checklist/docs | Implemented; row 37 wording, family-level tests, retired contracts | `docs/execution-plans/2026-11-05/474-checklist-docs.md` |
| P7 verify/closeout | Implemented; full gate, oracle cleanup, M2 closeout verification | `docs/execution-plans/2026-11-05/475-verify-closeout.md` |
| P8a M3.x charter | Approved 2026-11-06; P8b executed | `docs/governance/2026-11-05/M3-CHARTER.md` |
| P8b M3.x programme | Underpowered at the count gate; no development run; validation not opened | `docs/improvement/2026-11-05/M3-PREFLIGHT.md` |
| M4.x model programme | Development `no_edge`; five primary cells; validation not opened | `docs/governance/2026-11-07/M4-CHARTER.md`; `docs/improvement/2026-11-07/M4-DEV-SWARM.md` |
| MR5.x model programme | Development `no_edge`; 30 cells; validation not opened | `docs/execution-plans/2026-11-08/mr5-programme.md`; `docs/improvement/2026-11-08/MR5-DEV-SWARM.md` |
| MR6.x model programme | Development `no_edge`; 15 cells; validation not opened | `docs/execution-plans/2026-11-08/mr6-programme.md`; `docs/improvement/2026-11-08/MR6-DEV-SWARM.md` |
| MR7.x model programme | Development `no_edge`; 15 cells; validation not opened | `docs/execution-plans/2026-11-08/mr7-programme.md`; `docs/improvement/2026-11-08/MR7-DEV-SWARM.md` |
| M2.x model programme | Closed at `no_edge`; legacy removal #463 complete | `docs/execution-plans/2026-11-05/410-m2-model-programme.md` |
| Workflow documentation reset | Released in `v0.6.0` (PR #542) | `docs/execution-plans/2026-11-07/workflow-documentation-reset.md` |
| Simulation runs | Approved 2026-11-10; execution started | `docs/execution-plans/2026-11-10/simulation-runs.md` |

## Current architecture decisions

| Decision | Status | Path |
|---|---|---|
| Flat indicator/event model; composite axes removed from decisions | Accepted 2026-11-03 | `docs/adr/2026-11-03/flat-indicator-event-model.md` |
| History, retention, and display policy | Accepted | `docs/adr/2026-10-25/history-retention-and-display-policy.md` |
| Market analysis and full-execution backtest boundaries | Accepted; partly superseded by the 2026-11-05 cutover | `docs/adr/2026-10-27/analysis-and-backtest-boundaries.md` |
| Extended-range M1 window | Accepted | `docs/adr/2026-10-27/extended-range-window.md` |
| Abstraction levels and engine boundaries | Accepted 2026-10-30 | `docs/adr/2026-10-30/abstraction-levels-and-engine-boundaries.md` |
| Rule purity and statistics boundary | Accepted 2026-10-30 | `docs/adr/2026-10-30/rule-purity-and-statistics-boundary.md` |
| Stateless strategy and the position parameter | Accepted 2026-10-30 | `docs/adr/2026-10-30/stateless-strategy-position-parameter.md` |
| One-way timeframe ladder and trade-intent boundary | Accepted 2026-10-30 | `docs/adr/2026-10-30/one-way-timeframe-ladder.md` |
| Trustworthy backtest kata and architecture characteristics | Accepted 2026-10-31 | `docs/adr/2026-10-31/adopt-architecture-baseline.md` |

## Reading a work item

A work item's plan and its transparency log have the same base name and sit in two
sibling folders:

```
docs/execution-plans/2026-10-21/14-model-engine.md
docs/ai-transparency-log/2026-10-21/14-model-engine.md
```

The plan says what was decided and why. The log entry says what the AI actually did,
what was verified, and what was rejected or corrected.

AI transparency logs are point-in-time records. Status lines such as "pull request
is open" describe the moment the entry was written; the current state is in Git,
GitHub, and the execution plan.

## Dates

Every date segment is a UTC date, per `AGENTS.md` section 12. A work item is filed
under the UTC date on which it was recorded, which is not necessarily the local date
of the machine that produced it.
