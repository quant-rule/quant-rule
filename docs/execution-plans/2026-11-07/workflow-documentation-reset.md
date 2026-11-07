# Workflow Documentation Reset — Execution Plan

- Date: 2026-11-07 (UTC)
- Status: **Verified locally** (2026-11-07); publication and human acceptance pending
- Issue: none assigned yet; the parent issue is created after approval
- Branch: `docs/workflow-documentation-reset`
- Worktree: `quant-rule--workflow-documentation-reset`
- Claim type: documentation, workflow, tooling, and test-infrastructure change; no model, strategy, execution, portfolio, or policy behavior change; no edge claim
- Approval: human instruction “approved proceed with the full implementation”
- Audit: [`docs/audits/2026-11-07/workflow-status-audit.md`](../../audits/2026-11-07/workflow-status-audit.md)
- AI transparency: [`docs/ai-transparency-log/2026-11-07/workflow-documentation-reset.md`](../../ai-transparency-log/2026-11-07/workflow-documentation-reset.md)

## 1. Intent

Make every QuantRule workflow discoverable, current, and gated by one index;
add the missing research and engineering workflows; add a changelog and record
map; update `AGENTS.md` and `README.md` to the current S3 surface; add drift
checkers and a current run-store adapter. The result is a documentation and
operating model that a human or an agent can follow without reading every
contract first.

The change does not alter model, strategy, execution, portfolio, or policy
behavior. It updates documentation, tooling, tests, and record templates.

## 2. Current state

- The S3 command surface is implemented: every pipeline stage has a command,
  help entry, README row, tutorial, and artifact contract.
- The documentation still describes the pre-S3 state in `AGENTS.md`,
  `docs/workflows/improvement-workflow.md`, `docs/workflows/run-contract.md`,
  `docs/README.md`, `docs/architecture/README.md`,
  `docs/config/2026-10-30/policy-and-strategy.md`, and
  `docs/model/2026-11-05/model-m1.0.md`.
- There is no workflow index, no changelog, no status vocabulary, and no record
  map.
- The model research front half, the strategy and portfolio runbooks, and the
  policy, data, model-change, requirements, release, deprecation, and
  recordkeeping workflows are missing.
- The run store invokes the retired `quantrule backtest` command and reads the
  retired `artifacts/portfolio/report.json` path.
- `data/partitions.json` lists three instruments; the research configs use five
  instrument identities.
- The full gate passes on the baseline: 1,333 Rust tests, 0 failed; every Python
  gate passes; 15 commands implemented.

## 3. Scope

### Included

- Workflow index, status vocabulary, record map, and workflow template.
- Changelog and changelog gate.
- Status reconciliation across the entry points and affected contracts.
- Model research and model-change workflows.
- Strategy research and portfolio research workflows.
- Policy-change and data-lifecycle workflows.
- Requirements/design, release/operations, deprecation/retirement, and
  recordkeeping workflows.
- Workflow, changelog, release, and partition checkers, structure-checker update, CI update.
- Release index and tag rule (`docs/releases/README.md`).
- Issue and pull-request templates.
- Current run-store adapter, manifest v2, and evidence extraction.
- Partition instrument reconciliation and checker.
- Workflow lab tutorial.
- Verification record and AI transparency log.

### Excluded

- Changes to model formulas, event definitions, strategy rules, execution
  mechanics, portfolio accounting, or policy thresholds.
- A new strategy or portfolio hypothesis, a profitability claim, or an edge
  claim.
- Opening `model_validation`, `strategy_validation`, or `portfolio_holdout`.
- A live-trading, broker, order-book, tick, machine-learning, or statistical
  signal path.
- A server, service, graphical interface, or embedded library.
- Changes to an existing command's meaning.
- Releases, tags, deployments, and merges beyond the authorized workflow.
- Destructive legacy data cleanup; it is a separate, explicitly gated change.

## 4. Assumptions and uncertainty

- The human approves the workflow set, status vocabulary, changelog format,
  issue/PR templates, run manifest v2, and canonical instrument IDs.
- The S3 command surface is implemented; human acceptance of the S3 surface is
  separate and is not claimed here.
- The run-store adapter can invoke the current `backtest run`, `qualify`, and
  `analytics evaluate` commands on fixture data.
