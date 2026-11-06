# QuantRule Documentation

Work-item documents use `docs/<kind>/<YYYY-MM-DD>/<file>.md`: the kind comes first, then the UTC date of the work item, then the file. Living system documents sit directly under their kind, as `docs/architecture/<file>.md`. A kind directory appears when it is first used.

## Kinds

| Kind | Holds | Path |
|---|---|---|
| Execution plans | The plan for a substantial work item, per `AGENTS.md` section 11. | `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md` |
| AI transparency log | The record of material AI contributions for a work item, per `AGENTS.md` section 6. | `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md` |
| Agent roles | The quant and engineering roles, their handoffs, and when each is required. | `docs/roles/<YYYY-MM-DD>/<file>.md` |
| Architecture | The living system documentation: overview, product kata, architecture characteristics, style/quantum, and cross-cutting views. | `docs/architecture/<file>.md` |
| Model | The analytical model's contract and evidence. The implementation follows it. | `docs/model/<YYYY-MM-DD>/<file>.md` |
| Configuration | The engine JSON configuration: files, fields, validation, and cross-config rules. | `docs/config/<YYYY-MM-DD>/<file>.md` |
| Reports | Historical report bundles and verification records. | `docs/reports/<YYYY-MM-DD>/<file>.md` |
| Releases | The notes for a tagged release. | `docs/releases/<YYYY-MM-DD>/<file>.md` |
| Architecture decision records | A decision, the alternatives considered, and the consequences. | `docs/adr/<YYYY-MM-DD>/<file>.md` |
| Audits | Read-only findings, evidence, and remediation options. | `docs/audits/<YYYY-MM-DD>/<scope>-audit.md` |
| Governance | The improvement charter, green criteria, autonomy, and stop conditions. | `docs/governance/<YYYY-MM-DD>/<file>.md` |
| Workflows | The living Audit Workflow, Improvement Workflow, and run contract. | `docs/workflows/<file>.md` |
| Improvement registry | The committed run index, baselines, invalidation tombstones, and run record template. | `docs/improvement/<file>.md` |

The tutorial kind is defined in `AGENTS.md`; the current M1.0 lab tutorial is
linked below.

## Current architecture

| Document | Status | Path |
|---|---|---|
| System architecture entry point | Current | `docs/architecture/README.md` |
| The Trustworthy Backtest Kata | Current | `docs/architecture/kata.md` |
| Architecture characteristics | Current | `docs/architecture/characteristics.md` |
| Command surface | Current; implemented/planned stage contract and drift gate | `docs/architecture/command-surface.md` |
| S3.x command-surface plan | Approved 2026-11-06; Phase 1 implemented and verified locally, publication pending | `docs/execution-plans/2026-11-06/s3-command-surface.md` |

## Current model

| Contract | Status | Path |
|---|---|---|
| M1.0 flat model | Current baseline; path gate no edge; compression baselines accepted | `docs/model/2026-11-05/model-m1.0.md` |
| Indicator coverage | Implemented 2026-11-05: 46 indicators, 143 active events, stable event codes; 189 validation claims and 54/54 construct cases; predictive evidence pending | `docs/model/2026-11-05/indicator-coverage.md` |
| Model families | Approved 2026-11-05; `M1.0`, historical `M2.x`, and the future `M3.x` map | `docs/model/2026-11-05/model-families.md` |
| Model spec v1 | Implemented; condition-only | `docs/model/2026-11-05/model-spec-v1.md` |
| Model spec v2 | Implemented; reading overrides, active events, causal vote, panel/session | `docs/model/2026-11-05/model-spec-v2.md` |
| Indicator and parameter registry | Current; CI check | `docs/registry/README.md` |
| Current-architecture history index | Current; retired paths and their recovery commit | `docs/registry/history.md` |
| M1.0 model lab tutorial | Current; covers the data and model surface | `docs/tutorials/2026-11-06/m1.0-model-lab.md` |
| Strategy lab tutorial | Current; `report model`, `strategy explain`, `strategy run` | `docs/tutorials/2026-11-06/strategy-lab.md` |
| Execution lab tutorial | Current; `report model`, `strategy run`, `execution run` | `docs/tutorials/2026-11-06/execution-lab.md` |
| Portfolio lab tutorial | Current; `report model`, `strategy run`, `execution run`, `portfolio replay` | `docs/tutorials/2026-11-06/portfolio-lab.md` |
| Backtest lab tutorial | Current; the coupled `backtest run` and its stage bundles | `docs/tutorials/2026-11-06/backtest-lab.md` |
| Indicator verification matrix | Current; 46 rows, generated from the evidence map | `docs/model/2026-11-06/indicator-verification.md` |
| Event verification matrix | Current; 143 rows, generated from the evidence map | `docs/model/2026-11-06/event-verification.md` |
| Verification evidence map | Current; row/family/tool/n-a scope per class | `docs/registry/verification.json` |

