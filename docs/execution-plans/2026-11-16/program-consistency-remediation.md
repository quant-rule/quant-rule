# Execution Plan — Program Consistency Remediation

- Status: In Review
- Proposed: 2026-11-16 (UTC)
- Approved: 2026-11-16 (UTC)
- Review requested: 2026-11-16 (UTC)
- Claim type: Behaviour Claim
- Owner: Human
- Workflow: [Execution Plan](../../workflows/execution-plan.md) + [Audit Workflow](../../workflows/audit-workflow.md) remediation stage
- Review baseline: `0e7c450c` on `main`
- Branch: `docs/program-consistency-remediation` for this plan; one `docs/`, `fix/`, `feat/`, or `test/` branch per fix issue after approval
- AI log: [`docs/ai-transparency-log/2026-11-16/program-consistency-remediation.md`](../../ai-transparency-log/2026-11-16/program-consistency-remediation.md)
- Issue number: none exists yet. This plan uses `CONS-2026-11-16-NN` work-item IDs. The Planner assigns GitHub issue and sub-issue numbers after human approval, as `AGENTS.md` section 11 requires. No issue number is invented here.
- Changelog: the plan and AI log are records, so no changelog entry is needed for them. Each fix adds the release-type entry that fits (`Added`, `Changed`, or `Fixed`).

## 1. Intent

A read-only consistency review of the released tree found a set of contradictions between the current documentation, the shell parser, the browser client, the service boundaries, the storage keys, the release records, and the status index. None of them fails the Rust test suite, because the gate does not exercise most of these surfaces. Several of them do affect an operator or agent immediately:

- current runbooks contain shell commands that fail with `unknown option`;
- current contracts still name retired local commands and the retired policy layer;
- the browser cannot reach two capabilities that its Help screen and parity fixture claim it reaches;
- storage and remote-access docs describe behavior the code does not have;
- living docs and the release procedure use the wrong tag after the 2026-11-15 renumbering.

The intended outcome is a tree where every Current document describes the surface the code actually implements, every documented invocation parses, every registered capability is reachable from both clients or is recorded as an approved exception, and the release/status records name the current tags consistently. The code behavior itself is not being changed except where a client capability is missing and the plan explicitly says so.

## 2. Current state

| Area | Observed | Evidence |
|---|---|---|
| Baseline | `main` at `0e7c450c`, clean before this worktree; this plan is the only new file | `git status --short --branch`; `git worktree list` |
| Gate | `tools/gate.sh` passes: 2,118 tests passed, 0 failed, 14 ignored; fmt, clippy `-D warnings`, rustdoc, and 24/24 checkers clean | gate log on the review baseline |
| Web tests | `bun test` in `apps/web`: 115 passed, 0 failed | observed during review |
| Desktop tests | `bun test` in `apps/desktop-shell`: 35 passed; supervisor `cargo test`: 9 passed | observed during review |
| Executable runbooks | `run submit --data-version … --config …`, `run fetch … --out …`, and `inspect events --data-version …` fail against the shell parser | `docs/workflows/run-contract.md:211,213`; `portfolio-research.md:137,139`; `strategy-research.md:146,148,150`; `crates/quantrule-shell/src/command/parse.rs:298-308,356-375`; observed `error: unknown option '--data-version'` and `'--out'` |
| Current config contracts | `strategy-config`, `engine-artifacts`, `qualification-criteria`, `analytics-benchmarks`, and `run-v3` are `Current` but name retired commands and/or old run schemas | `docs/config/2026-11-06/*`; `docs/config/2026-11-09/analytics-benchmarks.md`; `docs/config/2026-11-11/run-v3-and-portfolio-constraints.md`; `docs/architecture/command-surface.md:189-195` |
| Run contract | `run_current.py` is called current and historical in the same document; the tooling README says it is historical; `MODEL_SHARDS` and S3-era prose remain | `docs/workflows/run-contract.md:4,114,195,371-379`; `docs/workflows/improvement-workflow.md:242`; `docs/README.md:248`; `tools/improvement/README.md:3-6` |
| Policy | Current workflows still require a policy envelope, prop-firm limits, and the Historical Policy Change workflow | `docs/workflows/improvement-workflow.md:43-45,79-86,91,160`; `portfolio-research.md:7,33,69,171,182,223`; `docs/workflows/README.md:57`; `AGENTS.md:923-925`; `docs/config/2026-11-11/run-v3-and-portfolio-constraints.md:54,60` |
| Browser parity | `dataset show` and `config show` are claimed as controls, but the panels have no detail view or API call; the parity test compares names only | `apps/web/src/lib/capabilities.ts:41,44`; `DatasetsPanel.svelte:78-99`; `ConfigsPanel.svelte:71-101`; `clientParity.test.ts:243-246`; `docs/architecture/client.md:28-32` |
| Chart export | The client contract promises an SVG download; there is no serializer, download control, or test | `docs/architecture/client.md:106`; `apps/web/src/lib/SimulationChart.svelte:294`; no `XMLSerializer`/`Blob` export in `apps/web/src` |
| Storage keys | Docs say `candles/{data_version}` and `ladder/{data_version}/{config_hash}/{timeframe}`; code writes `datasets/{hex}` and `ladder/{version}/full/{timeframe}` | `docs/architecture/data-plane.md:36-37`; `docs/workflows/data-lifecycle.md:205`; `crates/quantrule-data-service/src/routes.rs:205,430-433` |
| Upload status | Data Lifecycle says a duplicate upload returns `409`; code returns `200` | `docs/workflows/data-lifecycle.md:204`; `crates/quantrule-data-service/src/routes.rs:235-237`; `docs/architecture/http-api.md:133-134` |
| HTTP contract | Missing `/model/compute`, three error codes, required-header rule, and evidence-side submission ownership | `docs/architecture/http-api.md:29-53,78-79,116-131,174-182`; `crates/quantrule-gateway/src/server.rs:164,176-177`; `crates/quantrule-evidence-service/src/routes/runs.rs:101-148` |
| Control plane | Docs say gateway produces `run.requested`/`run.cancelled` and backtest sets `cancelled`; evidence does both; `MODEL_SHARDS` does not exist | `docs/architecture/control-plane.md:20-41,87`; `crates/quantrule-evidence-service/src/routes/runs.rs:148,238`; no `MODEL_SHARDS` in the tree; `crates/quantrule-runtime/src/config.rs:221` |
| Deployment/ops | Composed tier is documented as unwired, clients are documented as not sending the token, MinIO table is stale, operations says six of nine containers, characteristics says five deployables | `docs/architecture/deployment.md:48,81,91-92,103-105`; `docs/architecture/operations.md:84`; `docs/architecture/characteristics.md:81,123`; `Makefile:72`; `tools/composed.sh`; `compose.yaml:206` |
| Release/status | Living docs and a reusable release procedure use pre-renumbering tags; `docs/README` says run v3 current and v4 current; nine Historical tutorials still say `Status: Current`; M1.0 is called the current lab | `docs/architecture/services.md:99-104`; `deployment.md:149`; `docs/workflows/release-operations.md:181,194,203`; `docs/README.md:28,39,92-100,110,128,159,167,293`; `AGENTS.md:67`; `docs/tutorials/2026-11-06/*.md:9` |
| Model docs | 106 parameters in the coverage header; registry has 100; Current docs link M1.0 instead of M1.3 | `docs/model/2026-11-05/indicator-coverage.md:4,11`; `docs/registry/README.md:6`; `docs/workflows/model-change.md:7,160`; `docs/config/2026-11-06/engine-artifacts.md:8`; `docs/registry/indicators.json` |
| Model version field | Run record uses config `model_id`; manifest uses registry version; artifact provenance uses package version; API rustdoc calls the run field the registry version; current examples use `M1.0` | `crates/quantrule-api/src/run.rs:203`; `crates/quantrule-evidence-service/src/routes/runs.rs:35,101`; `crates/quantrule-service-api/src/storage.rs:73`; `crates/quantrule-model-service/src/bundle.rs:29`; `crates/quantrule-app/src/artifact/render_report/provenance.rs:50`; `config/examples/*/run.json:4`; `apps/web/src/lib/RunDetail.svelte:155` |
| Denied parser checks | `run submit --data-version … --config …`, `run fetch … --out …`, and `inspect events --data-version …` all fail; the correct forms are positional | observed against `target/debug/quantrule` during review |
| Gate coverage | `check_command_surface` checks names, not argument syntax, and does not scan `docs/config` or `docs/architecture`; `check_workflows` does not reject a Current workflow linking a Historical workflow; `check_client_docs` checks names, not controls | `tools/check_command_surface.py:235-285`; `tools/check_workflows.py`; `tools/check_client_docs.py` |

