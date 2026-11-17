# AI transparency log — Whole-project audit, 2026-11-17

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md), stage 1
  (Audit). Claim type: program audit; no model, strategy, edge, or profitability
  claim.
- Intent: run a read-only, whole-project audit of the v1.3.0 tree on a fixed
  baseline, publish the findings with reproducible evidence, and stop at the
  human approval gate before any remediation plan or implementation.
- Boundary: the audit did not modify code, tests, configuration, registry,
  issues, or pull requests. The only files added are the audit document and this
  log. The worktree is
  `/home/zantgo/Proyectos/quant-rule--whole-project-audit`, branch
  `docs/whole-project-audit-2026-11-17`, based on `main` at `a07def5c`. No
  commit, push, pull request, merge, or board change was made.

## What was read

- `AGENTS.md` sections 1–14; `docs/workflows/README.md`,
  `docs/workflows/audit-workflow.md`, `docs/workflows/execution-plan.md`,
  `docs/audits/README.md`, and the release and record contracts.
- The current status index, architecture documents, command surface, model
  contracts and registry, run contract, release notes, manual, and improvement
  records.
- The code paths for the data engine and data service, model readings and
  events, backtest strategy/execution/portfolio, application service, API and
  service-API contracts, runtime adapters, gateway, six services, shell, browser
  client, desktop shell and supervisor, simulation engine and service, and the
  research evaluation crate.
- The open findings from `AUDIT-2026-11-01-crypto-2023`,
  `AUDIT-2026-11-06-current-surface`, `AUDIT-2026-11-07-workflow-status`,
  `AUDIT-2026-11-11-app-service-memory`,
  `AUDIT-2026-11-11-native-tauri-window`, and
  `AUDIT-2026-11-16-full-program`.

## Method and evidence

1. Confirmed the baseline state with `git status --short --branch` and
   `git worktree list`; the tree was clean on `main` at `a07def5c`.
2. Ran the workspace gate on the baseline:
   `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --
   -D warnings`, `cargo test --workspace --no-fail-fast`,
   `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, and
   `bash tools/checkers.sh`. The result was 2,321 passed, 0 failed, 16 ignored,
   25 checkers, 1,025 linked Markdown files, 49 indicators, 150 events, 100
   parameters, 199 validation claims, 46 commands, and 12 manual chapters.
3. Ran the additional read-only checks `python3 tools/test_worktree_cleanup.py`
   (8 tests), `bash tools/test_verify.sh` (self-test), and
   `python3 tools/check_port_parity.py` (1 port).
4. Inspected the compose configuration with `docker compose config` (the image
   digest resolves to `null` when unset), the remote tag with
   `git ls-remote --tags origin v1.3.0`, and the GitHub release state with
   `gh release view v1.3.0` (not found while the workflow was running).
5. Delegated five read-only area passes to separate agent sessions: data/model,
   backtest/app, services/runtime, clients/docs, and tests/tooling. All five
   returned; their candidate findings were re-read against the current code and
   the verified ones are included. The audit's coverage table records the
   delegation and the limits of the evidence.
6. Consolidated candidate findings by root cause, then re-read the primary code
   or document path for each finding before publishing. The grouped findings
   (`AUD-2026-11-17-35`, `-36`, `-44`) list sub-items from the delegated passes;
   the orchestrator reproduced the primary path and marked the rest for the
   Planner. The final audit holds 53 findings: 4 Critical, 26 Major, 18 Minor,
   and 5 Info.
7. Ran the audit-table scan that produces the `AUD-2026-11-17-06` counts:
   5 Major `Open`, 24 Major `Fixed`, 1 Critical `Fixed`, 6 Minor `Open`, and 21
   Minor `Fixed`, with the remaining statuses distributed across `Verified`,
   `Stale`, and `Triaged`.

## Material findings and how they were verified

- The normative workspace map (`AUD-2026-11-17-01`) was checked against
  `Cargo.toml`, the two service `Cargo.toml` dependencies, and the module map;
  the omission and the adapter sentence are direct.
- The workflow narrative (`AUD-2026-11-17-02`) was checked against the workflow
  index; `grep -rn "strategy blueprint" docs/workflows` returns no match, and
  the swarms are role groups in an ADR, not workflow documents.
- The status lag (`AUD-2026-11-17-03`) was checked against the release note and
  the status index; the same stage-surface document contradicts itself.
- The digest identity (`AUD-2026-11-17-04`) was checked against the two route
  fallbacks, the health report, the compose environment, and the run contract.
- The release/audit closure (`AUD-2026-11-17-05`, `-06`) was checked against
  the tag list, the release note, the audit status line, the remediation plan
  and its 38 rows, and the audit-table scan.
- The stage-route omission (`AUD-2026-11-17-07`) was checked against the gateway
  route table and the stage-surface route list.
- The broken run index (`AUD-2026-11-17-08`) was checked with `nl` and the raw
  table block.
- The orphaned Postgres suites (`AUD-2026-11-17-09`) were checked by counting
  the eight `#[ignore]` attributes and searching every wired script for
  `QR_TEST_DATABASE_URL`.
