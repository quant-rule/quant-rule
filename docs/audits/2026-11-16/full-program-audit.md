# Full-Program Audit — v1.1.0 Desktop Profile

- Date: 2026-11-16 (UTC)
- Audit ID: `AUDIT-2026-11-16-full-program`
- Status: Published; 22 findings `Verified` by a sequential Improvement Verifier pass on 2026-11-20; independent verification pending
- Baseline: `main` at `95a977e3` (2026-11-16, clean working tree)
- Scope: the program end to end — data engine and data service, model engine and
  model service, backtest strategy/execution/portfolio and backtest service,
  shared application service, `quantrule.api/v1` and `quantrule.internal/v1`
  contracts, runtime adapters (Postgres/SQLite, S3/filesystem, RabbitMQ/
  in-process queue, migrations), gateway, the six services, command shell,
  browser client, desktop shell and supervisor, simulation engine and service,
  the gate, CI, release automation, configuration, and the current documentation
  and records.
- Workflow: [`audit-workflow.md`](../../workflows/audit-workflow.md)
- Claim type: program audit; no model, strategy, edge, or profitability claim.
- Auditor: Audit Agent (AI), read-only pass. Area passes were delegated to
  separate agent sessions; five failed on provider usage limits and were
  replaced by targeted direct review. This is not claimed as an independent
  human review or as exhaustive coverage.

## Summary

| Severity | Count |
|---|---:|
| Critical | 1 |
| Major | 7 |
| Minor | 13 |
| Info | 1 |

The baseline is a working, gated tree. `tools/gate.sh` passed on `95a977e3`:
format, clippy, the full workspace test suite, rustdoc, and all 23 repository
checkers are green. The audit found no crash, data corruption, non-deterministic
signal path, look-ahead read, or unrecoverable run-state defect in the tested
paths.

It did find one Critical and seven Major defects:

1. **Critical — fabricated volume readings.** Extended volume readings replace a
   **missing** candle volume with zero, so a price-only or partially
   missing-volume series publishes fabricated OBV/AD flow and inflated relative
   volume instead of `null`. The model contract says a missing denominator
   publishes `null`, never zero.
2. **Major — synthetic desktop identity.** A desktop run without
   `QR_IMAGE_DIGEST` records a digest of the literal `no-image-digest`, so two
   different desktop builds with the same data and configuration share a run id
   and the second reuses the first build's evidence.
3. **Major — retired command surface in normative and current documents.**
   `AGENTS.md` and the `Status: Current` strategy, portfolio, and run-contract
   documents name commands that the shell explicitly retires.
4. **Major — wrong release note after renumbering.** The release workflow
   resolves a note by the current tag's file name. After the 2026-11-15
   renumbering, tag `v1.0.0` resolves to the distributed-platform note
   (`v1.0.0.md`), not the desktop note (`v1.1.0.md`), so a release or recovery
   publishes the wrong body.
5. **Major — image published before checks.** The release workflow pushes the
   tag-named image before it runs the no-Python and image-contract checks; a
   failed release leaves a tag-named image in the registry, and the concurrency
   guard does not actually join a tag push and a manual recovery.
6. **Major — plan verification can be bypassed.** `check_execution_plans`
   compares the status line to the literal `Verified`; a bold
   `- Status: **Verified**`, which several plans use, skips the only row-level
   enforcement.
7. **Major — cleanup fails on the case it exists for.** `worktree_cleanup.py
   remove` archives ignored local data but leaves it in the worktree, then calls
   `git worktree remove` without `--force`; Git refuses to remove a dirty
   worktree, so the mandatory cleanup path fails exactly when there is local data
   to preserve.
8. **Major — browser parity is incomplete and unenforced.** The browser cannot
   reach five shell capabilities, and the cross-client parity test covers eight
   of the 27 commands, so the required feature parity is neither provided nor
   enforced.

The remaining thirteen Minor and one Info findings cover gate hardening
(verification `n/a`, engine-purity substring matching, partition validation,
CI/gate divergence, legacy scanning, the composed-tier stub), the still-compiled
local SQLite memory, two library `expect` paths, a check-then-act filesystem
store, a latent migration-ordering assumption, stale status counts and client
screens, simulation contract drift, client state on profile switch and
disconnection, and the desktop "Continue" action opening the wrong URL.

