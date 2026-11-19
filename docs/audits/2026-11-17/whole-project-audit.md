# Whole-Project Audit — v1.3.0 Stage Execution Surface

- Date: 2026-11-17 (UTC)
- Audit ID: `AUDIT-2026-11-17-whole-project`
- Status: 34 findings `Fixed` by the merged remediation (independent Verification pending); 19 `Open` (0 Critical, 11 Major, 7 Minor, 1 Info); human review pending
- Baseline: `main` at `a07def5c` (2026-11-17, clean working tree)
- Scope: the whole repository — data engine and data service, model engine and
  model service, backtest strategy/execution/portfolio and backtest service,
  shared application service, `quantrule.api/v1` and `quantrule.internal/v1`
  contracts, runtime adapters, gateway, the six services, command shell, browser
  client, desktop shell and supervisor, simulation engine and service, the
  research evaluation crate, the gate, CI, release automation, configuration,
  data partitions, and the current documentation, workflows, plans, audits, and
  records.
- Workflow: [`audit-workflow.md`](../../workflows/audit-workflow.md)
- Claim type: program audit; no model, strategy, edge, or profitability claim.
- Auditor: Audit Agent (AI), read-only pass. Five area passes were delegated to
  separate agent sessions; all five returned and their candidate findings were
  re-read against the code. The findings below are the orchestrator's directly
  verified evidence; this is not an independent human review and is not claimed
  as exhaustive coverage.

## Summary

| Severity | Count |
|---|---:|
| Critical | 4 |
| Major | 26 |
| Minor | 18 |
| Info | 5 |

The baseline is a gated, working tree. The Rust workspace gate passed on
`a07def5c`: format, clippy with `-D warnings`, the full workspace test suite
(2,321 passed, 0 failed, 16 ignored), rustdoc with `-D warnings`, and all 25
repository checkers and their unit tests. The link check covered 1,025 Markdown
files with no broken relative link, the registry reports 49 indicators, 150
active events, 100 parameters, and 199 validation claims, the command surface
reports 46 commands, the manual reports 12 chapters, and port parity reports one
registered port.

The audit found four Critical defects in recovery, research-partition
enforcement, portfolio accounting, and the run-cancel lifecycle, and twenty-six
Major defects concentrated in provenance, the canonical digest, model-artifact
promotion, model readings and event availability, test evidence, the release
gate, data coverage, service health and queue reliability, and the clients:

1. **Critical — restore can destroy a started stack and fail.** `tools/restore.sh`
   counts and drops only `catalog`, `runs`, `research`, and `simulation`, while a
   normal stack also owns `gateway` and `stage`; a `--force` restore drops the
   four and then `pg_restore` fails on the remaining objects.
2. **Critical — the research partition is not enforced.** A research manifest's
   partition is validated only as a tier label; nothing compares the run
   config's data window to the declared partition, so a `strategy_dev` manifest
   can submit a 2024/2025 config that reads the validation or holdout window.
3. **Critical — portfolio cash can go negative.** A fixed notional ignores
   equity and an equity fraction of `1.0` plus fees can exceed cash;
   `PortfolioLedger::open` accepts the fill, so the backtest computes returns,
   drawdown, and exposure on capital the account does not have.
4. **Critical — a concurrent cancel can overwrite a succeeded run.** The
   Postgres cancel update has no status predicate, so a `complete` that commits
   between the read and the update is replaced by `cancelled` with the digest
   and artifacts still attached.

The Major findings cover the fabricated image digest, the prior audit's
incomplete verification, the untriaged audit backlog, the FNV/SHA config
identity split, the canonical digest excluding `portfolio/metrics.json` while
`research compare` reads it, promotion requiring the full six-file config hash,
149 of 150 events claiming edge evidence through a readings-only test, the
release pushing the tag image before the composed suite and omitting the browser
and tooling checks, the 2025 holdout declaring instruments whose coverage ends
in 2024, the remote browser lacking secure-context APIs, a stale-detail race in
the browser stores, two disagreeing label tables, the simulation create/bundle
parity gap, the config-upload path escape, the entry-bar bracket path, the
session VWAP zero-fill, the divergence window look-ahead, the one-bar Donchian
underflow, the unused `structure.scale`, the off-by-one VWAP slope, and the
health/queue/concurrency gaps in the services. The Minor findings cover the
remaining documentation, configuration, retention, boundary, test-evidence, and
client-contract items; the Info observations cover the checker and record nits.

The audit reproduced no crash or non-determinism in the examined model/backtest
engine paths. The model pass found a conditional divergence-window look-ahead
and a boundary panic at `donchian_window = 1`; neither is triggered by the
default configuration, and both are recorded as Major. The Critical accounting,
partition, restore, and cancel findings are validation, recovery, and lifecycle
defects, not model-formula defects.