- The partition reconciliation can use aliases without rewriting historical
  configs.
- The workflow docs do not change a model or strategy rule. If one does, work
  stops and routes to the Quant roles.
- The legacy data cleanup is destructive and is deferred until a reference check
  and a separate human approval.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Workflow doc shape | QuantRule evidence-first skeleton with MatchPulse discoverability | Every workflow states purpose, stages, gates, artifacts, and stop conditions |
| `AGENTS.md` role | Normative contract plus workflow router | Invariants stay in `AGENTS.md`; procedural detail lives in `docs/workflows/` |
| Status source | `docs/README.md` is the single status index; `docs/workflows/README.md` is the workflow index | No duplicate status tables |
| Changelog | Root `CHANGELOG.md`, grouped by release, meaningful changes only, append-only per release | The release history is readable without duplicating the board |
| Templates | `docs/workflows/templates/` with a structure-checker allowlist | One template per record type |
| Run manifest | `quantrule.improvement-run/v2` is current; v1 is historical | The current adapter can name strategy and criteria files |
| Instrument IDs | Canonical source symbols plus aliases for historical research configs | No historical config rewrite |
| Legacy data cleanup | Separate destructive change | No evidence is removed as part of the documentation reset |

## 6. Acceptance criteria

- `docs/workflows/README.md` exists and lists every workflow with a status.
- Every workflow document has a `Status:` line and is indexed.
- `CHANGELOG.md` exists and records the program's meaningful changes under `## [Unreleased]`, grouped by release type.
- `AGENTS.md` and `README.md` describe the current S3 surface; no stale
  pre-S3 claim remains.
- The new tier and engineering runbooks exist and link to the contracts.
- The run-store adapter runs a fixture backtest and writes `results.json` and
  `evidence.json`.
- The release index, tag rule, and release checker exist and pass; historical
  tags are retained.
- The partitions and instrument aliases are consistent; the checker passes.
- The full gate passes.
- The verification record and AI log are complete.
- No model, strategy, execution, portfolio, or policy behavior changed.
- The human accepts the workflow program.

## 7. Work breakdown

| Phase | Work | Affected paths | Roles | PR |
|---:|---|---|---|---|
| 0 | Audit and plan of record | `docs/audits/2026-11-07/`, `docs/execution-plans/2026-11-07/`, `docs/ai-transparency-log/2026-11-07/` | Orchestrator, Audit Agent, Planner | PR-01, PR-02 |
| 1 | Workflow index, status, changelog, checkers, entry points | `docs/workflows/README.md`, `docs/workflows/templates/`, `CHANGELOG.md`, `AGENTS.md`, `README.md`, `docs/README.md`, `tools/check_*.py`, `.github/workflows/ci.yml` | Orchestrator, Implementation, Test, Code Reviewer | PR-03..PR-07 |
| 2 | Model, model change, strategy, portfolio, policy, data | `docs/workflows/`, `docs/governance/`, `docs/model/`, `docs/config/`, `docs/registry/`, `data/partitions.json`, `tools/check_partitions.py` | Quant Researcher, Quant Developer, Quant Validator, Implementation, Test | PR-08..PR-11 |
| 3 | Requirements, release/operations, deprecation, recordkeeping, release tag rule, tutorial | `docs/workflows/`, `docs/releases/`, `docs/tutorials/`, `tools/check_releases.py` | Architect, Planner, Release Verifier, Implementation, Test | PR-12, PR-13 |
| 4 | Current run-store adapter | `tools/improvement/`, `docs/workflows/run-contract.md` | Implementation, Test, Code Reviewer | PR-14 |
| 5 | Verification and closeout | `docs/reports/2026-11-07/`, `docs/ai-transparency-log/2026-11-07/`, `CHANGELOG.md`, `docs/README.md` | Orchestrator, Test, Code Reviewer, Improvement Verifier | PR-16 |

Only one implementation agent writes to a given file or workstream at a time.
Parallel investigation is allowed when scopes do not overlap.

## 8. Issue hierarchy

No GitHub issue numbers are invented. Numbers are assigned when the issues are
opened.

### Parent — `[Workflow] Rebuild the workflow documentation and operating model`