- The config-upload confinement (`AUD-2026-11-17-10`) was checked by following
  `upload::build` → `scan::record` → `RunConfig::load` → `bundle_files` and
  comparing the upload validator with the worker's `safe_key`; the existing
  escape test covers only a file-map key, not a `run.json` reference.
- The stage-retention race (`AUD-2026-11-17-11`) was checked against the
  retention loop, the `artifact_referenced` query, and the create/delete routes.
- The config cap (`AUD-2026-11-17-12`), mutable images (`-13`), index length
  (`-14`), cleanup test (`-15`), AI-log names (`-16`), and verification-map
  scope (`-17`) were each checked with the exact command or file reference in
  the audit.
- The restore schema list (`-18`) was checked against `restore.sh`, the gateway
  and stage migrations, and the round-trip test's table digests.
- The research-partition gap (`-19`) was checked by following `research create`
  → `validated_config` and confirming that only the tier/partition label is
  compared, while the run contract requires the config window to match.
- The portfolio cash path (`-20`) was checked against `SizingMode`, the ledger
  `open`, the config validation, and the test that enshrines the fixed-notional
  behavior.
- The config identity split (`-21`), canonical-digest exclusion (`-22`), and
  promotion hash (`-23`) were checked against `RunConfig::fingerprint`, the
  artifact metadata writes, the manifest allowlist, the portfolio artifact set,
  `research compare`, and the worker's promotion checks.
- The event edge claim (`-24`) was checked with
  `jq '[.events[] | select(.evidence.edge.tests == ["edge_cases::an_empty_series_publishes_no_readings"])] | length'`
  → 149 and by reading the readings-only test.
- The release gate (`-25`), holdout coverage (`-26`), remote browser (`-27`),
  selection race (`-28`), label split (`-29`), simulation parity/bundle (`-30`),
  and entry-bar bracket (`-31`) were checked against the release workflow, the
  data-change record and partition, the browser artifact/store code, the
  artifact and generated label tables, the shell parser and simulation service,
  and the execution engine and its test.
- The strategy boundary (`-32`), config loader (`-33`), ledger halt/funding
  (`-34`), test/checker group (`-35`), client/command group (`-36`), emitter
  checker (`-37`), cancel race (`-38`), health (`-39`), RabbitMQ confirms
  (`-40`), worker concurrency (`-41`), retention (`-42`), service-catalog digest
  boundary (`-43`), and the 30 s client deadline (`-44`) were each checked
  against the cited code path or document.