The audit is read-only. No code, test, registry, configuration, issue, or pull
request was changed. The artifacts added by this work item are this audit and
its AI transparency log.

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| AUD-2026-11-17-01 | Major | docs/normative | `AGENTS.md` gives an incomplete and partly false workspace and service map: it omits `quantrule-research` and `quantrule-simulation`, says every pipeline service is a thin adapter over `quantrule-app`, and says there is no current analytics crate | `AGENTS.md:48-65`; `Cargo.toml:2-18`; `crates/quantrule-evidence-service/Cargo.toml:17`; `crates/quantrule-simulation-service/Cargo.toml:18`; `docs/architecture/module-map.md:27-40`; `docs/architecture/services.md:39-52` | An agent reading the normative entry point sees 15 crates where the workspace has 17; an agent adding research-evaluation or simulation code is not pointed at the owning crate; the "no current analytics crate" sentence contradicts a current dependency; the adapter sentence contradicts the service catalog | A. Add both crates and qualify the analytics sentence; B. Replace the enumeration with a link to `module-map.md` and keep only the invariants; C. Leave the text and rely on the module map (rejected: `AGENTS.md` is normative) | A | The crate list equals the `Cargo.toml` members; the service sentence matches `services.md`; `check_structure` and `check_links` pass | Fixed | #895 |
| AUD-2026-11-17-02 | Major | docs/workflow | `AGENTS.md` names a strategy blueprint, an Engineering Swarm, and a Quant Swarm as if they were in the workflow index, and Stage 0 lists a subset of the current workflows | `AGENTS.md:11`; `AGENTS.md:1134-1138`; `docs/workflows/README.md:44-67`; `grep -rn "strategy blueprint" docs/workflows` returns no match; `docs/adr/2026-10-30/governing-swarm-and-improvement-loop.md:29-32` defines swarms as role groups | Workflow selection is misrouted; the strategy blueprint is a retired policy artifact, not a workflow; the swarms are role groups, not workflow documents; Simulation Research, Engine Execution, Backtest, End-to-End Verification, and Execution Plan are omitted from Stage 0 | A. Align the opening sentence and Stage 0 list with the index; B. Add the missing workflow documents (requires human approval and would contradict the index); C. Link the index without an enumeration | A | Every workflow named in `AGENTS.md` is in the index, and every index workflow is named or reached by the Stage 0 link | Fixed | #895 |
| AUD-2026-11-17-03 | Major | docs/workflow | The v1.3.0 status tables were not updated: Engine Execution is still `Target`, the stage-surface status table still marks the manual `Target`, and the HTTP API status line still says v0.9.0/v1.0.0 | `docs/workflows/engine-execution.md:4`; `docs/workflows/README.md:50`; `docs/architecture/stage-surface.md:4,158`; `docs/README.md:42`; `docs/architecture/http-api.md:3` | A shipped diagnostic workflow is presented as not usable; the status index is unreliable at the release boundary; the same stage-surface document says the manual landed and that it is `Target` | A. Update the status lines and add a release-time cross-check between a `Target` workflow and the command/route registry; B. Mark the workflow historical (wrong: it is current); C. Leave it and rely on the release note (rejected) | A | `check_workflows` rejects a `Target` workflow whose commands are implemented; the stage-surface status table has no `Target` row for shipped work; `http-api.md` names v1.3.0 | Fixed | #895 |
| AUD-2026-11-17-04 | Major | provenance/run identity | The evidence service fabricates `sha256(no-image-digest)` for a run or stage when `QR_IMAGE_DIGEST` is unset, while `/health` reports no digest; the documented `make up` path leaves it unset | `crates/quantrule-evidence-service/src/routes/runs.rs:122-126`; `crates/quantrule-evidence-service/src/routes/stages.rs:195-199`; `crates/quantrule-runtime/src/health.rs:60,84`; `compose.yaml:52-54`; `docs/architecture/deployment.md:125`; `docs/workflows/run-contract.md:369-372`; `docker compose config` reports `QR_IMAGE_DIGEST: null` | A local or misconfigured server run records a build identity it did not run; two different builds with the same data/config/model derive the same run id and the second reuses the first's run and artifacts; `/health` and provenance disagree | A. Refuse a run or stage submission when the server has no image digest and set it in `make up` from the built image; B. Embed the build digest at compile time; C. Record null and exclude it from identity (a run-contract change requiring human approval) | A | A server without `QR_IMAGE_DIGEST` returns a typed error; a configured digest appears in `/health` and provenance; two builds produce different run ids | Fixed | #881 |
| AUD-2026-11-17-05 | Major | release/records | The v1.3.0 tag is on the remote while the prior audit's Critical and seven Major findings are `Fixed`, not `Verified`, and its remediation plan is `In Review` with 38/38 verification rows `Pending`; no residual-risk acceptance is recorded | `git ls-remote --tags origin v1.3.0` (tag `9e764f22…`); `docs/releases/2026-11-17/v1.3.0.md:6,61-73`; `docs/audits/2026-11-16/full-program-audit.md:5`; `docs/execution-plans/2026-11-16/full-program-audit-remediation.md:3`; `docs/releases/README.md:47`; `docs/workflows/audit-workflow.md:106`; `AGENTS.md:1230` | The release precondition and the audit green criteria are not evidenced; a consumer cannot tell whether the Critical fix was independently checked; the release note claims verification without naming the audit state | A. Complete the Improvement Verifier pass and update the audit before the next tag; B. Record a dated human residual-risk acceptance for the `Fixed` Critical/Major findings and reference it in the release note; C. Add a release-gate check that fails when a Critical/Major finding on the changed path is not `Verified` or `Won't fix` | A + C | The release note names the audit status; the audit findings move to `Verified`; a release with an unverified Critical fails the gate | Open | — |
| AUD-2026-11-17-06 | Major | records/workflow | Thirty Critical/Major findings remain `Open` or `Fixed` across six audit documents (five `Open` Major, 24 `Fixed` Major, one `Fixed` Critical); the prior audit's recommended triage was not applied | The audit-table scan in §Gate evidence; `docs/audits/2026-11-01/crypto-2023-data-audit.md:5,33`; `docs/audits/2026-11-11/app-service-memory-audit.md:59`; `docs/audits/2026-11-11/native-tauri-window-audit.md:59-61`; `docs/audits/2026-11-06/current-surface-audit.md:5`; `docs/audits/2026-11-07/workflow-status-audit.md:4`; `docs/audits/2026-11-16/full-program-audit.md:735-747` | The audit status record is not a reliable source of truth; old findings may be stale but appear open; the audit green criteria cannot be evaluated; a reader cannot tell what remains | A. The Planner triages every old finding with a dated note and a status in the vocabulary; B. Mark the pre-v1.3.0 audit documents historical and start a new baseline; C. Leave them and rely on this audit (rejected: the status record is the contract) | A | Every old finding has a status in the vocabulary and a dated note; no Critical/Major remains `Open` without a decision; a checker reports the backlog | Open | — |
| AUD-2026-11-17-07 | Minor | docs/reference | The canonical HTTP contract omits the nine `/stages/*` routes that the stage-surface document names as its base contract | `docs/architecture/http-api.md:20-53`; `docs/architecture/stage-surface.md:36-51,72`; `crates/quantrule-gateway/src/server.rs:160-190` | A client implementer reading the HTTP contract cannot find the stage routes; the two documents disagree about where the contract lives | A. Add the stage routes to `http-api.md`; B. State that `stage-surface.md` is the extension contract and link it from the table; C. Generate the table from the gateway route registry | A | Every gateway route appears in `http-api.md` or a linked extension; a route test fails on a missing row | Fixed | #895 |
| AUD-2026-11-17-08 | Minor | docs/records | A blank line inside the committed run index breaks the Markdown table; the rows after it have no header | `docs/improvement/index.md:46-47` | The committed run index renders as a broken table; a reader sees the later run rows outside the columns | A. Remove the blank line and keep the append order; B. Regenerate the index from the run store; C. Leave it (rejected: the index is a record) | A | A Markdown table lint or a check that no blank line interrupts a table | Fixed | #895 |
| AUD-2026-11-17-09 | Minor | tests/tooling | Eight Postgres conformance tests are `#[ignore]`, and no gate, CI, or composed script sets `QR_TEST_DATABASE_URL`; the composed tier is HTTP-only | Eight `#[ignore = "needs QR_TEST_DATABASE_URL…"]` tests in `crates/*/tests`; `grep -rn QR_TEST_DATABASE_URL tools .github Makefile` returns no match; `tools/composed.sh:26-52`; `docs/releases/2026-11-17/v1.3.0.md:88` | The production Postgres store's direct conformance evidence is not produced by any automated tier; a SQLite/Postgres divergence is caught only through the gateway path, if at all | A. Add a disposable Postgres service and run the ignored suites in the release/composed tier; B. Add `tools/test_tiers.sh postgres` and require it before a release; C. Mark the suites `Proposed` and record the exception (honest but no coverage) | A | The tier runs the eight ignored tests against a disposable Postgres; a seeded divergence fails | Open | — |
| AUD-2026-11-17-10 | Major | security/input | An uploaded config's `run.json` can reference a path outside the bundle; the data service reads it, the catalog's `files` is then overwritten with the client's map, and the worker uses `data_key`/`strategy_key` without the `safe_key` check, so the config is valid with a hash over bytes absent from its bundle | `crates/quantrule-data-service/src/configs/upload.rs:51-63,84-113`; `crates/quantrule-app/src/config/run.rs:93-108`; `crates/quantrule-data-service/src/configs/scan.rs:95-120`; `crates/quantrule-backtest-service/src/backtest/workspace.rs:43-50,60-70,189-203`; the escape test at `crates/quantrule-data-service/tests/configs.rs:230-245` covers only a file-map key | A path outside the upload bundle is read by the data service; the catalog hash can be computed over bytes that are not in the bundle, so the config is not reproducible; a reference to `/dev/zero` or a huge file can block the blocking read until OOM; the worker can read a strategy path outside its workspace | A. Give `RunConfig::load` a containment root and reject a resolved path outside it; B. Require every `run.json` reference to be a submitted bundle key, store the referenced bytes, and apply `safe_key` to `data_key`/`strategy_key`; C. Reject an absolute or `..` reference before any read | A + B | A bundle with `"data":"../…"` or an absolute path is rejected at upload; a `/dev/zero` reference is refused; a valid bundle still works; the test covers a referenced path, not only a file key | Fixed | #905 |
| AUD-2026-11-17-11 | Minor | concurrency | Stage retention and delete check `artifact_referenced` after removing rows and then delete the blob; a concurrent stage creation can reference the same blob after the check | `crates/quantrule-evidence-service/src/stage/retention.rs:135-158`; `crates/quantrule-evidence-service/src/stage/postgres.rs:680-689`; `crates/quantrule-evidence-service/src/routes/stages.rs:556-580` | A new stage's artifact can become unreadable; the stage is advertised as content-addressed and verified | A. Delete under a store transaction or advisory lock shared with create; B. Tombstone the blob and delete after a grace period with a re-check; C. Accept and document the race | A | A two-writer test with a create between check and delete; the new stage's blob survives | Open | — |
| AUD-2026-11-17-12 | Minor | docs/config | The config upload route has a 4 MiB cap while the deployment table documents `QR_UPLOAD_LIMIT_BYTES` (512 MiB) as the upload cap; the 4 MiB config cap is undocumented | `crates/quantrule-data-service/src/routes.rs:62-66`; `crates/quantrule-data-service/src/configs/upload.rs:78-80`; `docs/architecture/deployment.md:133` | A user uploading a 5 MiB config gets `413 upload.too_large` despite the documented 512 MiB cap | A. Document the 4 MiB config cap; B. Apply `QR_UPLOAD_LIMIT_BYTES` to configs too; C. Raise the config cap | A | The documentation and the route agree; a 5 MiB config test asserts the documented result | Fixed | #895 |
| AUD-2026-11-17-13 | Minor | security/reproducibility | Compose pins MinIO by digest but uses mutable tags for Postgres and RabbitMQ, and defaults every store to the known `quantrule` credential | `compose.yaml:23,38,55-56,190,196,210,216,230,236`; `crates/quantrule-runtime/src/config.rs:14,20`; `docs/architecture/deployment.md:44-48,138`; `docs/architecture/security.md:95` | A fresh `make up` can pull a different database/broker than the verified run, and a compromised bridge container or shared host has the store credentials; the "No secret is committed" claim is weaker | A. Pin every image by digest and record it; require store passwords for non-local use and fail closed on the defaults; B. Document the tags/credentials as deliberate and require a fresh-pull/secret check | A | `docker compose config` shows digests and no default credential; the release note records the digests | Open | — |
| AUD-2026-11-17-14 | Minor | docs | `docs/README.md` is 350 lines, above the 330-line reference cap, and the length checker does not cover it | `wc -l docs/README.md` (350); `AGENTS.md:174`; `tools/check_doc_length.py:43-55` | The index is longer than the cap the project says it enforces; the checker's coverage is narrower than the rule | A. Trim the index to 330 or add an explicit override; B. Extend the checker to the index and current config documents; C. Scope the cap to architecture documents and say so | A | `check_doc_length` covers `docs/README.md`; the line count is within the cap or an override | Fixed | #895 |
| AUD-2026-11-17-15 | Info | tests/tooling | `tools/test_worktree_cleanup.py` has eight passing tests but is not in `tools/checker_tests.txt`, the gate, or CI | `tools/test_worktree_cleanup.py`; `tools/checker_tests.txt`; `.github/workflows/ci.yml:44-49`; `tools/gate.sh:18-23` | The mandatory cleanup path can regress without the gate noticing; its regression evidence is manual | A. Add the test to `tools/checker_tests.txt`; B. Add a `tools/test_tools.sh` step; C. Accept it as manual and record that | A | The gate runs the test; a seeded cleanup regression fails | Fixed | #902 |
| AUD-2026-11-17-16 | Info | docs/records | Nine execution plans have no same-named AI transparency log; some are sub-plans, some have a related log under another name | `docs/execution-plans/2026-10-24/14-model-engine-completion.md`; `2026-11-01/workflow-reset-cleanup.md`; `2026-11-05/410-legacy-inventory.md`; `2026-11-06/521-refactor-report-run.md`; `2026-11-11/588-tauri-platform.md`; `2026-11-13/kata-alignment.md`; `2026-11-13/649-run-worker.md`; `2026-11-14/776-646-salvage.md`; `2026-11-14/776-split-service-binaries.md`; `AGENTS.md` §6 | A work item's AI record is not discoverable by its plan name; the record may exist under a different base name | A. Add the missing logs or rename the related log; B. Allow a `- AI log:` link and check it; C. Mark sub-plans explicitly in the plan | A | A checker resolves each current plan to a log through the same name or a recorded link | Fixed | #896 |
| AUD-2026-11-17-17 | Info | tests/evidence | The verification map marks `edge` as `row` scope for all 49 indicators, but the named row tests assert only lengths for the empty, single, and short cases; gap, zero-volume, and equal-tie evidence is family-level and not named in the row entry | `crates/quantrule-model/tests/per_row_extended_indicators.rs:131-142`; `crates/quantrule-model/tests/edge_cases.rs:242-341`; `docs/registry/verification.json` (`edge.scope = row`); `tools/check_verification.py:78-94` | The matrix overstates row-level edge coverage; a reviewer cannot tell from the map which test proves which edge case | A. Name the family test in the row entry or add row-level assertions; B. Record the scope as `family` where the proof is family-level; C. Leave the scope and add a note to the registry README | A | A fixture row with a row-scoped edge test that only asserts length is rejected or corrected; the matrix names the proof | Fixed | #889 |
| AUD-2026-11-17-18 | Critical | recovery/restore | `tools/restore.sh` counts and drops only four schemas (`catalog`, `runs`, `research`, `simulation`), while a normal stack also owns `gateway` and `stage`; a `--force` restore drops the four and then `pg_restore` fails on the remaining objects | `tools/restore.sh:214-217,233-237`; `crates/quantrule-gateway/migrations/0007_profiles.sql:3`; `crates/quantrule-evidence-service/migrations/0014_stage.sql:9`; `tools/test_backup.sh:198-210` | A documented recovery path can lose the existing database and leave no working restore; no rollback exists after the drop | A. Include `gateway` and `stage` in the count, drop, and round-trip test; B. Restore only into a fresh empty database and refuse a non-empty target | A | A seeded `gateway`/`stage` row makes `--force` show the full count; a restore round-trip covers all six schemas | Fixed | #881 |
| AUD-2026-11-17-19 | Critical | evidence/research | A research manifest's partition is validated only as a tier label; nothing compares the run config's data window or dataset coverage to the declared partition, so a `strategy_dev` manifest can submit a 2024/2025 config | `crates/quantrule-api/src/improvement.rs:257-286`; `crates/quantrule-evidence-service/src/routes/research/create.rs:52-62`; `crates/quantrule-evidence-service/src/routes/runs.rs:52-116`; `docs/workflows/run-contract.md:73,275-276`; `config/examples/verify-mini/data.json` (`from`/`to`) | Validation and portfolio-holdout data can be consumed under a development label; the one-shot validation claim is procedural, not enforced | A. At `research create`/`submit`, compare the config's `from`/`to` (or dataset coverage) with the partition window and reject a non-contained window; B. Record the config bounds in the catalog and check them in the ledger | A | A `strategy_dev` manifest with a 2024 config is refused; a contained config passes; a regression test covers both | Fixed | #881 |
| AUD-2026-11-17-20 | Critical | backtest/portfolio | A fixed notional ignores equity and an equity fraction of `1.0` plus fees/commission can exceed cash; `PortfolioLedger::open` accepts the fill | `crates/quantrule-backtest/src/portfolio/sizing.rs:145-181`; `crates/quantrule-backtest/src/portfolio/ledger.rs:316-332`; `crates/quantrule-backtest/src/portfolio/config.rs:547-566`; `crates/quantrule-backtest/src/portfolio/tests.rs:359-369` | Cash can go negative while the config declares leverage exactly `1.0`; return, equity, exposure, and drawdown are computed on capital the account does not have | A. Cap the entry notional at `equity × leverage` and reject a fill that makes cash negative; B. Validate `fixed_notional <= initial_cash` and reserve entry costs, or add an explicit margin model | A | A fixed notional above cash is rejected; a fraction of `1.0` with costs does not go negative; tests cover both | Fixed | #882 |
| AUD-2026-11-17-21 | Major | provenance/config | The app writes a 64-bit FNV `config_hash` into artifact metadata as `config_id`/`model_config_id`, while the catalog and run record use SHA-256 over the same files | `crates/quantrule-app/src/config/run.rs:420-453`; `crates/quantrule-app/src/stages/backtest.rs:455,493`; `crates/quantrule-data-service/src/configs/scan.rs:129-147`; `crates/quantrule-evidence-service/src/routes/runs.rs:114-116`; `docs/workflows/run-contract.md:95` | The artifact provenance names a config identity different from the run record and model manifest; a verifier cannot tie the artifact to the config, and FNV is not collision-resistant for an identity field | A. Compute one SHA-256 in `RunConfig::load` with the catalog's canonicalization and use it in every stage; B. Rename the FNV value to `config_fingerprint` and never write it as `config_id`/`config_hash` | A | The run record, model manifest, and stage provenance agree; a cross-crate identity test | Fixed | #890 |
| AUD-2026-11-17-22 | Major | evidence/digest | The canonical digest includes `results.json`, `*.csv`, and `*.ndjson` but not `portfolio/metrics.json`; the portfolio stage writes `metrics.json`, and `research compare` and qualification read it | `crates/quantrule-service-api/src/manifest.rs:172-183`; `crates/quantrule-app/src/stages/portfolio/run.rs:245-262`; `crates/quantrule-evidence-service/src/routes/research/compare.rs:25`; `docs/workflows/run-contract.md:224-227`; `docs/adr/2026-11-12/07-canonical-digest-and-claim-boundary.md:20` | A metrics-only change (profit factor, monthly returns, open position, halt state) can leave the canonical digest unchanged while the research comparison changes | A. Include `metrics.json` (or rename it to `results.json`) in `canonical()` and add a mutation test; B. Make the comparison read a covered artifact | A | Mutating `metrics.json` changes the digest; the compare/qualification tests still pass | Fixed | #890 |
| AUD-2026-11-17-23 | Major | backtest/promotion | Promotion requires the model stage's full six-file config hash to equal the new run's full config hash, and that hash covers strategy/execution/portfolio, so a model stage can only be reused by an identical config | `crates/quantrule-backtest-service/src/backtest/worker.rs:382-386`; `crates/quantrule-evidence-service/src/routes/runs.rs:153-158`; `crates/quantrule-app/src/config/run.rs:436-438`; `docs/workflows/run-contract.md:109-113` | The expensive model compute cannot be reused across strategy cells, which is the stated purpose of promotion; a new strategy forces a new model stage | A. Add a model-input hash (data + model config + ladder) to stage provenance and compare that on promotion; B. Compare only the model-relevant files and add a promotion test where only `strategy.json` changes | A | A promotion test where only `strategy.json` changes succeeds; the model bytes equal a full run | Fixed | #891 |
| AUD-2026-11-17-24 | Major | tests/evidence | 149 of 150 events name only `edge_cases::an_empty_series_publishes_no_readings` as `edge` evidence, and the 150th also names it; the test constructs no event stream. Separately, `signal.breakout_retest_hold` golden evidence points at `signal_events_golden`, whose fixture contains no subtype 24 | `docs/registry/verification.json` (`jq` exact count 149, contains count 150); `crates/quantrule-model/tests/edge_cases.rs:243-252`; `crates/quantrule-model/tests/fixtures/signal_events_golden.csv` (subtypes 1–3 only); `tools/check_verification.py:106-117` | The `AGENTS.md` §4 claim that every event has edge-case coverage is not supported; a regression on empty/single/gap/zero-volume event inputs would pass, and the retest event has no golden or meaningful prefix test | A. Add event edge tests that construct `EventRecord`s for every family and a retest fixture, and point the manifest at them; B. Require `edge` scope `row` for events and narrow the contract/status claim | A | A fixture event edge test constructs events; the retest fixture emits subtype 24; the checker rejects a readings-only test as event edge evidence | Fixed | #889 |
| AUD-2026-11-17-25 | Major | release/verification | The release workflow pushes the tag-named image before the composed suite, and the release gate runs only `tools/gate.sh`, not the browser or tooling checks CI runs | `.github/workflows/release.yml:251-259,319`; `.github/workflows/release.yml:112-113`; `tools/gate.sh:18-23`; `.github/workflows/ci.yml` web and data/research steps | A composed or browser/tooling failure can leave a tag-named image pullable and a release body without the full verification suite | A. Run composed against the locally built image before `docker push` and add a release quality job with the CI web/tooling steps; B. Push a staging tag and promote after success | A | No push before composed passes; the release requires the web/tooling job; a seeded failure blocks | Open | — |
| AUD-2026-11-17-26 | Major | data/partitions | `portfolio_holdout` 2025 includes `COPPERCMDUSD` and `TSLAUSUSD`, but the committed data-change record declares coverage 2018-01-01..2024-12-31 for both | `data/partitions.json:109-123`; `docs/config/2026-11-08/data-change-MR7-01.md:14-19,42-45`; the record's holdout sentence claims they inherit the partition windows | The one-shot holdout cannot cover its declared instruments; a holdout run fails or silently uses a shortened sample | A. Admit 2025 COPPER (and TSLA if needed) and update the data-change record; B. Remove COPPER (and any uncovered instrument) from the holdout or declare an explicit tested subset | B until the data exists | A coverage check fails a holdout instrument without 2025 data; the data-change record matches the partition | Fixed | #881 |
| AUD-2026-11-17-27 | Major | clients/remote | The documented remote origin is plain `http://<server>:8080`, which is not a secure context, so the browser's artifact verification (`crypto.subtle`) and digest copy (`navigator.clipboard`) are unavailable; `run fetch`/`stage fetch` fail in the browser | `apps/web/src/lib/artifacts.ts:24`; `apps/web/src/lib/RunDetail.svelte:29-33`; `docs/architecture/deployment.md:155`; `docs/manual/08-operations.md:63` | The remote browser cannot download or verify run/stage artifacts; the digest copy silently does nothing; the shell is unaffected | A. Add a pure-JS SHA-256 fallback (or require HTTPS) and show an explicit secure-context error; B. Require HTTPS for remote access and document it | A | A remote HTTP origin downloads and verifies an artifact, or shows a clear error; a non-secure context test | Open | — |
| AUD-2026-11-17-28 | Major | clients/state | Run, stage, and research detail reads are guarded only by the profile `generation` counter, not by the selected id, so a late response for A can overwrite selection B; actions then use the wrong record's id | `apps/web/src/lib/runs.svelte.ts:65-85`; `apps/web/src/lib/stages.svelte.ts:237-304`; `apps/web/src/lib/research.svelte.ts:104-118` | A late response can show run A's digest under selected run B; Cancel/Delete/Decide can act on the wrong record | A. Capture the selected id in each read and discard a response unless it still matches; B. Use a per-selection request token incremented by `select`/`clearSelection` | A | A browser test with two rapid selections and out-of-order responses shows B, not A | Open | — |
| AUD-2026-11-17-29 | Major | labels/contract | Model labels are not one table: the server artifact renderer maps subtype `(1,1)` to `test` and `(1,4)` to `break_of_structure`, the browser generated table maps them to `test/retest` and `break of structure`, and the evidence service renders timeframe words | `crates/quantrule-app/src/artifact/labels.rs:54-60`; `apps/web/src/lib/labels.generated.ts:10-15`; `crates/quantrule-evidence-service/src/routes/research/compare.rs:323-332` | The same event code reads differently in `summary.txt` and the chart; the one-generated-table invariant is broken; the service-side table can drift independently | A. Generate both the Rust artifact labels and the TypeScript table from the registry, with a gate comparing the words; B. Remove labels from server artifacts and publish codes/seconds | A | A gate compares the server and client words; no service renders a label | Open | — |
| AUD-2026-11-17-30 | Major | clients/simulation | The browser's `simulation create` sends context `inputs`; the shell has no `--inputs` and sends an empty value, so the two clients create different inputs digests; the manual/tutorial pass `--bundle <data_version>` although the service reads `simulation/bundles/<sha256>.json`, and no current producer creates that bundle | `apps/web/src/lib/simulation.ts:49,110-118`; `crates/quantrule-shell/src/command/parse.rs:541-553`; `crates/quantrule-shell/src/command/dispatch.rs:731`; `crates/quantrule-simulation-service/src/routes/runs.rs:102-109`; `docs/manual/09-command-reference.md:88`; `docs/tutorials/2026-11-14/shell-http.md:138`; `crates/quantrule-simulation-service/src/bundle.rs:8,148` | The clients create different simulation identities, and the documented simulation procedure fails with a missing bundle; the research plane is reachable only by manually seeding object storage | A. Add `--inputs` and a bundle producer/command, update help/README/manual, and add a parity test; B. Remove the browser inputs field and change create to derive a bundle from a run/dataset | A | Both clients create the same inputs digest; a documented producer creates the bundle; the tutorial runs | Open | — |
| AUD-2026-11-17-31 | Major | backtest/execution | On the entry bar, a stop-entry position's bracket uses the bar's full high/low, including the range before the intrabar trigger; the test asserts a stop-out from a low that may precede the entry | `crates/quantrule-backtest/src/execution/engine/exit.rs:21-65`; `crates/quantrule-backtest/src/execution/engine/bracket_entry.rs:88-158`; `crates/quantrule-backtest/src/execution/bracket_entry_tests.rs:302-316` | A stop-entry backtest can record a stop-out/fill path OHLC cannot prove; the conservative path convention is not in the contract | A. Evaluate the bracket from the next bar for a stop entry; B. Document and test the conservative entry-bar convention in the execution contract and regenerate the golden deliberately | B if the conservative rule is intended | A fixture with a pre-trigger low below the stop; the contract states the rule; the golden is updated deliberately | Fixed | #893 |
| AUD-2026-11-17-32 | Minor | strategy/boundary | `EventStrategyInput` includes every event regardless of registry role and exposes `ContextBar`; the boundary test only scans `input.rs` source strings | `crates/quantrule-backtest/src/strategy/event_input.rs:18-40,59-60,186-197,487-512`; `crates/quantrule-backtest/src/strategy/event.rs:367-375`; `crates/quantrule-model/src/context.rs:3-6` | A public strategy implementation, or a future condition, can read a diagnostic event or a raw context value; the boundary invariant and its test do not prove what the docs claim | A. Filter `EventView`s by registry role and add a constructed-snapshot test containing a diagnostic event; B. Keep context only behind a registered context condition and a private-field boundary test | A | A snapshot with a diagnostic event cannot satisfy a rule; a constructed-snapshot test proves it | Fixed | #892 |
| AUD-2026-11-17-33 | Minor | config/run | `RunConfig::load` does not enforce the documented v4 check `strategy.execution_timeframe == execution.fill_timeframe`, and `run submit` accepts a v3 (no-strategy) config; the worker fails asynchronously | `crates/quantrule-app/src/config/run.rs:196-224,303-322`; `crates/quantrule-app/src/stages/strategy/context.rs:74-81`; `crates/quantrule-backtest-service/src/backtest/workspace.rs:47-50`; `docs/config/2026-11-13/run-v4-strategy.md:33`; `docs/config/2026-11-06/strategy-config.md:68` | An invalid v4 config is catalogued as valid and a v3 run is queued, retried, and failed instead of rejected at submission; the documented loader contract is false | A. Add `validate_context` to `RunConfig::load`/catalog validation and reject v3 at `run submit`; B. Route v3 explicitly to a model-only path and change the loader docs to say validation is run time | A | A 5m strategy/15m fill mismatch and a v3 submission are refused at submission; a valid config passes | Fixed | #892 |
| AUD-2026-11-17-34 | Minor | backtest/ledger | The halt is enforced only in `size_entry`; a later `apply_fill`/`open` can open a position while halted, and funding recomputes equity from the previous mark rather than `cash + position_value(price)` | `crates/quantrule-backtest/src/portfolio/ledger.rs:145-150,203-211,316-332`; `crates/quantrule-backtest/src/portfolio/ledger_tests.rs:523-565` | A standalone ledger caller can open after the halt; a funding-boundary drawdown is measured against a stale equity | A. Reject an entry fill when halted and mark at the funding price before `check_halt`; B. Make the halt a runner-only rule and document it | A | A halted ledger refuses an entry fill; a funding charge at a new price uses the marked equity | Fixed | #882 |
| AUD-2026-11-17-35 | Minor | tests/gate | The registry's determinism claims are repeat-call only; nine checkers have no dedicated unit test; port parity registers one target for eight marked modules; the boundary tests are substring scans; the matrix checker does not read the manifest; the base readings empty/single test asserts only finiteness; the no-partial-publish and scale-invariance claims are simulated, not a process kill or a one-vs-N worker comparison | `crates/quantrule-model/tests/per_row_base.rs:129-133`; `crates/quantrule-model/tests/per_row_base_events.rs:199-201`; `tools/checkers.txt` vs `tools/checker_tests.txt`; `docs/port-parity/registry.json:7`; `crates/quantrule-backtest/tests/boundaries.rs:37-57`; `tools/check_verification_matrix.py:35-71`; `crates/quantrule-model/tests/edge_cases.rs:24-27`; `crates/quantrule-backtest-service/tests/worker/cancel.rs:6`; `crates/quantrule-evidence-service/tests/sqlite_transactions.rs:118` | Claims are weaker than the registry says; an iteration-order, checker, or partial-publish regression can pass the gate | A. Add reordered-input tests, a checker meta-test, all port targets, a use-tree boundary, a matrix regeneration check, base-reading null assertions, a real process-kill test, and a one-vs-N worker digest test; B. Narrow the claims to what the tests prove | A | Each claim names a test that exercises it; a seeded regression fails | Open | — |
| AUD-2026-11-17-36 | Minor | clients/commands | The browser has stale local detail state, stage forms default to `cfg_abc`, the desktop shell inherits `QR_DESKTOP_PROFILE`, `parity_desktop.sh` is not in the release workflow, a malformed token blocks `help`, the README overstates one-call/global-flag uniformity, `simulation check` prints JSON where the table is documented, and the parity fixture exercises 33/46 shell and 32/46 browser commands | `apps/web/src/lib/ConfigsPanel.svelte:15,32-38,144-160`; `apps/web/src/lib/{Model,Strategy,Portfolio}StagePanel.svelte:20-21`; `apps/web/src/lib/simulation.ts`; `apps/desktop-shell/src/main.rs:73-77`; `apps/desktop-shell/supervisor/src/cli.rs:4-5,310-313`; `crates/quantrule-shell/src/command/dispatch.rs:43-45,852-853`; `README.md:174-177`; `docs/architecture/command-surface.md:29-31`; `docs/manual/09-command-reference.md:93`; `crates/quantrule-shell/tests/client_parity.rs`; `apps/web/src/lib/clientParity.test.ts:565-586` | Usability, parity, and documentation drift: a profile switch can show a stale config; a first stage submit targets a fixture id; a packaged app can start no sidecars; 13 commands are unexercised by parity | A. Fix each item and extend the parity fixture to all 46; B. Narrow the docs and record the exceptions | A | A profile-switch test, a first-submit test, a packaged launch test, a malformed-token help test, and a 46/46 parity test | Open | — |
| AUD-2026-11-17-37 | Info | tests/tooling | `check_event_emitters` treats `row_signal_tests.rs` and `row_tests.rs`, declared under `#[cfg(test)]`, as production emitters | `crates/quantrule-model/src/events/extended.rs:181-184`; `tools/check_event_emitters.py:34-42` | A test-only reference can mask a missing production detector; no current false pass was found | A. Exclude `#[cfg(test)]` modules and `*_tests.rs` from production sources; B. Derive the set from the module graph | A | Removing a production emitter fails the checker even when the test file references the code | Fixed | #904 |
| AUD-2026-11-17-38 | Critical | services/lifecycle | `PgRunStore::cancel` reads the run, checks `is_cancellable`, then issues an unconditional `UPDATE ... SET status='cancelled'`; a concurrent `complete` can commit `succeeded` between the read and the update, and the cancel overwrites it | `crates/quantrule-evidence-service/src/postgres.rs:352-364,520-545`; the stage store's conditional update at `crates/quantrule-evidence-service/src/stage/postgres.rs:542-560`; `docs/architecture/control-plane.md:103-105` | A succeeded run can become cancelled while retaining its digest and artifacts; `/runs/{id}/digest` returns a digest for a cancelled run; a second concurrent cancel returns `202` instead of `409` | A. Make the update conditional (`AND status IN ('queued','running')`), inspect `rows_affected`, and return `Conflict` on zero; B. Wrap the read/check/update in a transaction with `SELECT ... FOR UPDATE` | A | A concurrent cancel/complete Postgres test leaves the run succeeded or cancelled, never both; a second cancel is a conflict | Fixed | #881 |
| AUD-2026-11-17-39 | Major | services/health | The gateway registers only its own database probe; the shared report hardcodes `queue_depth: 0` and `workers: 0`, and `/health` always returns HTTP `200`, even when the aggregate state is `down` | `crates/quantrule-gateway/src/bin/qr_gateway.rs:27-36`; `crates/quantrule-runtime/src/health.rs:77-85`; `docs/architecture/http-api.md:31`; `docs/architecture/operations.md:18-29` | A data/model/evidence/simulation/queue/object-store outage still reports healthy; the documented worker and queue-depth signals can never fire; `make up`, the browser, and the shell all see a healthy gateway | A. Aggregate each service's `/health`, queue depth, and a worker heartbeat, and return `down`/`503` when a required dependency is down; B. Narrow the documented contract to a local liveness probe and add a separate `/ready` endpoint | A | Killing `qr-data` makes the gateway report `down`/`503`; a healthy stack reports workers and queue depth | Open | — |
| AUD-2026-11-17-40 | Major | services/queue | `RabbitQueue::publish` awaits `basic_publish` and discards the returned confirmation; the channel is not in confirm mode | `crates/quantrule-runtime/src/adapters/rabbitmq.rs:111-127`; `docs/architecture/services.md:87`; `docs/architecture/control-plane.md:74-78` | Evidence can return `202` with a `queued` row and no durable `run.requested` message; a client that does not resubmit leaves the run stranded; "nothing is silently dropped" is false | A. Enable publisher confirms (`confirm_select`) and await `Confirmation::Ack`, returning `503` on a nack/timeout; B. Add a transactional outbox or a republish sweeper for `queued` runs with no message | A | A closed channel after publish returns an error; a queued run with no message is recovered | Open | — |
| AUD-2026-11-17-41 | Major | services/concurrency | The run and stage consumers share one AMQP channel and one `basic_qos(1)` slot; `QR_BACKTEST_CONCURRENCY` is parsed but never read by a worker | `crates/quantrule-runtime/src/adapters/rabbitmq.rs:141-166`; `crates/quantrule-backtest-service/src/main.rs:211-226`; `crates/quantrule-runtime/src/config.rs:226-239`; `docs/architecture/control-plane.md:70-72,107-112`; `docs/architecture/deployment.md:135` | A long diagnostic stage occupies the only unacked slot and delays an evidence run; the documented isolation and the concurrency knob do nothing | A. Open a separate channel per consumer with its own `basic_qos(1)` and use `QR_BACKTEST_CONCURRENCY` to spawn N run consumers; B. Use separate processes or connections for the stage and run consumers | A | A composed test shows an evidence run is not blocked by a stage; setting the concurrency variable changes the run throughput | Open | — |
| AUD-2026-11-17-42 | Minor | services/retention | Stage retention is a single startup cleanup, not a running TTL; a never-restarted stack never expires stage rows or blobs | `crates/quantrule-evidence-service/src/main.rs:126-160`; `docs/architecture/stage-surface.md:27`; `compose.yaml:41-44`; `docs/architecture/operations.md:120-122` | The documented retention window is not enforced while the process runs; stage rows/objects grow until an operator restarts the service | A. Run the cleanup on a tokio interval or before stage submission/list; B. Document the sweep as startup-only and change the TTL wording | A | A long-running test with an expired stage removes the row/blob without a restart; the docs match | Open | — |
| AUD-2026-11-17-43 | Minor | docs/services | The service catalog says `backtest` must never own provenance or compute a digest, but the worker builds `RunManifest`, computes the canonical digest, and writes the manifest; the data-plane document assigns that step to the worker | `crates/quantrule-backtest-service/src/backtest/worker.rs:273-291`; `crates/quantrule-service-api/src/manifest.rs:54-71`; `docs/architecture/services.md:21`; `docs/architecture/data-plane.md:20-22`; `crates/quantrule-evidence-service/src/evidence/artifacts.rs:46-61` | Two documents disagree; evidence re-verifies today, but two digest implementations can drift and the "must never" is unenforced | A. Align the catalog with the data plane ("worker proposes; evidence owns and verifies") and add a guard; B. Move manifest/digest construction into evidence and have the worker send artifact refs | A | The two documents agree; a boundary test fails when the worker writes a manifest without evidence verification | Fixed | #895 |
| AUD-2026-11-17-44 | Minor | services/client | The internal client gives every proxied call a fixed 30 s deadline, including large uploads, model compute, and artifact downloads; the contract also drifts on `created`, correlation ids, `If-None-Match`, a non-enveloped simulation `chart` rejection, and missing `stage.*` error rows | `crates/quantrule-runtime/src/client.rs:287-289,375-377`; `docs/architecture/http-api.md:46,84-86,123-147`; `crates/quantrule-api/src/run.rs:277-284`; `crates/quantrule-data-service/src/routes.rs:457-466`; `crates/quantrule-simulation-service/src/routes/runs.rs:196-200` | A slow but successful upload/compute is reported as a failure and requires a retry; clients cannot rely on the documented envelope/error table; the correlation-id claim is false | A. Use route-specific or configurable deadlines for streaming uploads/downloads and compute, with a documented retry contract; B. Implement or remove the `If-None-Match`/correlation-id claims, add an envelope-returning chart rejection, and document `created`/`stage.*` | A | A slow upload/compute succeeds or returns a documented retry; the error table matches every route | Open | — |
| AUD-2026-11-17-45 | Major | model/session | Session VWAP treats a missing candle volume as zero, so a session with one missing-volume bar publishes a fabricated `session_vwap`; `level.session_vwap_test` can fire against it | `crates/quantrule-model/src/sessions.rs:158-161`; `docs/model/2026-11-05/indicator-coverage.md:51-53`; `crates/quantrule-model/src/events/session.rs` (`level.session_vwap_test`); no test covers missing volume for session VWAP | A missing measurement is replaced by zero in a volume-weighted reading, and a signal event can read it; this is the same class as the prior Critical `AUD-2026-11-16-01`, which fixed the extended readings but not the session path | A. Track session volume completeness and publish `session_vwap` as null from the first missing-volume bar for the rest of the session; B. Reject or flag missing volume for session-aware runs at ingestion | A | A session with a missing-volume bar publishes null VWAP; `level.session_vwap_test` cannot fire; a regression test covers the session path | Fixed | #883 |
| AUD-2026-11-17-46 | Major | model/divergence | The configurable divergence RSI window can read past the bar on which the event claims to be knowable: `end = occurred_at + window` while `available_bar_index` is the pivot confirmation bar | `crates/quantrule-model/src/events/divergence.rs:255-268,321-332`; `crates/quantrule-app/src/config/model_v4.rs:325-333`; the default major/2 is safe; no test sets window above the lookback | A divergence is a signal event; a `window` above the pivot lookback reads RSI bars after the confirmation bar, a look-ahead in a configurable event | A. Reject or clamp `window <= pivot_scale.lookback()` at config load; B. Compute only up to `pivot.available_at` and set the recorded availability to the end of the RSI window | A | A config with `window > lookback` is refused; a prefix test proves the event does not change when later bars are appended | Fixed | #884 |
| AUD-2026-11-17-47 | Major | model/extended | With a valid `donchian_window = 1`, `donchian_position[0]` is `Some`, and the event code evaluates `extended.donchian_position.get(bar - 1)` at `bar == 0`; debug builds panic, release wraps to `usize::MAX` and emits no event | `crates/quantrule-model/src/events/extended/signals.rs:243-245`; `crates/quantrule-model/src/readings/extended/structure.rs:154-167`; `crates/quantrule-model/src/spec/v2.rs:803` accepts any `donchian_window > 0` | A valid configuration can panic or silently skip a position event; the Keltner path has the same unguarded pattern | A. Add `bar > 0` to both position conditions; B. Require `donchian_window >= 2` (and the Keltner window) in config validation | A | A `donchian_window = 1` config is either refused or produces a safe null/absent event; a test covers bar 0 | Fixed | #885 |
| AUD-2026-11-17-48 | Major | model/config | `structure.scale` is parsed and serialized but never read; structure readings and events hard-code `PivotScale::Major` | `crates/quantrule-model/src/input.rs:193,206,208`; `rg -n structure_scale crates/quantrule-model/src` returns only `input.rs`; `docs/model/2026-11-05/model-m1.0.md:32-34`; `docs/registry/indicators.md:228` | A `structure.scale: "minor"` config silently produces major-scale structure output; the registry claims a behavior that does not exist; no test sets the scale | A. Plumb the configured scale into the structure readings/events; B. Remove the field from config/registry and update the docs | A | A minor-scale config changes the structure output; a registry/config test covers the scale | Fixed | #887 |
| AUD-2026-11-17-49 | Major | model/readings | VWAP slope subtracts `vwap[bar + 1 - window]` instead of the documented `vwap[bar - window]`, so a 24-bar window uses a 23-bar change and the first value appears one bar late | `crates/quantrule-model/src/readings/extended.rs:420-435`; `docs/model/2026-11-05/indicator-coverage.md:59`; peer readings `poc_migration`/`obv_slope` use `bar - window` | Every `vwap_slope` value is off by one bar; `signal.vwap_slope` events inherit the drift, and the golden fixture locks it in | A. Change the prior index to `bar - window` and regenerate the affected fixtures; B. If the non-overlapping window was intended, correct the registry formula/name and add a test | A | The formula matches the contract; a unit test checks the prior index; the golden is regenerated deliberately | Fixed | #886 |
| AUD-2026-11-17-50 | Minor | model/readings | ATR gates non-ATR outputs: `vwap_band_upper`, `vwap_band_lower`, and `vwap_slope` are null without ATR, and `range_expansion` requires ATR although its formula does not | `crates/quantrule-model/src/readings/extended.rs:420-435`; `crates/quantrule-model/src/readings/extended/structure.rs:129-133`; registry formulas at `docs/model/2026-11-05/indicator-coverage.md:59,90` | A run with ATR disabled loses valid VWAP bands/slope and range-expansion readings, contradicting their registered availability | A. Decouple these computations from ATR; B. Register ATR as a dependency and update availability/tests | A | A run with ATR disabled still publishes the non-ATR readings; the registry matches | Fixed | #888 |
| AUD-2026-11-17-51 | Minor | registry/model | `session_vwap` is published in `session.csv` and used by `level.session_vwap_test`, but the `session` indicator's registered outputs are only `session_code` and `session_age` | `crates/quantrule-app/src/artifact/model_bundle/artifacts.rs:305`; `docs/registry/indicators.json` (`session` outputs); the event at `:4387` | The registry is not the complete output contract; the event's input reading cannot be discovered or validated from the registry | A. Add `session_vwap` to the `session` outputs/formula; B. Move it to a separate registered indicator with its own claim and tests | A | The registry lists every published reading; a registry test covers the session outputs | Fixed | #888 |
| AUD-2026-11-17-52 | Minor | docs/data | `docs/config/2026-10-30/data-v3.md` requires the instrument and timeframes to be in a policy universe, but the policy document is `Historical`; the loader only checks that the instrument is non-empty and the timeframes are ordered/1m-derived | `docs/config/2026-10-30/data-v3.md:43,57-59`; `docs/config/2026-10-30/policy-and-strategy.md:3-4`; `crates/quantrule-app/src/config/data.rs:90-96` | The documented admission check does not exist; a data config with an unknown instrument is accepted | A. Remove the policy-universe clauses from data-v3; B. Validate against a current universe such as `data/partitions.json` | A | The data-v3 document and the loader agree; an unknown instrument is refused | Fixed | #894 |
| AUD-2026-11-17-53 | Info | model/API | `retention.rs` says the event batch/history contains every event, but `timeframe.rs` builds both views from the base `events` vector only; extended events exist only in `merged_events` | `crates/quantrule-model/src/retention.rs:5-8`; `crates/quantrule-model/src/timeframe.rs:296-305` | A public consumer of the batch/history misses extended events; the retention test only exercises base events; no production consumer was found | A. Build both views from `merged_events`; B. Rename/document them as base-only and adjust the retention contract | A | The batch/history includes a known extended event; the retention test covers it | Fixed | #894 |