The review is read-only. No fix is implemented by this plan.

## 3. Scope and non-goals

**In scope**

- The 24 consistency findings recorded in section 6, grouped into five ordered phases.
- Correcting Current documentation to the implemented 27-command gateway surface, the current `quantrule.run/v4` contract, and the current release-tag map.
- Implementing the two missing browser controls (`dataset show`, `config show`) and the documented SVG chart export, with tests.
- Aligning the storage, API, control-plane, deployment, operations, and architecture docs with the code, or recording a code change where the code is the defect.
- Clarifying `model_version` in prose and rustdoc without changing the wire contract; a rename is a separate decision.
- Extending the checkers so the same class of drift fails the gate.
- Verification, changelog, AI log, records, and worktree cleanup.

**Out of scope**

- Any model or strategy behavior change. No indicator, event, reading, threshold, strategy, execution, portfolio, or baseline is changed. No Quant Validator record is required because the signal path is untouched.
- A new HTTP endpoint, command, schema migration, run-contract version, or dependency. A fix that needs one stops for approval.
- Renaming a wire field (`model_version`). The plan records the naming decision; a rename is a separate contract change with its own API-version decision.
- Rewriting a published release note, tag, audit, or plan. Corrections are dated addenda or new records.
- Rewriting the prior audit plan or its finding statuses. This plan covers residual findings the prior remediation did not close.
- A general refactor of the services or clients. Each fix stays on its named seam.
- A release or tag. This plan produces a tree ready for release; tagging and publishing remain separate human gates.

## 4. Assumptions and uncertainty

- The review baseline `0e7c450c` is the evidence baseline. Each fix branch merges the then-current `main` before its pull request.
- The shell parser and the code are authoritative where a doc and code disagree unless the code is a defect. The storage keys, duplicate-upload status, submission ownership, and control-plane producers are treated as code-authoritative first; the docs are corrected in this plan. If the Architect finds the code itself wrong, the fix stops for a decision.
- The browser detail views can use the existing `GET /catalog/datasets/{version}` and `GET /catalog/configs` surfaces. If a new endpoint is needed, stop and return to Requirements & Design.
- The SVG export can serialize the existing chart SVG in the browser without a new runtime dependency. If a dependency is needed, stop; dependencies require approval.
- Updating `docs/config` contracts may reveal that a schema section is still current while a command section is historical. The plan updates the schema and replaces the command section with the gateway path; it does not delete a valid schema.
- The release-tag correction must not rewrite historical notes. The current-tag map and the note's tag note remain the authority.
- The gate hardening may reject an existing current document. The fix is to correct the document or record a narrow written exception; the checker is not weakened to pass.
- The 24 findings are independent enough to split into one issue per fix group, but P1 is the dependency for P2-P4 because it establishes the current command surface the other docs reference.

## 5. Decisions

