# AI transparency log — Full-program audit, remediation plan, and execution

- Date: 2026-11-16 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md), stages 1–2
  (Audit, Remediation Plan). Claim type: `Behaviour Claim` for the plan; the
  audit itself carries no model, strategy, or edge claim.
- Intent: run a read-only, end-to-end audit of the released v1.1.0 tree on a
  fixed baseline, publish the findings with reproducible evidence, triage the
  prior open findings, and draft a `Proposed` remediation plan for human
  approval.
- Boundary: the audit did not modify code, tests, configuration, registry,
  issues, or pull requests. The only files added are the audit document, this
  log, and the proposed execution plan. The worktree is
  `/home/zantgo/.local/share/opencode/worktree/bd3b02/audit-full-program`, branch
  `docs/audit-full-program-2026-11-16`, based on `main` at `95a977e3`. No commit,
  push, pull request, merge, or board change was made.

## What was read

- `AGENTS.md` sections 2–14; `docs/workflows/README.md`,
  `docs/workflows/audit-workflow.md`, `docs/workflows/execution-plan.md`,
  `docs/audits/README.md`, and the release and record contracts.
- The current status index, architecture documents, command surface, model
  contracts and registry, run contract, and release notes.
- The code paths for the data engine and service, model readings and events,
  backtest strategy/execution/portfolio, application service, API and
  service-API contracts, runtime adapters, gateway, six services, shell,
  browser client, desktop shell and supervisor, simulation engine and service,
  and the gate/checkers.
- The open findings from `AUDIT-2026-11-01-crypto-2023`,
  `AUDIT-2026-11-11-app-service-memory`, and
  `AUDIT-2026-11-11-native-tauri-window`.

## Method and evidence

1. Confirmed the baseline state with `git status --short --branch` and
   `git worktree list`; the tree was clean on `main` at `95a977e3`.
2. Ran `tools/gate.sh` on the baseline. It passed: `cargo fmt --check`,
   `cargo clippy --workspace --all-targets -D warnings`, the full workspace test
   suite, rustdoc, and all 23 checkers. The output records 27 implemented
   commands, 49 indicators, 150 active events, and 199 validation claims. It is
   cited in the audit's gate section.
3. Delegated eight read-only area passes to separate agent sessions: data,
   model, backtest, runtime/desktop backends, gateway/services, clients,
   docs/records, and tools/CI. Three completed (clients, docs/records,
   tools/CI). Five failed with "OpenCode Go usage limit reached" (data, model,
   backtest, runtime/desktop, gateway/services) and were replaced by targeted
   direct review by the orchestrator. The audit's coverage table and
   limitations section say so; the findings are the verified ones, not a claim
   that those areas are exhaustively covered.
4. Consolidated candidate findings by root cause rather than by symptom, then
   re-read each code path and contract before publishing it. The final audit
   holds 22 findings: 1 Critical, 7 Major, 13 Minor, and 1 Info. The material
   findings were verified directly:
   - extended readings: `readings/extended/shared.rs:17-19`, `flow.rs:42-49`,
     `participation.rs:41-53`, compared with the base `readings/rvol.rs` null
     rule, the edge-case test that passes `Some(0.0)`, and the price-only
     instruments in `docs/improvement/2026-11-08/MR7-DATA-REVIEW.md`;
   - desktop identity: `routes/runs.rs:101-106`, `config.rs:110-111,131`,
     `supervisor/src/run.rs:424-429`, the desktop README's "records no image
     digest" sentence, and the run contract's run-id formula;
   - command surface: `AGENTS.md:67` against `help.rs:391,394-413`, the status
     index, and the current research workflows and run contract;
   - release records and image order: the status index, the releases index,
     both 2026-11-15 notes, `check_releases.py`'s tag-line rule, and the push
     and check steps in `release.yml`;
   - browser parity: `App.svelte:88-102`, `RunDetail.svelte:128-147`, the shell
     registry, and the eight-capability parity fixtures;
   - gate and cleanup: the bold-status path in `check_execution_plans.py`, the
     `n/a` path in `check_verification.py`, the substring list in
     `check_structure.py`, `excluded_windows` in `check_partitions.py`, and the
     archive-then-`git worktree remove` sequence in `worktree_cleanup.py`;
   - local memory and runtime hardening: the module declaration, the `rusqlite`
     dependency, the absence of a non-test `record_run` caller, the filesystem
     `publish`/`write_atomic` sequence, and the migration `plan`.