## Verification note (2026-11-20)

The browser suite passed locally: 146 `bun test` cases and 53 Playwright cases,
with `bun run check` and `bun run build`. The composed tier passed on a clean
stack: both verify-mini runs published 13 canonical artifacts and the gateway
recomputed `sha256:a0372f02…`. The first composed attempt exposed a stale MinIO
bundle and the second exposed `tools/verify.sh` excluding `metrics.json`; the
verifier was fixed and the third run passed.

These runs remove the browser and composed evidence blockers for the remaining
P4/P5 work, but they do not close the specific findings that still need their
own tests: the health kill test, queue confirms/concurrency, Postgres retention,
restore round-trip, remote HTTP browser origin, selection race, labels, and
simulation parity.

## Remediation status (2026-11-17)

The P1 Critical, P2 model/evidence, P3.2 current-document, and P3.3 AI-log work
items are merged. Their 36 findings are `Fixed` with the pull-request link in the
table; the P3.1 release/audit closure and the P4–P7 service, client, test, and
closure work items remain `Open`. The Improvement Verifier must move a `Fixed`
finding to `Verified`; this audit does not self-verify.

## Detail

### AUD-2026-11-17-01 — the normative workspace map omits two current crates

- **Expected invariant:** `AGENTS.md` section 2 is the normative description of
  the workspace and the service graph; a reader can name every crate and knows
  which layer it belongs to (`AGENTS.md:48`).