| ID | Decision | Options | Recommended | Status |
|---|---|---|---|---|
| D1 | Current config contracts (`engine-artifacts`, `strategy-config`, `qualification-criteria`, `analytics-benchmarks`, `run-v3`) | A. Update each to the gateway/run-v4 surface, keeping valid schema sections; B. mark all Historical and write new gateway contracts; C. leave them and add a banner | A | Approved; content decision, human approval |
| D2 | Retired policy layer in current workflows | A. Remove policy/prop-firm from current workflows, point portfolio constraints to the portfolio config, and keep `policy-change.md` Historical; B. re-admit the policy layer; C. leave the routing and document the contradiction | A | Approved; methodology/content decision, human approval |
| D3 | Browser parity for `dataset show`, `config show`, and SVG export | A. Implement the detail views and the SVG download, and extend the parity test to controls; B. remove the claims from the docs and record a parity exception; C. implement only the detail views | A | Approved; functional change, human approval |
| D4 | Storage keys (`candles/` vs `datasets/`, ladder key) | A. Correct the docs to the code (`datasets/{hex}`, `ladder/{version}/full/{timeframe}`); B. change the code to the documented keys; C. document both as aliases | A | Approved; code is authoritative; Architect review |
| D5 | `/model/compute` exposure | A. Document it as a public gateway route and add it to the route table; B. remove the public proxy and keep it internal; C. document it as internal and add a boundary test | A, with B as a fallback if the route is not a client capability | Approved; architecture decision |
| D6 | Run submission and cancellation ownership | A. Correct the docs to the evidence service (the code); B. move the work back to the gateway; C. leave the docs and record the split | A | Approved; architecture/content decision |
| D7 | `MODEL_SHARDS` | A. Remove it from the docs because no setting exists; B. implement it; C. mark it as future | A | Approved; docs only unless the human wants the feature |
| D8 | Release-tag authority in living docs | A. Use the current-tag map, make the release procedure use a version variable, and correct living docs without rewriting historical notes; B. rewrite the historical notes; C. leave the old labels | A | Approved; release/recordkeeping decision |
| D9 | `model_version` semantics | A. Keep the wire field, correct the rustdoc to “model identity from the run config”, document the three contexts, and update current examples; B. rename the run field to `model_id` (breaking contract change); C. rename the artifact field to `product_version` (breaking contract change) | A; B and C are separate decisions | Approved; contract clarity only in this plan |
| D10 | Gate hardening scope | A. Parse documented invocations, scan current `docs/config` and `docs/architecture`, reject Current-to-Historical workflow links, validate client controls, and cross-check living release claims; B. only parse command syntax; C. leave the checks | A | Approved; tooling-only |
| D11 | Deployment/operations/characteristics docs | A. Correct the docs to compose/code: composed tier wired, token sent, pinned MinIO, nine containers, six services; B. change the code/compose; C. leave the docs | A | Approved; docs only |
| D12 | Profile header | A. Document the header as required with no default; B. implement the documented default; C. leave both statements | A | Approved; docs only |

The human approved the scope, the order, and D1-D12 at the recommended options on 2026-11-16. The implementation is complete and the plan is `In Review`; independent verification and the commit/push/PR gates remain separate.

## 6. Findings and order

The findings are reproduced here so the plan is reviewable without a second document. The evidence is the current tree at `0e7c450c`; the review commands are in section 8.

| ID | Severity | Finding | Primary evidence | Fix phase |
|---|---|---|---|---|
| CONS-2026-11-16-01 | Major | Current workflows document shell flags that do not parse | `docs/workflows/run-contract.md:211,213`; `portfolio-research.md:137,139`; `strategy-research.md:146,148,150`; parser rejections observed | P1 |
| CONS-2026-11-16-02 | Major | Current config contracts name retired local commands and old run schemas | `docs/config/2026-11-06/strategy-config.md:5,23`; `engine-artifacts.md:4,96-99,168,264,318,366,450`; `qualification-criteria.md:4`; `docs/config/2026-11-09/analytics-benchmarks.md:12`; `docs/config/2026-11-11/run-v3-and-portfolio-constraints.md:4,12,59` | P1 |
| CONS-2026-11-16-03 | Major | Run contract contradicts itself about `run_current.py`, the S3 surface, and the current execution path | `docs/workflows/run-contract.md:4,114,195,371-379`; `improvement-workflow.md:242`; `docs/README.md:248`; `tools/improvement/README.md:3-6` | P1 |
| CONS-2026-11-16-04 | Major | Current workflows require the retired policy envelope, prop-firm limits, and the Historical Policy Change workflow | `improvement-workflow.md:43-45,79-86,91,160`; `portfolio-research.md:7,33,69,171,182,223`; `docs/workflows/README.md:57`; `AGENTS.md:923-925`; `run-v3:54,60` | P1 |
| CONS-2026-11-16-05 | Major | Current workflows link Historical or stale contracts as their contract | `portfolio-research.md:7`; `strategy-research.md:8`; `model-change.md:7,160`; `docs/config/2026-10-30/policy-and-strategy.md` status Historical | P1 |
| CONS-2026-11-16-06 | Major | Browser cannot reach `dataset show` or `config show`; capability table and Help claim controls that do not exist; parity test checks names only | `apps/web/src/lib/capabilities.ts:41,44`; `DatasetsPanel.svelte:78-99`; `ConfigsPanel.svelte:71-101`; `clientParity.test.ts:243-246`; `docs/architecture/client.md:28-32` | P2 |
| CONS-2026-11-16-07 | Medium | SVG chart export is documented but not implemented or tested | `docs/architecture/client.md:106`; no serializer/download in `apps/web/src` | P2 |
| CONS-2026-11-16-08 | Major | Documented CAS keys do not match the data service | `docs/architecture/data-plane.md:36-37`; `docs/workflows/data-lifecycle.md:205`; `routes.rs:205,430-433` | P3 |
| CONS-2026-11-16-09 | Medium | Duplicate upload is documented as `409`; code returns `200` | `docs/workflows/data-lifecycle.md:204`; `routes.rs:235-237`; `http-api.md:133-134` | P3 |
| CONS-2026-11-16-10 | Low | Gateway exposes `/model/compute` but the HTTP route table omits it | `docs/architecture/http-api.md:29-53`; `gateway/server.rs:176-177`; `model-service/routes.rs:64` | P3 |
| CONS-2026-11-16-11 | Low | HTTP error table omits `hypothesis.not_found`, `hypothesis.invalid`, and `service.unavailable` | `docs/architecture/http-api.md:116-131`; `quantrule-api/src/error.rs:28-51` | P3 |
| CONS-2026-11-16-12 | Medium | Profile header is documented as defaulted and as never defaulted; code refuses a missing header | `docs/architecture/http-api.md:16,182`; `gateway/server.rs:226-230` | P3 |
| CONS-2026-11-16-13 | Major | Run submission is attributed to the gateway; the evidence service performs it | `docs/architecture/http-api.md:78-79`; `gateway/server.rs:164`; `evidence-service/routes/runs.rs:101-148` | P3 |
| CONS-2026-11-16-14 | Major | Control-plane state and producer ownership are wrong; `MODEL_SHARDS` does not exist | `docs/architecture/control-plane.md:20-41,87`; `evidence-service/routes/runs.rs:148,238`; no `MODEL_SHARDS` in code | P3 |
| CONS-2026-11-16-15 | Medium | Deployment docs are stale on the composed tier, bearer tokens, and the MinIO image | `docs/architecture/deployment.md:48,81,91-92,103-105`; `Makefile:72`; `tools/composed.sh`; `compose.yaml:206`; `security.md:51-55` | P3 |
| CONS-2026-11-16-16 | Medium | Operations says six of nine containers start; all nine start | `docs/architecture/operations.md:84`; `deployment.md:62`; `compose.yaml` | P3 |
| CONS-2026-11-16-17 | Medium | Architecture characteristics says five deployables and lists a sixth service as future | `docs/architecture/characteristics.md:81,123`; `services.md:8`; `compose.yaml` | P3 |
| CONS-2026-11-16-18 | Major | Living docs and the reusable release procedure use pre-renumbering tags; the procedure hard-codes an existing tag | `docs/architecture/services.md:99-104`; `deployment.md:149`; `docs/architecture/README.md:41-51`; `desktop.md:3`; `deployment.md:3`; `user-journey.md:3`; `docs/workflows/release-operations.md:181,194,203`; `docs/workflows/README.md:31-32`; `data-lifecycle.md:195`; `recordkeeping.md:169`; `run-contract.md:427`; `docs/README.md:39,112,114,159,167,293`; `AGENTS.md:67`; `apps/desktop-shell/README.md:9`; `docs/tutorials/2026-11-12/*.md:4` | P4 |
| CONS-2026-11-16-19 | Medium | Status index contradicts itself: run v3 vs v4; nine Historical tutorials say Current; current M1.0 lab; current docs unindexed | `docs/README.md:28,92-100,110,128`; `docs/tutorials/2026-11-06/*.md:9`; `docs/config/2026-11-13/run-v4-strategy.md`; `docs/tutorials/2026-11-12/01-run-a-backtest.md`; `02-verify-determinism.md`; `reviewer-verification.md` | P4 |
| CONS-2026-11-16-20 | Medium | Current model links point to M1.0; indicator coverage says 106 parameters while the registry has 100 | `docs/model/2026-11-05/indicator-coverage.md:4,11`; `docs/registry/README.md:6`; `model-change.md:7,160`; `engine-artifacts.md:8`; `model-macd.md:6`; `indicators.json` | P4 |
| CONS-2026-11-16-21 | Medium | `model_version` carries three meanings; API rustdoc says registry version; current examples use M1.0 while M1.3 is current | `quantrule-api/src/run.rs:203`; `evidence-service/routes/runs.rs:35,101`; `service-api/storage.rs:73`; `model-service/bundle.rs:29`; `render_report/provenance.rs:50`; `config/examples/*/run.json:4`; `RunDetail.svelte:155` | P4 |
| CONS-2026-11-16-22 | Low | `AGENTS.md` says the app service owns memory while the same file says it holds none and the module is removed | `AGENTS.md:53,96` | P4 |
| CONS-2026-11-16-23 | Low | Data-plane says the four-tuple is exactly what submission requires; command-surface says every command accepts `--profile` while `profile use` refuses it | `docs/architecture/data-plane.md:70-73`; `http-api.md:46`; `command-surface.md:99`; `dispatch.rs:99-103` | P4 |
| CONS-2026-11-16-24 | Major | The gate does not catch the classes above: no invocation grammar, no `docs/config`/`docs/architecture` scan, no Current-to-Historical link check, no client-control check, no living-tag cross-check | `tools/check_command_surface.py:235-285`; `tools/check_workflows.py`; `tools/check_client_docs.py`; `tools/check_releases.py` | P5 |