5. Treated the following candidate suggestions as rejected or corrected:
   - a claim that the 64-bit FNV hypothesis fingerprint is itself a defect was
     rejected: it is an established project convention for config and criteria
     fingerprints, and the evidence does not show a collision path;
   - a claim that the desktop supervisor's port check is a defect was rejected:
     the module documents the check-then-release window and retries allocation;
     the audit records it as a limitation, not a finding;
   - duplicate candidates were folded into one finding per root cause: the
     retired-command drift in `AGENTS.md` and the workflows is AUD-03, the
     legacy-scan gap is AUD-21, and the release-note and release-index drift is
     AUD-04;
   - the first draft of this work item carried four material findings and a
     shorter plan. Direct review and consolidation replaced it with the
     22-finding audit, eleven decisions, and 38 verification rows. The earlier
     counts in this log were stale and are corrected here; the published audit
     and plan are the records.

## Verification and evidence

- The audit is `docs/audits/2026-11-16/full-program-audit.md`. Every finding
  carries a stable `AUD-2026-11-16-NN` ID, a severity, file:line evidence,
  impact, at least two options for Critical and Major findings, a
  recommendation, a closure check, and status `Open`.
- At the audit/plan pass, the plan was
  `docs/execution-plans/2026-11-16/full-program-audit-remediation.md`, `Proposed`,
  with eleven decisions (D1–D11), phases P0–P6, a traceability row for all 22
  findings, and a 38-row verification table (V1–V38), every row `Pending`. No
  fix had been implemented then; the implementation pass is recorded below.
- Repository checkers run after the audit, plan, and log were rewritten, each
  passed:
  `python3 tools/check_links.py`,
  `python3 tools/check_terms.py`,
  `python3 tools/check_structure.py`,
  `python3 tools/check_execution_plans.py`,
  `python3 tools/check_doc_length.py`,
  `python3 tools/check_releases.py`,
  `python3 tools/check_command_surface.py`,
  `python3 tools/check_workflows.py`, and
  `python3 tools/check_legacy.py`.
- A consistency pass confirmed 22 findings table rows and 22 matching detail
  blocks, severities `1/7/13/1`, 22 traceability rows covering the full ID
  range, 11 decisions, and 38 contiguous verification rows, all `Pending`.
- Not run: the composed Postgres/RabbitMQ/S3 tier, the Windows package, a live
  gateway, and any market-data run. Findings that depend on those environments
  are marked as needing reviewer or host evidence. The composed tier is itself
  a stub today (AUD-2026-11-16-22).

## Learning

- The baseline gate proves a lot: 27 commands, 49 indicators, 150 events, 199
  claims, and all repository checkers. The findings are therefore
  contract-level gaps the gate cannot see, not build failures. A green gate is
  not evidence that the documented contracts are enforced.
- The highest-risk finding is the missing-volume zero-fill because it changes
  published model numbers on a supported source class; the fix is a Model
  Change with the Quant roles, not an ordinary audit fix.
- The desktop image digest is a provenance defect with a correctness
  consequence: a new build can reuse an old run id. The fix belongs to
  packaging and release operations, not to the evidence route.
- The parity requirement is only as strong as its test. The existing parity
  suite compares one fixture across eight capabilities; it does not enumerate
  the command registry, so five missing browser capabilities passed it.
- A checker's name is not its coverage. The plan-status, verification, purity,
  partition, legacy, and cleanup checks each had a narrow bypass that the green
  gate did not reveal.
- Five delegated passes failed on a provider usage limit. The orchestrator
  substituted targeted direct review and recorded the reduced coverage rather
  than presenting the audit as exhaustive. That is a limitation of this work
  item, not a property of the tree.
- The prior audits contain 18 open findings, several against the removed
  `apps/desktop`. The plan triages them rather than silently carrying them
  forward; the Planner owns the status notes, and the human owns any
  `Won't fix`.