- **Observed:** The list names fifteen crates and ends with
  `quantrule-simulation-service` and `quantrule-shell`. `Cargo.toml` has
  seventeen members, including `quantrule-research` and `quantrule-simulation`.
  The evidence service depends on `quantrule-research`
  (`quantrule-evidence-service/Cargo.toml:17`) and the simulation service on
  `quantrule-simulation` (`quantrule-simulation-service/Cargo.toml:18`). The
  same paragraph says "every pipeline service is a thin adapter over
  `quantrule-app`", but `services.md:39-52` says data, model, and backtest share
  `quantrule-app` and evidence and simulation use their own domain crates.
  The paragraph after the list says "there is no current analytics crate"; the
  module map lists `quantrule-research` as a current crate.
- **Root cause:** The distributed-platform rewrite added the research and
  simulation domain crates and their services, but the normative enumeration in
  `AGENTS.md` was not updated with them. The adapter sentence was written for
  the three pipeline services and then applied to all six.
- **Impact:** An agent that follows the normative entry point does not know
  where hypothesis evaluation or simulation-domain changes belong. An agent that
  trusts "no current analytics crate" may treat a live dependency as retired.
- **Affected contracts/docs:** `AGENTS.md` §2, `docs/architecture/module-map.md`,
  `docs/architecture/services.md`.
- **Dependencies:** None.
- **Effort:** Small; a normative text change.
- **Options:**
  A. Add `quantrule-research` and `quantrule-simulation` to the list, qualify
  the analytics sentence, and narrow the adapter sentence to data/model/backtest.
  B. Replace the enumeration with a link to `module-map.md` and keep only the
  invariants in `AGENTS.md`.
  C. Leave the text and rely on the module map (rejected because `AGENTS.md`
  is the normative entry point).
- **Recommendation:** A.
- **Verification:** The list equals the `Cargo.toml` members; the service
  sentence matches `services.md`; `python3 tools/check_structure.py` and
  `python3 tools/check_links.py` pass.
- **Links:** None yet.

### AUD-2026-11-17-02 — the workflow narrative is not the workflow index

- **Expected invariant:** `AGENTS.md` section 1 says the workflows are indexed in
  `docs/workflows/README.md`; the index is the single source of workflow status
  (`docs/workflows/README.md:44-67`).
- **Observed:** `AGENTS.md:11` names "a human-owned strategy blueprint", "an
  Engineering Swarm", and "a Quant Swarm" among the indexed workflows. The
  index has no strategy-blueprint workflow, no Engineering Swarm, and no Quant
  Swarm; it has a Research Swarm, and section 10 defines the Engineering and
  Quant swarms as role groups. The strategy blueprint is a retired policy
  artifact (`docs/README.md:125`; `docs/config/2026-10-30/policy-and-strategy.md`).
  Stage 0 (`AGENTS.md:1134-1138`) lists Audit, Refactor, Model Research, Model
  Change, Strategy Research, Portfolio Research, Data Lifecycle, Requirements &
  Design, Release & Operations, Deprecation & Retirement, Recordkeeping, or the
  Improvement Workflow, and omits Simulation Research, Engine Execution,
  Backtest, End-to-End Verification, and Execution Plan, all of which are
  `Current` in the index.
- **Root cause:** The methodology reset replaced the policy/strategy blueprint
  and the swarm-shaped workflow narrative with the current workflow index, but
  the opening paragraph and Stage 0 list were not reconciled with it.
- **Impact:** An agent choosing a workflow is pointed at documents that do not
  exist and is not pointed at five workflows that do. The human-owned strategy
  is a blueprint artifact, not a workflow; conflating the two hides the actual
  strategy-research route.
- **Affected contracts/docs:** `AGENTS.md` §1, §13; `docs/workflows/README.md`;
  `docs/README.md:125`; the retired policy blueprint.
- **Dependencies:** A workflow change is a human-approved methodology change;
  this finding is a documentation correction, not a workflow change.
