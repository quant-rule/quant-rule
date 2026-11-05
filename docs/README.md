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

The tutorial kind remains defined in `AGENTS.md`; no current tutorial exists after
the four-axis cutover.

## Current architecture

| Document | Status | Path |
|---|---|---|
| System architecture entry point | Current | `docs/architecture/README.md` |
| The Trustworthy Backtest Kata | Current | `docs/architecture/kata.md` |
| Architecture characteristics | Current | `docs/architecture/characteristics.md` |

## Current model

| Contract | Status | Path |
|---|---|---|
| M1.0 flat model | Current baseline; path gate no edge; compression baselines accepted | `docs/model/2026-11-05/model-m1.0.md` |
| Indicator coverage | Implemented 2026-11-05: 46 indicators, 143 events, stable event codes; 188 validation claims and 54/54 construct cases | `docs/model/2026-11-05/indicator-coverage.md` |
| Model and strategy families | Approved 2026-11-05; `M1.0` and the historical `S1.x` / `M2.x` maps | `docs/model/2026-11-05/model-families.md` |
| Model spec v1 | Implemented; condition-only | `docs/model/2026-11-05/model-spec-v1.md` |
| Model spec v2 | Implemented; reading overrides, active events, causal vote, panel/session | `docs/model/2026-11-05/model-spec-v2.md` |
| Indicator and parameter registry | Current; CI check | `docs/registry/README.md` |
| Current-architecture history index | Current; retired paths and their recovery commit | `docs/registry/history.md` |

## Current run contracts

| Contract | Status | Path |
|---|---|---|
| Policy blueprint | Current; `run v2`, `policy v1`, data schema 3, model schema 4, `execution v2`, `portfolio v1` | `docs/config/2026-10-30/policy-and-strategy.md` |
| Config tree | Current; current, evidence, and historical directories | `config/README.md` |
| Available timeframes | `quantrule.data/v3` | `docs/config/2026-10-30/data-v3.md` |
| MACD reading and events | Current | `docs/config/2026-11-03/model-macd.md` |
| Improvement run contract | Current workflow contract; no run is current after M2 closeout | `docs/workflows/run-contract.md` |

The current run does not reference `strategy.json` or `analytics.json`. The loader
rejects them with an unknown-field error; S3.x adds a strategy file back.

## Current evidence

| Evidence | Status | Path |
|---|---|---|
| M1.0 model path gate | Development gate no edge; validation not opened | [`docs/improvement/2026-11-04/MODEL-V4-GATE.md`](improvement/2026-11-04/MODEL-V4-GATE.md) |
| Compression-breakout 4 ATR baseline | Accepted model baseline (human, 2026-11-04) | [`docs/improvement/2026-11-04/COMPRESSION-BASELINE-CANDIDATE.md`](improvement/2026-11-04/COMPRESSION-BASELINE-CANDIDATE.md) |
| Compression-range next-bar resolution | Accepted model baseline (human, 2026-11-04) | [`docs/improvement/2026-11-04/COMPRESSION-RANGE-CANDIDATE.md`](improvement/2026-11-04/COMPRESSION-RANGE-CANDIDATE.md) |
| M2.x model programme | P5 `no_edge`; P7 closeout complete; validation not opened | [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](execution-plans/2026-11-05/410-m2-model-programme.md) |
| M2.x model charter | Approved 2026-11-05; strategy tier closed | [`docs/governance/2026-11-05/M2-CHARTER.md`](governance/2026-11-05/M2-CHARTER.md) |

Pre-reset experiment records under `docs/improvement/` are historical. They are not
improvement runs under the current run contract and are not imported as baselines.

## Current workflows

| Workflow | Status | Path |
|---|---|---|
| Audit Workflow | Current | `docs/workflows/audit-workflow.md` |
| Improvement Workflow | Current | `docs/workflows/improvement-workflow.md` |
| Improvement run contract | Current | `docs/workflows/run-contract.md` |
| Research Swarm Runbook | Current contract; no active campaign | `docs/workflows/research-swarm.md` |

## Current roles