## Current run contracts

| Contract | Status | Path |
|---|---|---|
| Policy blueprint | Current; `run v2`, `policy v1`, data schema 3, model schema 4, `execution v2`, `portfolio v1` | `docs/config/2026-10-30/policy-and-strategy.md` |
| Config tree | Current; current, evidence, and historical directories | `config/README.md` |
| Available timeframes | `quantrule.data/v3` | `docs/config/2026-10-30/data-v3.md` |
| MACD reading and events | Current | `docs/config/2026-11-03/model-macd.md` |
| Engine artifacts | Current; stage bundle, provenance, and handoff contract | `docs/config/2026-11-06/engine-artifacts.md` |
| Strategy configuration | Current; `quantrule.strategy/v1`, both generic engines, and the decision record | `docs/config/2026-11-06/strategy-config.md` |
| Qualification criteria | Current; human-owned criteria, verdict semantics, and human acceptance | `docs/config/2026-11-06/qualification-criteria.md` |
| Improvement run contract | S3.x target; no current run uses it | `docs/workflows/run-contract.md` |

The current run does not reference `strategy.json` or `analytics.json`. The loader
rejects them with an unknown-field error. S3.x passes the strategy file and the
qualification criteria as explicit command arguments; the run schema stays
`quantrule.run/v2`.

## Current evidence

| Evidence | Status | Path |
|---|---|---|
| M1.0 model path gate | Development gate no edge; validation not opened | [`docs/improvement/2026-11-04/MODEL-V4-GATE.md`](improvement/2026-11-04/MODEL-V4-GATE.md) |
| Compression-breakout 4 ATR baseline | Accepted model baseline (human, 2026-11-04) | [`docs/improvement/2026-11-04/COMPRESSION-BASELINE-CANDIDATE.md`](improvement/2026-11-04/COMPRESSION-BASELINE-CANDIDATE.md) |
| Compression-range next-bar resolution | Accepted model baseline (human, 2026-11-04) | [`docs/improvement/2026-11-04/COMPRESSION-RANGE-CANDIDATE.md`](improvement/2026-11-04/COMPRESSION-RANGE-CANDIDATE.md) |
| M2.x model programme | P5 `no_edge`; P7 closeout complete; validation not opened | [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](execution-plans/2026-11-05/410-m2-model-programme.md) |
| M3.x model programme | Preflight `underpowered`; no development run; validation not opened | [`docs/improvement/2026-11-05/M3-PREFLIGHT.md`](improvement/2026-11-05/M3-PREFLIGHT.md) |
| M2.x model charter | Approved 2026-11-05; strategy tier closed | [`docs/governance/2026-11-05/M2-CHARTER.md`](governance/2026-11-05/M2-CHARTER.md) |
| Current-surface audit verification | Merged in PR #496; per-row indicator/event matrix pending | [`docs/reports/2026-11-06/current-surface-audit-verification.md`](reports/2026-11-06/current-surface-audit-verification.md) |
| Indicator/event verification evidence | Scope map: 33 row-level, 1410 family-level, 332 tool gates, 304 N/A | [`docs/reports/2026-11-06/indicator-event-verification.md`](reports/2026-11-06/indicator-event-verification.md) |