- **Effort:** Small.
- **Options:**
  A. Align the opening sentence and Stage 0 list with the index.
  B. Add the missing workflow documents (requires human approval and would
  contradict the index's single-source rule).
  C. Link the index without an enumeration.
- **Recommendation:** A.
- **Verification:** Every workflow named in `AGENTS.md` is in the index, and
  every index workflow is named or reached by the Stage 0 link.
- **Links:** None yet.

### AUD-2026-11-17-03 — the v1.3.0 status tables lag the release

- **Expected invariant:** A `Current` workflow is usable now; a `Target`
  workflow is an approved direction that is not yet usable
  (`docs/workflows/README.md:161-168`). A document marked `Current` describes
  the current release.
- **Observed:** `docs/workflows/engine-execution.md:4` says
  `Status: Target; the contract is approved, the stage commands land in
  #850–#854`; the index row at `docs/workflows/README.md:50` repeats `Target`
  and "Commands land in #850–#854". `docs/README.md:42` says those commands and
  the stage surface are implemented, and the v1.3.0 release note lists the
  Engine Execution workflow as included. `stage-surface.md:158` says the manual
  is `Target (#859)` while line 4 of the same document says the manual landed in
  #859 and `docs/README.md:42` says it is implemented. `http-api.md:3` says
  `released as v0.9.0 ... unchanged in the v1.0.0 desktop release`, while the
  document includes research and operations routes added for v1.3.0.
- **Root cause:** The release-time status propagation updated the release note,
  the status index, and the changelog, but did not update the workflow index,
  the stage-surface status table, or the HTTP API status line.
- **Impact:** A shipped diagnostic workflow is presented as not usable, so an
  agent may avoid it or route a diagnostic run through the wrong workflow. The
  status index cannot be trusted at the release boundary.
- **Affected contracts/docs:** `docs/workflows/engine-execution.md`,
  `docs/workflows/README.md`, `docs/architecture/stage-surface.md`,
  `docs/architecture/http-api.md`.
- **Dependencies:** A workflow status change is a human-approved methodology
  change; this finding recommends recording the status that the release already
  implies.
- **Effort:** Small; a status-line update plus a checker.
- **Options:**
  A. Update the status lines and add a release-time cross-check between a
  `Target` workflow and the command/route registry.
  B. Mark the workflow historical (wrong: it is current).
  C. Leave it and rely on the release note (rejected: the index is the source).
- **Recommendation:** A.
- **Verification:** `check_workflows` rejects a `Target` workflow whose commands
  are implemented; the stage-surface status table has no `Target` row for
  shipped work; `http-api.md` names v1.3.0.
- **Links:** None yet.

### AUD-2026-11-17-04 — a run or stage without an image digest records a fabricated one

- **Expected invariant:** A run's `image_digest` is "reported by `/health` and
  recorded in provenance" (`docs/workflows/run-contract.md:369-372`); the same
  value is one of the four run-identity components.
- **Observed:** When `state.version.image_digest` is `None`, the evidence
  service computes `Digest::sha256(b"no-image-digest")` and uses it for the run
  (`routes/runs.rs:122-126`) and for a stage (`routes/stages.rs:195-199`).
  `/health` reports the same field as `None` (`runtime/src/health.rs:60,84`).
  `compose.yaml:52-54` leaves `QR_IMAGE_DIGEST` empty, and
  `docker compose config` resolves it to `null`; the documented `make up` path
  therefore starts the server without a digest. The desktop supervisor now
  computes a package digest, so the packaged desktop path is correct; the
  server/local path is not.
- **Root cause:** The evidence service treats a missing version identity as a
  value to fill in rather than a precondition to refuse. The desktop fix added a
  digest at the supervisor boundary, but the server boundary still has the
  fallback.
- **Impact:** A local or misconfigured server run claims a fixed build identity
  it did not run. Two different builds with the same data, config, and model
  produce the same `run_id`; the second submission returns `Existing` and
  reuses the first build's run and artifacts. Provenance and reproducibility
  claims are false, and `/health` disagrees with the record.
- **Affected contracts/docs:** `docs/workflows/run-contract.md`,
  `docs/architecture/data-plane.md`, `docs/architecture/deployment.md`,
  `compose.yaml`, the evidence run and stage routes.
- **Dependencies:** The fix touches run identity; it needs the Run Contract
  owner's review and a human decision on the local profile behavior.
- **Effort:** Small to medium; a typed refusal plus a `make up` digest.
- **Options:**
  A. Refuse a run or stage submission when the server has no image digest, and
  set it in `make up` from the built image.
  B. Embed the build digest at compile time and use it when the environment is
  silent.
  C. Record `null` and exclude it from identity (a run-contract change that
  requires human approval).
- **Recommendation:** A.
- **Verification:** A server without `QR_IMAGE_DIGEST` returns a typed error;
  a configured digest appears in `/health` and provenance; two builds produce
  different run ids; the composed tier still passes with the image digest set.
- **Links:** Prior `AUD-2026-11-16-02` covered the desktop path; this finding
  covers the server/local path that remains.

### AUD-2026-11-17-05 — the release proceeded without audit closure

- **Expected invariant:** A release is created only when the gate passes, the
  changelog and note exist, no Critical audit finding is open on the changed
  path, and the human authorizes it (`docs/releases/README.md:42-51`);
  the Audit Workflow is green only when every Critical and Major finding is
  `Verified` or the human has recorded a dated `Won't fix` with residual-risk
  acceptance (`docs/workflows/audit-workflow.md:102-119`); no release proceeds
  with an open Critical finding unless the human records residual-risk
  acceptance (`AGENTS.md:1230`).
- **Observed:** At the audit baseline, the `v1.3.0` tag was on the remote
  (`9e764f22…`); the release note at
  `docs/releases/2026-11-17/v1.3.0.md` still said `Prepared`, and the GitHub
  release did not yet exist while the release workflow was running.
  The prior audit is `Published; 22 findings Fixed by PR #828; independent
  verification pending` (`docs/audits/2026-11-16/full-program-audit.md:5`). Its
  Critical finding `AUD-2026-11-16-01` and seven Major findings are `Fixed`,
  not `Verified`. The remediation plan is `In Review`
  (`docs/execution-plans/2026-11-16/full-program-audit-remediation.md:3`) and
  all 38 verification rows are `Pending`. The v1.3.0 note's verification table
  does not name the audit state, and no `Won't fix` or residual-risk acceptance
  for the `Fixed` findings is recorded.
- **Root cause:** The release workflow's gate is the workspace gate and the
  composed tier; it has no check on the audit status. The audit remediation plan
  is a separate work item that was still in review when the release was cut.
- **Impact:** A consumer of the release cannot tell whether the Critical fix
  was independently verified. The Audit Workflow's green criteria and the
  release precondition are not evidenced; the release note presents a verified
  surface without qualifying the audit state.
- **Affected contracts/docs:** `docs/releases/README.md`,
  `docs/workflows/audit-workflow.md`, `docs/audits/2026-11-16/full-program-audit.md`,
  `docs/execution-plans/2026-11-16/full-program-audit-remediation.md`, the
  v1.3.0 note, `release.yml`.
- **Dependencies:** The Improvement Verifier role and the human acceptance
  gate; the release tag is already published, so the fix is forward-looking.
- **Effort:** Medium; a verifier pass, a note update, and a release-gate check.
- **Options:**
  A. Complete the Improvement Verifier pass and update the audit before the
  next tag.
  B. Record a dated human residual-risk acceptance for the `Fixed` Critical and
  Major findings and reference it in the release note.
  C. Add a release-gate check that fails when a Critical/Major finding on the
  changed path is not `Verified` or `Won't fix`.
- **Recommendation:** A + C.
- **Verification:** The v1.3.0 note names the audit status; the audit findings
  move to `Verified`; a release with an unverified Critical fails the gate.
- **Links:** `AUD-2026-11-16-01` to `-08`; PR #828; the remediation plan.
- **Planner note (2026-11-17):** The v1.3.0 note was updated on current `main`
  (`7ae3a984`) to `Released`, with the image digest and desktop assets. The
  note-status evidence in this finding is corrected by this dated note; the
  audit-closure part remains: the prior audit is still
  `Fixed; independent verification pending`, and its remediation plan is
  `In Review` with unresolved rows. The release is now published, so the fix is
  forward-looking, not a pre-tag gate.

### AUD-2026-11-17-06 — the audit backlog is untriaged

- **Expected invariant:** The Planner verifies each finding against the current
  code, marks stale or duplicate findings, and moves the finding to
  `Triaged`/`Planned`; execution moves it to `Fixed`; only the Improvement
  Verifier moves it to `Verified` (`docs/workflows/audit-workflow.md:40-70`;
  `AGENTS.md` §9 "Audit, refactor, and improvement tracking").
- **Observed:** An audit-table scan finds 90 findings across the audit
  documents: 5 Major `Open`, 24 Major `Fixed`, 1 Critical `Fixed`, 1 Info
  `Fixed`, 6 Minor `Open`, 21 Minor `Fixed`, and the remainder
  `Verified`/`Stale`/`Triaged`. The five `Open` Major findings are
  `AUD-2026-11-01-70` (2023 crypto gaps), `AUD-2026-11-11-04` (shared Cargo
  target), and `AUD-2026-11-11-11`–`-13` (native Tauri window). The 2026-11-16
  audit's "Prior open findings" table recommends `Stale`/`Duplicate` for several
  of them, but the old documents still say `Open`; the Planner did not apply the
  status changes. The `AUD-2026-11-06` and `AUD-2026-11-07` documents still say
  independent verification is pending.
- **Root cause:** The 2026-11-16 audit left the prior-finding triage to the
  Planner, and the remediation plan's verification row for the triage is still
  `Pending`. No follow-up pass moved the statuses.
- **Impact:** The audit status record is not a reliable source of truth. A
  reader cannot tell which old findings are stale, which are fixed, and which
  remain; the audit green criteria cannot be evaluated; the release gate in
  `AUD-2026-11-17-05` has no trustworthy input.
- **Affected contracts/docs:** the six audit documents listed above;
  `docs/workflows/audit-workflow.md`; `AGENTS.md` §9.
- **Dependencies:** The Planner role; a human decision for `Won't fix`; no code
  change.
- **Effort:** Medium; a triage pass over 30 findings.
- **Options:**
  A. The Planner triages every old finding with a dated note and a status in
  the vocabulary.
  B. Mark the pre-v1.3.0 audit documents historical and start a new baseline.
  C. Leave them and rely on this audit (rejected: the status record is the
  contract).
- **Recommendation:** A.
- **Verification:** Every old finding has a status in the vocabulary and a
  dated note; no Critical/Major remains `Open` without a decision; a checker
  reports the backlog.
- **Links:** `docs/audits/2026-11-16/full-program-audit.md:735-747`.

### AUD-2026-11-17-07 — the HTTP contract omits the stage routes

- **Expected invariant:** The HTTP contract is the client-facing route
  reference; the stage-surface document says "The implemented base contract is
  in `http-api.md`" and lists the stage routes
  (`docs/architecture/stage-surface.md:36`).
- **Observed:** `docs/architecture/http-api.md:20-53` lists `/health`,
  `/profiles`, `/version`, `/operations`, `/hypotheses`, `/catalog`,
  `/datasets`, `/model`, `/runs`, `/inspect`, and `/research`. It does not list
  `POST /stages/{kind}/runs`, `GET /stages/runs`, `GET /stages/runs/{id}`,
  `GET /stages/runs/{id}/artifacts`, the download route, `cancel`, or `delete`
  (seven stage routes; the stage-surface table lists them at lines 40-50).
  The proxy paragraph mentions `/stages/*`, so the omission is in the endpoint
  table, not the narrative.
- **Root cause:** The stage surface added its routes to `stage-surface.md` and
  the gateway route table, but the base HTTP table was not extended.
- **Impact:** A client implementer reading the canonical HTTP contract cannot
  find the stage routes; the two documents disagree about where the contract
  lives.
- **Affected contracts/docs:** `docs/architecture/http-api.md`,
  `docs/architecture/stage-surface.md`, the gateway route table.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Add the stage routes to `http-api.md`.
  B. State that `stage-surface.md` is the extension contract and link it from
  the table.
  C. Generate the table from the gateway route registry.
- **Recommendation:** A.
- **Verification:** Every gateway route appears in `http-api.md` or a linked
  extension; a route test fails on a missing row.
- **Links:** None yet.

### AUD-2026-11-17-08 — a blank line breaks the committed run index

- **Expected invariant:** A Markdown table is a contiguous header, separator,
  and row block; the committed run index is a record that renders as one table.
- **Observed:** `docs/improvement/index.md:46` is a blank line after the
  `STRATEGY-2026-11-08-02` row; line 47 starts `| PROGRAM-2026-11-04-05 |` with
  no header or separator. The rows after the blank line render outside the
  table's columns.
- **Root cause:** Later rows were appended after a blank line instead of into
  the existing table.
- **Impact:** The committed run index is visually broken; a reader can miss the
  later run rows.
- **Affected contracts/docs:** `docs/improvement/index.md`.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Remove the blank line and keep the append order.
  B. Regenerate the index from the run store.
  C. Leave it (rejected: the index is a record).
- **Recommendation:** A.
- **Verification:** A Markdown table lint or a check that no blank line
  interrupts a table.
- **Links:** None yet.

### AUD-2026-11-17-09 — the Postgres conformance suites are orphaned

- **Expected invariant:** The composed tier runs against the real Compose stack
  (services, database, queue, and object store) and is required before a release
  (`AGENTS.md` §4). The SQLite and Postgres stores share one conformance body
  (`crates/quantrule-evidence-service/tests/sqlite_conformance.rs:1-10`).
- **Observed:** Eight tests are `#[ignore = "needs QR_TEST_DATABASE_URL pointing
  at an empty, disposable Postgres"]` (data, evidence, simulation, gateway,
  runtime). `grep -rn QR_TEST_DATABASE_URL tools .github Makefile` returns no
  match; `tools/composed.sh:26-52` checks `/health` and `/version` and runs
  `verify.sh`; it never runs `cargo test -- --ignored`. The v1.3.0 note's risk
  section says "Opt-in Postgres research tests... run only with a disposable
  Postgres; the Compose tier exercises Postgres for the service path".
- **Root cause:** The Postgres conformance suites are opt-in by environment
  variable, and no tier sets it; the composed tier exercises the service path
  but not the direct store bodies.
- **Impact:** The production Postgres store's direct conformance evidence is
  not produced by any automated tier; a SQLite/Postgres divergence is caught
  only through the gateway path, if at all. The risk is disclosed but not
  covered.
- **Affected contracts/docs:** `AGENTS.md` §4, `docs/workflows/release-operations.md`,
  the six service test suites, `tools/composed.sh`.
- **Dependencies:** A disposable Postgres in the composed/release job.
- **Effort:** Medium.
- **Options:**
  A. Add a disposable Postgres service and run the ignored suites in the
  release/composed tier.
  B. Add `tools/test_tiers.sh postgres` and require it before a release.
  C. Mark the suites `Proposed` and record the exception (honest but no
  coverage).
- **Recommendation:** A.
- **Verification:** The tier runs the eight ignored tests against a disposable
  Postgres; a seeded divergence fails.
- **Links:** None yet.

### AUD-2026-11-17-10 — an uploaded config can reference a path outside its bundle

- **Expected invariant:** A client never sends a filesystem path; it sends an
  identifier (`AGENTS.md` §3). The upload validator refuses a bundle key that is
  absolute or contains `..` (`crates/quantrule-data-service/src/configs/upload.rs:84-113`),
  and the worker refuses the same shape before writing (`workspace.rs:67-70,189-203`).
- **Observed:** The upload validator checks only the **keys of the uploaded
  file map**. The `run.json` body is then parsed by `RunConfig::load`, which
  joins `run.data`, `run.model`, `run.execution`, `run.portfolio`, and
  `run.strategy` to the run file's parent without a confinement check
  (`config/run.rs:96-105`). `scan::record` calls `bundle_files`, which reads each
  referenced path and inserts its bytes into the catalog record under the raw
  path (`configs/scan.rs:95-120`). A run config with
  `"data": "../../../../etc/passwd"` (or an absolute path) makes the data
  service read that file. The worker later refuses the bundle because the
  referenced key is not a safe path, so the run fails with a workspace error.
- **Root cause:** The confinement check was added at the uploaded-key and
  worker boundaries, but not at the `run.json`-reference boundary, and
  `RunConfig::load` is shared with the trusted repository mount.
- **Impact:** The data service reads a file outside the upload bundle and
  stores its bytes in the catalog. The content reaches the internal config
  bundle and the worker workspace; the worker then refuses the unsafe key, so
  the config is valid at upload and unusable at run time. If a future route
  returns the bundle, the read becomes a content disclosure. The immediate
  effect is a boundary violation and a late, confusing run failure.
- **Affected contracts/docs:** `configs/upload.rs`, `configs/scan.rs`,
  `config/run.rs`, `docs/architecture/http-api.md` (config upload), the
  client-path rule.
- **Dependencies:** The run-config contract; the worker already has `safe_key`
  to share.
- **Effort:** Small.
- **Options:**
  A. Validate every path referenced by `run.json` against the uploaded file set
  and the staging root.
  B. Flatten the bundle and assign server-side names.
  C. Apply the worker's `safe_key` to every referenced path before
  `RunConfig::load`.
- **Recommendation:** A + C.
- **Verification:** A bundle with `"data":"../…"` is rejected at upload; a
  valid bundle still works; the escape test covers a referenced path, not only
  a file key.
- **Links:** None yet.

### AUD-2026-11-17-11 — stage retention and delete race a new reference

- **Expected invariant:** A stage blob is removed only when no remaining stage
  artifact references its hash (`stage/retention.rs:1-10`); a stage artifact is
  content-addressed and readable until it expires.
- **Observed:** `purge_expired` deletes the expired rows and returns their
  artifact hashes; `remove_expiry` then calls `artifact_referenced` for each
  hash and deletes the blob when the answer is false
  (`stage/retention.rs:135-158`). `artifact_referenced` is a separate
  `SELECT EXISTS` (`stage/postgres.rs:680-689`); there is no lock shared with
  `StageStore::create`. The explicit delete route does the same after deleting
  the row (`routes/stages.rs:556-580`).
- **Root cause:** The reference check and the blob delete are two steps with no
  transaction or lock covering a concurrent create.
- **Impact:** A new stage created between the check and the delete can reference
  the same blob; the delete then removes it, and the new stage's artifact is
  unreadable. The stage is diagnostic, not evidence, but it is advertised as
  content-addressed and verified.
- **Affected contracts/docs:** `stage/retention.rs`, `stage/postgres.rs`,
  `routes/stages.rs`, `docs/architecture/stage-surface.md` §7.
- **Dependencies:** The stage store transaction boundary.
- **Effort:** Medium.
- **Options:**
  A. Delete under a store transaction or advisory lock shared with create.
  B. Tombstone the blob and delete after a grace period with a re-check.
  C. Accept and document the race.
- **Recommendation:** A.
- **Verification:** A two-writer test with a create between check and delete;
  the new stage's blob survives.
- **Links:** None yet.

### AUD-2026-11-17-12 — the config-upload cap is undocumented

- **Expected invariant:** The deployment table documents the upload cap; a
  client can predict the `413` threshold (`docs/architecture/deployment.md:133`).
- **Observed:** `QR_UPLOAD_LIMIT_BYTES` defaults to 512 MiB and is documented
  as "Upload cap". The config upload route has a fixed
  `DefaultBodyLimit::max(4 * 1024 * 1024)` (`routes.rs:62-66`), and the bundle
  validator refuses a total over 4 MiB (`configs/upload.rs:78-80`). The 4 MiB
  config cap is not in the deployment table or the config-upload documentation.
- **Root cause:** The dataset upload and the config upload have separate limits;
  only the dataset limit is documented.
- **Impact:** A user uploading a 5 MiB config gets `413 upload.too_large`
  despite the documented 512 MiB cap.
- **Affected contracts/docs:** `docs/architecture/deployment.md`, the config
  upload section of `http-api.md`/the manual.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Document the 4 MiB config cap.
  B. Apply `QR_UPLOAD_LIMIT_BYTES` to configs too.
  C. Raise the config cap.
- **Recommendation:** A.
- **Verification:** The documentation and the route agree; a 5 MiB config test
  asserts the documented result.
- **Links:** None yet.

### AUD-2026-11-17-13 — infrastructure images are mutable tags

- **Expected invariant:** The release unit is a container image digest and the
  stack is reproducible (`docs/releases/README.md`; `AGENTS.md` §12).
- **Observed:** `compose.yaml:210` pins MinIO by digest, but
  `compose.yaml:190` uses `postgres:17` and `compose.yaml:230` uses
  `rabbitmq:4-management`. `docs/architecture/deployment.md:44-48` lists the
  tags as the topology. The same file defaults the database, object store, and
  broker to the known `quantrule` password (`compose.yaml:23,55-56,196,216,236`),
  and `config.rs:14,20` repeats the defaults.
- **Root cause:** The MinIO pin was added for a registry decision; the Postgres
  and RabbitMQ tags and the local default credentials were left in place.
- **Impact:** A fresh `make up` can pull a different database or broker than the
  verified run; the reproducibility and supply-chain claims are weaker.
- **Affected contracts/docs:** `compose.yaml`, `docs/architecture/deployment.md`,
  `docs/workflows/release-operations.md`.
- **Dependencies:** A registry decision for each image; a digest update process.
- **Effort:** Small.
- **Options:**
  A. Pin both by digest and record them.
  B. Document the tags as deliberate and require a fresh-pull check.
  C. Use digest overrides.
- **Recommendation:** A.
- **Verification:** `docker compose config` shows digests; the release note
  records them.
- **Links:** None yet.

### AUD-2026-11-17-14 — the status index exceeds the reference cap

- **Expected invariant:** A reference document may reach 330 lines; the caps
  apply to current documents from 2026-11-12 and are enforced by
  `tools/check_doc_length.py` (`AGENTS.md:174`).
- **Observed:** `docs/README.md` is 350 lines. The checker's `targets()` covers
  `docs/architecture/*.md`, `docs/adr/**`, and `docs/tutorials/**`; it does not
  cover `docs/README.md` (`tools/check_doc_length.py:43-55`).
- **Root cause:** The checker was scoped to the architecture set and dated
  records; the index is outside both.
- **Impact:** The index is longer than the cap the project says it enforces,
  and the checker's coverage is narrower than the rule.
- **Affected contracts/docs:** `docs/README.md`, `AGENTS.md` §5,
  `tools/check_doc_length.py`.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Trim the index to 330 or add an explicit override.
  B. Extend the checker to the index and current config documents.
  C. Scope the cap to architecture documents and say so.
- **Recommendation:** A.
- **Verification:** `check_doc_length` covers `docs/README.md`; the line count
  is within the cap or an override.
- **Links:** None yet.

### AUD-2026-11-17-15 — the worktree-cleanup tests are not wired

- **Expected invariant:** The cleanup guard is mandatory after a merge
  (`AGENTS.md` §9); a regression test for it should run in a gate or CI.
- **Observed:** `tools/test_worktree_cleanup.py` has eight tests and passes
  (`python3 tools/test_worktree_cleanup.py`: 8 tests, OK). It is not in
  `tools/checker_tests.txt`, `tools/gate.sh`, or `.github/workflows/ci.yml`;
  the only references are historical records.
- **Root cause:** The tool test is a standalone script; the checker lists were
  not extended.
- **Impact:** The mandatory cleanup path can regress without the gate noticing;
  its regression evidence is manual.
- **Affected contracts/docs:** `tools/test_worktree_cleanup.py`,
  `tools/checker_tests.txt`, `tools/gate.sh`, CI.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Add the test to `tools/checker_tests.txt`.
  B. Add a `tools/test_tools.sh` step.
  C. Accept it as manual and record that.
- **Recommendation:** A.
- **Verification:** The gate runs the test; a seeded cleanup regression fails.
- **Links:** `AGENTS.md` §9.

### AUD-2026-11-17-16 — nine plans have no same-named AI log

- **Expected invariant:** The AI transparency log is one file per work item,
  named to match its execution plan where the work item has one (`AGENTS.md` §6;
  `docs/README.md:329-340`).
- **Observed:** The nine plans listed in the findings table have no file with
  the same base name under `docs/ai-transparency-log/<date>/`. Several have a
  related log under another name (for example `workflow-reset.md` beside
  `workflow-reset-cleanup.md`; `649-run-worker-plan.md` beside
  `649-run-worker.md`; `468-legacy-inventory.md` beside
  `410-legacy-inventory.md`); others are sub-plans or superseded plans.
- **Root cause:** The plan and the log are named independently, and the
  sub-plan relation is not recorded in the plan.
- **Impact:** A work item's AI record is not discoverable by its plan name; a
  reviewer must guess the related file.
- **Affected contracts/docs:** the nine plans; `docs/ai-transparency-log/`;
  `AGENTS.md` §6.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Add the missing logs or rename the related log.
  B. Allow a `- AI log:` link and check it.
  C. Mark sub-plans explicitly in the plan.
- **Recommendation:** A.
- **Verification:** A checker resolves each current plan to a log through the
  same name or a recorded link.
- **Links:** The nine plan paths in the findings table.

### AUD-2026-11-17-17 — the verification map overstates row-level edge evidence

- **Expected invariant:** A row-scoped evidence entry names a test that proves
  the row's class; the model-layer contract requires an empty series, a single
  candle, a short series, an unaligned boundary, a gap, a zero-volume candle,
  and equal highs/lows where a tie is broken (`AGENTS.md` §4).
- **Observed:** `docs/registry/verification.json` marks `edge` as `row` for all
  49 indicators and names a row contract test. The extended-indicator helper
  `assert_extended_indicator` checks the golden fixture, prefix causality,
  determinism, and only the **lengths** of the empty, single, and short
  results (`per_row_extended_indicators.rs:131-142`). The gap, zero-volume,
  missing-volume, and equal-tie checks are family-level tests in
  `edge_cases.rs:242-341`. The row entry does not name them.
- **Root cause:** The verification map's `edge` entry was filled with the row
  contract, and the family tests were not referenced.
- **Impact:** The matrix overstates row-level edge coverage; a reviewer cannot
  tell from the map which test proves which edge case. The tests exist, so this
  is a traceability defect, not missing coverage.
- **Affected contracts/docs:** `docs/registry/verification.json`,
  `tools/check_verification.py`, the model test contracts.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Name the family test in the row entry or add row-level assertions.
  B. Record the scope as `family` where the proof is family-level.
  C. Leave the scope and add a note to the registry README.
- **Recommendation:** A.
- **Verification:** A fixture row with a row-scoped edge test that only asserts
  length is rejected or corrected; the matrix names the proof.
- **Links:** None yet.

### AUD-2026-11-17-18 — restore can destroy a started stack and fail

- **Expected invariant:** A restore replaces the state of a clean or explicitly
  replaced stack and leaves a working database; a `--force` restore is the
  documented recovery path (`tools/restore.sh:1-30`; `docs/architecture/operations.md`).
- **Observed:** The target count selects `catalog`, `runs`, `research`,
  `simulation`, and `public.schema_migrations` only
  (`tools/restore.sh:214-217`). The drop statement drops the same four schemas
  (`tools/restore.sh:233-237`). A normal stack also owns `gateway`
  (`crates/quantrule-gateway/migrations/0007_profiles.sql:3`) and `stage`
  (`crates/quantrule-evidence-service/migrations/0014_stage.sql:9`). The
  round-trip test seeds and compares only the four schemas
  (`tools/test_backup.sh:198-210`), so it cannot see the gap.
- **Root cause:** The restore and its round-trip test were written before the
  gateway and stage schemas were added and were not extended.
- **Impact:** A `--force` restore drops four schemas, then `pg_restore` fails on
  the existing `gateway`/`stage` objects; the target is partially destroyed and
  there is no rollback. A clean target with only `gateway`/`stage` rows is not
  even counted, so the restore can fail without the warning.
- **Affected contracts/docs:** `tools/restore.sh`, `tools/test_backup.sh`,
  `docs/architecture/operations.md`, the backup/restore tutorial.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Include `gateway` and `stage` in the count, the drop, and the round-trip
  test.
  B. Restore only into a fresh empty database and refuse a target with any of
  the six schemas or the migration ledger non-empty.
- **Recommendation:** A.
- **Verification:** A seeded `gateway`/`stage` row makes `--force` show the full
  count; a restore round-trip covers all six schemas; a partial target is
  refused.
- **Links:** None yet.

### AUD-2026-11-17-19 — the research partition is not enforced

- **Expected invariant:** A run's partition determines what it may read; the run
  config must match the declared partition, and a config that reads the
  validation or holdout window under a development partition is a contract
  breach that invalidates the run (`docs/workflows/run-contract.md:73,275-276`).
- **Observed:** `ImprovementManifest::validate` checks only that the partition
  belongs to the tier, that the frozen set matches, and that the holdout flag
  matches (`crates/quantrule-api/src/improvement.rs:257-286`).
  `research create` validates the dataset exists and the config parses, then
  creates the manifest (`research/create.rs:52-62`); `validated_config` checks
  the dataset and config only (`routes/runs.rs:52-116`). The config's
  `data.from`/`data.to` (or the dataset coverage) is never compared with the
  partition window.
- **Root cause:** The partition is treated as a label on the research record,
  and the data-window check was left to qualification after the run; the
  pre-registration gate does not enforce it.
- **Impact:** A `strategy_dev` manifest can submit a 2024 or 2025 config, so the
  validation or holdout data is consumed under a development label. The one-shot
  validation/holdout claim is procedural, not enforced by the system.
- **Affected contracts/docs:** `docs/workflows/run-contract.md`, the research
  create/submit routes, `data/partitions.json`, the run config loader.
- **Dependencies:** The Data Steward and the run-contract owner; the partition
  windows are declared in `data/partitions.json`.
- **Effort:** Medium.
- **Options:**
  A. At `research create`/`submit`, compare the config's `from`/`to` (or the
  dataset coverage) with the partition window and reject a non-contained window.
  B. Record the config bounds in the catalog and add a ledger check plus a
  regression test for `strategy_dev` + 2024 data.
- **Recommendation:** A.
- **Verification:** A `strategy_dev` manifest with a 2024 config is refused; a
  contained config passes; the regression test covers both.
- **Links:** None yet.

### AUD-2026-11-17-20 — portfolio cash can go negative

- **Expected invariant:** The portfolio is a cash account at leverage `1.0`;
  opening a long spends the notional and the account cannot hold a position it
  cannot fund (`crates/quantrule-backtest/src/portfolio/ledger.rs:1-16`).
- **Observed:** `FixedNotional` sizes `notional / price` without looking at
  equity (`sizing.rs:145-181`). `EquityFraction` allows `fraction = 1.0`; the
  entry then spends the whole equity, and the fee/commission is subtracted from
  cash (`ledger.rs:316-332`). `PortfolioConfig::validate` checks leverage is
  exactly `1.0` but never checks the notional against cash
  (`portfolio/config.rs:547-566`). The test
  `fixed_notional_ignores_the_equity` explicitly enshrines the behavior
  (`portfolio/tests.rs:359-369`).
- **Root cause:** Sizing was treated as a position-size rule independent of
  funding; the ledger's cash check was not added.
- **Impact:** A fixed notional above cash, or an equity fraction of `1.0` with
  fees, produces negative cash while the config claims leverage `1.0`. Return,
  equity, exposure, and drawdown are computed on capital the account does not
  have.
- **Affected contracts/docs:** `docs/config/2026-11-06/strategy-config.md`,
  `docs/workflows/portfolio-research.md`, the portfolio config and ledger.
- **Dependencies:** The portfolio contract and the Quant Validator; the config
  is human-owned.
- **Effort:** Small to medium.
- **Options:**
  A. Cap the entry notional at `equity × leverage` and reject a fill that makes
  cash negative.
  B. Validate `fixed_notional <= initial_cash` and reserve entry costs, or add
  an explicit margin model.