## Implementation passes (same work item)

The sections above describe the audit and plan pass. The human approved the plan
("approved proceed with full implementation") and D1–D11 at their recommended
options, and the implementation pass below followed. At the implementation pass
it did not commit, push, open a pull request, merge, change the board, or move
an audit finding to `Fixed`; those remained gated. The human then authorized a
commit and pull request, recorded in the handoff section at the end. Findings
remain `Open` until review.

### Roles and completeness

- The model change (AUD-01) used sequential Quant Researcher, Quant Developer,
  and Quant Validator passes by one agent. They were not independent agents and
  not independent model judgments. The Quant Validator pass reproduced the new
  null behavior from the tests but did not observe a cross-machine run.
- Engineering work used sequential Architect, Implementation, Test Engineer,
  and Code Reviewer passes by the same agent. No independent reviewer ran.
- The browser work was verified at three layers: the shared Rust/Bun parity
  fixture, the Bun unit suite, and Playwright against the built client.
- No implementation began before the plan and the decisions were present; the
  plan status is now `In Progress`, and the 38 verification rows remain
  `Pending` for the reviewer.

### P1 — missing volume

- `mean_volume`, VWAP, volume profile, flow, MFI, CMF, and participation now
  propagate `None` through their numeric fields when any candle in the window
  has missing volume. OBV and AD publish `None` from the first missing volume
  onward. `Some(0.0)` remains a valid value and is still tested.
- Changed: `crates/quantrule-model/src/readings/extended/{shared,flow,profile,participation,vwap}.rs`,
  `crates/quantrule-model/src/readings/extended/tests.rs`, and
  `crates/quantrule-model/tests/edge_cases.rs`.
- Verified with the model tests and the full workspace test suite. The
  contract note is `docs/model/2026-11-05/indicator-coverage.md`.

### P2 — provenance, release, and cleanup

- AUD-02: the desktop supervisor hashes the six installed sidecars in a fixed
  order and exports `QR_IMAGE_DIGEST` only when the environment does not set it.
  `package.rs` uses a dependency-free SHA-256 with NIST vectors; lifecycle tests
  cover the computed digest and an explicit override.
- AUD-04: `docs/releases/README.md` carries the authoritative current-tag map.
  `check_releases.py --note <tag>` resolves it, and `test_check_releases.py`
  covers invalid JSON, duplicate keys, missing targets, table drift, and an
  unmapped tag.
- AUD-05: the release workflow builds and runs the no-Python and image-contract
  checks before a separate push step; the concurrency key is
  `release-${{ inputs.release_tag || github.ref_name }}`. A textual regression
  test seeds both old behaviors.
- AUD-07: `worktree_cleanup.py` verifies every archive entry (existence, bytes,
  SHA-256), deletes only the archived local data, and then calls
  `git worktree remove` without `--force`. A broken-symlink archive and a
  tampered archive both leave the worktree and branch intact.

### P3 — clients, contracts, and state

- AUD-08: added the Inspect panel (candles, timeframes, events), the Verify
  action, and a verified artifact download. `apps/web/e2e/inspect.e2e.ts`
  covers the three inspections; `run-detail.e2e.ts` covers verify and the
  matching and substituted artifact cases. The shared
  `tests/fixtures/client-parity/api.json` now carries the command list,
  inspection responses, digest, and a 12-byte artifact whose SHA-256 matches the
  run record. Both parity tests enumerate all 27 commands; the browser test
  fails if a command has no control, and the shell test reads the registry from
  `help all`.
- AUD-17: `docs/README.md` now names 49 indicators and 150 events. The client
  document names every implemented component and store, the Inspect and Help
  screens, and the verify/download actions. `tools/check_client_docs.py`
  validates the tables, the navigation labels, and the registry counts.
- AUD-18: `CreateSimulationRequest` now carries `inputs` (defaulted and omitted
  from the body when empty) and `SimulationSummary` carries `bundle` and
  `marked_windows`. The service re-exports both from `quantrule-api`; the
  browser, shell, runtime counts, and SQLite/Postgres summaries use one type.
  The shared `simulation-create.json` is serialized by the browser test and
  decoded by the API test.