Pre-reset experiment records under `docs/improvement/` are historical. They are not
improvement runs under the current run contract and are not imported as baselines.

## Current workflows

| Workflow | Status | Path |
|---|---|---|
| Audit Workflow | Current | `docs/workflows/audit-workflow.md` |
| Improvement Workflow | S3.x target; the M1.0 shell does not expose the tiered `backtest` command | `docs/workflows/improvement-workflow.md` |
| Improvement run contract | S3.x target; no current run uses it | `docs/workflows/run-contract.md` |
| Research Swarm Runbook | Current contract; no active campaign | `docs/workflows/research-swarm.md` |

## Current roles

| Role | Manual | Status |
|---|---|---|
| Quant Researcher | [Quant Researcher Manual](roles/2026-10-29/quant-researcher-manual.md) | Historical pre-M1.0 manual; the role definition in `AGENTS.md` section 10 remains normative |
| Quant Developer | [Quant Developer Manual](roles/2026-10-29/quant-developer-manual.md) | Historical pre-M1.0 manual; the role definition in `AGENTS.md` section 10 remains normative |
| Quant Validator | [Quant Validator Manual](roles/2026-10-29/quant-validator-manual.md) | Historical pre-M1.0 manual; the role definition in `AGENTS.md` section 10 remains normative |
| All three | [Quant roles overview](roles/2026-10-29/quant-roles.md) | Current role map and handoff chain |
| Improvement Workflow | [Improvement Workflow roles](roles/2026-10-30/improvement-workflow-roles.md) | S3.x target; model-only current programme uses the research tooling |
| Audit Workflow | [Audit Workflow roles](roles/2026-10-30/audit-workflow-roles.md) | Current |

## Local workspace

`data/` is a local workspace. Only its README files are committed; candles,
reports, and run artifacts stay local. There is no database store in the current
surface.

## Developer tools

| Tool | Purpose | Path |
|---|---|---|
| Progression replay oracle | Regenerates and checks the per-bar Progression and retained-leg golden fixtures independently of the Rust implementation. | [`tools/oracle/README.md`](../tools/oracle/README.md) |
| Verification manifest builder | Regenerates `verification.json` and both verification matrices from the registry and claims. | `tools/build_verification.py` |
| Legacy label scan | Rejects retired model, strategy, and architecture labels in the current surface. | `tools/check_legacy.py` |
| Registry, validation, matrix, evidence, emitter, link, and term checks | CI gates for the current contracts. | `tools/check_registry.py`, `tools/check_validation.py`, `tools/check_verification_matrix.py`, `tools/check_verification.py`, `tools/check_event_emitters.py`, `tools/check_links.py`, `tools/check_terms.py` |
| Command-surface checker | Fails when `help.rs`, the README command table, the command registry, or a tutorial disagree; keeps planned commands out of the available surface. | `tools/check_command_surface.py` |

The experiment matrix runner, improvement run store, and research model-cell runner
are historical. They target the retired backtest surface and are not current
tools. Their retained status is recorded in
[`tools/improvement/README.md`](../tools/improvement/README.md),
[`tools/experiment/README.md`](../tools/experiment/README.md), and
[`tools/research/README.md`](../tools/research/README.md).

## Releases

| Release | Tag | Notes |
|---|---|---|
| Workflow reset baseline | `v0.5.0` | [`docs/releases/2026-11-01/v0.5.0.md`](releases/2026-11-01/v0.5.0.md) |
| Lab Summary and Help | `v0.3.0` | [`docs/releases/2026-10-30/v0.3.0.md`](releases/2026-10-30/v0.3.0.md) |
| MVP v1 | `v0.2.0` | [`docs/releases/2026-10-28/v0.2.0.md`](releases/2026-10-28/v0.2.0.md) |

These releases describe surfaces that the P3 cutover later removed. They remain the
record of what was released.

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
| M2.x model programme | Closed at `no_edge`; legacy removal #463 complete | `docs/execution-plans/2026-11-05/410-m2-model-programme.md` |

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