- **Recommendation:** A.
- **Verification:** A fixed notional above cash is rejected; a fraction of
  `1.0` with costs does not go negative; tests cover both.
- **Links:** None yet.

### AUD-2026-11-17-21 — artifact config identity is FNV, not the run's SHA-256

- **Expected invariant:** The run record's `config_hash` and the artifact
  provenance name the same configuration (`docs/workflows/run-contract.md:95`).
- **Observed:** `RunConfig::config_hash` is a 64-bit FNV-1a value formatted as
  sixteen hex characters (`config/run.rs:420-453`). The strategy and execution
  artifact metadata write it as `model_config_id` and `config_id`
  (`stages/backtest.rs:455,493`). The data service hashes the same files with
  SHA-256 (`configs/scan.rs:129-147`), and the evidence service records that
  SHA-256 as the run's `config_hash` (`routes/runs.rs:114-116`).
- **Root cause:** The app's loader kept the pre-cutover FNV fingerprint as its
  "hash", while the catalog introduced a SHA-256 content identity; the two were
  never unified.
- **Impact:** The artifact provenance names a different config identity from the
  run record and model manifest; a verifier cannot tie the artifact to the
  config, and a 64-bit non-cryptographic hash is not collision-resistant for an
  identity field.
- **Affected contracts/docs:** the run contract, the stage/run manifests, the
  artifact renderer.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Compute one SHA-256 in `RunConfig::load` with the catalog's
  canonicalization and use it in every stage.
  B. Rename the FNV value to `config_fingerprint` and never write it as
  `config_id`/`config_hash`.
- **Recommendation:** A.
- **Verification:** The run record, model manifest, and stage provenance agree;
  a cross-crate identity test.
- **Links:** None yet.

### AUD-2026-11-17-22 — the canonical digest excludes the compared metrics

- **Expected invariant:** The canonical digest is the claim boundary for a run;
  the values a research comparison reads are covered by it
  (`docs/adr/2026-11-12/07-canonical-digest-and-claim-boundary.md:20`).
- **Observed:** `canonical()` includes `results.json`, `*.csv`, and `*.ndjson`
  (`service-api/src/manifest.rs:172-183`). The portfolio stage writes
  `metrics.json` (`stages/portfolio/run.rs:245-262`), and `research compare` and
  qualification read `portfolio/metrics.json` (`research/compare.rs:25`;
  `stages/qualification/evidence.rs:207`). `results.json` is a historical
  improvement-run artifact and is not written by the current coupled runner.
- **Root cause:** The digest allowlist was fixed on the older `results.json`
  artifact set before the staged bundle introduced `metrics.json`.
- **Impact:** A metrics-only change (profit factor, monthly returns, open
  position, halt state) can leave the canonical digest unchanged while the
  research comparison changes. The digest does not cover the values the
  comparison uses.
- **Affected contracts/docs:** the canonical-digest ADR, the run contract, the
  portfolio/qualification artifacts, `research compare`.
- **Dependencies:** A contract change to the canonical set; the Quant Validator
  and the human approve it.
- **Effort:** Small.
- **Options:**
  A. Include `metrics.json` (or rename it to `results.json`) in `canonical()`
  and add a mutation test.
  B. Make the comparison read a covered artifact and state the digest boundary.
- **Recommendation:** A.
- **Verification:** Mutating `metrics.json` changes the digest; the
  compare/qualification tests still pass.
- **Links:** None yet.

### AUD-2026-11-17-23 — promotion cannot reuse a model across strategy configs

- **Expected invariant:** `run submit --model-artifact` reuses a frozen model
  artifact byte-for-byte and recomputes strategy, execution, and portfolio
  (`docs/workflows/run-contract.md:109-113`).
- **Observed:** `load_promoted_model` requires
  `stage.provenance.config_hash == record.config_hash`
  (`backtest/worker.rs:382-386`; `routes/runs.rs:153-158`). The config hash is a
  fingerprint over run, data, model, execution, portfolio, and strategy
  (`config/run.rs:436-438`). A new strategy config therefore changes the hash,
  and the stage is rejected as "another config hash".
- **Root cause:** The stage identity uses the full run config, not the
  model-input subset; the promotion path was tested with the same config.
- **Impact:** The expensive model compute cannot be reused across strategy
  cells, which is the stated purpose of promotion. A new strategy forces a new
  model stage, so the path adds cost rather than removing it.
- **Affected contracts/docs:** the run contract, the stage manifest, the
  backtest worker.
- **Dependencies:** A stage identity change; the Quant Validator and the human
  approve it.
- **Effort:** Medium.
- **Options:**
  A. Add a model-input hash (data + model config + ladder) to stage provenance
  and compare that on promotion.
  B. Compare only the model-relevant files and add a promotion test where only
  `strategy.json` changes.
- **Recommendation:** A.
- **Verification:** A promotion test where only `strategy.json` changes
  succeeds; the model bytes equal a full run.
- **Links:** None yet.

### AUD-2026-11-17-24 — event edge evidence is a readings-only test

- **Expected invariant:** Every active event has edge-case coverage: an empty
  series, a single candle, a short series, an unaligned boundary, a gap, a
  zero-volume candle, and equal highs/lows where a tie is broken
  (`AGENTS.md` §4).
- **Observed:** 149 events name only
  `edge_cases::an_empty_series_publishes_no_readings` as their `edge` evidence;
  the 150th also names it (`docs/registry/verification.json`; exact-name count
  149, contains count 150). That test computes `Readings`,
  `ExtendedReadings`, `SessionReadings`, and `PanelReadings` and constructs no
  event stream (`edge_cases.rs:243-252`). The retest event
  `signal.breakout_retest_hold` points its golden evidence at
  `signal_events_golden`, whose fixture contains only subtypes 1–3; no fixture
  emits subtype 24 (`signal_events_golden.csv`). `check_verification.py:106-117`
  only checks that the named test exists and that the name contains the row
  token; it does not verify that the test constructs an event.
- **Root cause:** The family entry was reused for every event to satisfy the
  checker, and the checker cannot see what the test asserts.
- **Impact:** The `AGENTS.md` every-event edge claim is unsupported; a
  regression on empty/single/gap/zero-volume event inputs would pass the gate.
- **Affected contracts/docs:** the verification map, the model test contract,
  `check_verification.py`.
- **Dependencies:** None.
- **Effort:** Medium.
- **Options:**
  A. Add event edge tests that construct `EventRecord`s for every family and
  point the manifest at them.
  B. Require `edge` scope `row` for events and narrow the contract/status claim.
- **Recommendation:** A.
- **Verification:** A fixture event edge test constructs events; the checker
  rejects a readings-only test as event edge evidence.
- **Links:** None yet.

### AUD-2026-11-17-25 — the release pushes before the composed suite and skips the browser gate

- **Expected invariant:** The release gate runs the full verification suite
  before an image is published; a failed check leaves no tag-named image
  (`docs/releases/README.md`; `AGENTS.md` §4).
- **Observed:** The image is pushed at `.github/workflows/release.yml:251-259`,
  then the composed suite runs at `release.yml:319`. The release `gate` job runs
  only `tools/gate.sh` (`release.yml:112-113`); `tools/gate.sh:18-23` contains
  no `bun` or Python tooling step. CI runs the browser `bun run check`,
  `bun test`, `bun run e2e`, and the data/research Python suites
  (`ci.yml`), so the release gate is narrower than CI.
- **Root cause:** The composed suite needs the running image, so it was placed
  after the push; the browser/tooling checks were treated as CI-only.
- **Impact:** A composed or browser/tooling failure can leave a tag-named image
  pullable and a release body without the full verification suite. The release
  note claims the browser suite; the release workflow does not run it.
- **Affected contracts/docs:** `release.yml`, `tools/gate.sh`, the release
  operations workflow, the release note.
- **Dependencies:** A release workflow change; the human authorizes it.
- **Effort:** Medium.
- **Options:**
  A. Run the composed suite against the locally built image before `docker
  push`, and add a release quality job with the CI web/tooling steps.
  B. Push a staging tag, run the full suite, and promote the tag only after
  success.
- **Recommendation:** A.
- **Verification:** No push before composed passes; the release requires the
  web/tooling job; a seeded failure blocks.
- **Links:** None yet.

### AUD-2026-11-17-26 — the 2025 holdout declares instruments whose coverage ends in 2024

- **Expected invariant:** A partition's instruments have coverage for its window;
  the data-change record and the partition agree (`docs/workflows/data-lifecycle.md`).
- **Observed:** `portfolio_holdout` 2025 includes `COPPERCMDUSD` and
  `TSLAUSUSD` (`data/partitions.json:109-123`). The committed data-change record
  declares coverage `2018-01-01..2024-12-31` for both
  (`docs/config/2026-11-08/data-change-MR7-01.md:14-19`), and its Partition
  section says "Holdout: unchanged; these instruments inherit the declared
  partition windows" (`:42-45`).
- **Root cause:** The MR7 data change extended the instruments to 2024 and
  inherited the holdout label without admitting 2025 data.
- **Impact:** The one-shot holdout cannot cover its declared instruments; a
  holdout run fails or silently uses a shortened sample, invalidating its
  coverage claim.
- **Affected contracts/docs:** `data/partitions.json`, the data-change record,
  the Data Lifecycle workflow, the portfolio-research workflow.
- **Dependencies:** A 2025 COPPER source and a Data Steward admission, or a
  partition change.
- **Effort:** Data work, not code.
- **Options:**
  A. Admit 2025 COPPER (and TSLA if needed) and update the data-change record.
  B. Remove COPPER (and any uncovered instrument) from the holdout, or declare
  and test an explicit holdout subset.
- **Recommendation:** B until the data exists.
- **Verification:** A coverage check fails a holdout instrument without 2025
  data; the data-change record matches the partition.
- **Links:** `DATA-2026-11-08-MR7-01`.

### AUD-2026-11-17-27 — the remote browser is not a secure context

- **Expected invariant:** The browser client can download and verify artifacts
  over the documented remote deployment (`docs/manual/08-operations.md`).
- **Observed:** `sha256Hex` calls `crypto.subtle.digest`
  (`apps/web/src/lib/artifacts.ts:24`), which is unavailable outside a secure
  context; `copy` guards `navigator.clipboard` and silently returns when it is
  absent (`RunDetail.svelte:29-33`). The deployment document names
  `http://<server>:8080` as the remote origin (`deployment.md:155`). Playwright
  uses loopback, which is a secure context, so it masks the failure.
- **Root cause:** The client uses secure-context-only APIs, and the documented
  remote origin is plain HTTP.
- **Impact:** The remote browser cannot download or verify run/stage artifacts,
  and the digest copy does nothing. The shell is unaffected.
- **Affected contracts/docs:** `docs/architecture/deployment.md`,
  `docs/manual/08-operations.md`, the browser artifact module.
- **Dependencies:** A TLS decision or a pure-JS fallback.
- **Effort:** Small to medium.
- **Options:**
  A. Add a pure-JS SHA-256 fallback (or require HTTPS) and show an explicit
  secure-context error.
  B. Require HTTPS for remote access and document it.
- **Recommendation:** A.
- **Verification:** A remote HTTP origin downloads and verifies an artifact, or
  shows a clear error; a non-secure context test.
- **Links:** None yet.

### AUD-2026-11-17-28 — a late detail response can overwrite the selected record

- **Expected invariant:** A detail view shows the record the user selected; a
  late response for an earlier selection is discarded
  (`docs/architecture/client.md` §4).
- **Observed:** `loadDetail` captures the profile `generation` only
  (`runs.svelte.ts:65-73`; `stages.svelte.ts:237-257`;
  `research.svelte.ts:104-118`). `select` sets `selected` and then calls
  `loadDetail`; two rapid selects A then B can receive A's response after B's,
  and the guard `at !== generation` is false, so A's detail replaces B's while
  `selected` is B. Actions read `detail.run_id`, `detail.stage_run_id`, or
  `detail.improvement_id`.
- **Root cause:** The read guards the profile generation but not the selected id.
- **Impact:** The UI can show run A's digest under selected run B; Cancel,
  Delete, or Decide can act on the wrong record.
- **Affected contracts/docs:** `docs/architecture/client.md`, the run, stage,
  and research stores.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Capture the selected id in each read and discard a response unless it still
  matches.
  B. Use a per-selection request token incremented by `select`/`clearSelection`.
- **Recommendation:** A.
- **Verification:** A browser test with two rapid selections and out-of-order
  responses shows B, not A.
- **Links:** None yet.

### AUD-2026-11-17-29 — server artifact labels and the browser table disagree

- **Expected invariant:** One generated table maps numeric codes to words; the
  gateway and services forward codes (`AGENTS.md` §3).
- **Observed:** `artifact/labels.rs:54-60` maps `(1,1)` to `test` and `(1,4)` to
  `break_of_structure`; `labels.generated.ts:10-15` maps the same codes to
  `test/retest` and `break of structure`. `research/compare.rs:323-332` renders
  timeframe words (`1m`, `5m`, `1h`). `check_labels.py` only checks the
  TypeScript table.
- **Root cause:** The server artifact renderer kept a hand-written table, and
  the evidence service kept a local timeframe formatter.
- **Impact:** The same event code reads differently in `summary.txt` and the
  chart; the one-table invariant is broken, and the service table can drift.
- **Affected contracts/docs:** the registry, the label generator, the artifact
  renderer, `research compare`.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Generate both the Rust artifact labels and the TypeScript table from the
  registry, with a gate comparing the words.
  B. Remove labels from server artifacts and publish codes/seconds.
- **Recommendation:** A.
- **Verification:** A gate compares the server and client words; no service
  renders a label.
- **Links:** None yet.

### AUD-2026-11-17-30 — simulation create is not a usable parity surface

- **Expected invariant:** Both clients reach the same simulation use case with
  the same inputs and identity (`AGENTS.md` §2; the parity rule).
- **Observed:** The browser sends `inputs` (`apps/web/src/lib/simulation.ts:49,110-118`);
  the shell parser has `premise`, `bundle`, `seed`, and `criteria` only and the
  dispatch sets `inputs: String::new()` (`parse.rs:541-553`;
  `dispatch.rs:731`). The manual and tutorial pass `--bundle <data_version>`
  (`manual/09-command-reference.md:88`; `tutorials/2026-11-14/shell-http.md:138`),
  while the service loads `simulation/bundles/<sha256>.json`
  (`bundle.rs:8,148`). No current route or command creates that object; the
  AI log says a producer must write it, and the Python harness was retired.
- **Root cause:** The browser added context inputs and a bundle input, and the
  shell and the manual were not extended; the bundle producer was external.
- **Impact:** A browser run with context inputs has a different `inputs_digest`
  and identity from the closest shell run, and the documented simulation
  procedure fails with a missing bundle. The research plane is reachable only by
  manually seeding object storage.
- **Affected contracts/docs:** the command surface, the manual, the tutorial,
  the simulation service and the shell.
- **Dependencies:** A bundle-producing command or a create-from-run design.
- **Effort:** Medium.
- **Options:**
  A. Add `--inputs` and a bundle producer/command, update the references, and
  add a parity test.
  B. Remove the browser inputs field and change create to derive a bundle from a
  run/dataset.
- **Recommendation:** A.
- **Verification:** Both clients create the same inputs digest; a documented
  producer creates the bundle; the tutorial runs.
- **Links:** None yet.

### AUD-2026-11-17-31 — the entry-bar bracket assumes an unproven intrabar path

- **Expected invariant:** A backtest does not assert an intrabar path that OHLC
  cannot prove; the two-sided entry already refuses an ambiguous bar
  (`execution/engine/mod.rs:37-44`).
- **Observed:** On the entry bar, `apply_bracket` uses the bar's full high/low
  (`exit.rs:21-65`). A stop entry fills at an intrabar boundary
  (`bracket_entry.rs:88-158`); a low below the stop may have occurred before
  the entry trigger, but the engine treats it as a same-bar stop-out. The test
  `the_position_bracket_is_live_on_the_entry_bar` asserts exactly that path
  (`bracket_entry_tests.rs:302-316`).
- **Root cause:** The entry-bar rule was written for a market entry at the open
  and then applied to a stop entry; the contract does not state the convention.
- **Impact:** A stop-entry backtest can record a stop-out or fill path OHLC
  cannot prove; the conservative convention is not in the contract, so a
  candidate can be rejected on an assumed path.
- **Affected contracts/docs:** the execution contract, the bracket entry and
  exit code, the golden fixtures.
- **Dependencies:** A Quant Validator decision on the intended convention.
- **Effort:** Medium.
- **Options:**
  A. Evaluate the bracket from the next bar for a stop entry.
  B. Document and test the conservative entry-bar convention in the execution
  contract and regenerate the golden deliberately.
- **Recommendation:** B if the conservative rule is intended; otherwise A.
- **Verification:** A fixture with a pre-trigger low below the stop; the
  contract states the rule; the golden is updated deliberately.
- **Links:** None yet.

### AUD-2026-11-17-32 — the strategy event snapshot is not role-filtered

- **Expected invariant:** A strategy may read only registered `signal`/`context`
  events and states, confirmed levels, bar identity, and the as-of context; a
  diagnostic never enters a decision (`AGENTS.md` §2-3).
- **Observed:** `EventStrategyInput` carries every event that became knowable
  in the window (`event_input.rs:18-40,186-197`) and exposes
  `context: Option<ContextBar>` (`:59-60`). The boundary test scans source
  strings in `input.rs` only (`:487-512`); it does not construct a snapshot or
  filter by registry role. The config validator rejects a diagnostic event when
  it builds a condition (`event.rs:367-375`), but the public snapshot still
  carries it.
- **Root cause:** The role check lives in the condition builder, not in the
  snapshot projection, and the boundary test targets the narrow
  `StrategyInput`, not `EventStrategyInput`.