## 7. Acceptance criteria

1. Every shell command line in a Current workflow or Current tutorial parses against the shell grammar; a test runs each documented invocation through the parser and fails on an unknown option or a wrong argument count.
2. Every Current config contract names only the 27 implemented commands and the current run schema status: `quantrule.run/v4` for a full run, `quantrule.run/v3` accepted for a model-only run. No Current contract names a retired local command as current.
3. No Current workflow requires a policy envelope, prop-firm limit, or the Historical Policy Change workflow; the workflow index's `Use now?` column and the routing bullets agree.
4. The run contract has one execution path: the gateway `run submit`/`run status`/`run fetch` path or a clearly labeled historical adapter, not both. `MODEL_SHARDS` and the S3-era prose are removed or corrected.
5. The browser reaches `dataset show`, `config show`, and SVG export; the capability table names controls that exist; the parity test enumerates all 27 commands and verifies each control, not only the name.
6. The storage docs match the code: `datasets/{hex}` and `ladder/{version}/full/{timeframe}`. A duplicate upload is documented as `200`.
7. The HTTP contract documents `/model/compute`, every `ErrorCode`, the required profile header, and evidence-side submission/cancellation. The data-plane submission sentence is corrected.
8. `MODEL_SHARDS` is absent or implemented; the control-plane state and producer tables name the evidence service.
9. Deployment, operations, and characteristics match the stack: composed tier wired, token sending documented, pinned MinIO image, nine containers, six services.
10. Living docs use current tags: `v0.9.0` for the distributed platform and `v1.0.0` for the desktop release, with historical notes clearly “prepared as”. The release procedure uses a version variable, not a hard-coded tag.
11. The status index says run v4 is current and v3 is accepted for model-only; the nine Historical tutorials say `Status: Historical`; the M1.0 lab is not called current; the v4 contract, 2026-11-12 tutorials, and reviewer-verification doc are indexed.
12. Current model docs point to M1.3 and say 100 parameters; the registry and the coverage header agree.
13. The `model_version` rustdoc and prose state the three contexts (run-config identity, model-bundle registry version, artifact package version); current examples use the current model identity or explicitly mark the label as historical. No wire field is renamed without a separate approved decision.
14. `AGENTS.md` has one memory ownership statement consistent with the removed module.
15. The gate passes on the merged tree; the new checks fail on seeded regressions; web and desktop tests pass; the plan table is `Pass` or a recorded `Accepted risk`.
16. The changelog, AI log, plan status, worktrees, and branches are updated and cleaned up.

## 8. Work breakdown