Goal: make every workflow discoverable, current, and gated by one index; add the
missing research and engineering workflows; add a changelog and record map;
update the entry points to the current S3 surface; add drift checkers and a
current run-store adapter.

Done when: every workflow is indexed; the changelog exists; the entry points are
current; the tier and engineering runbooks exist; the run-store adapter is
tested; the partitions are consistent; the full gate passes; the verification
record is complete; the human accepts the program.

### Sub-issues

| Sub-issue | Scope | Depends on |
|---|---|---|
| `[Audit] Workflow and status drift audit` | Findings F-01..F-15 with reproducible evidence | — |
| `[Plan] Commit the workflow documentation reset plan` | Plan of record and AI log | Audit |
| `[Workflow] Add the workflow index, status vocabulary, and record map` | Index, selection guide, diagram, record map, template | Plan |
| `[Workflow] Reconcile the current S3 status` | Entry points and affected contracts | Index |
| `[Workflow] Add the changelog and its gate` | `CHANGELOG.md`, checker, PR template | Index |
| `[Workflow] Add the workflow and changelog checkers to CI` | Checkers, tests, CI | Index, changelog |
| `[Workflow] Update AGENTS.md and README as the entry points` | Workflow router, completion report, docs map | Index, status, changelog |
| `[Workflow] Add the model research and model change workflows` | Model runbooks, charter, registry links | Index |
| `[Workflow] Add the strategy and portfolio research workflows` | Strategy and portfolio runbooks | Model workflows |
| `[Workflow] Add the policy change and data lifecycle workflows` | Policy/data runbooks and templates | Index |
| `[Workflow] Reconcile the data partitions and instrument aliases` | Partitions, aliases, checker | Policy/data |
| `[Workflow] Add the requirements/design, release/operations, deprecation, and recordkeeping workflows` | Engineering lifecycle | Index |
| `[Workflow] Add the workflow lab tutorial` | Tutorial | Tier runbooks |
| `[Tooling] Add the current run-store adapter and evidence extraction` | Manifest v2, adapter, evidence, tests | Entry points, model, strategy/portfolio |
| `[Workflow] Add issue and pull-request templates` | Templates | Index |
| `[Verification] Run the full workflow verification and close the program` | Verification record, AI log, changelog | All |
| `[Cleanup] Retire the legacy data workspace documentation` (optional) | `data/README.md`, legacy directory docs | Policy/data, partitions |

## 9. Verification