- **Impact:** A public strategy implementation, or a future condition, can read
  a diagnostic event or a raw context value; the boundary invariant and its
  test do not prove what the docs claim.
- **Affected contracts/docs:** the strategy boundary test, the event input
  projection, the model context contract.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Filter `EventView`s by registry role and add a constructed-snapshot test
  containing a diagnostic event.
  B. Keep context only behind a registered context condition and add a
  private-field boundary test.
- **Recommendation:** A.
- **Verification:** A snapshot with a diagnostic event cannot satisfy a rule; a
  constructed-snapshot test proves it.
- **Links:** None yet.

### AUD-2026-11-17-33 — the run loader does not enforce the documented v4 check

- **Expected invariant:** The run loader rejects a strategy whose execution
  timeframe differs from the fill timeframe, and `run submit` accepts only the
  config schema the coupled path supports
  (`docs/config/2026-11-13/run-v4-strategy.md:33`;
  `docs/config/2026-11-06/strategy-config.md:68`).
- **Observed:** `RunConfig::load` cross-validates the config but does not compare
  `strategy.execution_timeframe` with `execution.fill_timeframe`
  (`config/run.rs:196-224`); the check runs later in `validate_context`
  (`stages/strategy/context.rs:74-81`). The parser accepts `quantrule.run/v3`
  without a strategy (`run.rs:303-322`), and `run submit` does not reject it;
  the worker fails with "names no strategy" (`workspace.rs:47-50`).
- **Root cause:** Validation was split between the loader and the stage runner,
  and the v3 schema remained accepted on the coupled route.
- **Impact:** An invalid v4 config is catalogued as valid and a v3 run is
  queued, retried, and failed instead of rejected at submission; the documented
  loader contract is false.
- **Affected contracts/docs:** the run and strategy config contracts, the run
  submit route, the loader.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Add `validate_context` to `RunConfig::load`/catalog validation and reject
  v3 at `run submit`.
  B. Route v3 explicitly to a model-only path and change the loader docs to say
  validation is run time.
- **Recommendation:** A.
- **Verification:** A 5m strategy/15m fill mismatch and a v3 submission are
  refused at submission; a valid config passes.
- **Links:** None yet.

### AUD-2026-11-17-34 — the ledger halt and funding equity are not enforced

- **Expected invariant:** A halted ledger opens no position; a funding charge
  measures drawdown on the current marked equity
  (`crates/quantrule-backtest/src/portfolio/ledger.rs:1-16`).
- **Observed:** The halt check is in `size_entry` (`ledger.rs:145-150`), but
  `open` has no halt check (`:316-332`). `apply_funding` computes
  `equity = self.state.equity - signed` (`:203-211`) instead of
  `cash + position_value(price)`. The test named "blocks entries" calls
  `size_entry` only (`ledger_tests.rs:523-565`).
- **Root cause:** The halt was treated as a sizing rule, and funding reused the
  previous mark's equity.
- **Impact:** A standalone ledger caller can open after the halt; a
  funding-boundary drawdown is measured against a stale equity, so halt and
  risk-event numbers can be wrong.
- **Affected contracts/docs:** the portfolio ledger, the halt/risk-event
  contract.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Reject an entry fill when halted and mark at the funding price before
  `check_halt`.
  B. Make the halt a runner-only rule and document it.
- **Recommendation:** A.
- **Verification:** A halted ledger refuses an entry fill; a funding charge at a
  new price uses the marked equity.
- **Links:** None yet.

### AUD-2026-11-17-35 — test and checker evidence is weaker than the registry claims

- **Expected invariant:** A registry claim names a test that exercises it;
  a checker that enforces a rule has a seeded-failure test
  (`AGENTS.md` §4).
- **Observed:** The per-row determinism tests call the same computation twice on
  the same ordered input (`per_row_base.rs:129-133`;
  `per_row_base_events.rs:199-201`). Nine of the 25 checkers have no dedicated
  unit test (`tools/checkers.txt` vs `tools/checker_tests.txt`). The port-parity
  registry names one target while eight modules carry the marker
  (`docs/port-parity/registry.json:7`). The module boundary tests are
  substring scans (`crates/quantrule-backtest/tests/boundaries.rs:37-57`). The
  verification-matrix checker does not read `verification.json`
  (`tools/check_verification_matrix.py:35-71`). The base read empty/single
  test asserts only finiteness (`edge_cases.rs:24-27`). The
  no-partial-publish test simulates a kill/cancel rather than sending a signal
  to the worker (`tests/worker/cancel.rs:6`;
  `tests/sqlite_transactions.rs:118`), and no test compares one worker with N
  workers on the same canonical digest.
- **Root cause:** The evidence was added to satisfy the registry shape, not to
  exercise the specific failure modes; the checker tests cover the most
  recently changed checkers.
- **Impact:** Claims are weaker than the registry says; an iteration-order or
  checker regression can pass the gate.
- **Affected contracts/docs:** the verification map and matrices, the checker
  list, the port-parity registry, the boundary tests.
- **Dependencies:** None.
- **Effort:** Medium.
- **Options:**
  A. Add reordered-input tests, a checker meta-test, all port targets, a
  use-tree boundary, a matrix regeneration check, and base-reading null
  assertions.
  B. Narrow the claims to what the tests prove.
- **Recommendation:** A.
- **Verification:** Each claim names a test that exercises it; a seeded
  regression fails.
- **Links:** None yet.

### AUD-2026-11-17-36 — client and command-contract gaps

- **Expected invariant:** The browser and shell expose the same use cases and
  the command reference matches the observed output (`AGENTS.md` §2-3).
- **Observed:** `ConfigsPanel.svelte` keeps its selected detail in local state
  and does not clear it on a profile reset (`:15,32-38,144-160`). All three stage
  forms default to the fixture `cfg_abc` (`Model/Strategy/PortfolioStagePanel.svelte:20-21`).
  The packaged shell passes no `--profile`, so a stray `QR_DESKTOP_PROFILE`
  can make it start no sidecars (`apps/desktop-shell/src/main.rs:73-77`;
  `supervisor/cli.rs:4-5,310-313`). `tools/parity_desktop.sh` is not invoked by
  the release workflow. `help` constructs `Session` before handling the local
  command, so a malformed token makes `help` fail
  (`dispatch.rs:43-45,852-853`). The README says every command is one gateway
  call and accepts every global flag, while `stage explain/events/fetch` make
  multiple calls and `profile use` rejects `--profile`
  (`README.md:174-177`; `command-surface.md:29-31`). `simulation check` prints
  one JSON line where the table is documented
  (`help.rs:402`; `manual/09-command-reference.md:93`). The parity fixture
  exercises 33/46 shell and 32/46 browser commands.
- **Root cause:** The client and command surfaces were extended faster than the
  local-state resets, defaults, release hook, and documentation.
- **Impact:** Usability, parity, and documentation drift: a profile switch can
  show a stale config; a first stage submit targets a fixture id; a packaged app
  can start no sidecars; 13 commands are unexercised by the parity suite.
- **Affected contracts/docs:** the client screens and stores, the shell, the
  command reference, the release workflow, the parity fixture.
- **Dependencies:** None.
- **Effort:** Medium.
- **Options:**
  A. Fix each item and extend the parity fixture to all 46.
  B. Narrow the docs and record the exceptions.
- **Recommendation:** A.
- **Verification:** A profile-switch test, a first-submit test, a packaged launch
  test, a malformed-token help test, and a 46/46 parity test.
- **Links:** None yet.

### AUD-2026-11-17-37 — the emitter checker sees test-only emitters

- **Expected invariant:** The event-emitter checker scans production source only
  (`tools/check_event_emitters.py:1-20`).
- **Observed:** `production_sources` skips `tests.rs` and a `tests` directory,
  but includes `row_signal_tests.rs` and `row_tests.rs`, which are declared under
  `#[cfg(test)]` (`events/extended.rs:181-184`;
  `check_event_emitters.py:34-42`).
- **Root cause:** The exclusion is path-based, not module-graph-based.
- **Impact:** A test-only reference can mask a missing production detector; no
  current false pass was found.
- **Affected contracts/docs:** the emitter checker, the model test module
  layout.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Exclude `#[cfg(test)]` modules and `*_tests.rs` from production sources.
  B. Derive the set from the module graph.
- **Recommendation:** A.
- **Verification:** Removing a production emitter fails the checker even when
  the test file references the code.
- **Links:** None yet.

### AUD-2026-11-17-38 — a concurrent cancel can overwrite a succeeded run

- **Expected invariant:** A run's terminal state is monotonic; a succeeded run is
  never changed to cancelled; a cancel that loses the race returns a conflict
  (`docs/architecture/control-plane.md:103-105`).
- **Observed:** `PgRunStore::cancel` loads the record, checks `is_cancellable`,
  then runs `UPDATE runs.runs SET status = 'cancelled', finished_at = now()
  WHERE run_id = $1` with no status predicate and no transaction
  (`postgres.rs:352-364`). `complete` is conditional on `status = 'running'`
  (`:530-538`). If complete commits between the read and the update, cancel
  overwrites succeeded. The stage store already uses the conditional update and
  inspects `rows_affected` (`stage/postgres.rs:542-560`).
- **Root cause:** The run cancel path predates the stage store's conditional
  update and was not brought in line.
- **Impact:** A succeeded run can become cancelled while retaining its digest
  and artifacts; `/runs/{id}/digest` returns a digest for a cancelled run; a
  second concurrent cancel returns `202` instead of `409`.
- **Affected contracts/docs:** `docs/architecture/control-plane.md`, the run
  store, the cancel route.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Make the update conditional (`AND status IN ('queued','running')`),
  inspect `rows_affected`, and return `Conflict` on zero.
  B. Wrap the read/check/update in a transaction with `SELECT ... FOR UPDATE`.
- **Recommendation:** A.
- **Verification:** A concurrent cancel/complete Postgres test leaves the run
  succeeded or cancelled, never both; a second cancel is a conflict.
- **Links:** None yet.

### AUD-2026-11-17-39 — `/health` does not aggregate the services

- **Expected invariant:** `/health` reports the state of the services and the
  queue and worker signals the operator uses to decide whether the stack is
  ready (`docs/architecture/http-api.md:31`;
  `docs/architecture/operations.md:18-29`).
- **Observed:** The gateway registers only its own database probe
  (`qr_gateway.rs:27-36`). `HealthReport::report` hardcodes `queue_depth: 0` and
  `workers: 0` (`health.rs:77-85`), and the health handler returns `Json(report)`
  with HTTP 200 regardless of the aggregate state. No probe reaches data, model,
  evidence, simulation, the object store, or the broker.
- **Root cause:** The health registry was designed for one service's local
  dependencies; the gateway report was never wired to the other services.
- **Impact:** A data/model/evidence/simulation/queue/object-store outage still
  reports healthy; the documented worker and queue-depth signals never fire;
  `make up`, the browser, and the shell all see a healthy gateway.
- **Affected contracts/docs:** `http-api.md`, `operations.md`, the compose
  healthcheck, the gateway.
- **Dependencies:** Service-to-service health calls and a worker heartbeat.
- **Effort:** Medium.
- **Options:**
  A. Aggregate each service's `/health`, queue depth, and a worker heartbeat,
  and return `down`/`503` when a required dependency is down.
  B. Narrow the documented contract to a local liveness probe and add a
  separate `/ready` endpoint with the aggregation.
- **Recommendation:** A.
- **Verification:** Killing `qr-data` makes the gateway report `down`/`503`; a
  healthy stack reports workers and queue depth.
- **Links:** None yet.

### AUD-2026-11-17-40 — RabbitMQ publish has no publisher confirm

- **Expected invariant:** A publish that the broker did not durably accept is
  reported as a failure; the submission path does not return `202` for a
  message that was dropped (`docs/architecture/services.md:87`).
- **Observed:** `RabbitQueue::publish` awaits `basic_publish` and maps the
  returned confirmation to `()`, discarding it; the channel is not in confirm
  mode (`rabbitmq.rs:111-127`).
- **Root cause:** The adapter was written for the happy path; the confirm
  handshake was not enabled.
- **Impact:** Evidence can return `202` with a `queued` row and no durable
  `run.requested` message; a client that does not resubmit leaves the run
  stranded until an operator intervenes; "nothing is silently dropped" is
  false.
- **Affected contracts/docs:** the control-plane contract, the services
  catalog, the queue adapter.
- **Dependencies:** A confirm-mode channel or an outbox.
- **Effort:** Small to medium.
- **Options:**
  A. Enable publisher confirms (`confirm_select`) and await
  `Confirmation::Ack`, returning `503` on a nack/timeout.
  B. Add a transactional outbox or a republish sweeper for `queued` runs with no
  message.
- **Recommendation:** A.
- **Verification:** A closed channel after publish returns an error; a queued
  run with no message is recovered.
- **Links:** None yet.

### AUD-2026-11-17-41 — worker concurrency and stage/run isolation are not wired

- **Expected invariant:** The run worker scales with
  `QR_BACKTEST_CONCURRENCY`, and a diagnostic stage does not occupy the
  evidence run's only slot (`docs/architecture/control-plane.md:70-72,107-112`).
- **Observed:** The run and stage consumers share one AMQP channel and call
  `basic_qos(1)` on it (`rabbitmq.rs:141-166`). `x-priority=-1` is a
  same-queue consumer property and does nothing across the separate queues.
  `QR_BACKTEST_CONCURRENCY` is parsed into `Config` but never read by a worker
  (`config.rs:226-239`; no reader in `quantrule-backtest-service`).
- **Root cause:** The consumers were added to one channel for simplicity, and
  the concurrency knob was documented before it was wired.
- **Impact:** A long diagnostic stage occupies the only unacked slot and delays
  an evidence run; setting `QR_BACKTEST_CONCURRENCY` has no effect; the operator
  cannot tune the worker.
- **Affected contracts/docs:** the control-plane and deployment documents, the
  queue adapter, the backtest service.
- **Dependencies:** A consumer-per-channel design.
- **Effort:** Medium.
- **Options:**
  A. Open a separate channel per consumer with its own `basic_qos(1)` and use
  `QR_BACKTEST_CONCURRENCY` to spawn N run consumers.
  B. Use separate processes or connections for the stage and run consumers.
- **Recommendation:** A.
- **Verification:** A composed test shows an evidence run is not blocked by a
  stage; the concurrency variable changes run throughput.
- **Links:** None yet.

### AUD-2026-11-17-42 — stage retention is startup-only

- **Expected invariant:** A stage artifact expires under
  `QR_STAGE_RETENTION_DAYS`; the retention window is a time bound
  (`docs/architecture/stage-surface.md:27`).
- **Observed:** The evidence service calls `run_cleanup` once at startup
  (`main.rs:126-160`); there is no timer, request hook, or cron. The operations
  document says "No cron, no daemon, no scheduled job"
  (`docs/architecture/operations.md:120-122`), and the compose comment says the
  rows "expire after 30 days".
- **Root cause:** The cleanup was implemented as a startup pass and the docs
  describe a TTL.
- **Impact:** A `restart: unless-stopped` stack that never restarts never
  expires stage rows or blobs; they grow until an operator restarts evidence.
- **Affected contracts/docs:** the stage-surface and operations documents, the
  evidence startup.
- **Dependencies:** A timer or a request hook.
- **Effort:** Small.
- **Options:**
  A. Run the cleanup on a tokio interval or before stage submission/list.
  B. Document the sweep as startup-only and change the TTL wording.
- **Recommendation:** A.
- **Verification:** A long-running test with an expired stage removes the
  row/blob without a restart; the docs match.
- **Links:** None yet.

### AUD-2026-11-17-43 — the service catalog contradicts the digest ownership

- **Expected invariant:** The service catalog's "must never" is the boundary
  the data-plane contract and the code follow (`docs/architecture/services.md:21`).
- **Observed:** `services.md` says `backtest` must never "own provenance or
  compute a digest", but the worker builds `RunManifest::from_artifacts`, which
  computes the canonical digest, and writes the manifest
  (`worker.rs:273-291`). `data-plane.md:20-22` assigns that step to the worker,
  and evidence re-verifies it (`evidence/artifacts.rs:46-61`).
- **Root cause:** The catalog sentence predates the split of compute and
  finalize.
- **Impact:** Two documents disagree; evidence re-verification prevents a
  current integrity break, but two digest implementations can drift and the
  "must never" is unenforced.