The phases run in dependency order. P1 establishes the current surface that P2-P4 reference. P2 is the only functional client change. P3 and P4 are contract/record corrections. P5 hardens the gate and verifies.

| Phase | Item | Findings | Depends on | Roles | Paths | Exit criterion |
|---|---|---|---|---|---|---|
| P0 | Plan, decisions, records | — | — | Planner, Orchestrator | this plan; AI log | Plan `Proposed`; decisions D1-D12 listed; no fix started |
| P1 | Executable current surface | CONS-01..05 | P0, D1, D2 | Implementer, Code Reviewer, Test Engineer | `docs/workflows/**`, `docs/config/**`, `AGENTS.md`, `docs/architecture/command-surface.md`, `tools/check_command_surface.py` | Every documented invocation parses; no Current doc names a retired command or policy layer; run contract has one path |
| P2 | Browser parity and chart export | CONS-06, CONS-07 | P0, D3 | Implementer, Test Engineer, Code Reviewer | `apps/web/src/**`, `apps/web/e2e/**`, `tests/fixtures/client-parity/**`, `crates/quantrule-shell/tests/client_parity.rs`, `docs/architecture/client.md` | Two detail views and SVG export exist; parity test covers controls and all 27 commands |
| P3 | Storage, HTTP, control-plane, deployment contracts | CONS-08..17 | P0, D4, D5, D6, D7, D11, D12 | Implementer, Architect, Release/Operations, Code Reviewer | `docs/architecture/data-plane.md`, `http-api.md`, `control-plane.md`, `services.md`, `deployment.md`, `operations.md`, `characteristics.md`, `docs/workflows/data-lifecycle.md`, `compose.yaml` (read-only reference) | Docs match code/compose; no undocumented public route or nonexistent setting |
| P4 | Release, status, model identity | CONS-18..23 | P0, D8, D9 | Implementer, Recordkeeper, Release/Operations, Architect | `docs/README.md`, `docs/architecture/**`, `docs/workflows/**`, `docs/model/**`, `docs/registry/**`, `docs/config/2026-11-13/**`, `AGENTS.md`, `CHANGELOG.md` | Current tags/status/links agree; `model_version` is documented; no historical rewrite |
| P5 | Gate hardening, verification, closure | CONS-24 + all | P1-P4, D10 | Implementer, Test Engineer, Improvement Verifier, Orchestrator | `tools/check_*.py`, `tools/test_check_*.py`, `tools/gate.sh`, `tools/checkers.txt`, `tools/checker_tests.txt`, `CHANGELOG.md`, plan table, AI log | Seeded failures caught; gate/web/desktop pass; all rows `Pass` or `Accepted risk`; cleanup done |

### P0 — Plan and record intake

- The human approves, changes, or rejects D1-D12 and the phase order.
- The Planner creates one fix issue per phase group, or sub-issues per finding group, with one claim type each. The findings stay `Open`; this plan does not change code or findings.
- The plan and AI log are committed as records. No implementation starts before approval.

### P1 — Executable current surface

1. **Fix the command syntax (CONS-01).** Replace the flag forms in the three workflows with the parser forms:
   - `quantrule run submit <data_version> <config_id>`
   - `quantrule run fetch <run_id> <directory>`
   - `quantrule inspect events <data_version> --timeframe <tf>`
   Add a parser test that runs every documented invocation (or a fixture list of them) and fails on an unknown option or a wrong argument count. The test must fail before the correction.
2. **Correct the current config contracts (CONS-02).** For each `Current` doc:
   - `strategy-config.md`: keep the strategy schema; replace the retired `strategy explain`/`strategy run` command section with `run submit`/`run fetch`; replace `quantrule.run/v2` with the v4/v3 status.
   - `engine-artifacts.md`: keep the valid artifact/provenance tables; replace the local stage-command tables with the gateway path and the run artifact set; update the “current stage commands” statement.
   - `qualification-criteria.md`: replace the `qualify` command with the qualification evidence artifact produced by the run; update the checker claim.
   - `analytics-benchmarks.md`: replace `analytics evaluate` with the run result artifact; keep the benchmark definitions.
   - `run-v3-and-portfolio-constraints.md`: change the status to “v3 accepted for model-only; v4 current”; remove `run check` as current; keep the portfolio constraints.
   If a doc is purely historical after the edit, mark it Historical and link the current contract instead.
3. **Correct the run execution path (CONS-03).** Choose one current path in `run-contract.md`; remove the “S3 command surface” and the contradictory `run_current.py` current claims; align `improvement-workflow.md:242` and `docs/README.md:248`; keep `run.py`/`run_current.py` explicitly historical if they remain.
4. **Remove the retired policy layer from current workflows (CONS-04, CONS-05).** Remove policy/prop-firm from the tier tables, entry conditions, freeze lists, routing bullets, and related-document lists. Route portfolio constraints to `portfolio.json` and the run-v3 portfolio section. Keep `policy-change.md` Historical and remove it from current “use this” lists. Repoint `portfolio-research.md` and `strategy-research.md` contract links to the current run/strategy contracts.
5. **Extend `check_command_surface.py` (part of CONS-24).** Parse the `usage` lines and validate documented invocations in Current workflows/tutorials against the shell grammar; add a seeded wrong-flag fixture. Add a scan of `docs/config` and `docs/architecture` for retired command names, with a narrow allowlist for the explicit retirement table.
6. **Add a Current-to-Historical workflow link check (part of CONS-24).** `check_workflows.py` fails when a Current workflow links a Historical workflow in a “use this” context; allow explicit historical references and the workflow index.

### P2 — Browser parity and chart export

1. **Dataset detail (CONS-06).** Add a dataset detail view to `DatasetsPanel.svelte` (or a dedicated component) that calls `GET /catalog/datasets/{version}` and shows `object_key`, `sha256`, `quality`, and `created_at`. Add a unit test and an e2e path.
2. **Config detail (CONS-06).** Add a config detail view to `ConfigsPanel.svelte` that calls `GET /catalog/configs` for the selected config and shows the full summary, origin, validity, parser message, and config identity. Add a unit test and an e2e path.
3. **SVG export (CONS-07).** Serialize the existing chart SVG and offer a download control on the simulation run detail. Add a unit test that the exported bytes contain the rendered SVG and the expected deterministic path data. No new dependency unless approved.
4. **Extend the parity test (CONS-06).** The shared capability fixture and both parity tests must verify that each command names a control that exists (a component/test id), not only a command name. Add a seeded missing-control case that fails.
5. **Update client docs.** `docs/architecture/client.md` screens table and the Help description match the implemented controls.