| Check | Expected |
|---|---|
| `cargo fmt --all -- --check` | Clean |
| `cargo clippy --all-targets -- -D warnings` | Clean |
| `cargo test --workspace` | 0 failed |
| `python3 tools/check_file_size.py` | Pass |
| `python3 tools/check_structure.py` | Pass |
| `python3 tools/check_workflows.py` | Pass |
| `python3 tools/check_changelog.py` | Pass |
| `python3 tools/check_partitions.py` | Pass |
| `python3 tools/check_registry.py` | Pass |
| `python3 tools/check_validation.py` | Pass |
| `python3 tools/check_verification.py` | Pass |
| `python3 tools/check_verification_matrix.py` | Pass |
| `python3 tools/check_event_emitters.py` | Pass |
| `python3 tools/check_command_surface.py` | Pass |
| `python3 tools/check_links.py` | No broken links |
| `python3 tools/check_terms.py` | No banned terms |
| `python3 tools/check_legacy.py` | Pass |
| `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | Pass |
| `python3 -m unittest discover -s tools/research -p 'test_*.py'` | Pass |
| `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps` | No warnings |
| `git diff --check` | Clean |

A docs-only change does not require a Quant Validator record. The run-store
adapter does not change signal behavior; the PR records why the Quant Validator
record is not applicable.

## 10. Risks and recovery

- **Docs-only changes break links.** Run `check_links.py` after every PR.
- **New terms trip the term/legacy gates.** Use the approved vocabulary; run
  `check_terms.py` and `check_legacy.py`.
- **Status reconciliation claims S3 acceptance.** Use “S3 surface implemented;
  no tiered run yet; human acceptance pending”.
- **Workflow docs duplicate each other.** One workflow per decision boundary; the
  index is the only status table.
- **Run-store v2 breaks historical tooling.** Keep the old scripts; add
  `run_current.py`; v1 is historical.
- **Partition reconciliation breaks research configs.** Add aliases; do not
  rewrite historical configs.
- **Legacy data cleanup deletes evidence.** Separate destructive PR; reference
  check; human approval.
- **Too much ceremony.** One workflow per boundary; templates optional; required
  fields only.

Rollback is the task branch. No history is rewritten, no release is tagged, and
no remote state is changed without a separate authorization.

## 11. Stop conditions

- A workflow change alters model, strategy, execution, portfolio, or policy
  behavior.
- A new dependency is required.
- The run manifest v2 schema is not approved.
- The canonical data instrument IDs are not approved.
- A destructive cleanup is requested without explicit approval.
- A gate fails for a reason outside the PR scope.
- The human rejects the workflow set, status vocabulary, or changelog format.
- A workflow document duplicates another or creates a second source of truth.
- The S3 human acceptance is claimed without the human's decision.

## 12. Verification record

Observed locally on 2026-11-07:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | 1,333 passed, 0 failed, 8 ignored |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | no warnings |
| `python3 tools/check_workflows.py` | 15 workflows, 6 templates, 15 index rows |
| `python3 tools/check_changelog.py` | 1 release section, 13 entries |
| `python3 tools/check_releases.py` | 3 release notes, 0 changelog versions, baseline 2026-11-07 |
| `python3 tools/check_partitions.py` | 5 instruments, 6 partitions, 4 aliases |
| `python3 tools/check_links.py` | 635 Markdown files, no broken links |
| `python3 tools/check_terms.py` | 387 files, no banned terms |
| `python3 tools/check_legacy.py` | 706 current files, pass |
| `python3 tools/check_command_surface.py` | 15 implemented, 0 planned, 9 tutorial files |
| `python3 -m unittest discover -s tools -p 'test_*.py'` | 45 tests, pass |
| `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | 95 tests, pass |
| `python3 -m unittest discover -s tools/research -p 'test_*.py'` | 43 tests, pass |
| `python3 -m unittest discover -s tools/oracle -p 'test_*.py'` | 54 tests, pass |
| Fixture `run_current.py` | `ok STRATEGY-2026-11-07-01 -> /tmp/opencode/workflow-run-test` |
| Fixture `evidence.py` | `ok STRATEGY-2026-11-07-01 evidence -> .../evidence.json` |

The full record is
[`docs/reports/2026-11-07/workflow-verification.md`](../../reports/2026-11-07/workflow-verification.md).

## 13. Traceability

| Requirement | Contract | Verification |
|---|---|---|
| Every workflow is discoverable | `docs/workflows/README.md` | `check_workflows.py` |
| Every workflow has a status | Workflow docs | `check_workflows.py` |
| Every meaningful change has a changelog entry; the board tracks execution | `CHANGELOG.md` | `check_changelog.py` |
| Release tags are immutable and aligned with the changelog | `docs/releases/README.md` | `check_releases.py` |
| Entry points describe the current surface | `AGENTS.md`, `README.md`, `docs/README.md` | Grep; link check |
| Model research is end to end | `docs/workflows/model-research.md` | Workflow checker |
| Model changes keep the registry in sync | `docs/workflows/model-change.md` | Workflow checker; registry gates |
| Strategy and portfolio tiers are ready | `docs/workflows/strategy-research.md`, `portfolio-research.md` | Workflow checker |
| Policy and data changes are explicit | `docs/workflows/policy-change.md`, `data-lifecycle.md` | Workflow checker |
| Engineering lifecycle is complete | `docs/workflows/requirements-design.md`, `release-operations.md`, `deprecation-retirement.md`, `recordkeeping.md` | Workflow checker |
| Run store uses the current commands | `tools/improvement/run_current.py` | Unit tests; fixture run |
| Evidence extraction uses the current bundles | `tools/improvement/evidence.py` | Unit tests |
| Partitions and instruments agree | `data/partitions.json` | `check_partitions.py` |
| Records are traceable | `docs/workflows/recordkeeping.md` | Workflow checker; link check |
| No model/strategy behavior change | Diff review | Full gate; Quant Validator not applicable |