The audit is read-only. No code, test, registry, configuration, issue, or pull
request was changed. The artifacts added by this work item are this audit, its
AI transparency log, and the proposed remediation plan.

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| AUD-2026-11-16-01 | Critical | model/readings | Extended volume readings treat a missing candle volume as zero; OBV/AD publish `0.0` and relative volume is computed against a depressed baseline instead of publishing `null` | `crates/quantrule-model/src/readings/extended/shared.rs:17-19`; `crates/quantrule-model/src/readings/extended/flow.rs:42-49`; `crates/quantrule-model/src/readings/extended/participation.rs:41-53`; `crates/quantrule-model/src/readings/rvol.rs:11-16,104-122`; `docs/model/2026-11-05/indicator-coverage.md:50`; `crates/quantrule-model/tests/edge_cases.rs:288-301`; `crates/quantrule-model/src/readings/extended/tests.rs:33-44`; `AGENTS.md` §3 | A price-only source publishes fabricated flow and volume ratios; a partially missing-volume source inflates them; active signal events — OBV/CMF/MFI divergences, crosses, and slopes — can read values that were never measured | A. Treat any missing volume in the window as unavailable and publish `null`; B. reject price-only data for models with volume readings; C. document the zero-fill (rejected by the null rule) | A | New unit/edge tests with `None` volume for OBV, AD, CMF, MFI, VWAP, rvol, profile, and volume oscillator; a price-only run publishes `null`, never a fabricated zero; golden fixture updated | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-02 | Major | provenance/desktop | A packaged desktop run records `sha256:…` of the literal `no-image-digest` when `QR_IMAGE_DIGEST` is unset, instead of recording the build identity | `crates/quantrule-evidence-service/src/routes/runs.rs:101-106`; `crates/quantrule-runtime/src/config.rs:110-111,131`; `apps/desktop-shell/supervisor/src/run.rs:424-429`; `apps/desktop-shell/README.md:97-99`; `docs/workflows/run-contract.md:434-445`; `docs/architecture/deployment.md:126` | Every desktop run claims a fixed image identity; two different desktop builds with the same data/config/model derive the same run id, so the second build reuses the first build's run, artifacts, and canonical digest; provenance and reproducibility claims are false | A. The supervisor computes a package digest over the installed sidecars (or embeds the build digest) and sets `QR_IMAGE_DIGEST`; B. make `image_digest` optional and exclude it from the desktop run id (a run-contract change); C. record the package version string as a non-digest field | A | A packaged desktop run's `/health` and run record carry the package digest; two builds produce different run ids; the parity harness still holds the digest constant; a regression test | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-03 | Major | docs/normative | `AGENTS.md` and the `Status: Current` strategy, portfolio, and run-contract documents present retired local commands as current; the shell retires 15 of them | `AGENTS.md:67`; `crates/quantrule-shell/src/command/help.rs:391,394-413`; `docs/workflows/portfolio-research.md:55,136-138`; `docs/workflows/strategy-research.md:55,146-147`; `docs/workflows/run-contract.md:131,211,370`; `tools/check_command_surface.py` scans tutorials only; `tools/check_legacy.py:30-40` | Every agent reads the normative entry point and is told the retired surface is current; a Quant Developer following the current research runbooks types `strategy run`, `execution run`, `portfolio replay`, and `backtest run`, which the gateway shell does not implement; the run contract's `command` field names a retired command | A. Replace the normative and current-workflow references with the 27-command gateway surface and link the registry; B. mark the local runbooks historical and write gateway runbooks; C. generate the command lists from the registry and check them in the gate | A + B + C | No retired command is named as current in `AGENTS.md`, the research workflows, or the run contract; `check_command_surface` passes; a gate check cross-checks the normative and workflow command lists against `COMMANDS` | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-04 | Major | release/records | The release workflow resolves a note by the current tag's file name, but after the 2026-11-15 renumbering the current tag and the note filename differ; the status table and line 48 also disagree on the current tag | `.github/workflows/release.yml:102,540`; `docs/releases/README.md:57-95`; `docs/README.md:48,114,254-258`; `docs/releases/2026-11-15/v1.0.0.md:3-9`; `docs/releases/2026-11-15/v1.1.0.md:3-9`; `tools/check_releases.py:75-84` | A release or recovery run for tag `v1.0.0` finds `v1.0.0.md` — the distributed-platform note — and publishes it as the desktop release body; the note's own "current tag is v1.0.0" tag note is never consulted; the status index labels the historical tag names as current, so a reader cannot tell which note belongs to which current tag | A. Add a machine-readable current-tag map (or a required `- Current tag:` line) and make the workflow resolve through it; B. rename the note files to the current tags with a dated addendum preserving the old identity; C. keep the filenames and require the workflow to read the tag note | A + C | A resolution test maps every current tag to exactly one note; a release dry-run for `v1.0.0` selects the desktop note; `check_releases` validates the map; the status table names the current and historical tag | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-05 | Major | release/integrity | The release workflow pushes the tag-named image before the no-Python and image-contract checks, and its concurrency group does not join a tag push with a manual recovery | `.github/workflows/release.yml:46-49,209-248`; the tag push uses `github.ref` (`refs/tags/vX`) while a manual dispatch uses `vX` | A failed release leaves a tag-named image in the registry that consumers can pull; the checks that should gate publication run after the irreversible push; a tag push and a manual recovery can run concurrently for the same tag despite the comment | A. Build and check locally, then push only after both checks pass; B. push to a staging tag and promote after checks; C. keep the order and document the residual risk | A | No registry push occurs before both checks pass; a failed check leaves no tag-named image; a tag push and a manual recovery for one tag share one concurrency key | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-06 | Major | gate/plans | `check_execution_plans` cannot enforce the verification rows of a plan whose status is bold; several plans use `- Status: **Verified**` or `**Approved**` | `tools/check_execution_plans.py:107,143-150`; `docs/execution-plans/2026-11-10/audit-model-structures-remediation.md:6`; `docs/execution-plans/2026-11-11/608-local-research-memory.md:6`; `docs/execution-plans/2026-11-12/clarity-and-p34-shared-pipeline.md:6` | A plan can be marked `**Verified**` with `Pending` or `Fail` rows and the gate passes; the lifecycle rule that a `Verified` plan has every row `Pass` or `Accepted risk` is not enforced for the bold form the repository actually uses | A. Normalize the status by stripping Markdown emphasis/backticks and a trailing note before comparison; B. validate the status against the closed vocabulary and reject an unrecognized form; C. parse the status line with a strict grammar and require a separate `- State:` token | A + B | A fixture plan with `- Status: **Verified**` and a `Pending` row fails; every current plan parses to a known status; the gate passes after normalization | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-07 | Major | workflow/cleanup | `worktree_cleanup.py remove` archives local data but leaves it in the worktree, then calls `git worktree remove` without `--force`; Git refuses to remove a dirty worktree | `tools/worktree_cleanup.py:220-290`; `tools/worktree_cleanup.py:245-260` (archive copies, does not move); `AGENTS.md` §9 (cleanup is mandatory after merge) | The approved cleanup path fails exactly when there is ignored `data/` to preserve; the operator is pushed toward `git worktree remove --force`, which bypasses the archive check, or toward leaving the worktree and branch in place | A. After the archive manifest is written, move or delete the archived local data, then remove the worktree; B. pass `--force` only after re-reading the archive manifest and verifying every file; C. document a manual two-step and require the operator to remove the data first | A, with B as a guarded fallback | A temporary worktree with ignored `data/` is archived, the manifest matches, and the worktree is removed without `--force`; no archive, no removal | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-08 | Major | clients/parity | The browser client cannot reach five shell capabilities; the cross-client parity test covers eight of the 27 commands, so the required feature parity is neither provided nor enforced | `crates/quantrule-shell/src/command/help.rs:31-243`; `apps/web/src/App.svelte:88-102`; `apps/web/src/lib/RunDetail.svelte:128-147`; no `inspect` or `verify` call in `apps/web/src`; `crates/quantrule-shell/tests/client_parity.rs:32-72`; `apps/web/src/lib/clientParity.test.ts:23-31`; `docs/architecture/command-surface.md:10-15`; `AGENTS.md` §2 | A browser user cannot inspect candles, timeframes, or events, verify a run's canonical digest, or fetch artifact bytes; the release's "two clients" claim and the parity rule are unmet; the parity suite passes on a subset and cannot detect the gap | A. Add the missing inspect, verify, and artifact-download screens and extend the parity test to the full command registry; B. record an explicit shell-only exception (AGENTS calls a one-client capability a parity gap, not a completed feature); C. add one generic inspect panel plus a download action | A | A browser test reaches every registered capability; the parity test enumerates the command registry and fails when a capability is missing; a missing screen is a test failure | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-09 | Minor | gate/verification | `check_verification` allows `n/a` for every evidence class, including the four classes the contract requires for every active indicator and event | `tools/check_verification.py:78-94`; `AGENTS.md` §4; `docs/registry/verification.json` (no required class is `n/a` today) | The matrix is honest today, but a future row can mark causality, determinism, golden, or edge as `n/a` and the gate will accept it; the only enforcement of the model-layer evidence contract can be satisfied without a test | A. Reject `n/a` for causality, determinism, golden, and edge; allow it only for the classes where inapplicability is a real property; B. require a written reason and a reviewer for every `n/a`; C. keep the current rule and rely on review | A | A fixture row with `causality: n/a` fails; the current manifest passes without a required `n/a`; the checker prints the required-class scope | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-10 | Minor | gate/structure | The engine-purity check searches for exact strings such as `use std::fs`; a grouped import, a fully qualified path, or an alias passes | `tools/check_structure.py:27-38,135-155`; the negative tests cover the exact strings only | An engine crate can gain filesystem, network, process, clock, or thread access without a gate failure; the purity claim is weaker than the contract | A. Parse `use` trees and also scan fully qualified `std::fs`, `std::net`, `std::process`, `std::env`, `SystemTime`, `Instant`, `rand`, `thread`, and `Mutex` tokens; B. add a compile-time boundary test per engine; C. keep the substring list and document the limit | A | A fixture with `use std::{fs, io};` and a fixture with `std::fs::read` both fail; the current engines pass; the exact-string tests remain | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-11 | Minor | gate/partitions | The partition checker never reads `excluded_windows` and only sees a top-level string `instrument`; a nested instrument declaration is not checked | `tools/check_partitions.py:31-46,49-117`; `data/partitions.json` has `excluded_windows` and all seven instruments; config files may use nested objects | A malformed or overlapping excluded window passes; a partition can silently include a date the program excludes; an instrument declared inside a nested config object is not checked against the canonical set | A. Validate `excluded_windows` (schema, order, reason) and reject a partition window that intersects one; B. walk nested `instrument` and `instruments` fields; C. document the config shape the checker supports | A + B | A fixture with an invalid excluded window fails; a partition overlapping 2023 fails; a nested undeclared instrument fails; the current file passes | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-12 | Minor | gate/CI | CI and `tools/gate.sh` enforce different checker sets; CI also leaves the Bun setup action on a mutable tag while the release workflow pins a commit | `.github/workflows/ci.yml:34-157,169,195`; `tools/gate.sh:13-38`; `.github/workflows/release.yml:335` | A pull request can pass CI while `check_doc_length` or `check_labels` fails, and a local gate can pass a checker whose unit test is broken; the two "gates" do not mean the same thing; the CI Bun action can move under the repository | A. Make both entry points call one shared checker list and one shared unit-test list; B. have `gate.sh` run the checker unit tests and CI run every checker; C. document the split as intentional | A + B | The checker sets match; a seeded doc-length failure fails both; a seeded checker unit-test failure fails both; the Bun action is pinned by commit | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-13 | Minor | app/legacy | The local SQLite research memory is still compiled into `quantrule-app` even though the status index says it was removed; no service or client reaches its write path | `docs/README.md:111`; `AGENTS.md:53`; `docs/architecture/module-map.md:36`; `crates/quantrule-app/src/lib.rs:40`; `crates/quantrule-app/Cargo.toml:15`; `crates/quantrule-app/src/memory/` (2,953 lines); `crates/quantrule-app/src/service.rs:166-190`; no non-test caller of `record_run` | The product binary carries a retired SQLite surface and its schema migrations; the status index and module map disagree; the six open v0.9 memory findings remain attached to code that no client reaches | A. Retire the module under the Deprecation & Retirement workflow and mark the old findings `Stale`; B. re-admit it as a supported surface and fix the six findings; C. keep it as a test-only module and document that | A | `quantrule-app` no longer contains `memory` or `rusqlite`; the status index and module map agree; the gate passes; the old findings are resolved with a dated note | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-14 | Minor | code/panic | Two library paths still use `expect`: the wall clock before the Unix epoch and the simulation review's stored schema words | `crates/quantrule-runtime/src/clock.rs:20-29`; `crates/quantrule-simulation/src/exit.rs:535`; `crates/quantrule-simulation/src/review.rs:185-186,334-335,384-385`; `AGENTS.md` §3; prior `AUD-2026-11-10-08` | A host with a pre-epoch clock, or a malformed stored mark that reaches review, panics instead of returning an error; the library-code rule is violated in the same class the M1.3 audit already corrected | A. Return `Result`/`Option` and map the failure to an unavailable reason or a typed error; B. move the panic to the binary boundary with an actionable message; C. narrow allow with a written justification | A | No `expect` in the library paths; a malformed mark/bundle returns an error; a pre-epoch clock test returns an error | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-15 | Minor | runtime/concurrency | The filesystem content-addressed store's immutability check is check-then-act: two writers can both pass `publish`'s read/compare and the second rename replaces the first on Unix | `crates/quantrule-runtime/src/adapters/filesystem.rs:26-30,326-346,354-361,398-401`; `crates/quantrule-runtime/src/adapters/filesystem.rs:601-633` (single-threaded tests only) | The module promises an immutable object and a different body is refused; under concurrent writers a different body can silently win, so a reader can see bytes that do not match the key's first publication | A. Use a no-replace primitive (`renameat2(RENAME_NOREPLACE)`, `link`+unlink, or `MoveFileEx` without replace) and map an existing key to a refusal; B. add a per-key lock in the process and a lock file across processes; C. document the key-ownership invariant and make the race out of scope | A | A concurrent two-writer test with different bodies produces one winner and one refusal; no replacement; the existing single-writer tests still pass | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-16 | Info | runtime/migrations | `migrations::plan` assumes `available` is sorted: it uses `available.last()` as the highest version and only checks adjacent duplicates | `crates/quantrule-runtime/src/migrations.rs:101-138`; callers pass sorted constant arrays today | A future caller that passes an unsorted set gets a wrong "database ahead" decision and can miss a duplicate version; latent, not reachable through the current callers | A. Sort or validate the input once in `plan`; B. assert the precondition at the call site; C. document the precondition | A | An unsorted `available` test is refused; the current sorted callers are unchanged; the gate passes | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-17 | Minor | docs/status | The status index and the web client architecture describe an older surface: the verification matrices are listed as 46/143 rows while the registry has 49/150, and `client.md` lists Memory and Settings screens the browser does not have and says v1.0.0 | `docs/README.md:3,102-103`; `docs/architecture/client.md:3,25,28`; `apps/web/src/App.svelte:88-102`; `apps/web/src/lib/` has no Memory or Settings panel; `docs/model/2026-11-06/indicator-verification.md` (49 data rows); `docs/model/2026-11-06/event-verification.md` (150 data rows) | A reader believes verification covers 46 indicators and 143 events, understating the 3 indicators and 7 events that the current registry adds; the client description hides the five-capability parity gap and describes the retired v0.9 UI | A. Update the counts and the screens table to the current registry and components; B. generate both from the registry/component tree; C. mark the client document historical | A | The counts match `check_registry`; every screen row names an implemented component; the status line matches the current release; a doc-to-component check passes | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-18 | Minor | contracts/simulation | `quantrule-api`'s simulation create request has no `inputs` field and its summary is a subset of the service's summary; the service defines its own request and summary types | `crates/quantrule-api/src/simulation.rs:104-130`; `crates/quantrule-simulation-service/src/routes/runs.rs:25-40`; `crates/quantrule-simulation-service/src/records.rs:183-200`; `docs/architecture/http-api.md:146-147`; `apps/web/src/lib/simulation.ts:85-89` | The shared `quantrule.api/v1` crate is not the wire contract for simulation: a Rust client using it cannot send the `inputs` the browser sends and the service records, and the summary omits `bundle` and `marked_windows`; the next client can drift the same way | A. Move the service request and summary types into `quantrule-api` and make the service use them; B. document the service-local extension as a known exception; C. add the missing fields to the API types only | A | A contract test serializes the browser's create body and deserializes it through the API type; the service and the API type agree; the docs list every field | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-19 | Minor | clients/state | Switching profile reloads only datasets and configs; runs and simulation keep the previous profile's state, and a run detail keeps rendering after a load error | `apps/web/src/lib/ProfileSwitcher.svelte:8-12`; `apps/web/src/lib/profiles.svelte.ts:54-60`; `apps/web/src/lib/runs.svelte.ts:30-42`; `apps/web/src/lib/RunDetail.svelte:77-78,128-148` | A user can see another profile's runs or simulation rows until the next poll; a disconnected run detail can keep showing stale content beside an error; profiles are organisational, but the display is still wrong | A. Reset and reload every profile-scoped store on select and on disconnect, and render a disconnected state instead of stale detail; B. key each store by profile; C. document eventual consistency | A | A browser test switches profiles and sees runs and simulation reset; a disconnect test clears the detail; no stale row is shown | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-20 | Minor | desktop/launch | With an update available, "Continue to QuantRule" opens the typed candidate URL, not the probed fallback target the status line names | `apps/desktop-shell/ui/shell.js:130-145,177-211,234`; `apps/desktop-shell/ui/shell-lib.js:191-198` | On a supervisor fallback port, the launcher says the gateway is reachable at the fallback but Continue opens the unreachable candidate; the human sees a connection failure on the path the launcher recommended | A. Store the resolved target and make Continue call `openGateway(target)`; B. set `elements.gateway.value = target` when the probe succeeds; C. document the fallback and require Retry | A or B | A shell test with an unreachable candidate, a reachable fallback, and an update available navigates to the fallback; the typed URL is still persisted | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-21 | Minor | gate/legacy | `check_legacy` does not scan `AGENTS.md`, `docs/workflows`, `docs/tutorials`, or `docs/governance`, so a retired command in a current workflow is invisible to it | `tools/check_legacy.py:30-40`; `docs/workflows/portfolio-research.md:136-138`; `docs/workflows/strategy-research.md:146-147` | The checker that is supposed to catch retired labels cannot catch the AUD-03 drift in the current workflow documents; the gate reports a clean legacy scan while the documents name retired commands | A. Add the current-document paths with explicit history allowlists; B. scan every current document and exempt the history kinds by directory; C. rely on `check_command_surface` and extend it to workflows | B + C | A seeded retired command in a current workflow fails; the history directories remain exempt; the current tree passes after AUD-03 is fixed | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |
| AUD-2026-11-16-22 | Minor | tests/tiers | The composed tier named by the test contract is a stub: `tools/test_tiers.sh composed` prints "not wired yet" and exits 2 | `tools/test_tiers.sh:22-27`; `AGENTS.md` §4; `.github/workflows/release.yml:272-305` (the stack health check is the only composed evidence) | The release process cannot run the composed suite the contract requires before a release; the stack health check proves startup, not the composed behavior; the tier is acknowledged by #643/#644 but remains a gap | A. Wire the composed suite to the Compose stack; B. mark the tier `Proposed` in AGENTS and record a release exception until it exists; C. replace the tier with the release stack smoke and document the reduced claim | A, with B until wired | `tools/test_tiers.sh composed` runs a real suite and returns pass/fail; the release note records its result; no release claims an unrun tier | Verified | [plan](../../execution-plans/2026-11-16/full-program-audit-remediation.md); [PR #828](https://github.com/quant-rule/quant-rule/pull/828) |