### P3 — Storage, HTTP, control-plane, deployment contracts

1. **Storage keys (CONS-08).** Correct `data-plane.md`, `services.md`, and `data-lifecycle.md` to `datasets/{hex}` and `ladder/{version}/full/{timeframe}`. Add a checker assertion or a test that reads the key format from the code where practical.
2. **Upload status (CONS-09).** Correct `data-lifecycle.md` to `200` for a duplicate and state the idempotent behavior.
3. **HTTP route table (CONS-10).** Add `POST /model/compute` to `http-api.md` if D5=A; otherwise remove the public proxy and add a boundary test. Document the request/response shape.
4. **Error codes (CONS-11).** Add `hypothesis.not_found`, `hypothesis.invalid`, and `service.unavailable` to the table with their HTTP statuses.
5. **Profile header (CONS-12).** State that the header is required on API routes except the open paths; remove the “default `default`” claim or add the explicit open-path exception.
6. **Submission and cancellation ownership (CONS-13, CONS-14).** Correct `http-api.md` and `control-plane.md`: the evidence service validates the config, derives the run id, writes `queued`, and publishes `run.requested`/`run.cancelled`; the gateway proxies. Remove `MODEL_SHARDS` (D7=A) or implement it in a separate feature decision.
7. **Deployment (CONS-15).** Correct `make test-composed` to the wired tier; correct the bearer-token paragraph to say both clients send it; correct the MinIO image row to the pinned legacy image and note the override.
8. **Operations (CONS-16).** Correct “six of nine” to the actual `make up` behavior: all nine containers start; the quiesce section still stops the non-store services.
9. **Characteristics (CONS-17).** Change “five deployables” to six services and remove “a sixth service” from the watch list or restate the watch item.
10. **Data-plane submission sentence and command-surface profile claim (CONS-23).** Correct the four-tuple sentence to say the server derives config_hash/model_version/image_digest from the submission; correct “every command accepts `--profile`” to name the `profile use` exception.

### P4 — Release, status, model identity

1. **Status index (CONS-19).** Add the run-v4 contract row; mark v3 “accepted for model-only”; change the nine Historical tutorials' `Status` lines to `Historical`; replace “current M1.0 lab” with the current shell HTTP tutorial; index `run-v4-strategy.md`, `01-run-a-backtest.md`, `02-verify-determinism.md`, and `reviewer-verification.md`.
2. **Release tags (CONS-18).** Correct the living docs to the current map: `v0.9.0` distributed, `v1.0.0` desktop. Correct `services.md`, `deployment.md`, `release-operations.md`, `workflows/README.md`, `run-contract.md`, `data-lifecycle.md`, `recordkeeping.md`, `docs/README.md:39,112,114,159,167,293`, `AGENTS.md:67`, and `apps/desktop-shell/README.md:9`. Clarify the “prepared as v1.1.0” status lines in `docs/architecture/{README,desktop,deployment,user-journey}.md` and the 2026-11-12 tutorials where they name the distributed release. Replace the hard-coded release step with a version variable; keep historical notes unchanged.
3. **Model links and count (CONS-20).** Point `docs/registry/README.md`, `docs/workflows/model-change.md`, `docs/config/2026-11-06/engine-artifacts.md`, `docs/model/2026-11-05/indicator-coverage.md`, and `docs/config/2026-11-03/model-macd.md` to M1.3 (or say “M1.0 historical” where the feature is unchanged). Change 106 to 100 and add a registry-count assertion to the checker if practical.
4. **`model_version` clarity (CONS-21, D9=A).** Correct the `quantrule-api` rustdoc to “model identity from the run configuration”. Document the three contexts in `http-api.md` and the model/artifact contracts. Update current example `model_id` labels to M1.3 or explicitly mark them as historical labels. Do not rename a wire field; record the rename as a separate decision if the human wants it.
5. **Memory and data-plane statements (CONS-22, CONS-23).** Make `AGENTS.md` consistent: the application service holds no research memory; the evidence service owns it. Correct the four-tuple sentence and the command-surface `--profile` claim.
6. **Changelog.** Add one `Changed`/`Fixed` entry per user-facing correction; do not add one row per finding.

### P5 — Gate hardening, verification, closure

1. **Gate hardening (CONS-24, D10=A):**
   - `check_command_surface.py`: parse `usage` and validate documented invocations; scan Current `docs/config` and `docs/architecture` for retired command names; keep a narrow allowlist for the explicit retirement table.
   - `check_workflows.py`: reject a Current workflow that sends the reader to a Historical workflow.
   - `check_client_docs.py`: verify each capability control resolves to a component/test id, not only a label.
   - `check_releases.py` or a new checker: cross-check living-doc tag claims against the current-tag map and the releases table.
   - Add one seeded failing fixture per new rule.
2. **Verification:** run `tools/gate.sh`, `bun test` in `apps/web` and `apps/desktop-shell`, the supervisor tests, and the composed tier where the change touches deployment. Record observed output.
3. **Records:** update this plan's verification table, the AI log, the changelog, and the audit/consistency finding statuses. The Improvement Verifier checks the merged work; the implementer does not verify its own work.
4. **Cleanup:** merge, verify acceptance, remove the worktree and local/remote task branches, and leave the board item `Done` as history. Do not delete or edit a board item.

## 9. Verification

