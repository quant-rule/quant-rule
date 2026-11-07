# Workflow and Status Drift Audit

- Date: 2026-11-07 (UTC)
- Status: Open findings; remediation implemented locally in the `workflow-documentation-reset` worktree and recorded in `docs/execution-plans/2026-11-07/workflow-documentation-reset.md`; findings move to `Fixed` after the pull request merges
- Scope: current documentation status, workflow coverage, run-store tooling, data partitions, and record templates
- Baseline: `209530b` (merge of PR #540), clean `main`
- Claim type: documentation and tooling audit; no model, strategy, or edge claim.

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| AUD-2026-11-07-01 | Major | docs | `AGENTS.md` says the shell exposes the model surface only, but every pipeline stage is implemented. | `AGENTS.md:51`; `README.md` command table; `docs/architecture/command-surface.md` | Agents and humans are told a capability does not exist when it does. | A: update `AGENTS.md` to the current surface. B: move the status to `docs/README.md` and link it. | A+B | Grep for the stale sentence; link check. | Open | Plan §1.3 F-01 |
| AUD-2026-11-07-02 | Major | docs | The Improvement Workflow and run contract still say “S3.x target / M1.0 shell has no `backtest`”. | `docs/workflows/improvement-workflow.md:4-6`; `docs/workflows/run-contract.md:4-6` | The evidence workflow appears unusable. | A: update the status. B: replace the duplicated status with a link to `docs/README.md`. | A+B | Workflow checker; grep. | Open | Plan §1.3 F-02 |
| AUD-2026-11-07-03 | Major | docs | `docs/README.md` repeats the stale S3 status in four rows. | `docs/README.md:71,100,101,112` | The global index is not a reliable status source. | A: update each row. B: make `docs/README.md` the single status index and remove duplicate status tables. | B | Link check; workflow checker. | Open | Plan §1.3 F-03 |
| AUD-2026-11-07-04 | Major | docs | Architecture, policy, and model docs say strategy/execution are libraries or not reachable. | `docs/architecture/README.md:30-40`; `docs/config/2026-10-30/policy-and-strategy.md:79,123`; `docs/model/2026-11-05/model-m1.0.md:84` | The current pipeline is described as future work. | A: update the three documents. B: add a current-status section and link the contracts to it. | A+B | Grep; link check. | Open | Plan §1.3 F-04 |
| AUD-2026-11-07-05 | Major | workflows | There is no workflow index or selection guide. | `docs/workflows/` has five files; no `README.md` | A human or agent cannot choose a workflow without reading five long documents. | A: add `docs/workflows/README.md`. B: add the index to `docs/README.md` only. | A | Workflow checker. | Open | Plan §1.3 F-05 |
| AUD-2026-11-07-06 | Major | records | There is no changelog. | Repository root | There is no per-PR change record. | A: add root `CHANGELOG.md`. B: use release notes only. | A | Changelog checker. | Open | Plan §1.3 F-06 |
| AUD-2026-11-07-07 | Major | workflows | The model research front half is missing. | `docs/workflows/research-swarm.md` is programme-specific; no intake/screen/power-preflight workflow | The next model programme can repeat the M3 underpowered outcome. | A: add `model-research.md`. B: extend `research-swarm.md` only. | A | Workflow checker. | Open | Plan §1.3 F-07 |
| AUD-2026-11-07-08 | Major | workflows | There are no strategy or portfolio research runbooks. | `docs/workflows/` | The next tier would invent its process under pressure. | A: add `strategy-research.md` and `portfolio-research.md`. B: add one combined tier runbook. | A | Workflow checker. | Open | Plan §1.3 F-08 |
| AUD-2026-11-07-09 | Major | workflows | There are no policy-change, data-lifecycle, or model-change workflows. | `docs/workflows/` | Policy, data, and indicator/event changes have no explicit route. | A: add the three workflows. B: cover them in `AGENTS.md` only. | A | Workflow checker. | Open | Plan §1.3 F-09 |
| AUD-2026-11-07-10 | Major | workflows | There are no requirements/design, release/operations, deprecation, or recordkeeping workflows. | `docs/workflows/` | The engineering lifecycle is incomplete. | A: add the four workflows. B: add a single lifecycle workflow. | A | Workflow checker. | Open | Plan §1.3 F-10 |
| AUD-2026-11-07-11 | Major | tooling | The run-store tooling is stale. | `tools/improvement/run.py:316-325` invokes retired `quantrule backtest`; `evidence.py:117` reads retired `portfolio/report.json` | The run store would fail or extract nothing. | A: add a current adapter and update evidence extraction. B: retire the run store and use a manual procedure. | A | Unit tests; fixture run. | Open | Plan §1.3 F-11 |
| AUD-2026-11-07-12 | Major | data | Partition instruments and research instruments disagree. | `data/partitions.json` lists BTCUSD/ETHUSD/XAUUSD; M2/M3 configs use BTC/XAU/EURUSD/SP500 | Data identity is ambiguous. | A: canonical IDs plus aliases. B: rewrite the historical configs. | A | Partition checker; `program.py --check`; `coverage.py`. | Open | Plan §1.3 F-12 |
| AUD-2026-11-07-13 | Minor | data | `data/README.md` documents retired directories. | `data/README.md` lists `charts`, `reports`, `labs`, `research`, `db`, `notes`, `scratch` | The local workspace description is stale. | A: update the README. B: remove the legacy directories. | A now; B as a separate destructive change | Grep; link check. | Open | Plan §1.3 F-13 |
| AUD-2026-11-07-14 | Minor | docs | There is no status vocabulary and no record map. | Docs use “S3.x” for both the command surface and the strategy family | Readers cannot tell current, target, and historical. | A: add the vocabulary and record map. B: keep the ad hoc wording. | A | Workflow checker. | Open | Plan §1.3 F-14 |
| AUD-2026-11-07-15 | Minor | templates | Issue/PR templates do not carry workflow, changelog, or record IDs. | `.github/ISSUE_TEMPLATE/remediation.md`; `.github/pull_request_template.md` | The record chain is incomplete. | A: add templates. B: keep the current PR template only. | A | Template review. | Open | Plan §1.3 F-15 |

## Detail: AUD-2026-11-07-01

- Expected invariant: `AGENTS.md` describes the current shell surface.
- Observed: section 2 says the shell exposes the model surface only and that no strategy or backtest command is reachable.
- Root cause: the S3 command surface landed after the section was written; the status was not reconciled.
- Impact: an agent may refuse a valid command or propose work that already exists.
- Verification: `grep -n "model surface only" AGENTS.md` returns no match after the fix.

## Detail: AUD-2026-11-07-02

- Expected invariant: the Improvement Workflow and run contract are current or explicitly target.
- Observed: both carry the pre-S3 “M1.0 shell does not expose the tiered backtest command” status.
- Root cause: the status line was not updated when the S3 phases merged.
- Impact: the evidence workflow looks unusable; a run may not be attempted.
- Verification: `python3 tools/check_workflows.py` reports the workflow as `Current`; the status line is absent.

## Detail: AUD-2026-11-07-03

- Expected invariant: the global docs index has one current status per row.
- Observed: four rows repeat the pre-S3 status.
- Root cause: the status was copied into the index instead of linked.
- Impact: the index contradicts the command surface.
- Verification: `python3 tools/check_links.py` and `python3 tools/check_workflows.py` pass; a grep for the stale sentence returns no match.

## Detail: AUD-2026-11-07-04

- Expected invariant: architecture, policy, and model contracts describe the current pipeline.
- Observed: they say strategy/execution are libraries or not reachable.
- Root cause: the S3 cutover updated the command surface but not the surrounding contracts.
- Impact: a reader cannot tell which stage is implemented.
- Verification: grep for “remain libraries”, “no strategy command”, “does not run execution”, and “S3.x builds” returns no match in the current docs.

## Detail: AUD-2026-11-07-05

- Expected invariant: one workflow index names every workflow and how to choose it.
- Observed: no `docs/workflows/README.md`.
- Root cause: workflows were added one at a time without an index.
- Impact: discovery depends on memory or a full-text search.
- Verification: `python3 tools/check_workflows.py` reports every workflow indexed.

## Detail: AUD-2026-11-07-06

- Expected invariant: every merged PR has a changelog row.
- Observed: no changelog.
- Root cause: the repository relied on Git and release notes.
- Impact: the change history is not readable in one place.
- Verification: `python3 tools/check_changelog.py` passes.

## Detail: AUD-2026-11-07-11

- Expected invariant: the run store invokes the current shell command and reads the current bundle.
- Observed: `run.py` invokes `quantrule backtest`; `evidence.py` reads `artifacts/portfolio/report.json`.
- Root cause: the S3 command surface changed the command name and the portfolio bundle shape.
- Impact: a run would fail before writing evidence.
- Verification: `python3 -m unittest tools.improvement.test_run_current` and `test_evidence` pass; a fixture run writes `results.json` and `evidence.json`.

## Detail: AUD-2026-11-07-12

- Expected invariant: every instrument used by a run is declared in `data/partitions.json`.
- Observed: the file lists BTCUSD/ETHUSD/XAUUSD; the M2/M3 configs use BTC/XAU/EURUSD/SP500.
- Root cause: the research programme introduced new instruments without reconciling the partition file.
- Impact: a run can cite an instrument the partition does not declare.
- Verification: `python3 tools/check_partitions.py` passes; `program.py --check` and `coverage.py` pass.

## Rules

- This audit is read-only. It does not modify code, tests, issues, or pull requests.
- Evidence is a command, a file path, or a reproducible observation.
- The remediation plan is `docs/execution-plans/2026-11-07/workflow-documentation-reset.md`.
- Finding status moves to `Fixed` only after the remediation PR merges; only the Improvement Verifier moves it to `Verified`.