## Detail

### AUD-2026-11-16-01 — extended volume readings zero-fill a missing volume

- **Expected invariant:** a missing measurement is `null` with an unavailable
  reason and is never replaced by zero; a valid zero is a value (`AGENTS.md` §3).
  The model contract states the same rule for a missing denominator: "Division
  by zero or a missing denominator publishes `null`, never zero"
  (`docs/model/2026-11-05/indicator-coverage.md:50`). The base relative-volume
  reading implements it: it returns `None` when the current volume is absent or
  any volume in the baseline window is absent
  (`crates/quantrule-model/src/readings/rvol.rs:11-16,104-122`).
- **Observed:** the extended-reading helper is
  `candle.volume().unwrap_or(0.0)`
  (`crates/quantrule-model/src/readings/extended/shared.rs:17-19`).
  `mean_volume` sums that helper over the window, so a missing volume lowers the
  baseline. `flow_series` writes `obv[0] = Some(0.0)` and
  `ad_line[0] = Some(0.0)` before reading any volume, then adds a term that is
  zero for a missing volume (`flow.rs:42-49`). A price-only series therefore
  publishes constant-zero OBV/AD as measured values, and a partially missing
  series publishes flow that treats absent bars as zero-volume bars.
  `participation_series` uses the same helper for `rvol_fast`, `rvol_slow`, and
  `volume_oscillator`, so the baseline is depressed and the ratio is inflated
  (`participation.rs:38-57`).