| Check | Command | Expected | Limitation |
|---|---|---|---|
| Format/lints/tests/docs | `tools/gate.sh` | pass | Fast tier only |
| Web tests | `cd apps/web && bun test` | pass, including the new detail/export/control tests | The parity fixture is not a live gateway |
| Web build/typecheck | `cd apps/web && bun run check && bun run build` | pass | Needs the installed Bun dependencies |
| Desktop UI tests | `cd apps/desktop-shell && bun test` | pass | No Windows host |
| Supervisor tests | `cd apps/desktop-shell/supervisor && cargo test` | pass | Linux host only |
| Documented command grammar | new parser test over every Current workflow/tutorial invocation | every invocation parses; a wrong-flag fixture fails | Does not prove runtime behavior against a live gateway |
| Command-surface scan | `python3 tools/check_command_surface.py` | pass; seeded retired command in `docs/config` fails | A checker cannot judge prose truth |
| Workflow link check | `python3 tools/check_workflows.py` | pass; seeded Current-to-Historical link fails | Allowlisted historical references still need review |
| Client parity | `bun test` and `cargo test -p quantrule-shell --test client_parity` | all 27 commands and controls agree | The fixture is not the live UI |
| Storage keys | review plus a key-format assertion where practical | docs match code | Keys are still code-owned |
| Release/status | `python3 tools/check_releases.py`; a living-tag cross-check | current tags and rows agree | Historical notes are not rewritten |
| Composed tier | `tools/test_tiers.sh composed` | pass, or a recorded exception | Needs a local Compose stack |
| Plan structure | `python3 tools/check_execution_plans.py` | pass | The plan is `Proposed` until approved |

## 10. Risks and recovery

| Risk | Likelihood | Impact | Mitigation / recovery |
|---|---|---|---|
| P1 doc corrections reveal that a code path, not the doc, is wrong | Medium | High | Stop at the decision boundary; route the code fix to the owning workflow and re-plan. Do not silently change behavior in a docs PR |
| P1 mark a still-current schema Historical | Low | Medium | Keep the schema section; replace only the command/run-schema section. Reviewer checks the contract boundary |
| P2 detail views need a new endpoint | Medium | Medium | Stop and return to Requirements & Design; do not add a route inside this plan |
| P2 SVG export needs a new dependency | Low | Medium | Stop for approval; no dependency is added without human authorization |
| P3 storage-key correction chooses the wrong authority | Medium | High | Default to the code; Architect confirms. If the code is wrong, split the code change and test |
| P3 `/model/compute` is intentionally internal | Medium | Medium | Choose D5=B, remove the public proxy, and add a boundary test instead of documenting a route |
| P4 tag correction rewrites historical evidence | Low | High | Use the current-tag map and dated addenda; never rewrite a published tag or note |
| P5 checker tightening rejects a valid current doc | Medium | Low | Fix the doc or record a narrow written exception; do not weaken the checker |
| P5 browser control check is too strict for Help/presentation commands | Medium | Low | Allow `help` as presentation-only with an explicit exception in the fixture |
| The plan grows into a general refactor | Medium | Medium | Keep each PR on one finding group; stop if a new seam is needed |
| A test is weakened to make a checker pass | Low | High | Reviewer rejects test removal or skip; every new rule has a fail-before fixture |

## 11. Stop conditions

Stop and request human direction when:

- a fix needs a new endpoint, command, dependency, schema migration, run-contract version, or API field rename not named in D1-D12;
- a doc/code conflict shows the code is the defect and the correct behavior is not already settled;
- the browser detail view or SVG export needs a new server capability;
- the storage-key correction would change existing objects rather than the documentation;
- `/model/compute` is intentionally internal and removing the proxy would break a supported client;
- a Current document cannot be corrected without rewriting a Historical record;
- a checker cannot be tightened without weakening a test or rejecting a valid current document;
- a finding is proposed as `Won't fix` (human only);
- a verification row fails and the correction changes the approved scope;
- a worktree or branch cannot be cleaned safely.

## Verification table

At approval, every row names the item, criterion, evidence, and expected result. `Observed`, `Status`, `Reviewer`, and `Date` are filled during execution and review. The plan is not `Verified` while any row is `Pending`, `Fail`, or `Blocked`.

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | P0 plan structure | The plan has status, claim type, all required sections, and a complete table | `python3 tools/check_execution_plans.py` | pass | check_execution_plans.py passed | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V2 | P0 links/terms | New documents resolve and use current terms | `python3 tools/check_links.py`; `python3 tools/check_terms.py` | pass | check_links.py: 975 files; check_terms.py: 599 files | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V3 | P1 workflow syntax | Every Current workflow command parses | new parser test; observed shell output | no unknown option | Parser grammar test passes; runbooks use positional forms | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V4 | P1 config contracts | No Current config contract names a retired command or old run schema as current | `check_command_surface`; review | no retired command; v4/v3 status correct | check_command_surface.py passes; five Current config contracts corrected | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V5 | P1 run path | The run contract has one current execution path | review; `check_legacy` | one path; S3/run_current resolved | Run contract has one gateway path; run_current.py documented historical | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V6 | P1 policy removal | No Current workflow requires policy/prop-firm or routes to Historical Policy Change | review; `check_workflows` | no policy requirement; link check passes | No Current workflow policy route; check_workflows.py passes | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V7 | P1 contract links | Current workflows link current contracts, not Historical/stale ones | `check_links`; review | links resolve; status agrees | Current workflow links resolve to Current contracts | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V8 | P2 dataset detail | Browser reaches `dataset show` | unit/e2e test; component review | detail view and API call exist | Dataset detail view; unit test and e2e pass | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V9 | P2 config detail | Browser reaches `config show` | unit/e2e test; component review | detail view and API call exist | Config detail view; unit test and e2e pass | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V10 | P2 SVG export | Chart export downloads the rendered SVG | unit test; browser check | bytes contain the chart; no dependency | SVG export unit test and e2e download pass | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V11 | P2 parity controls | Every command names an existing control; seeded missing control fails | `bun test`; `cargo test -p quantrule-shell --test client_parity` | all controls resolve | Control test ids verified; 121 web tests pass | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V12 | P3 storage keys | Docs match `datasets/{hex}` and `ladder/{version}/full/{tf}` | review; key assertion | exact match | Docs match datasets/{hex} and ladder/{version}/full/{tf} | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V13 | P3 upload status | Duplicate upload documented as `200` | review | `200`, idempotent | Duplicate upload documented as 200 and idempotent | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V14 | P3 HTTP route | `/model/compute` is documented or removed | review; boundary test | one choice, tested | /model/compute documented with request and response | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V15 | P3 error codes | Every `ErrorCode` is in the HTTP table | diff/review | no omission | Every ErrorCode value appears in the HTTP table | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V16 | P3 profile header | Header rule is one statement | review | required, no silent default | Profile header required; missing header is never defaulted | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V17 | P3 ownership | Submission/cancel ownership names the evidence service | review; code evidence | docs match code | HTTP contract names the evidence service for submission | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V18 | P3 control plane | `MODEL_SHARDS` removed or implemented; state/producer tables correct | review | no nonexistent setting | MODEL_SHARDS removed; state and producer tables name evidence | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V19 | P3 deployment | Composed tier, token, MinIO docs match code/compose | review; `check_links` | all three match | Composed tier, token, and MinIO docs match code/compose | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V20 | P3 operations | Nine containers start | review | nine | Operations says all nine containers start | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V21 | P3 characteristics | Six deployables | review | six | Characteristics says six services; watch item updated | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V22 | P4 status index | Run v4 current, v3 accepted, tutorials Historical, M1.0 not current, docs indexed | `check_client_docs`; review | all agree | Run v4 current, v3 accepted, tutorials Historical, docs indexed | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V23 | P4 tags | Living docs and release procedure use current tags | `check_releases`; review | no hard-coded tag | check_releases.py living-tag check passes | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V24 | P4 model docs | M1.3 links and 100 parameters | review; `check_registry` | agree | M1.3 links and 100 parameters; check_registry.py passes | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V25 | P4 model_version | Rustdoc/prose describe three contexts; examples current; no wire rename | review; `cargo doc` | clear, no field rename | Rustdoc, prose, examples, and UI label updated; cargo doc in gate | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V26 | P4 AGENTS memory | One memory ownership statement | review | consistent | AGENTS.md memory ownership is consistent | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V27 | P4 data-plane/profile | Four-tuple sentence and `--profile` claim corrected | review | accurate | Data-plane submission sentence and --profile claim corrected | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V28 | P5 gate hardening | Every new rule has a seeded failing fixture | `tools/checkers.sh`; `tools/test_checkers` | 24+ checkers pass; seeds fail | Grammar, config, link, control, and tag checks have seeded tests | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V29 | P5 full gate | Workspace gate passes | `tools/gate.sh` | pass | 2,118 passed, 0 failed, 14 ignored; 24/24 checkers | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V30 | P5 client/supervisor tests | Web, desktop, and supervisor tests pass | `bun test`; `cargo test` | pass | Web 121, desktop 35, supervisor 9, e2e 41; svelte-check/build clean | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V31 | P5 records | Changelog, AI log, findings, and plan status updated | review | all present | Changelog, AI log, and plan table updated | Pass | Implementer (sequential) | 2026-11-16 | Sequential implementer pass; independent review pending |
| V32 | P5 closure | Worktree and branches removed; board item stays `Done` | `git worktree list`; branch list | clean | Not merged; worktree and branch remain | Pending | Implementer (sequential) | 2026-11-16 | Awaiting commit/review/merge and cleanup |