- AUD-19: every profile-scoped store has a generation counter and a reset.
  `profileScope.svelte.ts` selects, clears, and reloads datasets, configs, runs,
  and simulations in one place; disconnection, `unauthorized`, and `mismatch`
  clear the same scope. The inspect result, simulation selection, and config
  dataset selection clear on a profile change. `profile-scope.e2e.ts` holds the
  alpha response open to prove no old row survives, and opens a run detail
  before disconnecting to prove it is cleared.
- AUD-20: `resolveLaunch` returns the probed target separately from the launch
  plan; `shell.js` stores it before the update-only early return and Continue
  calls `openGateway` instead of re-reading the typed candidate. The typed URL
  is still what `rememberGateway` persists.

### P4 and P5 — gate and runtime hardening

- P4: the plan-status parser, required verification classes, engine purity,
  partitions, command surface, legacy scan, gate/CI parity, and composed tier
  each received the fix and the seeded regression test described in the plan.
  `tools/checkers.txt` and `tools/checker_tests.txt` are the shared lists.
- P5: the local memory module and `rusqlite` were removed; the six older memory
  findings were marked `Stale` with a dated note, and the two workspace findings
  remain `Open`. The clock returns `Result`, filesystem CAS uses no-replace
  semantics, and migration planning rejects unordered versions.

### Rejected or corrected suggestions

- The first simulation-contract edit added `inputs: ""` to every shell create
  body. The shell contract test caught the wire-shape change. The field now has
  `skip_serializing_if = "String::is_empty"`, so the browser sends `inputs` and
  the shell body is unchanged.
- The first Playwright artifact route matched the percent-encoded run id
  literally and fell through to the run-detail JSON. The fixture now decodes the
  pathname, and the download test asserts both the file name and the hash check.
- The desktop Continue fix was first considered as overwriting the typed field
  with the fallback. The plan says the typed URL is the persisted setting, so
  the fix keeps the typed field and stores the resolved target for the action.

### Verification observed

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace --no-fail-fast` | pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | pass |
| `bash tools/checkers.sh` | `checker gate passed: 24 checkers` |
| `bash tools/gate.sh` | `workspace gate passed with CARGO_TARGET_DIR=<worktree>/target/gate` |
| `bun test` in `apps/web` | `115 pass`, 0 fail |
| `bun run check` in `apps/web` | 0 errors, 0 warnings |
| `bun run build` in `apps/web` | pass |
| `bun run e2e` in `apps/web` | `39 passed` |
| `bun test` in `apps/desktop-shell` | `35 pass`, 0 fail |
| `yaml.safe_load` on `.github/workflows/{ci,release}.yml` | both parse |
| `node --check` on `apps/desktop-shell/ui/{shell,shell-lib}.js` | both parse |
| `tools/test_tiers.sh composed` | gateway healthy, product 1.1.0, run parity verified |

The composed run used project `qr-audit22` on the local Docker/Compose
installation. It is local evidence, not a release claim; the Windows package
was unavailable, and no cross-machine CPU comparison was observed.

### Limitations

- The 38 plan rows are still `Pending`. Filling them is the reviewer's act; this
  log reports the commands and results, not a self-verification.
- The model change is a behavior change on missing-volume inputs. It was
  reproduced by tests, but no independent Quant Validator observed it and no
  market-data baseline was run.
- The composed tier and Playwright suite ran on this host only. The release
  workflow, container registry, and desktop package were not exercised.
- No merge, release, or board update was made; the authorized commit and pull
  request are recorded in the handoff below.

## Handoff

- Commit: `b2c8ebfd` — `fix: remediate full-program audit findings`.
- Branch: `docs/audit-full-program-2026-11-16`, pushed to `origin`.
- Pull request: [#828](https://github.com/quant-rule/quant-rule/pull/828),
  opened against `main` after the human authorized the commit and PR.
- The audit findings remain `Open`, the plan is `In Review`, and the 38
  verification rows remain `Pending`. The human review is the next gate; no
  merge, release, or board edit has been made by the agent.