- **Reachability:** the data engine admits a candle with no volume
  (`crates/quantrule-data/src/candle.rs:335-359`), and the MR7 data review admits
  `COPPERCMDUSD` and `TSLAUSUSD` as price-only instruments
  (`docs/improvement/2026-11-08/MR7-DATA-REVIEW.md:17-30`); both are in the
  current instrument list (`data/partitions.json`). Every model run computes the
  extended readings, so the values are published even when no configured event
  reads them.
- **Test gap:** the edge suite's zero-volume case uses `Some(0.0)`, not `None`
  (`tests/edge_cases.rs:288-301`); the extended unit tests that say "without
  volume" also pass `0.0` (`readings/extended/tests.rs:33-44`). The base
  relative-volume tests do cover `None` (`readings/rvol.rs:114-122`). The
  verification evidence for OBV and AD points at
  `per_row_extended_indicators::*_row_contract`, which has no missing-volume
  case (`docs/registry/verification.json`, `obv` and `ad_line` rows;
  `docs/model/2026-11-06/indicator-verification.md:14`), so the matrix reports
  coverage that the test does not exercise.
- **Impact:** the registry marks OBV, CMF, MFI, and the volume oscillator as
  active `signal`/`context` readings with divergence, cross, and slope events
  (`docs/registry/indicators.md:25-29,117-131`). A strategy can therefore read a
  fabricated measurement on a supported source class; the model contract's null
  rule and the "no value that was never supplied" property are violated. This is
  a silent wrong result in the model output, so it is `Critical` even though no
  accepted strategy baseline currently consumes the affected events.
- **Options:** A. make `mean_volume` return `None` when any volume in its window
  is absent, and make the flow/profile/VWAP/participation series `None` until
  their volume inputs are complete; B. refuse price-only data for a model that
  publishes volume readings; C. keep the zero-fill and amend the model contract
  (not recommended; it contradicts the null rule, the base reading, and the
  "valid zero is a value" distinction).
- **Recommendation:** A, with a per-reading availability reason.
- **Verification:** a unit test per extended volume reading with `None` volume
  and a mixed window; a price-only model run whose bundle carries `null` for
  every volume reading; an updated golden fixture only after the contract is
  accepted; a Quant Validator record because this changes model output.

### AUD-2026-11-16-02 — the desktop invents an image digest

- **Expected invariant:** a run id is
  `sha256(data_version ‖ config_hash ‖ model_version ‖ image_digest)`, and a
  different image digest is a different execution even when the canonical digest
  matches (`docs/workflows/run-contract.md:434-445`). The image digest is
  reported by `/health` and recorded in provenance
  (`docs/architecture/deployment.md:126`).
- **Observed:** the evidence submission route falls back to
  `Digest::sha256(b"no-image-digest")` when the process has no image digest
  (`routes/runs.rs:101-106`). The runtime configuration defaults
  `image_digest` to `None` unless `QR_IMAGE_DIGEST` is set
  (`config.rs:110-111,131,249-251`). The desktop supervisor passes
  `QR_PROFILE`, `QR_DATA_DIR`, `QR_BIND`, the service URLs, `QR_STATIC_DIR`,
  and `QR_CONFIG_ROOT`, but never `QR_IMAGE_DIGEST`
  (`apps/desktop-shell/supervisor/src/run.rs:424-429`). The desktop README says
  a packaged run without one "records no image digest yet"
  (`apps/desktop-shell/README.md:97-99`); the record actually carries a digest
  of a constant.
- **Impact:** every desktop run records the same synthetic identity. A new
  installer with the same data and configuration derives the same run id, so the
  evidence service returns the old run, artifacts, and canonical digest; the
  new build is never executed and its provenance is misstated. The
  server↔desktop parity harness avoids the problem only because it explicitly
  sets the server digest (`apps/desktop-shell/tests/smoke-package-parity-linux.sh:46-51`).
- **Options:** A. compute a package digest at build time (a hash over the six
  sidecars or the packaged artifact) and have the supervisor export it as
  `QR_IMAGE_DIGEST`; B. make `image_digest` optional and exclude it from the
  desktop run id, with an unavailable-reason field (a run-contract change);
  C. record the package version string in a separate field and keep the digest
  absent (does not satisfy the current run-id contract).
- **Recommendation:** A. It preserves the contract, makes an upgrade a distinct
  execution, and keeps the parity harness's explicit override.
- **Verification:** a packaged run's `/health` and run record carry the package
  digest; two package builds with the same inputs derive different run ids; the
  parity harness still reproduces the server digest when it holds the value
  constant; a regression test fails before the fix.

### AUD-2026-11-16-03 — the normative and current workflows name retired commands

- **Expected invariant:** the command set is the product's public interface, and
  the current surface is the 27 gateway commands in
  `docs/architecture/command-surface.md`; the shell help and the current
  documents must agree, and a retired command is not documented (`AGENTS.md`
  §2-3; `crates/quantrule-shell/src/command/help.rs:1-6`).
- **Observed:** `AGENTS.md:67` says "The current tree implements the v0.3.0 local
  surface" and lists data inspection, the event batch, `report model`,
  `strategy run`, `execution run`, `portfolio replay`, `backtest run`,
  `qualify`, `analytics evaluate`, `run check`, `fetch`, `memory`, and help.
  The shell's own test asserts that every one of those names is **not**
  documented and that the registry has exactly 27 commands
  (`help.rs:391,394-413`). The status index says 27 implemented commands
  (`docs/README.md:40`). The drift is not limited to `AGENTS.md`: the
  `Status: Current` portfolio workflow tells the Quant Developer to run
  `execution run`, `portfolio replay`, and `backtest run`
  (`docs/workflows/portfolio-research.md:55,136-138`); the strategy workflow
  uses `strategy run` and `backtest run`
  (`docs/workflows/strategy-research.md:55,146-147`); and the run contract's
  `command` field and examples use `backtest run`/`report model`
  (`docs/workflows/run-contract.md:131,211,370`).
- **Impact:** agents and readers trust the normative entry point and the current
  runbooks over the generated help; they attempt retired commands, mis-scope
  work, and cannot tell which contract is current. The discovery gate does not
  catch the drift because `check_command_surface` scans the help, README, and
  tutorials, not `AGENTS.md` or the workflow documents, and `check_legacy` does
  not scan those paths either (AUD-2026-11-16-21).
- **Options:** A. replace the `AGENTS.md` paragraph and the workflow/run-contract
  examples with the v1.1.0 gateway surface and a link to the registry; B. move
  the local runbooks under `Historical` and write gateway runbooks for the same
  research loop; C. generate the command lists from the registry and add a gate
  check for the normative and workflow documents.
- **Recommendation:** A + B for the documents that describe a procedure, and C
  as the guard so the drift cannot return.
- **Verification:** no retired command is named as current in `AGENTS.md`, the
  research workflows, or the run contract; `python3 tools/check_command_surface.py`
  passes; a new gate check compares the normative and workflow command lists with
  `COMMANDS`; a seeded retired command fails.

### AUD-2026-11-16-04 — release note resolution and the tag records disagree

- **Expected invariant:** a release tag resolves to exactly one note, and the
  release index is the single mapping from a note to the current tag
  (`docs/releases/README.md:24-40,57-95`).