- **Affected contracts/docs:** `services.md`, `data-plane.md`, the worker,
  the evidence verifier.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Align the catalog with the data plane ("worker proposes; evidence owns and
  verifies") and add a guard.
  B. Move manifest/digest construction into evidence and have the worker send
  artifact refs.
- **Recommendation:** A.
- **Verification:** The two documents agree; a boundary test fails when the
  worker writes a manifest without evidence verification.
- **Links:** None yet.

### AUD-2026-11-17-44 — the internal client has a fixed 30 s deadline and contract drifts

- **Expected invariant:** A proxied call has a deadline that fits its work, and
  every documented field and error shape is accurate (`docs/architecture/http-api.md`).
- **Observed:** The internal client uses a fixed 30 s request timeout for every
  proxied call, including 512 MiB uploads, `/model/compute`, and artifact
  downloads (`client.rs:287-289,375-377`). The gateway returns `503` while the
  upstream continues. The contract also drifts on `SubmitRunResponse.created`
  (`http-api.md:84-86`), the "logged with a correlation id" claim (no
  correlation id exists), `If-None-Match` (the CSV handlers always return
  `200`), a non-numeric simulation `chart` (axum's plain-text 400, not the
  envelope), and the missing `stage.*` error rows.
- **Root cause:** The proxy reused one timeout and the contract was written
  before the streaming and stage surfaces.
- **Impact:** A slow but successful upload/compute is reported as a failure and
  requires a retry; clients cannot rely on the documented envelope/error table;
  the correlation-id support claim is false.
- **Affected contracts/docs:** `http-api.md`, the runtime client, the data and
  simulation routes.
- **Dependencies:** None.
- **Effort:** Medium.
- **Options:**
  A. Use route-specific or configurable deadlines for streaming
  uploads/downloads and compute, with a documented retry contract.
  B. Implement or remove the `If-None-Match`/correlation-id claims, add an
  envelope-returning chart rejection, and document `created`/`stage.*`.
- **Recommendation:** A.
- **Verification:** A slow upload/compute succeeds or returns a documented
  retry; the error table matches every route.
- **Links:** None yet.

### AUD-2026-11-17-45 — session VWAP zero-fills a missing volume

- **Expected invariant:** A missing measurement is `null` with an unavailable
  reason, never zero (`AGENTS.md` §3); a volume-weighted reading whose window
  contains a missing volume is null (`indicator-coverage.md:51-53`).
- **Observed:** `sessions.rs:158` computes
  `let volume = candle.volume().unwrap_or(0.0);` and accumulates
  `vwap_weighted += typical * volume; vwap_volume += volume`. A session with a
  missing-volume bar publishes a fabricated `session_vwap`; the session level
  event `level.session_vwap_test` can fire against it. No test covers missing
  volume for the session path.
- **Root cause:** The prior critical fix changed the extended readings, but the
  session accumulator kept the zero-fill.
- **Impact:** A price-only or partially missing-volume source publishes a
  fabricated session VWAP and can emit a signal event from it. This is the same
  class as `AUD-2026-11-16-01`, which was marked `Fixed`.
- **Affected contracts/docs:** the indicator coverage contract, the session
  readings, the session level event.
- **Dependencies:** A Quant Validator pass because a signal event changes.
- **Effort:** Small.
- **Options:**
  A. Track session volume completeness and publish `session_vwap` as null from
  the first missing-volume bar for the rest of the session.
  B. Reject or flag missing volume for session-aware runs at ingestion.
- **Recommendation:** A.
- **Verification:** A session with a missing-volume bar publishes null VWAP;
  `level.session_vwap_test` cannot fire; a regression test covers the session
  path.
- **Links:** Prior `AUD-2026-11-16-01`.

### AUD-2026-11-17-46 — the divergence RSI window can read past confirmation

- **Expected invariant:** A signal event that claims to be knowable at
  `available_bar_index` does not read a later bar (`AGENTS.md` §3).
- **Observed:** `rsi_window_value` sets `end = occurred_at + window`
  (`divergence.rs:255-268`), while the event records
  `available_bar_index = current.available_at` (`:321-332`), the pivot
  confirmation bar. With `window > scale.lookback()` — for example minor/3 or
  major/6 — the RSI window includes bars after the event became knowable. The
  config parser accepts any representable window (`model_v4.rs:325-333`); the
  default major/2 is safe.
- **Root cause:** The default window equals the lookback, so the boundary case
  was never distinguished; the config allows a wider window.
- **Impact:** A configurable signal event can read future RSI and emit an event
  earlier than its data supports.
- **Affected contracts/docs:** the model causality contract, the divergence
  event, the model v4 config.
- **Dependencies:** A Quant Validator pass; the fix can change event output for
  a non-default config.
- **Effort:** Small.
- **Options:**
  A. Reject or clamp `window <= pivot_scale.lookback()` at config load.
  B. Compute only up to `pivot.available_at` and set the recorded availability
  to the end of the RSI window.
- **Recommendation:** A.
- **Verification:** A config with `window > lookback` is refused; a prefix test
  proves the event does not change when later bars are appended.
- **Links:** None yet.

### AUD-2026-11-17-47 — a one-bar Donchian window underflows at bar 0

- **Expected invariant:** A valid configuration never panics or silently skips
  an event; a window of one is a valid boundary (`AGENTS.md` §3-4).
- **Observed:** With `donchian_window = 1`, `donchian_position[0]` is `Some`
  when the bar range is non-zero (`structure.rs:154-167`). The event code then
  evaluates `extended.donchian_position.get(bar - 1)` at `bar == 0`
  (`signals.rs:243-245`). Debug builds panic; release wraps to `usize::MAX` and
  emits no event. The Keltner position has the same unguarded `bar - 1` pattern
  (`signals.rs:176`), normally unreachable because ATR is unavailable at bar 0.
- **Root cause:** The event code assumes a prior bar without checking `bar > 0`.
- **Impact:** A valid config can panic in a debug/test build or silently lose a
  position event in release.
- **Affected contracts/docs:** the model event code, the model v4 config.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Add `bar > 0` to both position conditions.
  B. Require `donchian_window >= 2` (and the Keltner window) in config
  validation.
- **Recommendation:** A.
- **Verification:** A `donchian_window = 1` config is either refused or
  produces a safe null/absent event; a test covers bar 0.
- **Links:** None yet.

### AUD-2026-11-17-48 — `structure.scale` is parsed but unused

- **Expected invariant:** A configuration field that selects a documented
  behavior changes that behavior (`docs/registry/README.md`).
- **Observed:** `MarketStateConfig::structure_scale` is assigned in
  `input.rs:193,206,208` and serialized in `model_v4.rs:74,761`, but no
  computation reads it; structure readings and events hard-code
  `PivotScale::Major`. The model contract and the registry say the scale
  selects which confirmed pivots an event may reference
  (`model-m1.0.md:32-34`; `indicators.md:228`).
- **Root cause:** The config field was added before the structure projection
  consumed it, and the projection keeps the default.
- **Impact:** A `structure.scale: "minor"` config silently produces major-scale
  structure output; the registry claims a behavior that does not exist.
- **Affected contracts/docs:** the model v4 config, the structure readings and
  events, the registry.
- **Dependencies:** A Quant Validator pass if the output changes.
- **Effort:** Small to medium.
- **Options:**
  A. Plumb the configured scale into the structure readings/events.
  B. Remove the field from config/registry and update the docs.
- **Recommendation:** A.
- **Verification:** A minor-scale config changes the structure output; a
  registry/config test covers the scale.
- **Links:** None yet.

### AUD-2026-11-17-49 — VWAP slope is off by one bar

- **Expected invariant:** `S = VWAP - VWAP[n]` uses the value `n` bars before
  the current bar (`indicator-coverage.md:59`).
- **Observed:** `extended.rs:430-434` computes
  `let prior = bar + 1 - config.vwap_window;` and subtracts `vwap[prior]`. A
  24-bar window at bar 24 uses bar 1, a 23-bar change; the first value appears
  one bar late. Peer readings `poc_migration` and `obv_slope` use
  `bar - window`.
- **Root cause:** An off-by-one in the prior index; the golden fixture locks the
  current value.
- **Impact:** Every `vwap_slope` value is off by one bar; `signal.vwap_slope`
  events inherit the drift.
- **Affected contracts/docs:** the indicator coverage contract, the extended
  readings, the VWAP slope event, the golden fixture.
- **Dependencies:** A Quant Validator pass; the golden must be regenerated
  deliberately.
- **Effort:** Small.
- **Options:**
  A. Change the prior index to `bar - window` and regenerate the affected
  fixtures.
  B. If the non-overlapping window was intended, correct the registry formula
  and name and add a test.
- **Recommendation:** A.
- **Verification:** The formula matches the contract; a unit test checks the
  prior index; the golden is regenerated deliberately.
- **Links:** None yet.

### AUD-2026-11-17-50 — ATR gates readings that do not use ATR

- **Expected invariant:** A reading is null only when its own formula is
  unavailable (`docs/registry/README.md`).
- **Observed:** `extended.rs:420-435` returns early when ATR is missing, so
  `vwap_band_upper`, `vwap_band_lower`, and `vwap_slope` are null even though
  they use MAD/VWAP only; `structure.rs:129-133` requires ATR for
  `range_expansion` although its formula does not use it.
- **Root cause:** The ATR guard is shared by a block that mixes ATR and
  non-ATR outputs.
- **Impact:** A run with ATR disabled loses valid readings, contradicting their
  registered availability.
- **Affected contracts/docs:** the indicator coverage contract, the extended
  readings.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Decouple these computations from ATR.
  B. Register ATR as a dependency and update availability/tests.
- **Recommendation:** A.
- **Verification:** A run with ATR disabled still publishes the non-ATR
  readings; the registry matches.
- **Links:** None yet.

### AUD-2026-11-17-51 — `session_vwap` is published but unregistered

- **Expected invariant:** The registry is the complete index of every published
  reading (`docs/registry/README.md`).
- **Observed:** `session_vwap` is written to `session.csv`
  (`artifact/model_bundle/artifacts.rs:305`) and used by
  `level.session_vwap_test`, but the `session` indicator's output list contains
  only `session_code` and `session_age` (`indicators.json`).
- **Root cause:** The session VWAP was added to the writer and event before the
  registry entry.
- **Impact:** The registry is not the complete output contract; the event's
  input reading cannot be discovered or validated from the registry.
- **Affected contracts/docs:** the registry, the session writer, the session
  level event.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Add `session_vwap` to the `session` outputs/formula.
  B. Move it to a separate registered indicator with its own claim and tests.
- **Recommendation:** A.
- **Verification:** The registry lists every published reading; a registry test
  covers the session outputs.
- **Links:** None yet.

### AUD-2026-11-17-52 — data-v3 references a retired policy universe

- **Expected invariant:** A current data contract describes a check the loader
  actually performs (`docs/config/2026-10-30/data-v3.md`).
- **Observed:** `data-v3.md:43,57-59` requires the instrument and timeframes to
  be in a policy universe, but `policy-and-strategy.md:3-4` is `Historical`;
  `data.rs:90-96` only checks that the instrument is non-empty and the
  timeframes are ordered/1m-derived.
- **Root cause:** The policy universe was retired with run v3, but the data
  contract kept the clause.
- **Impact:** A data config with an unknown instrument is accepted; the
  documented admission check does not exist.
- **Affected contracts/docs:** the data-v3 contract, the data config loader,
  the historical policy document.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Remove the policy-universe clauses from data-v3.
  B. Validate against a current universe such as `data/partitions.json`.
- **Recommendation:** A.
- **Verification:** The data-v3 document and the loader agree; an unknown
  instrument is refused.
- **Links:** None yet.

### AUD-2026-11-17-53 — the event batch/history excludes extended events

- **Expected invariant:** The event batch/history contains every event the run
  computed (`retention.rs:5-8`).
- **Observed:** `timeframe.rs:296-305` builds `EventBatch` and `EventHistory`
  from the base `events` vector only; extended events exist only in
  `merged_events`.
- **Root cause:** The merged vector was added for the report, but the public
  batch/history views still use the base vector.
- **Impact:** A public consumer of the batch/history misses extended events; the
  retention test only exercises base events. No production consumer was found.
- **Affected contracts/docs:** the retention/model API.
- **Dependencies:** None.
- **Effort:** Small.
- **Options:**
  A. Build both views from `merged_events`.
  B. Rename/document them as base-only and adjust the retention contract.
- **Recommendation:** A.
- **Verification:** The batch/history includes a known extended event; the
  retention test covers it.
- **Links:** None yet.

## Gate evidence

The baseline was `main` at `a07def5c` with a clean working tree. The gate was
run in the baseline worktree with `CARGO_TARGET_DIR=<baseline>/target/gate`:

```text
$ cargo fmt --all -- --check
exit 0
$ cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile ... in 4m 43s
$ cargo test --workspace --no-fail-fast
2,321 passed; 0 failed; 16 ignored
$ RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
exit 0
$ bash tools/checkers.sh
checker gate passed: 25 checkers
```

The checker output includes:

```text
checked 1025 Markdown files; no broken relative links
checked 626 file(s); no banned terms; 13 binary, large, or unreadable file(s) skipped
structure ok: module docs, engine purity, naming, layout, and data allowlist pass
changelog ok: 10 releases, 161 entries
releases ok: 12 release notes, 13 current tags, 9 changelog versions, baseline 2026-11-07
partitions ok: 7 instruments, 6 partitions, 6 aliases
registry ok: 49 indicators, 150 events, 100 parameters
labels ok: the generated client table matches the registry
file size ok: every Rust source file is at or under 1000 lines
manual ok: 12 chapters, 46 commands, 13 screens, 4 tabs
```

Additional read-only checks:

```text
$ python3 tools/test_worktree_cleanup.py      # 8 tests, OK
$ bash tools/test_verify.sh                   # verify self-test ok
$ python3 tools/check_port_parity.py          # port parity ok: 1 registered port
$ docker compose config | grep QR_IMAGE_DIGEST
      QR_IMAGE_DIGEST: null
$ git ls-remote --tags origin v1.3.0
9e764f22abf9ef5b7566667bd57d5a18e3b0ab58  refs/tags/v1.3.0
$ gh release view v1.3.0
release not found
```

The audit-table scan behind `AUD-2026-11-17-06`:

```text
Critical Fixed 1; Critical Verified 2; Critical `Won't fix` 1
Major Open 5; Major Fixed 24; Major Stale 2; Major Verified 9
Minor Open 6; Minor Fixed 21; Minor Stale 5; Minor Triaged 1; Minor Verified 11
Info Fixed 1; Info Triaged 1
total 90
```

The v1.3.0 release workflow was still running at the audit baseline; the tag
was on the remote but the GitHub release did not yet exist. The release note
still said `Prepared` and recorded no image digest or desktop assets. The audit
therefore makes no claim that the v1.3.0 release completed; the finding above is
about the audit state that the release note does not name, not about the
in-flight workflow's outcome.

## Coverage and limitations

| Area | Examined | Result |
|---|---|---|
| Data engine | Candle validation, window ordering, timeframe construction, gaps, partial buckets, UTC, source cadence | No reproduced defect; the strict complete-bucket rule is a documented data-quality trade-off |
| Model engine | Readings, pivots, levels, events, progression, context, session/panel, retention, registry, golden and edge tests | Major session VWAP, divergence-window, Donchian, structure-scale and VWAP-slope findings; no default-config non-determinism reproduced; counts match 49/150/199 |
| Backtest | Strategy boundary, event input, execution fills and brackets, portfolio ledger and sizing, bracket tie rule, worker promotion, no-partial-publish and scale-invariance tests | Critical portfolio cash, Major promotion, canonical-digest, config-identity, execution-path and test-evidence findings; no reproduced model-formula defect |
| Services and runtime | Gateway routes and auth, service boundaries, run/stage lifecycle, evidence/research/stage stores, retention, migrations, object store, backup/restore | Critical restore; Minor path-confinement, retention race, and digest-identity findings |
| Clients | Shell registry and command output, browser capability table and parity test, desktop supervisor and package digest, manual and command reference | Remote secure-context, selection race, label split, simulation parity/bundle, and command-contract findings; 46/46 table presence verified, 33/46 exercised |
| Tests and gate | 2,321 passing tests, 16 ignored, eight Postgres suites, checker behaviour, CI/gate sets, port parity, worktree cleanup test, `test_verify.sh` | Event edge evidence is a readings-only test; eight orphaned Postgres suites; checker and parity gaps |
| Docs and records | `AGENTS.md`, architecture, workflows, model, config, manual, releases, changelog, audits, plans, AI logs, registry, improvement records | Six Major and four Minor documentation/record findings |
| Config and data | `data/partitions.json`, config examples, validate/verify paths, upload limits, compose images | Major holdout-coverage finding; two Minor config findings |
| Security | Auth middleware, token comparison, static-file confinement, config-upload confinement, secret scan, container user | One Minor input-confinement finding; no committed secret found; image runs as UID 10001 |

Limitations:

- Five delegated area passes were launched and all five returned (data/model,
  backtest/app, services/runtime, clients/docs, tests/tooling). Their candidate
  findings were re-read against the current code by the orchestrator, and the
  verified ones are included. The audit is still not a claim that every file was
  read line by line or that the delegated sessions were independent human
  reviews.
- The browser suite (`bun run check`, `bun test`, `bun run build`, Playwright)
  was not run in this audit: `apps/web/node_modules` was absent. The v1.3.0
  release note records 146 `bun test` passes and 53 Playwright passes at the
  release commit; this audit did not reproduce them.
- The composed tier (Postgres, RabbitMQ, S3) was not started. The release note
  records a passed composed run; this audit did not reproduce it. The eight
  direct Postgres conformance suites were not run (no disposable database).
- The Windows desktop package and the Tauri installer were not built. The
  desktop supervisor's package digest and the parity harness were reviewed
  statically.
- Cross-machine determinism was not observed; the contract assigns that to
  reviewer verification.
- The GitHub Project board, issues, and pull requests were not queried. The
  audit reads the repository records; board status is outside this document.
- Historical records under `docs/improvement/`, `docs/reports/`,
  `docs/analytics/`, `docs/course/`, and the pre-reset experiment directories
  were sampled for link and table integrity, not re-audited as current
  behavior.
- Grouped findings (`AUD-2026-11-17-35`, `-36`, `-44`) list several evidence
  points from the delegated passes; the orchestrator reproduced the primary path,
  and the Planner should reproduce each sub-item before planning a fix.
- The audit did not modify code, tests, registry, configuration, issues, or
  pull requests. Every finding is `Open`; the Planner owns the remediation plan
  and the human approves it.

## Recommended next step

The Audit Workflow's next stage is the Remediation Plan. The Planner should
reproduce or code-read each finding, deduplicate it against the prior audit, and
write one plan at `docs/execution-plans/<date>/<issue>-<slug>.md` with the
finding-to-issue-to-verification traceability table. The human approves the plan
before any implementation. The four Critical findings
(`AUD-2026-11-17-18`, `-19`, `-20`, `-38`) should be triaged first: a restore
that can destroy data, a validation/holdout partition that is not enforced, a
negative-cash accounting path, and a cancel that can overwrite a succeeded run
are the highest-risk defects in the tree. The two release/record findings
(`AUD-2026-11-17-05`, `-06`) follow because they determine whether the next
release can be called verified.