| Role | Manual | Status |
|---|---|---|
| Quant Researcher | [Quant Researcher Manual](roles/2026-10-29/quant-researcher-manual.md) | Current; normative in `AGENTS.md` section 10 |
| Quant Developer | [Quant Developer Manual](roles/2026-10-29/quant-developer-manual.md) | Current; normative in `AGENTS.md` section 10 |
| Quant Validator | [Quant Validator Manual](roles/2026-10-29/quant-validator-manual.md) | Current; normative in `AGENTS.md` section 10 |
| All three | [Quant roles overview](roles/2026-10-29/quant-roles.md) | Current; role table and handoff chain |
| Improvement Workflow | [Improvement Workflow roles](roles/2026-10-30/improvement-workflow-roles.md) | Current |
| Audit Workflow | [Audit Workflow roles](roles/2026-10-30/audit-workflow-roles.md) | Current |

## Local workspace

`data/` is a local workspace. Only its README files are committed; candles,
reports, and run artifacts stay local. There is no SQLite store in the current
surface.

## Developer tools

| Tool | Purpose | Path |
|---|---|---|
| Progression replay oracle | Regenerates and checks the Progression golden fixtures independently of the Rust implementation. | [`tools/oracle/README.md`](../tools/oracle/README.md) |
| Legacy label scan | Rejects retired model and strategy labels in the current surface. | `tools/check_legacy.py` |
| Registry, validation, link, and term checks | CI gates for the current contracts. | `tools/check_registry.py`, `tools/check_validation.py`, `tools/check_links.py`, `tools/check_terms.py` |

The experiment matrix runner, improvement run store, and research model-cell runner
are historical. They target the retired backtest surface and are not current tools.

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
| Clean current architecture | P0.1–P5 merged; P6 checklist/docs implemented | `docs/execution-plans/2026-11-05/467-clean-current-architecture.md` |
| P2 M1.0 naming | Merged; M1.0 contract, config identities, retired model parsers, legacy scan | `docs/execution-plans/2026-11-05/470-m1-naming.md` |
| P3 four-axis removal | Merged in PR #484 at `84ad934` | `docs/execution-plans/2026-11-05/471-four-axis-removal.md` |
| P4 config cleanup | Merged in PR #485 at `50549dd` | `docs/execution-plans/2026-11-05/472-config-cleanup.md` |
| P5 panel/session artifacts | Merged in PR #486 at `392ce80` | `docs/execution-plans/2026-11-05/473-panel-session-artifacts.md` |
| P6 checklist/docs | Implemented; row 37 wording, family-level tests, retired contracts | `docs/execution-plans/2026-11-05/474-checklist-docs.md` |
| M2.x model programme | Closed at `no_edge`; legacy removal #463 complete | `docs/execution-plans/2026-11-05/410-m2-model-programme.md` |

## Current architecture decisions

| Decision | Status | Path |
|---|---|---|
| Flat indicator/event model; axes removed from decisions | Accepted 2026-11-03 | `docs/adr/2026-11-03/flat-indicator-event-model.md` |
| Four-axis analytical model | Retired 2026-11-05; retained as history | `docs/adr/2026-10-21/four-axis-analytical-model.md` |
| SQLite per-bar store and research schema | Retired 2026-11-05; retained as history | `docs/adr/2026-10-24/sqlite-store.md` |
| History, retention, and display policy | Accepted | `docs/adr/2026-10-25/history-retention-and-display-policy.md` |
| Market analysis and full-execution backtest boundaries | Accepted; partly superseded by the 2026-11-05 cutover | `docs/adr/2026-10-27/analysis-and-backtest-boundaries.md` |
| Extended-range M1 window | Accepted | `docs/adr/2026-10-27/extended-range-window.md` |
| Config-driven engines and per-engine reports | Historical | `docs/adr/2026-10-28/config-driven-engines-and-reports.md` |
| Abstraction levels and engine boundaries | Accepted 2026-10-30 | `docs/adr/2026-10-30/abstraction-levels-and-engine-boundaries.md` |
| Rule purity and statistics boundary | Accepted 2026-10-30 | `docs/adr/2026-10-30/rule-purity-and-statistics-boundary.md` |
| Stateless strategy and the position parameter | Accepted 2026-10-30 | `docs/adr/2026-10-30/stateless-strategy-position-parameter.md` |
| One-way timeframe ladder and trade-intent boundary | Accepted 2026-10-30 | `docs/adr/2026-10-30/one-way-timeframe-ladder.md` |
| Retirement of the axis-agreement strategy | Accepted 2026-10-30 | `docs/adr/2026-10-30/axis-agreement-retirement.md` |
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