- **Observed:** the 2026-11-15 renumbering kept the note filenames and headings.
  The current desktop tag is `v1.0.0`, whose historical note is `v1.1.0.md`; the
  previous distributed tag is `v0.9.0`, whose historical note is `v1.0.0.md`
  (`docs/releases/README.md:64-83`). The release workflow resolves the body with
  `find docs/releases -type f -name "$tag.md"` at both the gate step
  (`release.yml:102`) and the publish step (`release.yml:540`). For the current
  tag `v1.0.0` it therefore selects `docs/releases/2026-11-15/v1.0.0.md`, the
  distributed-platform note, and publishes it as the desktop release body. Both
  notes do carry a "Tag note" that states the current tag, but the workflow
  never reads it. The status index also contradicts itself: line 48 says
  `v1.0.0` is the latest published release and `v0.9.0` is previous, while the
  releases table labels the desktop note `v1.1.0` and the distributed note
  `v1.0.0` (`docs/README.md:48,254-258`), and line 114 says `v1.1.0` is the
  latest release record. `tools/check_releases.py:75-84` enforces that the
  `- Tag:` line equals the filename, so the historical identity is locked in.
- **Impact:** a release or recovery run for `v1.0.0` publishes the wrong release
  body; a reader or automation that trusts the table or the `- Tag:` line names a
  tag that no longer exists; the mapping is only discoverable in a separate
  document. This is a release-record integrity defect, not a cosmetic drift.
- **Options:** A. add a machine-readable current-tag map (or a required
  `- Current tag:` line in each renumbered note) and make the workflow resolve
  through it; B. rename the note files and tag lines to the current tags with a
  dated addendum that preserves the old identity; C. keep the filenames and
  require the workflow to read the tag note and the index.
- **Recommendation:** A + C. The map is one source of truth, the workflow stops
  guessing from a filename, and a dry-run test can prove the mapping.
- **Verification:** a resolution test maps every current tag to exactly one note;
  a release dry-run for `v1.0.0` selects the desktop note; `check_releases`
  validates the map; the status table names the current tag and labels the
  historical one.

### AUD-2026-11-16-05 — the image is published before its contract is checked

- **Expected invariant:** a release publishes only an image that passed the
  no-Python and image-contract checks; a tag push and a manual recovery for one
  tag cannot run at the same time
  (`docs/workflows/release-operations.md`; `release.yml:46-49`).
- **Observed:** the build step pushes `$REF` and `sha-$(git rev-parse HEAD)` at
  `release.yml:219-225`. The no-Python check runs at line 230 and the image
  contract at lines 235-248. A failure in either leaves the tag-named image in
  the registry. The concurrency group is
  `release-${{ inputs.release_tag || github.ref }}` (`release.yml:46-49`): a tag
  push supplies `refs/tags/vX`, a manual dispatch supplies `vX`, so the two do
  not share a group despite the comment that says they do.
- **Impact:** a bad image can be pulled under the release tag before the release
  workflow reports failure; a retry cannot remove the published tag without a
  separate registry operation. A tag push and a manual recovery can race on the
  same tag and produce two release updates.
- **Options:** A. build and check locally, then push only after both checks pass;
  B. push to a staging tag and promote (retag or copy) after the checks; C. keep
  the order and document the residual risk and the manual cleanup.
- **Recommendation:** A, with the concurrency key normalized to
  `release-${{ inputs.release_tag || github.ref_name }}`.
- **Verification:** a workflow-level test or dry-run proves no registry push
  before both checks; a failed check leaves no tag-named image; a tag push and a
  manual recovery for one tag serialize.

### AUD-2026-11-16-06 — a bold Verified status bypasses the plan row check

- **Expected invariant:** a plan marked `Verified` has every verification row
  `Pass` or `Accepted risk` (`docs/workflows/execution-plan.md`;
  `check_execution_plans.py:143-150`).
- **Observed:** the status regex captures the rest of the line
  (`STATUS = re.compile(r"^- Status:\s*(.+)$")`), and the comparison is
  `plan_status == "Verified"` (`tools/check_execution_plans.py:107,143`). A plan
  that writes `- Status: **Verified**` yields `**Verified**`, so the row check is
  skipped. The repository uses that form: the M1.3 remediation plan, the 608
  memory plan, and the P3/P4 clarity plan all carry bold statuses, and several
  other plans use `**Approved**` or a status with a trailing note
  (`docs/execution-plans/2026-11-10/audit-model-structures-remediation.md:6`;
  `docs/execution-plans/2026-11-11/608-local-research-memory.md:6`;
  `docs/execution-plans/2026-11-12/clarity-and-p34-shared-pipeline.md:6`). The
  checker also does not validate the plan status against the closed vocabulary,
  so an arbitrary status is accepted.
- **Impact:** the only row-level enforcement of the plan lifecycle is skipped
  for the form the repository actually uses; a plan can be marked verified with
  pending or failed rows and the gate passes. This undermines the execution-plan
  record and the reviewer checklist.
- **Options:** A. normalize the status by stripping Markdown emphasis, backticks,
  and a trailing note before comparison; B. validate the status against the
  closed vocabulary and reject an unrecognized form; C. parse a separate
  machine-readable state field.
- **Recommendation:** A + B, with a fixture test for the bold and annotated
  forms.
- **Verification:** a fixture plan with `- Status: **Verified**` and a `Pending`
  row fails; every current plan parses to a known status; the gate passes after
  normalization.

### AUD-2026-11-16-07 — the cleanup tool cannot remove the worktree it archived

- **Expected invariant:** after a merged task, the worktree is archived when it
  holds local data and then removed; the archive is verified before removal
  (`AGENTS.md` §9; `tools/worktree_cleanup.py:1-20`).