- The session VWAP (`-45`), divergence window (`-46`), one-bar Donchian
  (`-47`), unused `structure.scale` (`-48`), VWAP slope (`-49`), ATR gating
  (`-50`), `session_vwap` registry (`-51`), data-v3 policy universe (`-52`), and
  event batch/history (`-53`) were checked against the model source, the registry,
  the config parser, and the fixtures. The session VWAP is the same zero-fill
  class as the prior Critical, so it is flagged as a repeat path rather than a
  new class.

## Rejected or corrected suggestions

- A candidate "missing scale-invariance test" finding was rejected: the model
  service has `one_shard_merged_equals_the_full_pass` and
  `one_pass_and_four_shards_share_one_digest`
  (`crates/quantrule-model-service/src/model/compute.rs:189,217`), and the
  control plane says a backtest run is never sharded inside a run. The
  requirement is satisfied.
- A candidate "no look-ahead" finding was rejected: the pivot series indexes
  confirmed pivots by availability bar, and the divergence test
  `an_event_is_not_emitted_before_the_newer_pivot_is_confirmed` covers the
  confirmation delay. No counterexample was found in the examined paths.
- A candidate "auth bypass" finding was rejected: the bearer middleware covers
  every API route, the open paths are only `/health` and `/version`, and the
  token comparison has no early exit.
- A candidate "arbitrary file write" escalation of `AUD-2026-11-17-10` was
  downgraded: the worker's `safe_key` refuses the unsafe bundle before writing,
  so the observed effect is a server-side read and a late run failure, not an
  arbitrary write. The finding is recorded as a Minor input-confinement defect,
  not a Critical.
- The release note's `Prepared` status was not recorded as a standalone defect:
  the v1.3.0 workflow was still running and the GitHub release did not yet
  exist. The audit records the state as a limitation and keeps the finding on
  the audit closure, not on the in-flight workflow.
- A candidate "stage/coupled decision parity" finding was rejected as a defect:
  `docs/architecture/stage-surface.md` §4 explicitly documents that the stage
  simulates its own position and can diverge from the coupled run, and says a
  stage decision is never fed into a backtest. The diagnostic is documented as
  a diagnostic, not a preview.
- The execution-path candidate was downgraded from Critical to Major
  (suspected): the current entry-bar rule is conservative (stop first) and no
  fixture shows an overstated profit, but the contract does not state the
  convention and the test asserts an OHLC-unknowable path. It needs a Quant
  Validator decision, not a unilateral code change.
- The `no-partial-publish`/kill-test gap was kept inside the grouped test
  finding (`AUD-2026-11-17-35`) rather than split into its own row: the existing
  worker and stage tests cover the simulated failure, and the remaining gap is
  the absence of a real process kill.
- The default store credentials were folded into the images finding
  (`AUD-2026-11-17-13`) rather than reported separately: both are deployment
  hardening of the same compose file.
- The model pass's "150 of 150" event-edge count was corrected to "149 exact,
  150 contains": one event also names a second test, while the other 149 name
  the readings-only test alone. The distinction is recorded in `AUD-24`.
- The session VWAP zero-fill was kept as Major rather than promoted to Critical:
  it is the same class as the prior Critical (`AUD-2026-11-16-01`), but it
  affects one reading and one session event and is conditional on a missing
  volume. The audit calls it a repeat path and leaves the severity decision to
  the Planner.
- No default-config look-ahead or fabricated value was confirmed in the model
  pass; the divergence and Donchian findings are conditional on a non-default
  or boundary configuration, and the audit records that condition.

## Learning

The engines and the gate are in good shape: the test suite is broad, the
availability and determinism tests are real, and the service boundaries are
guarded by source tests. The defects cluster at the boundary between the
release record and the code it describes: status tables, image identity, audit
closure, and input confinement. A release-time cross-check that reads the
command/route/audit status from the tree would catch most of them earlier than a
human review does.

The delegated area passes did not return within the audit window. The audit is
therefore honest about its coverage: the findings are the orchestrator's
verified evidence, not a claim that five independent reviews completed. A later
pass can append their findings as dated addenda without rewriting this record.