## 12. Traceability

| Finding | Work item | Proposed branch | Closure evidence |
|---|---|---|---|
| CONS-2026-11-16-01 | P1.1 workflow command syntax | `docs/consistency-command-syntax` | Parser test; three workflows corrected |
| CONS-2026-11-16-02 | P1.2 current config contracts | `docs/consistency-config-contracts` | Five docs; `check_command_surface` |
| CONS-2026-11-16-03 | P1.3 run execution path | `docs/consistency-run-path` | One path; no S3/run_current contradiction |
| CONS-2026-11-16-04 | P1.4 policy removal | `docs/consistency-policy-removal` | No policy in Current workflows |
| CONS-2026-11-16-05 | P1.5 contract links | `docs/consistency-contract-links` | Current links only |
| CONS-2026-11-16-06 | P2.1 browser detail views | `feat/browser-detail-views` | Unit/e2e; control parity |
| CONS-2026-11-16-07 | P2.2 SVG export | `feat/chart-svg-export` | Unit test; browser check |
| CONS-2026-11-16-08 | P3.1 storage keys | `docs/storage-key-contract` | Docs match code |
| CONS-2026-11-16-09 | P3.2 upload status | `docs/upload-status` | 200 documented |
| CONS-2026-11-16-10 | P3.3 model route | `docs/model-route` | Route documented or proxy removed |
| CONS-2026-11-16-11 | P3.4 error codes | `docs/http-error-codes` | All codes listed |
| CONS-2026-11-16-12 | P3.5 profile header | `docs/profile-header` | One rule |
| CONS-2026-11-16-13 | P3.6 submission ownership | `docs/submission-ownership` | Evidence service named |
| CONS-2026-11-16-14 | P3.7 control-plane ownership | `docs/control-plane-ownership` | Evidence named; MODEL_SHARDS gone |
| CONS-2026-11-16-15 | P3.8 deployment docs | `docs/deployment-consistency` | Composed/token/MinIO corrected |
| CONS-2026-11-16-16 | P3.9 operations | `docs/operations-topology` | Nine containers |
| CONS-2026-11-16-17 | P3.10 characteristics | `arch/six-deployables` | Six services |
| CONS-2026-11-16-18 | P4.1 tag drift | `docs/current-tag-drift` | Current map; no hard-coded tag |
| CONS-2026-11-16-19 | P4.2 status index | `docs/status-index` | v4 current; tutorials Historical |
| CONS-2026-11-16-20 | P4.3 model links/count | `docs/model-contract-links` | M1.3; 100 parameters |
| CONS-2026-11-16-21 | P4.4 model_version clarity | `docs/model-version-clarity` | Rustdoc/prose; no wire rename |
| CONS-2026-11-16-22 | P4.5 AGENTS memory | `docs/agents-memory` | One statement |
| CONS-2026-11-16-23 | P4.6 data-plane/profile | `docs/api-clarity` | Two sentences corrected |
| CONS-2026-11-16-24 | P5 gate hardening | `test/consistency-gate` | Seeded fixtures; gate pass |

## 13. Related documents

- [`docs/workflows/execution-plan.md`](../../workflows/execution-plan.md)
- [`docs/workflows/audit-workflow.md`](../../workflows/audit-workflow.md)
- [`docs/architecture/command-surface.md`](../../architecture/command-surface.md)
- [`docs/architecture/http-api.md`](../../architecture/http-api.md)
- [`docs/architecture/services.md`](../../architecture/services.md)
- [`docs/architecture/client.md`](../../architecture/client.md)
- [`docs/releases/README.md`](../../releases/README.md)
- [`AGENTS.md`](../../../AGENTS.md)