- **Observed:** `archive_worktree` copies every local-data file to the archive
  and writes a manifest, but does not move or delete the source
  (`tools/worktree_cleanup.py:245-260`). `remove_worktree` then calls
  `_git(main, "worktree", "remove", str(worktree))` without `--force`
  (`tools/worktree_cleanup.py:220-240`). Git refuses to remove a worktree that
  contains modified or untracked files. The ignored `data/` that triggered the
  archive is still there, so the removal fails on exactly the case the tool
  exists to handle. The operator's likely workaround, `git worktree remove
  --force`, bypasses the check the tool just performed.
- **Impact:** the mandatory cleanup path cannot complete for a task with local
  data; the worktree and branch stay, or the operator force-removes and loses the
  safety property. The tool's own documentation promises "archive, verify the
  branch is merged, then remove".
- **Options:** A. after the archive manifest is written, move or delete the
  archived local data, then remove the worktree; B. pass `--force` only after
  re-reading the manifest and verifying every archived file; C. document a manual
  two-step and require the operator to remove the data first.
- **Recommendation:** A, with B as a guarded fallback for platforms where the
  move fails.
- **Verification:** a temporary worktree with ignored `data/` is archived, the
  manifest matches, and the worktree is removed without `--force`; no archive,
  no removal; a file that cannot be archived leaves the worktree in place.

### AUD-2026-11-16-08 — the browser cannot reach five shell capabilities

- **Expected invariant:** the browser client and the command shell share one
  HTTP contract and one use-case layer and are required to have feature parity;
  a capability that only one client can reach is a parity gap, not a completed
  capability, and parity is enforced by a test (`AGENTS.md` §2;
  `docs/architecture/command-surface.md:10-15`).
- **Observed:** the shell registry has 27 commands (`help.rs:31-243`). The web
  client has panels for connection, profiles, datasets, configs, runs, and
  simulation (`App.svelte:88-99`); any other section renders "This screen
  arrives with its issue" (`App.svelte:100-102`). There is no inspect panel for
  candles, timeframes, or events, no verify action, and no artifact download:
  `RunDetail.svelte` lists the artifact names, sizes, and hashes but no link or
  button (`RunDetail.svelte:128-147`). `grep -r "inspect\|verify" apps/web/src`
  returns no matches outside tests.
- **Parity test:** `crates/quantrule-shell/tests/client_parity.rs:32-72` and
  `apps/web/src/lib/clientParity.test.ts:23-31` compare a normalized view of
  health, datasets, dataset, configs, run list, run detail, and profiles — eight
  capabilities. Neither test enumerates the 27-command registry, so the five
  missing capabilities pass the parity suite.
- **Impact:** a human using the browser cannot inspect the data it uploaded,
  check the market state, verify a run's canonical digest, or download the
  results; the v1.1 release's "two clients" statement overstates the surface.
- **Options:** A. add the missing panels/actions and make the parity test
  enumerate the registry; B. declare a shell-only exception and record it
  (AGENTS says that is a gap, not completion); C. add a generic inspect panel
  and a download action as a smaller first step.
- **Recommendation:** A, delivered as one browser-parity phase with the parity
  test extended first so it fails before the screens are added.
- **Verification:** the browser parity test walks every registered command and
  fails on a missing screen or action; a browser test inspects a fixture candle,
  timeframe, and event set; a run detail downloads an artifact and checks its
  digest; the shell parity test stays green.

### AUD-2026-11-16-09 — the verification checker accepts `n/a` for required classes

- **Expected invariant:** every active indicator and event has causality,
  determinism, golden, and edge evidence (`AGENTS.md` §4; `AGENTS.md` §3).
- **Observed:** `check_verification` requires each class to carry a `scope` and
  a `tests` list, and if the scope is `n/a` it accepts the entry without a test
  (`tools/check_verification.py:78-94`). The current manifest has no required
  class marked `n/a`, so the matrix is honest today; the checker permits the
  next row to claim coverage without a test.
- **Impact:** the gate's only enforcement of the model-layer evidence contract
  can be satisfied by writing `n/a`; a future indicator or event can ship
  without the causality/determinism/golden/edge coverage the contract requires.
- **Options:** A. reject `n/a` for causality, determinism, golden, and edge;
  allow it only for the classes where inapplicability is a real property;
  B. require a written reason and a reviewer for every `n/a`; C. keep the rule
  and rely on review.
- **Recommendation:** A.
- **Verification:** a fixture row with `causality: n/a` fails; the current
  manifest passes without a required `n/a`; the checker prints the required
  scope.

### AUD-2026-11-16-10 — engine purity matches exact strings only

- **Expected invariant:** the data, model, and backtest engines perform no I/O,
  clock, randomness, thread, or global-mutable-state access (`AGENTS.md` §3;
  `crates/quantrule-model/tests/boundaries.rs`).
- **Observed:** `FORBIDDEN_ENGINE_IMPORTS` is a tuple of exact strings such as
  `"use std::fs"`, `"use std::net"`, `"use rand"`, and `"static mut "`
  (`tools/check_structure.py:27-38`), and the check is a substring test on the
  file text (`tools/check_structure.py:135-155`). A grouped import
  (`use std::{fs, io};`), a fully qualified call (`std::fs::read`), or an alias
  passes. The negative tests cover only the exact strings.
- **Impact:** an engine crate can gain filesystem, network, process, clock, or
  thread access without a gate failure; the purity claim is weaker than the
  contract, and the boundary test list does not close the gap.
- **Options:** A. parse `use` trees and also scan fully qualified paths and
  known tokens; B. add a compile-time boundary test per engine; C. keep the
  substring list and document the limit.
- **Recommendation:** A, with a negative fixture for a grouped import and a
  fully qualified path.
- **Verification:** a fixture with `use std::{fs, io};` and a fixture with
  `std::fs::read` both fail; the current engines pass; the exact-string tests
  remain.

### AUD-2026-11-16-11 — the partition checker ignores excluded windows and nested instruments

- **Expected invariant:** `data/partitions.json` is the single partition map;
  every partition window respects the excluded windows, and every instrument a
  config cites is declared (`AGENTS.md` §4; `docs/workflows/data-lifecycle.md`).
- **Observed:** `check` validates the instrument list, aliases, partition
  windows, uses, and top-level instrument lists (`tools/check_partitions.py:49-117`).
  It never reads the top-level `excluded_windows` array that the current
  `data/partitions.json` carries, and `config_instruments` reads only a
  top-level string `instrument` (`tools/check_partitions.py:31-46`). A malformed
  excluded window, a partition that overlaps 2023, or an instrument declared
  inside a nested config object passes.
- **Impact:** the gate cannot prove the 2023 exclusion or the nested config
  instrument mapping; a future partition edit can silently violate the data
  contract.
- **Options:** A. validate `excluded_windows` (schema, order, reason) and reject
  a partition window that intersects one; B. walk nested `instrument` and
  `instruments` fields; C. document the config shape the checker supports.
- **Recommendation:** A + B.
- **Verification:** a fixture with an invalid excluded window fails; a partition
  overlapping 2023 fails; a nested undeclared instrument fails; the current file
  passes.

### AUD-2026-11-16-12 — the two gates do not enforce the same checks

- **Expected invariant:** the documented gate is the same contract everywhere;
  a pull request that passes CI has passed the checks `tools/gate.sh` names
  (`AGENTS.md` §4; `docs/workflows/release-operations.md`).
- **Observed:** `tools/gate.sh` runs 23 checkers, including `check_doc_length`
  and `check_labels` (`tools/gate.sh:13-38`). CI runs most of the same checkers
  but omits those two, and it runs the checker unit tests that `gate.sh` omits
  (`.github/workflows/ci.yml:34-157`). CI also uses
  `oven-sh/setup-bun@v2` at lines 169 and 195, a mutable tag, while the release
  workflow pins the same action by commit (`release.yml:335`).
- **Impact:** a pull request can pass CI while a documented gate check fails,
  and a local gate can pass a checker whose unit test is broken; the two "gates"
  do not mean the same thing. The unpinned CI action can move under the
  repository.
- **Options:** A. make both entry points call one shared checker list and one
  shared unit-test list; B. have `gate.sh` run the checker unit tests and CI run
  every checker; C. document the split as intentional.
- **Recommendation:** A + B, and pin the CI Bun action by commit.
- **Verification:** the checker sets match; a seeded doc-length failure fails
  both; a seeded checker unit-test failure fails both; the Bun action is pinned.

### AUD-2026-11-16-13 — the retired local memory is still compiled

- **Expected invariant:** a retired surface is removed or clearly marked
  historical; the status index is the single current-status source
  (`AGENTS.md` §2, §5; `docs/workflows/recordkeeping.md`).
- **Observed:** `docs/README.md:111` says "Local research memory | Historical;
  local SQLite removed in v1.0.0; memory moves to the evidence service (#661)",
  while `AGENTS.md:53` still lists research memory as a `quantrule-app`
  responsibility and the module map repeats it
  (`docs/architecture/module-map.md:36`). `quantrule-app` still declares
  `pub mod memory` (`lib.rs:40`), depends on `rusqlite` (`Cargo.toml:15`), and
  contains 2,953 lines of schema, store, and transfer code;
  `AppService::record_run` still writes it (`service.rs:166-190`). No service or
  client calls `record_run`; the module is exercised only by its own tests.
- **Impact:** the product link carries a retired SQLite surface and migrations;
  the module map still lists memory as a `quantrule-app` responsibility; six
  open v0.9 findings are attached to code that no supported surface reaches, so
  their status is misleading.
- **Options:** A. retire the module and its dependency under the Deprecation &
  Retirement workflow, and mark the old findings `Stale` with a dated note;
  B. re-admit it as a supported local surface and fix the six findings;
  C. keep it as a test-only module and document that.
- **Recommendation:** A, because the status index and the distributed design
  already retired it. This is a scope/architecture decision and needs the human.
- **Verification:** the module and `rusqlite` are gone from `quantrule-app`; the
  status index and module map agree; the gate passes; the old findings carry a
  dated resolution.

### AUD-2026-11-16-14 — library `expect` paths

- **Expected invariant:** library code returns `Result` and does not panic on
  invalid input (`AGENTS.md` §3). The M1.3 audit corrected the same class in
  `readings/extended/band.rs` (`AUD-2026-11-10-08`).
- **Observed:** `clock::now_utc` calls `.expect("the system clock is after the
  Unix epoch")` (`clock.rs:20-29`); the simulation review parses stored schema
  words with `.expect("stored schema word")` in five places
  (`review.rs:185-186,334-335,384-385`) and in `exit.rs:535`.
- **Impact:** a pre-epoch host panics in the service boundary; a malformed
  stored mark or bundle that reaches review panics instead of returning an
  error. The current `validate_mark` path constructs valid enums, so the
  simulation case is a defensive/round-trip panic, not a confirmed user path;
  the clock case is a host-failure panic.
- **Options:** A. return `Result`/`Option` and map to an unavailable reason or a
  typed error; B. move the panic to the binary entry point with an actionable
  message; C. narrow `allow` with a written justification.
- **Recommendation:** A for the clock; A for the review parse (a stored record
  is external input), and no `expect` in library code.
- **Verification:** the paths return an error and the caller names the input;
  a malformed mark test; a pre-epoch clock test; the gate.

### AUD-2026-11-16-15 — filesystem CAS immutability is check-then-act

- **Expected invariant:** an object is immutable once published; repeating the
  same bytes is a no-op and a different body under an existing key is refused
  (`filesystem.rs:26-30`).
- **Observed:** `publish` reads the final path, compares the bytes, and then
  calls `write_atomic`, which stages a temporary file and renames it onto the
  final path (`filesystem.rs:326-346,354-361,398-401`). On Unix `rename`
  replaces an existing destination. Two concurrent writers with different bytes
  can both see `NotFound` or the same old body, both proceed, and the second
  rename replaces the first. The tests exercise the refusal and the killed
  writer, but not two concurrent writers (`filesystem.rs:601-633`).
- **Impact:** the module's immutability promise is false under concurrency; a
  reader sees a complete but wrong object. The current keys are content-addressed
  or unique per run, so the normal path has identical bytes and the race is not
  known to be triggered in production; the defect is in the guarantee, not a
  reproduced run.
- **Options:** A. use a no-replace primitive (`renameat2(RENAME_NOREPLACE)`,
  `link` + unlink, or `MoveFileEx` without replace) and map an existing key to a
  refusal; B. add a per-key lock in the process and a lock file across
  processes; C. document the single-writer key-ownership invariant and make the
  race out of scope.
- **Recommendation:** A, with a process-local lock as a fallback where the
  platform primitive is unavailable.
- **Verification:** a two-thread/two-process test writes different bodies to one
  key and observes one success and one refusal; the final bytes equal the
  winner; the existing single-writer and killed-writer tests still pass.

### AUD-2026-11-16-16 — the migration planner assumes sorted input

- **Expected invariant:** the planner refuses a duplicate version, an unknown
  applied version, a checksum change, and a database ahead of the binary
  (`migrations.rs:1-10,94-139`).
- **Observed:** `plan` uses `available.last()` as the highest available version
  and checks only adjacent pairs for duplicates (`migrations.rs:105-118`). The
  current callers pass sorted constant arrays, so the behavior is correct today;
  an unsorted caller would get a wrong "database ahead" result and could miss a
  duplicate.
- **Impact:** latent; a future caller or a generated migration list can silently
  violate the contract.
- **Options:** A. sort or validate the input once in `plan`; B. assert the
  precondition at the call site; C. document the precondition.
- **Recommendation:** A.
- **Verification:** an unsorted input test is refused; the sorted callers and
  the gate are unchanged.

### AUD-2026-11-16-17 — the status index and client document are stale

- **Expected invariant:** a current architecture document describes the current
  surface, and a status count matches the registry (`AGENTS.md` §5).
- **Observed:** `docs/README.md:102-103` lists the indicator matrix as 46 rows
  and the event matrix as 143 rows; the current registry and matrices have 49
  indicators and 150 events (`tools/check_registry.py`;
  `docs/model/2026-11-06/indicator-verification.md:76`;
  `docs/model/2026-11-06/event-verification.md:160`). The web client document
  says "released v1.0.0" and lists a Memory screen and a Settings screen
  (`docs/architecture/client.md:3,25,28`) that the browser does not implement
  (`apps/web/src/App.svelte:88-102`; no Memory or Settings panel in
  `apps/web/src/lib/`). It also omits the inspect, verify, and
  artifact-download capabilities that the parity finding records.
- **Impact:** the status index understates the current model coverage by 3
  indicators and 7 events; the client document hides the parity gap and
  describes the retired v0.9 UI. A reader trusts counts and a screen catalogue
  that do not match the tree.
- **Options:** A. update the counts and the screens table to the current
  registry and components, and fix the status line; B. generate the counts and
  the table from the registry/component tree; C. mark the client document
  historical.
- **Recommendation:** A, with B as a guard if the table drifts again.
- **Verification:** the counts match `check_registry`; every screen row names an
  implemented component; the status matches the current release; a
  doc-to-component check passes.

### AUD-2026-11-16-18 — the simulation contract crate is not the wire contract

- **Expected invariant:** `quantrule-api` holds the `quantrule.api/v1` contract
  types, and a service uses them rather than redefining the same request
  (`AGENTS.md` §2; `crates/quantrule-api/src/lib.rs`).
- **Observed:** `CreateSimulationRequest` has `premise`, `bundle`, `seed`, and
  `criteria`, but no `inputs` (`crates/quantrule-api/src/simulation.rs:118-130`);
  `SimulationSummary` has `sim_id`, `premise`, `status`, `windows`, and `marks`
  but not `bundle` or `marked_windows` (`simulation.rs:103-116`). The service
  defines its own `CreateRequest` with a defaulted `inputs`
  (`routes/runs.rs:25-40`) and its own `SimulationSummary` with `bundle` and
  `marked_windows` (`records.rs:183-200`). The HTTP contract says POST accepts
  `inputs` and GET returns the inputs digest
  (`docs/architecture/http-api.md:146-147`), and the browser sends `inputs`
  (`apps/web/src/lib/simulation.ts:85-89`). The service works, but the shared
  crate does not describe it, and a Rust client using the crate cannot send the
  field the browser sends.
- **Impact:** the contract crate is a partial, parallel definition; the next
  client can drift the same way, and a compile-time consumer cannot rely on the
  documented shape.
- **Options:** A. move the service request and summary types into
  `quantrule-api` and make the service use them; B. document the service-local
  extension as a known exception; C. add the missing fields to the API types
  only.
- **Recommendation:** A.
- **Verification:** a contract test serializes the browser's create body and
  deserializes it through the API type; the service and the API type agree; the
  docs list every field.

### AUD-2026-11-16-19 — profile switch and disconnection leave stale client state

- **Expected invariant:** a profile-scoped view shows only the active profile,
  and a disconnected view does not present stale content as current
  (`docs/architecture/client.md:53-60`; `AGENTS.md` §2).
- **Observed:** `ProfileSwitcher.change` selects the profile and reloads only
  datasets and configs (`ProfileSwitcher.svelte:8-12`); `profiles.select` only
  updates the active id (`profiles.svelte.ts:54-60`); the runs store has no
  profile reset (`runs.svelte.ts:30-42`). A run detail renders its content after
  an error (`RunDetail.svelte:77-78,128-148`), so a failed refresh can leave the
  previous detail beside the error.
- **Impact:** a user can see another profile's runs or simulation rows until the
  next poll (2 s active, 30 s idle), and a disconnected run detail can keep
  showing stale content. Profiles are organisational, but the display is still
  wrong.
- **Options:** A. reset and reload every profile-scoped store on select and on
  disconnect, and render a disconnected state instead of stale detail; B. key
  each store by profile; C. document eventual consistency.
- **Recommendation:** A.
- **Verification:** a browser test switches profiles and sees runs and
  simulation reset; a disconnect test clears the detail; no stale row is shown.

### AUD-2026-11-16-20 — the desktop Continue button opens the candidate, not the fallback

- **Expected invariant:** when a launch probes several gateway candidates, the
  action that opens the gateway uses the address that answered
  (`apps/desktop-shell/ui/shell.js:208-211`).
- **Observed:** `evaluate` computes `target = reachableAt ?? candidate` and only
  assigns `gateway = target` on the auto-open path (`shell.js:177-211`). When a
  newer release is available, `launchPlan.open` is false, so the function
  returns before that assignment. The Continue button calls `connect`
  (`shell.js:234`), and `connect` normalizes `elements.gateway.value`, which is
  still the typed `candidate` (`shell.js:130-145`). On a supervisor fallback
  port, the status line says the gateway is reachable at the fallback, but
  Continue navigates to the unreachable candidate.
- **Impact:** the recommended action fails on the fallback path; the human sees
  a connection failure after being told the gateway is reachable.
- **Options:** A. store the resolved target and make Continue call
  `openGateway(target)`; B. set `elements.gateway.value = target` when the probe
  succeeds; C. document the fallback and require Retry.
- **Recommendation:** A, with B as a small consistency fix.
- **Verification:** a shell test with an unreachable candidate, a reachable
  fallback, and an update available navigates to the fallback; the typed URL is
  still persisted.

### AUD-2026-11-16-21 — the legacy checker does not scan the current workflow documents

- **Expected invariant:** the retired-label check covers every current artifact,
  not only the README and the architecture tree (`tools/check_legacy.py:1-20`).
- **Observed:** `SCAN` lists `README.md`, `docs/README.md`, `docs/architecture`,
  `docs/model/2026-11-05`, `docs/config`, `config/defaults`,
  `config/examples`, `config/research`, and `crates`
  (`tools/check_legacy.py:30-40`). It omits `AGENTS.md`, `docs/workflows`,
  `docs/tutorials`, and `docs/governance`. The current portfolio and strategy
  workflows name retired commands (`docs/workflows/portfolio-research.md:136-138`;
  `docs/workflows/strategy-research.md:146-147`), and the checker reports a
  clean scan.
- **Impact:** the gate cannot catch the AUD-03 drift in the current workflow
  documents; the clean legacy result is narrower than it appears.
- **Options:** A. add the current-document paths with explicit history
  allowlists; B. scan every current document and exempt the history kinds by
  directory; C. rely on `check_command_surface` and extend it to workflows.
- **Recommendation:** B + C.
- **Verification:** a seeded retired command in a current workflow fails; the
  history directories remain exempt; the current tree passes after AUD-03 is
  fixed.

### AUD-2026-11-16-22 — the composed test tier is a stub

- **Expected invariant:** the three test tiers are fast, composed, and parity;
  composed runs against the real Compose stack and is required before a release
  (`AGENTS.md` §4; `tools/test_tiers.sh:1-10`).
- **Observed:** `tools/test_tiers.sh composed` prints "the composed suite is not
  wired yet; see #643/#644" and exits 2 (`tools/test_tiers.sh:22-27`). The
  release workflow starts the nine-container stack and checks `/health`,
  `/version`, and the container count (`release.yml:272-305`), but that is a
  startup smoke, not the composed suite.
- **Impact:** the release process cannot run the composed suite its own contract
  requires; the stack smoke proves startup, not composed behavior. The gap is
  acknowledged by #643/#644, so this is a tracked limitation, not an unknown
  defect.
- **Options:** A. wire the composed suite to the Compose stack; B. mark the tier
  `Proposed` in `AGENTS.md` and record a release exception until it exists;
  C. replace the tier with the release stack smoke and document the reduced
  claim.
- **Recommendation:** A, with B until wired.
- **Verification:** `tools/test_tiers.sh composed` runs a real suite and returns
  pass/fail; the release note records its result; no release claims an unrun
  tier.

## Prior open findings

The audit examined the open findings from the earlier audits against the current
tree. The Planner, not this audit, owns the status change; these are the
recommended triage notes for the remediation plan.

| Prior finding | Current evidence | Triage |
|---|---|---|
| `AUD-2026-11-01-70` (2023 crypto gaps) | 2023 is excluded from every current partition (`docs/README.md:44`; `data/partitions.json`); the 2023 cells remain historical | `Stale` for the current partitions, with a dated note; the historical experiment keeps its limitation |
| `AUD-2026-11-11-02`, `-05`, `-06`, `-07` (local memory) | The memory module is still present but unreachable from any supported surface; `docs/README.md:111` says removed | `Stale` if the module is retired (AUD-2026-11-16-13 option A); otherwise they remain open |
| `AUD-2026-11-11-03` (workspace canonicalization) | `Workspace::open` now canonicalizes the root (`crates/quantrule-app/src/workspace.rs:42-66`) | `Stale`; the code changed |
| `AUD-2026-11-11-04` (shared Cargo target) | `tools/gate.sh` uses `target/gate` inside the worktree | `Stale`; verify the gate on a fresh worktree |
| `AUD-2026-11-11-08`, `-09`, `-12`, `-13`, `-14`, `-16`, `-17`, `-18` and the other native-window findings | `apps/desktop` no longer exists; the current desktop is `apps/desktop-shell` over the browser client | `Stale`; the replaced surface removes the code path |
| `AUD-2026-11-11-11` (GUI and shell are not one program) | The current browser client still cannot reach inspect, verify, or artifact fetch (AUD-2026-11-16-08) | `Duplicate` of AUD-2026-11-16-08; the old native path is gone, the parity gap is not |
| `AUD-2026-11-11-10` (post-release docs) | Partly corrected; `AGENTS.md:67`, the release table, and `client.md` still drift (AUD-2026-11-16-03/-04/-17) | `Duplicate` of the new documentation findings; keep the remaining items open |

## Gate evidence

The full gate was run on the baseline before the audit findings were written:

```text
$ tools/gate.sh
cargo fmt --all -- --check                     pass
cargo clippy --workspace --all-targets -D warnings  pass
cargo test --workspace --no-fail-fast          pass
cargo doc --workspace --no-deps                pass
all 23 repository checkers                     pass
workspace gate passed with CARGO_TARGET_DIR=<baseline worktree>/target/gate
```

The gate output records 27 implemented commands, 49 indicators, 150 active
events, 199 validation claims, 970 Markdown files with no broken relative links,
and a clean Python-boundary scan.

The document checkers were re-run after this audit and its companions were
written; the AI transparency log records the observed results. The plan's
verification table stays `Pending` until human review and the fixes it
describes. The composed tier (Postgres, RabbitMQ, S3) and the Windows
package were not run in this audit; those remain reviewer/host evidence. The
composed tier is a stub today (AUD-2026-11-16-22), so the release contract's
composed evidence cannot be produced from this tree.

## Coverage and limitations

| Area | Examined | Result |
|---|---|---|
| Data engine and data service | Candle validation, timeframe construction, gaps, window, catalog, upload, SQLite/Postgres repositories | Direct targeted review; the delegated pass failed on a provider limit. No confirmed Critical/Major finding in the examined paths. |
| Model engine | Readings, extended readings, events, pivots, availability, progression, context, registry claims, golden fixtures | AUD-01; otherwise the causality/determinism/golden suites pass |
| Backtest stack | Strategy input and boundary, execution, portfolio, run orchestration, canonical digest, retries, cancellation | Direct targeted review; the delegated pass failed on a provider limit. No confirmed Critical/Major finding in the examined paths. |
| Application service and contracts | Workspace, dataset, config, run, export, memory, API/service-api types, canonical digest, simulation contract | AUD-13, AUD-18; the canonical digest and manifest verification match the contract |
| Runtime | SQLite/filesystem/queue adapters, migrations, health, client, config, supervisor | AUD-14, AUD-15, AUD-16; the queue and SQLite state-machine tests pass |
| Gateway and services | Routes, profile scoping, auth seam, schema ownership, simulation plane, error mapping | Direct targeted review; the delegated pass failed on a provider limit. No confirmed Critical/Major finding in the examined paths. |
| Clients | Shell command set, help, rendering, browser parity, desktop boundary | AUD-03, AUD-08, AUD-17, AUD-19, AUD-20; the shell byte-identical output tests pass, but the cross-client parity test covers a subset |
| Tooling and CI | Gate, checkers, release workflow, Python boundary, partitions, cleanup, test tiers | AUD-04, AUD-05, AUD-06, AUD-07, AUD-09, AUD-10, AUD-11, AUD-12, AUD-21, AUD-22 |
| Documentation and records | AGENTS, status index, releases, changelog, workflows, execution plans, audits, client architecture | AUD-03, AUD-04, AUD-17, prior-finding triage |

Limitations:

- The audit is one AI pass with delegated area sessions. Five of the eight
  delegated passes (data, model, backtest, runtime/desktop, gateway/services)
  failed with "OpenCode Go usage limit reached" and were replaced by targeted
  direct review. The data, backtest, gateway/service, runtime, and model areas
  are therefore not exhaustively covered; the findings reported here are the
  verified ones, not a claim that no other defect exists.
- The composed tier and the Windows package were not executed. A finding that
  depends on Postgres, RabbitMQ, S3, or Windows behavior is marked as such and
  needs the composed/host evidence before closure.
- The audit did not evaluate profitability, edge, or any model/strategy result.
- The audit did not modify the tree; the three files it adds are this document,
  its AI transparency entry, and the proposed remediation plan.

## Verification addendum (2026-11-20)

- **Sequential verifier pass.** All 22 findings are `Verified` against the merged
  tree. The evidence is the current full gate, the browser suite (152 unit and
  54 Playwright cases), the composed tier (13 canonical artifacts per run,
  digest `sha256:a0372f02…`), and the targeted tests recorded in the
  2026-11-17 whole-project audit.
- **Independence.** This is a sequential single-agent pass, not an independent
  verifier. The human remains the final acceptance authority.
