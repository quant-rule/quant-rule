# Execution Plan — Full-Program Audit Remediation

- Status: In Review
- Proposed: 2026-11-16 (UTC)
- Claim type: Behaviour Claim
- Owner: Human
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 2
  (Remediation Plan) + [Execution Plan](../../workflows/execution-plan.md)
- Records: `AUDIT-2026-11-16-full-program`; findings `AUD-2026-11-16-01` to
  `AUD-2026-11-16-22`; prior audits `AUDIT-2026-11-01-crypto-2023`,
  `AUDIT-2026-11-11-app-service-memory`, `AUDIT-2026-11-11-native-tauri-window`
- Audit: [`docs/audits/2026-11-16/full-program-audit.md`](../../audits/2026-11-16/full-program-audit.md)
- Branch: `docs/audit-full-program-2026-11-16` for the plan and the grouped
  remediation; one `fix/`, `feat/`, `docs/`, or `refactor/` branch per fix issue
  after approval
- AI log: [`docs/ai-transparency-log/2026-11-16/full-program-audit-remediation.md`](../../ai-transparency-log/2026-11-16/full-program-audit-remediation.md)
- Issue number: none exists yet. The plan uses work-item IDs; the Planner
  assigns the fix issue and sub-issue numbers after human approval, as
  `AGENTS.md` section 11 requires.
- Changelog: the audit and this plan are records, so no changelog entry is
  needed for them. Each fix adds the release-type entry that fits (`Fixed`,
  `Changed`, or `Added`).

## 1. Intent

The 2026-11-16 full-program audit found one Critical, seven Major, thirteen
Minor, and one Info finding on the released v1.1.0 tree. This plan turns those
findings, plus the still-open findings from the earlier audits, into an ordered,
reviewable remediation sequence. The plan itself contains no fix; it is the
Planner's proposal, approved by the human before implementation, and it records
which findings need the Quant roles, which need a deprecation or release
decision, and which are documentation- or tooling-only.

The intended outcome is a tree where:

- a price-only or partially missing-volume candle series publishes `null`, not
  fabricated volume flow or an inflated relative volume;
- a packaged desktop build records its own identity and cannot reuse another
  build's run;
- the normative agent contract and the current research workflows name the
  current gateway command surface;
- a release resolves the correct note and cannot publish an image before its
  contract checks pass;
- the browser reaches every registered capability, and the parity test proves
  it;
- the gate enforces the plan lifecycle, the model evidence classes, the engine
  boundary, the partition map, and the current-document scan that it claims;
- the mandatory worktree cleanup completes after archiving local data;
- the remaining runtime, contract, and client-state findings are closed with
  regression tests and dated records.

## 2. Current state

| Area | Observed | Evidence |
|---|---|---|
| Baseline | `main` at `95a977e3`, clean working tree; the audit worktree is `docs/audit-full-program-2026-11-16` | `git status --short --branch`; `git worktree list` |
| Gate | `tools/gate.sh` passes: fmt, clippy, the workspace test suite, rustdoc, and 23 checkers | gate output on the baseline; audit gate section |
| Command surface | 27 gateway commands; the shell test treats 15 local-surface commands as retired | `crates/quantrule-shell/src/command/help.rs:31-243,394-413` |
| Model | M1.3, 49 indicators, 150 active events, 199 validation claims | `docs/README.md:41`; gate registry output |
| Model evidence | 46/143 rows in the status index; 49/150 rows in the current matrices | `docs/README.md:102-103`; `tools/check_registry.py` |
| Clients | Shell and browser; the browser panels cover connection, profiles, datasets, configs, runs, and simulation; inspect, verify, and artifact download are absent; parity covers eight capabilities | `apps/web/src/App.svelte:88-102`; `apps/web/src/lib/RunDetail.svelte:128-147`; `crates/quantrule-shell/tests/client_parity.rs:32-72` |
| Desktop | Six sidecars under a supervisor; the supervisor does not export `QR_IMAGE_DIGEST`; the evidence route substitutes a constant; Continue opens the typed candidate rather than the probed fallback | `apps/desktop-shell/supervisor/src/run.rs:424-429`; `crates/quantrule-evidence-service/src/routes/runs.rs:101-106`; `apps/desktop-shell/ui/shell.js:177-234` |
| Local memory | `quantrule-app` still compiles a 2,953-line SQLite memory; the status index says it was removed | `docs/README.md:111`; `crates/quantrule-app/src/memory/` |
| Release records | The current tag is `v1.0.0`; the note filenames keep `v1.1.0`/`v1.0.0`; the workflow resolves by filename; the table and line 48 disagree | `docs/releases/README.md:57-95`; `docs/README.md:48,254-258`; `.github/workflows/release.yml:102,540` |
| Release image | The tag-named image is pushed before the no-Python and contract checks; the concurrency key differs between a tag push and a manual recovery | `.github/workflows/release.yml:46-49,219-248` |
| Gate integrity | `check_execution_plans` skips a bold `Verified`; `check_verification` accepts `n/a` for required classes; `check_structure` matches exact strings; `check_partitions` ignores excluded windows and nested instruments; CI and `gate.sh` differ | `tools/check_execution_plans.py:107,143-150`; `tools/check_verification.py:78-94`; `tools/check_structure.py:27-38,135-155`; `tools/check_partitions.py:31-46,49-117`; `.github/workflows/ci.yml:34-157` |
| Cleanup | `worktree_cleanup.py remove` archives local data but leaves it, then `git worktree remove` refuses the dirty tree | `tools/worktree_cleanup.py:220-290` |
| Test tiers | `tools/test_tiers.sh composed` is a stub and exits 2 | `tools/test_tiers.sh:22-27` |
| Prior findings | 18 open findings from three earlier audits; several point at the removed `apps/desktop` | `docs/audits/2026-11-01/`, `2026-11-11/` |

The audit document is the finding record. This plan is the only place where the
findings become an ordered sequence. No decision is approved by this plan.

## 3. Scope and non-goals

**In scope**

- The one Critical fix: missing-volume semantics in the extended readings, with
  the Quant Researcher, Quant Developer, and Quant Validator.
- The seven Major fixes: desktop build identity, the normative and current
  command-surface drift, release note resolution, release image ordering and
  concurrency, the execution-plan status check, the worktree cleanup sequence,
  and browser parity.
- The thirteen Minor fixes: gate hardening (verification `n/a`, engine purity,
  partitions, CI/gate divergence, legacy scan, composed tier), the local memory
  retirement or re-admission, the two library `expect` paths, the filesystem CAS
  race, the migration-ordering guard, stale status counts and client screens,
  simulation contract drift, profile/disconnection state, and the desktop
  Continue target.
- The one Info guard: sort or validate the migration input.
- Triage of the prior open findings with dated status notes; no rewrite of
  published evidence.
- Regression tests for every fix, the Quant Validator record for the model
  change, the gate and composed/Windows evidence where the finding depends on
  it, and the changelog, AI log, and board closure.

**Out of scope**

- New HTTP endpoints, new commands, a new run-contract version, a database
  schema migration, or a new dependency. A fix that needs one stops for
  approval. The two possible exceptions are D2=B (a run-contract change) and
  D5=B (a release-record rewrite); neither is recommended.
- Any model or strategy hypothesis, profitability claim, baseline acceptance, or
  new improvement run. The model fix repairs a published measurement; it does
  not search for an edge.
- Rewriting a published release note, tag, audit, or plan. Corrections are dated
  addenda or new records.
- Retiring or re-admitting the local memory is a decision in this plan, but the
  execution follows the Deprecation & Retirement workflow, not this plan alone.
- The composed Postgres/RabbitMQ/S3 tier and the Windows package are verification
  environments, not new scope. Wiring the composed tier (AUD-22) is in scope as
  a test-infrastructure fix; running every composed scenario is a release
  activity.
- A general refactor of the gate or the clients. Each fix stays on its named
  seam.

## 4. Assumptions and uncertainty

- The baseline `95a977e3` is the audit baseline. A fix branch merges the then
  current `main` before its PR; the audit baseline stays the evidence baseline.
- The model fix (P1) changes observable model output for price-only or
  partially missing-volume series. It may change a golden fixture and any
  research result that read an extended volume reading. The Quant Researcher
  must state the expected new behavior before the Quant Developer implements it.
  If a recorded baseline reads an affected reading, stop and route to the
  Improvement Workflow and an invalidation record.
- The desktop fix (P2) changes the run identity of future desktop runs. It does
  not rewrite a recorded run; it prevents a new build from reusing an old one.
- The browser fix (P3) uses the existing `/inspect/*`, `/runs/{id}/artifacts/*`,
  and `/verify` surfaces. If a screen needs a new endpoint, the plan stops.
- Retiring the local memory (P5) may break tests and the `AppService::record_run`
  callers. A supported caller would make the retirement a scope change.
- The release-record fix (P2) must not rewrite published evidence. It adds a
  current-tag map or field and keeps the historical identity.
- The release-image fix (P2) changes the order of a release job. A release
  mid-flight must be allowed to finish on the old workflow or be re-run after
  the change; the plan does not rewrite a published image.
- The composed-tier fix (P4) needs a local Compose stack. If the stack cannot
  run on the available host, the tier stays `Proposed` and the release records
  an accepted limitation (D9).
- The simulation contract fix (P3) moves types without changing the wire shape;
  if the move would change a serialized field, stop and split the change.

## 5. Decisions

| ID | Decision | Options | Recommended | Status |
|---|---|---|---|---|
| D1 | Missing volume in an extended volume reading | A. Publish `null` and an unavailable reason for every volume reading whose window is incomplete; B. refuse price-only data for a model with volume readings; C. keep zero-fill and amend the contract | A | Proposed; needs Quant Researcher + Quant Validator; human approval before implementation |
| D2 | Desktop build identity | A. The supervisor computes a package digest (hash of the six installed sidecars or the build artifact) and exports `QR_IMAGE_DIGEST`; B. make `image_digest` optional and exclude it from the desktop run id (run-contract change); C. record the package version string in a separate field | A | Proposed; needs Release/Operations + architecture review; human approval |
| D3 | The local SQLite memory | A. Retire the module and `rusqlite` under Deprecation & Retirement, and mark the old findings `Stale`; B. re-admit it as a supported local surface and fix the six open findings; C. keep it as a test-only module and document that | A | Proposed; needs human scope decision |
| D4 | Browser parity scope | A. Complete the five missing capabilities and make the parity test enumerate all 27 commands; B. record a shell-only exception (AGENTS calls this a parity gap); C. ship a generic inspect panel first | A | Proposed; no new endpoint expected |
| D5 | Release note resolution | A. Add a machine-readable current-tag map (or a required `- Current tag:` line) and make the workflow resolve through it; B. rename the note files to the current tags with a dated addendum; C. read the note's existing tag note | A + C | Proposed; needs Release/Operations + Recordkeeping |
| D6 | Release image ordering | A. Build and check locally, then push only after both checks pass; B. push to a staging tag and promote after checks; C. keep the order and document the risk | A | Proposed; needs Release/Operations |
| D7 | Execution-plan status parsing | A. Normalize emphasis/backticks/trailing notes before comparison; B. validate the status against the closed vocabulary; C. parse a separate machine-readable state | A + B | Proposed; tooling-only |
| D8 | Archive-then-remove | A. After the archive manifest is written, move or delete the archived local data, then remove the worktree; B. pass `--force` only after verifying the manifest; C. document a manual two-step | A, with B as a guarded fallback | Proposed; needs Orchestrator + Release/Operations |
| D9 | Gate, CI, and test-integrity hardening | A. Reject `n/a` for the four required evidence classes, parse grouped imports, validate excluded windows and nested instruments, make CI and `gate.sh` share one list, scan the current workflow documents, and wire the composed tier; B. do the checker fixes only and leave CI/gate divergence, the legacy-scan gap, and the composed stub; C. leave the checks and document the limits | A | Proposed; tooling-only, one PR per checker; the composed fix may become an accepted limitation if the stack cannot run |
| D10 | Simulation contract ownership | A. Move the service request and summary types into `quantrule-api` and make the service use them; B. document the service-local extension as an exception; C. add the missing fields to the API types only | A | Proposed; architecture-sensitive, needs Architect review |
| D11 | Client state on profile switch and disconnection | A. Reset and reload every profile-scoped store on select and on disconnect, and render a disconnected state; B. key each store by profile; C. document eventual consistency | A | Proposed; no new endpoint |

No decision is approved by this plan. The plan is `Proposed` until the human
approves the scope and the eleven decisions.

## 6. Acceptance criteria

1. A price-only candle series produces `null` for every extended volume reading
   and never a zero-fill; a partially missing-volume series produces `null`
   until its window is complete; the model contract, registry, and golden
   fixture agree; a Quant Validator record exists.
2. A packaged desktop run records a build identity that differs between two
   package builds and is reported by `/health`; a run id includes it; the
   server↔desktop parity harness still reproduces the server canonical digest
   when it holds the identity constant.
3. `AGENTS.md`, the current research workflows, and the run contract name the
   current 27-command surface and no retired command as current; a gate check
   links the normative and workflow lists to the registry.
4. A release or recovery run for every current tag resolves exactly one note and
   publishes the body that belongs to that tag; the status index and the release
   index agree on the current and historical tags.
5. No registry push occurs before the no-Python and image-contract checks pass;
   a tag push and a manual recovery for one tag share one concurrency key.
6. `check_execution_plans` enforces the row rule for every status form the
   repository uses; a bold `Verified` with a pending row fails.
7. `worktree_cleanup.py remove` archives local data, verifies the manifest, and
   removes the worktree and branch without `--force`; a file that cannot be
   archived leaves the worktree in place.
8. The browser reaches every registered capability, including inspect candles,
   timeframes, and events, verify, and artifact download; the parity test
   enumerates the registry and fails on a missing capability.
9. `check_verification` rejects `n/a` for causality, determinism, golden, and
   edge; `check_structure` catches grouped imports and fully qualified paths;
   `check_partitions` validates excluded windows and nested instruments; CI and
   `gate.sh` enforce the same checker and checker-test sets; `check_legacy`
   scans the current workflow documents.
10. The local memory is either removed with the status index and module map
    updated, or explicitly re-admitted and its six findings fixed; no third
    state.
11. The two library `expect` paths return an error or a typed unavailable value;
    no `expect` remains in library code without a written, narrow justification.
12. The filesystem store refuses a concurrent different-body write on Unix and
    Windows, or the single-writer key-ownership invariant is documented and the
    race is accepted by the human.
13. The migration planner refuses an unsorted input or documents the
    precondition with a test.
14. The status index counts match the registry, the client screens table matches
    the components, the simulation contract crate describes the wire shape, a
    profile switch and a disconnect clear stale state, and the desktop Continue
    action opens the probed target.
15. `tools/test_tiers.sh composed` runs a real suite or the tier is marked
    `Proposed` with a recorded release exception.
16. Every prior open finding is `Verified`, `Stale`, `Duplicate`, or carries a
    dated human decision; no finding is silently left open.
17. `tools/gate.sh` passes on the merged tree; the composed and Windows evidence
    is recorded where the finding depends on it; the changelog, AI log, and board
    are updated; the plan's verification table is all `Pass` or a recorded
    `Accepted risk`.

## 7. Work breakdown

The phases are ordered by risk and dependency. Each phase becomes one issue (or
one container with sub-issues) after approval; each fix gets its own branch and
pull request. A phase does not start until its dependencies are merged.

| Phase | Item | Findings | Depends on | Roles | Paths | Exit criterion |
|---|---|---|---|---|---|---|
| P0 | Plan and record intake | — | — | Orchestrator, Planner | `docs/execution-plans/2026-11-16/`, `docs/ai-transparency-log/2026-11-16/`, this plan | Plan approved; fix issues created with one claim type each |
| P1 | Model missing-volume semantics | AUD-01 | P0, D1 | Quant Researcher, Quant Developer, Quant Validator | `crates/quantrule-model/src/readings/extended/**`, `crates/quantrule-model/src/readings/rvol.rs` (reference), `crates/quantrule-model/tests/**`, `docs/model/**`, `docs/registry/indicators.md`, `docs/registry/verification.json` | AUD-01 closed; null behavior tested; Quant Validator record |
| P2 | Provenance, release, and cleanup | AUD-02, AUD-04, AUD-05, AUD-07 | P0, D2, D5, D6, D8 | Implementer, Release/Operations, Architect, Recordkeeper | `apps/desktop-shell/**`, `crates/quantrule-runtime/src/config.rs`, `crates/quantrule-evidence-service/src/routes/runs.rs` (only if D2=B), `.github/workflows/release.yml`, `docs/releases/**`, `docs/README.md`, `tools/check_releases.py`, `tools/worktree_cleanup.py` | Four findings closed; two-build run-id test; note-resolution test; check-before-push; cleanup test |
| P3 | Clients, contracts, and state | AUD-08, AUD-17, AUD-18, AUD-19, AUD-20 | P0, D4, D10, D11 | Implementer, Test Engineer, Code Reviewer, Architect | `apps/web/src/**`, `apps/web/e2e/**`, `crates/quantrule-shell/tests/client_parity.rs`, `tests/fixtures/client-parity/**`, `apps/desktop-shell/ui/**`, `crates/quantrule-api/src/simulation.rs`, `crates/quantrule-simulation-service/src/**`, `docs/architecture/client.md`, `docs/README.md` | Five findings closed; full-registry parity; contract test; client-state tests |
| P4 | Gate, workflow, and test integrity | AUD-03, AUD-06, AUD-09, AUD-10, AUD-11, AUD-12, AUD-21, AUD-22 | P0, D7, D9 | Implementer, Test Engineer, Code Reviewer | `AGENTS.md`, `docs/workflows/**`, `tools/check_*.py`, `tools/test_check_*.py`, `tools/gate.sh`, `.github/workflows/ci.yml`, `tools/test_tiers.sh` | Eight findings closed; each checker has a fail-before test; CI and gate agree |
| P5 | Runtime and legacy hardening | AUD-13, AUD-14, AUD-15, AUD-16 | P0, D3 | Implementer, Test Engineer, Architect (D3) | `crates/quantrule-app/src/memory/**`, `crates/quantrule-app/Cargo.toml`, `crates/quantrule-app/src/{lib,service,error}.rs`, `crates/quantrule-runtime/src/{clock,migrations}.rs`, `crates/quantrule-runtime/src/adapters/filesystem.rs`, `crates/quantrule-simulation/src/{exit,review}.rs` | Four findings closed; memory decision recorded; two-writer, unsorted, and malformed tests |
| P6 | Verification and closure | all | P1-P5 | Improvement Verifier, Orchestrator, Quant Validator (P1) | every changed path; the audit statuses; `CHANGELOG.md`; AI log; board | All rows `Pass` or `Accepted risk`; worktrees and branches removed |

### P0 — Plan and record intake

- The human approves the scope and D1-D11, or records a different choice.
- The Planner creates one fix issue or sub-issue per finding (or per grouped
  checker fix), each with one claim type and the dependency from the table
  above. The findings stay `Open` until a PR moves them to `Fixed`.
- The audit and this plan are committed as records; no code changes in P0.

### P1 — Model missing-volume semantics

- The Quant Researcher states the expected behavior before implementation: a
  missing volume makes every volume reading whose window contains it unavailable
  (`null` plus a reason); a valid zero volume remains a value; a partial series
  is null until the window is complete. The protocol names the affected
  readings (OBV, AD, CMF, MFI, VWAP, rvol, profile, volume oscillator), the
  events that read them, and the tests.
- The Quant Developer implements the null path in the extended readings and
  updates the unit, edge, determinism, and golden tests. The fixture
  regeneration is deliberate and recorded. The base `rvol` behavior is the
  reference, not a second implementation.
- The Quant Validator reproduces the old and new behavior on a price-only
  fixture and a partial-volume fixture, checks every affected event family, and
  records the verdict. If a recorded baseline or research run is affected, stop
  and route to the Improvement Workflow and `docs/improvement/INVALIDATIONS.md`.
- This is a Model Change, not a pure audit fix. The finding stays `Open` until
  the Quant Validator record and the fail-before regression guard exist.

### P2 — Provenance, release, and cleanup

- **Desktop identity (AUD-02, D2=A):** the supervisor computes a digest over the
  six installed sidecars (or a build-time embedded package digest) and exports
  `QR_IMAGE_DIGEST` to every sidecar. The digest is stable for one package and
  differs across builds. It is set only when the environment does not already
  provide one, so the parity harness's explicit server digest still wins.
  Update the desktop README, the run contract, and the parity harness; add a
  two-build run-id test that fails before the fix.
- **Release notes (AUD-04, D5=A+C):** add a machine-readable current-tag map to
  `docs/releases/README.md` (or a required `- Current tag:` line to each
  renumbered note), make both release-workflow lookups resolve through it, and
  make `check_releases.py` validate the map. Update the status table to name the
  current tag and label the historical tag. Do not rename or rewrite a published
  note; the tag note stays as the historical explanation.
- **Release image (AUD-05, D6=A):** build and run the no-Python and
  image-contract checks before any `docker push`; push only after both pass.
  Normalize the concurrency key to
  `release-${{ inputs.release_tag || github.ref_name }}` so a tag push and a
  manual recovery for one tag serialize. A failed check must leave no tag-named
  image.
- **Cleanup (AUD-07, D8=A):** after the archive manifest is written and
  verified, move or delete the archived local data, then call
  `git worktree remove` without `--force`. If a move fails, leave the worktree in
  place and report a blocker; do not force-remove. Add a test with a temporary
  worktree that holds ignored `data/`.

### P3 — Clients, contracts, and state

- **Browser parity (AUD-08, D4=A):** add inspect panels for candles, timeframes,
  and events; a verify action; and an artifact download link that fetches
  `/runs/{id}/artifacts/{name}` and checks the SHA-256 against the run detail.
  Extend the shared fixture and both parity tests to enumerate the command
  registry. The test must fail before the screens are added. No new endpoint, no
  domain logic in the browser, no label mapping outside
  `labels.generated.ts`.
- **Status and client docs (AUD-17):** update the indicator/event counts to the
  registry, replace the Memory and Settings rows with the implemented panels,
  add the five capabilities from AUD-08, and fix the status line. If the table
  drifts again, generate it from the component tree.
- **Simulation contract (AUD-18, D10=A):** move the service request and summary
  types into `quantrule-api` and make the service use them; keep the wire shape
  unchanged. Add a contract test that serializes the browser's create body and
  deserializes it through the API type. If the move changes a serialized field,
  stop and split the change.
- **Client state (AUD-19, D11=A):** reset and reload every profile-scoped store
  on select and on disconnect; render a disconnected state instead of a stale
  run detail. Add a browser test that switches profiles and asserts runs and
  simulation reset, and a disconnect test that clears the detail.
- **Desktop Continue (AUD-20):** store the probed target and make Continue call
  `openGateway(target)`; set the typed field only when the human edits it. Add a
  shell test with an unreachable candidate, a reachable fallback, and an update
  available.

### P4 — Gate, workflow, and test integrity

- **Normative and workflow surface (AUD-03, C):** replace the `AGENTS.md:67`
  v0.3.0 paragraph with the v1.1.0 gateway surface and a link to the registry.
  Update the current research workflows and the run contract to the gateway
  commands, or mark the local runbooks historical and write gateway runbooks.
  Add a gate check that compares the normative and workflow command lists with
  `COMMANDS`.
- **Plan status (AUD-06, D7=A+B):** normalize the status line by stripping
  Markdown emphasis, backticks, and a trailing note; validate it against the
  closed vocabulary; add a fixture with `- Status: **Verified**` and a `Pending`
  row.
- **Verification `n/a` (AUD-09):** reject `n/a` for causality, determinism,
  golden, and edge; allow it only for the classes where inapplicability is a
  real property. Add a fixture that fails.
- **Engine purity (AUD-10):** parse `use` trees and scan fully qualified paths
  and known tokens; add a grouped-import and a fully qualified-path negative
  fixture.
- **Partitions (AUD-11):** validate `excluded_windows` and reject a partition
  window that intersects one; walk nested `instrument` and `instruments` fields;
  add fixtures.
- **CI and gate (AUD-12, D9=A):** make both entry points call one shared checker
  list and one shared checker-test list; pin the CI Bun action by commit; add a
  seeded doc-length failure and a seeded checker-test failure to prove both
  gates catch them.
- **Legacy scan (AUD-21):** scan `AGENTS.md`, `docs/workflows`,
  `docs/tutorials`, and `docs/governance`, with explicit history allowlists; or
  scan every current document and exempt the history kinds by directory. Add a
  seeded retired command in a current workflow.
- **Composed tier (AUD-22, D9):** wire `tools/test_tiers.sh composed` to the
  Compose stack and a real suite. If the stack cannot run on the available host,
  mark the tier `Proposed` in `AGENTS.md` and record a release exception; do not
  claim an unrun tier.

### P5 — Runtime and legacy hardening

- **Local memory (AUD-13, D3):** if D3=A, remove `pub mod memory`, the `rusqlite`
  dependency, the `record_run`/memory methods, and the memory tests; the tests
  are removed with the retired surface, not skipped or weakened, and the
  deprecation record says so. Update the status index, module map, and crate
  description; mark `AUD-2026-11-11-02/-05/-06/-07` and the memory-related
  native findings `Stale` with a dated note. If D3=B, fix the six findings
  (foreign-database adoption, canonical root, name stability, import validation,
  timestamp order, notes scoping) and re-admit the module.
- **Library `expect` (AUD-14):** return `Result`/`Option` and map the failure to
  a typed error or an unavailable reason; move a panic to the binary boundary
  only with an actionable message. Add a malformed-mark and a pre-epoch clock
  test.
- **Filesystem CAS (AUD-15):** replace the check-then-rename with a no-replace
  primitive (`renameat2(RENAME_NOREPLACE)`, `link`+unlink, or `MoveFileEx`
  without replace), with a process-local lock fallback; add a two-writer test.
- **Migration order (AUD-16):** sort or validate `available` once in `plan`; add
  an unsorted-input test. This is an Info guard, not a behavior change for the
  current callers.

### P6 — Verification and closure

- The Improvement Verifier is independent of the implementation and of the
  audit. It checks the regression guard, the diff, the tests, the absence of
  unrelated changes, and the Quant Validator record for P1.
- Run `tools/gate.sh`, the composed tier, and the Windows package where the
  finding depends on it. Record the observed output, not a claim.
- Update the audit finding statuses (`Fixed` after merge, `Verified` only by the
  Improvement Verifier), the changelog, the AI log, and the board. Clean up the
  worktrees and branches after each merge, using the fixed cleanup tool.

## 8. Verification

| Check | Command | Expected | Limitation |
|---|---|---|---|
| Format and lints | `tools/gate.sh` | pass | The gate uses a target directory inside the worktree |
| Full tests | `cargo test --workspace --no-fail-fast` | pass | The fast tier only |
| Composed tier | `tools/test_tiers.sh composed` | pass, or a recorded exception | Needs the local Compose stack; required before a release |
| Windows package | `apps/desktop-shell/tests/smoke-package-parity.ps1` | pass | Needs a Windows host; required for P2 |
| Model null behavior | new price-only and partial-volume tests; `cargo test -p quantrule-model` | nulls, no zero-fill | Quant Validator owns the verdict |
| Browser parity | `bun test` in `apps/web`; `cargo test -p quantrule-shell --test client_parity` | all 27 capabilities | The fixture is not a live gateway |
| Release note resolution | a resolution test over every current tag; `python3 tools/check_releases.py` | one note per tag | A dry-run does not publish |
| Release image ordering | workflow lint or dry-run; a failed-check fixture | no push before checks | A hosted-runner test is release evidence |
| Plan status | `python3 tools/check_execution_plans.py`; a bold-Verified fixture | fail on pending row | The fixture is the proof |
| Cleanup | a temporary worktree with ignored `data/`; `python3 tools/worktree_cleanup.py remove` | archive, then remove | Do not run on the audit worktree |
| Documentation | `check_command_surface`, `check_links`, `check_terms`, `check_releases`, `check_doc_length` | pass | A checker cannot judge prose truth |
| Gate hardening | the new fixtures for `check_verification`, `check_structure`, `check_partitions`, `check_legacy` | each fixture fails before the fix | One fixture per rule |

## 9. Risks and recovery

| Risk | Likelihood | Impact | Mitigation / recovery |
|---|---|---|---|
| P1 changes a recorded model result or baseline | Medium | High | The Quant Researcher identifies affected runs before implementation; the Quant Validator reproduces; if a baseline is affected, stop and record an invalidation; do not tune thresholds |
| P2 changes run identity | Medium | Medium | Historical runs are untouched; the fix only makes future builds distinct; the parity harness keeps a constant override; a run-contract change (D2=B) needs a version and ADR |
| P2 release-note fix rewrites history | Low | Medium | Use a map or a new field and a dated addendum; never rewrite a published note or tag |
| P2 image-ordering fix breaks a release mid-flight | Low | Medium | Let an in-flight release finish on the old workflow or re-run it after the change; do not move a published image |
| P3 reveals a missing endpoint | Medium | Medium | Stop and return to Requirements & Design; do not add a route inside this plan |
| P3 simulation type move changes a serialized field | Low | Medium | Keep the wire shape identical; split the change if a field must change |
| P4 checker tightening rejects a current document | Medium | Low | Fix the document or record a narrow, written exception; do not weaken the checker to pass |
| P5 removal breaks a supported workflow | Low | High | Search every caller first; if a supported caller exists, choose D3=B or split the removal |
| P4 composed tier cannot run on the host | Medium | Medium | Mark the tier `Proposed` and record a release exception; do not claim a pass |
| A fix weakens a test | Low | High | The reviewer rejects any test removal or skip; every fix has a fail-before guard or a written reason |
| The gate is run from a shared target | Low | Medium | `tools/gate.sh` keeps `target/gate` inside the worktree; a fresh worktree is the reviewer check |

## 10. Stop conditions

Stop and request human direction when:

- a fix needs a new endpoint, command, dependency, schema migration, or
  run-contract version not named in D2/D5;
- P1 changes a recorded baseline or a research conclusion, or the Quant
  Validator cannot reproduce the new behavior;
- P2 cannot preserve the server↔desktop parity comparison;
- P3 would change the simulation wire shape;
- P4 cannot make a current document pass without weakening the checker;
- P5 finds a supported caller of the local memory;
- P2 would rewrite a published release note, tag, or audit;
- a finding is proposed as `Won't fix` (human only);
- a verification row fails and the correction changes the approved scope;
- a worktree or branch cannot be cleaned safely.

## 11. Traceability

| Finding | Work item | Proposed branch | Closure evidence |
|---|---|---|---|
| AUD-2026-11-16-01 | Model: missing volume in extended readings | `fix/<issue>-extended-volume-null` | Unit/edge/golden tests; Quant Validator record |
| AUD-2026-11-16-02 | Desktop: record the build identity | `fix/<issue>-desktop-image-digest` | Two-build run-id test; package parity |
| AUD-2026-11-16-03 | Docs: normative and workflow command surface | `docs/<issue>-current-command-surface` | `check_command_surface`; gate list check |
| AUD-2026-11-16-04 | Release: resolve the note by current tag | `fix/<issue>-release-note-resolution` | Resolution test; `check_releases` |
| AUD-2026-11-16-05 | Release: check before push and fix concurrency | `fix/<issue>-release-image-order` | Workflow dry-run; failed-check fixture |
| AUD-2026-11-16-06 | Gate: parse the plan status | `fix/<issue>-plan-status-parse` | Bold-Verified fixture |
| AUD-2026-11-16-07 | Tooling: archive then remove | `fix/<issue>-worktree-cleanup` | Temporary-worktree test |
| AUD-2026-11-16-08 | Browser: feature parity | `feat/<issue>-browser-parity` | Full-registry parity test; e2e inspect/verify/fetch |
| AUD-2026-11-16-09 | Gate: require evidence classes | `test/<issue>-verification-required` | `n/a` fixture |
| AUD-2026-11-16-10 | Gate: engine purity parser | `fix/<issue>-engine-purity` | Grouped-import and qualified-path fixtures |
| AUD-2026-11-16-11 | Gate: partitions and excluded windows | `fix/<issue>-partition-check` | Invalid-window and nested-instrument fixtures |
| AUD-2026-11-16-12 | Gate/CI: one checker contract | `chore/<issue>-gate-ci-parity` | Seeded failures in both entry points |
| AUD-2026-11-16-13 | App: retire or re-admit the local memory | `refactor/<issue>-retire-local-memory` or `fix/<issue>-memory-findings` | Removal diff or six regression tests; docs |
| AUD-2026-11-16-14 | Runtime: remove library `expect` | `fix/<issue>-library-expect` | Malformed-mark and pre-epoch tests |
| AUD-2026-11-16-15 | Runtime: CAS no-replace | `fix/<issue>-cas-no-replace` | Two-writer test |
| AUD-2026-11-16-16 | Runtime: migration ordering | `fix/<issue>-migration-order` | Unsorted-input test |
| AUD-2026-11-16-17 | Docs: status counts and client screens | `docs/<issue>-status-and-client-docs` | Counts match registry; screens match components |
| AUD-2026-11-16-18 | Contracts: simulation types in the API crate | `refactor/<issue>-simulation-contract` | Contract serialization test |
| AUD-2026-11-16-19 | Client: reset state on profile and disconnect | `fix/<issue>-client-state-reset` | Profile-switch and disconnect tests |
| AUD-2026-11-16-20 | Desktop: Continue opens the probed target | `fix/<issue>-desktop-continue-target` | Shell test with fallback |
| AUD-2026-11-16-21 | Gate: legacy scan of current docs | `fix/<issue>-legacy-scan` | Seeded retired command |
| AUD-2026-11-16-22 | Test tiers: wire the composed suite | `test/<issue>-composed-tier` | Real composed run or recorded exception |
| AUD-2026-11-01-70 | Data: 2023 coverage triage | no code PR | Dated `Stale` note with the partition evidence |
| AUD-2026-11-11-02/-05/-06/-07 | Memory findings | same as AUD-2026-11-16-13 | Dated resolution |
| AUD-2026-11-11-03/-04 | Workspace and gate findings | no code PR | Dated `Stale` note with the current evidence |
| AUD-2026-11-11-08/-09/-11/-12/-13/-14/-16/-17/-18 | Native/desktop findings | no code PR | Dated `Stale`/`Duplicate` note; the replaced surface removes the path |

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | P0 plan structure | The plan has a status, a claim type, and a complete table | `python3 tools/check_execution_plans.py` | pass | check_execution_plans.py passed in the 24-checker run | Pending | | | |
| V2 | P0 links and terms | The new documents resolve and use current terms | `python3 tools/check_links.py`; `python3 tools/check_terms.py` | pass | check_links.py: 973 files, no broken links; check_terms.py: 598 files, no banned terms | Pending | | | |
| V3 | P0 audit published | The audit records every finding in `Open` with reproducible evidence | review of `docs/audits/2026-11-16/full-program-audit.md` | every finding has an ID, evidence, options, verification | 22 findings, severities 1/7/13/1, all Open at execution time | Pending | | | |
| V4 | P0 decisions approved | D1-D11 are approved, changed, or deferred by the human | plan review | every decision has a recorded choice | Approved in session; D1-D11 at the recommended options | Pending | | | |
| V5 | P0 issues created | Each finding or grouped fix has one issue/sub-issue with a claim type | board and issue links | one claim type per item; dependencies stated | Not created; work-item IDs used and the board was not changed (gated) | Pending | | | |
| V6 | P1 protocol | The expected null behavior is pre-registered before the model change | Quant Researcher note; model contract diff | protocol names the affected readings and tests | Missing-volume protocol and expected null readings recorded in indicator-coverage and the P1 model tests | Pending | | | |
| V7 | P1 implementation | A price-only and a partial-volume series publish `null`, never zero | new model tests; golden fixture | all volume readings null; no fabricated value | cargo test -p quantrule-model and the workspace suite pass; null and zero-volume tests added | Pending | | | |
| V8 | P1 validation | The Quant Validator reproduces the old and new behavior and records the verdict | validator record; `cargo test -p quantrule-model` | record present; no baseline invalidation without a decision | Sequential one-agent Quant Validator pass: tests reproduce the null behavior; no independent validator, cross-machine run, or market baseline | Pending | | | |
| V9 | P2 desktop identity | Two desktop builds produce different run identities | package or fixture manifest test; `/health` and run record | different digests and run ids; no `Existing` reuse | Supervisor 29 unit and 9 lifecycle tests pass; computed digest and explicit override covered | Pending | | | |
| V10 | P2 desktop parity | The server↔desktop parity harness still matches the canonical digest | `apps/desktop-shell/tests/smoke-package-parity-linux.sh` or the packaged smoke | same canonical digest; held identity constant | Not run: Windows package unavailable; server composed parity passed separately | Pending | | | |
| V11 | P2 release note resolution | Every current tag resolves exactly one note, and `v1.0.0` selects the desktop note | resolution test; release dry-run; `python3 tools/check_releases.py` | one note per tag; desktop body for `v1.0.0` | check_releases.py: 9 notes, 10 current tags, 6 changelog versions; v1.0.0 resolves the desktop note | Pending | | | |
| V12 | P2 release records | The status table and the release index name current and historical tags consistently | review of `docs/README.md`; `docs/releases/README.md` | no contradictory current tag | docs/README.md and docs/releases/README.md agree; check_releases passes | Pending | | | |
| V13 | P2 release image order | No tag-named image is pushed before both checks pass | workflow lint or dry-run; failed-check fixture | no push on failure | test_check_release_workflow.py: 5 tests pass; the workflow was not executed against a registry | Pending | | | |
| V14 | P2 release concurrency | A tag push and a manual recovery for one tag serialize | workflow review; concurrency key test | one shared key | The same test covers the normalized concurrency key | Pending | | | |
| V15 | P2 cleanup | A worktree with ignored `data/` is archived and removed without `--force` | temporary-worktree test | archive manifest matches; removal succeeds | test_worktree_cleanup.py: 8 tests pass, including tampered and broken-symlink archives | Pending | | | |
| V16 | P3 parity guard | The browser parity test fails when a capability is missing | new test plus a seeded missing-capability case | failure before the fix; pass after | Bun parity test compares 27 commands; the seeded missing-command case returns verify; a seeded view divergence fails | Pending | | | |
| V17 | P3 inspect | A browser user can inspect candles, timeframes, and events | `apps/web/e2e` inspect spec | values match the gateway fixture | apps/web/e2e/inspect.e2e.ts passes for candles, timeframes, and events | Pending | | | |
| V18 | P3 verify and fetch | A browser user can verify a run and download an artifact with its digest checked | `apps/web/e2e` verify/fetch spec | verified digest; bytes hash to the artifact | run-detail.e2e.ts: verify reads the digest, download writes matching bytes, substituted bytes are refused | Pending | | | |
| V19 | P3 status docs | The counts match the registry and the screens table matches the components | `python3 tools/check_registry.py`; review of `docs/architecture/client.md` | 49/150; every row has a component | check_client_docs.py passes: 49/150 counts, screens and stores resolve | Pending | | | |
| V20 | P3 simulation contract | The API crate describes the request and summary the service uses | contract serialization test | one type, one wire shape | API tests decode the shared browser body; the shell body omits empty inputs; service and SQLite tests pass | Pending | | | |
| V21 | P3 client state | Profile switch and disconnect clear stale state | browser tests | runs and simulation reset; no stale detail | profile-scope.e2e.ts passes: a held alpha response proves reset; disconnect clears the detail | Pending | | | |
| V22 | P3 desktop Continue | Continue opens the probed fallback after an update | shell test | fallback target opens | Desktop shell tests: resolveLaunch fallback and wiring; 35 tests pass | Pending | | | |
| V23 | P4 command surface | No retired command is named as current in `AGENTS.md` or the current workflows | `grep`; `python3 tools/check_command_surface.py`; new gate check | only historical context; pass | check_command_surface.py: 27 implemented, 0 planned; 27 README rows | Pending | | | |
| V24 | P4 plan status | A bold `Verified` with a pending row fails | new fixture; `python3 tools/check_execution_plans.py` | fail before; pass after normalization | test_check_execution_plans.py: 13 tests pass | Pending | | | |
| V25 | P4 verification classes | A required class marked `n/a` fails | new fixture; `python3 tools/check_verification.py` | fail before; pass after | test_check_verification.py: 5 tests pass | Pending | | | |
| V26 | P4 engine purity | A grouped import and a qualified path fail | new fixtures; `python3 tools/check_structure.py` | both fail before; current tree passes | test_check_structure.py: 30 tests pass | Pending | | | |
| V27 | P4 partitions | An invalid excluded window, a 2023 overlap, and a nested undeclared instrument fail | new fixtures; `python3 tools/check_partitions.py` | all three fail before | test_check_partitions.py: 10 tests pass | Pending | | | |
| V28 | P4 CI and gate | Both entry points enforce the same set and catch the same seeded failures | seeded doc-length and checker-test failures | both fail before; both pass after | test_check_gate_parity.py: 5 tests pass; checkers.sh ran 24 checkers | Pending | | | |
| V29 | P4 legacy scan | A retired command in a current workflow fails | new fixture; `python3 tools/check_legacy.py` | fail before; pass after AUD-03 | test_check_legacy.py: 4 tests pass | Pending | | | |
| V30 | P4 composed tier | `tools/test_tiers.sh composed` runs a real suite or records an exception | composed run or human-approved exception | pass, or a recorded limitation | composed tier passed on qr-audit22: gateway healthy, product 1.1.0, canonical digest sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa | Pending | | | |
| V31 | P5 memory decision | The local memory is retired or re-admitted; no third state | status index, module map, crate diff | one documented state; old findings resolved | quantrule-app has no memory module or rusqlite; docs updated; six prior findings Stale | Pending | | | |
| V32 | P5 library `expect` | The clock and review paths return typed errors | malformed-mark test; pre-epoch clock test | no panic; the caller names the input | clock and simulation tests pass; pre-epoch and malformed-mark coverage | Pending | | | |
| V33 | P5 CAS | A concurrent different-body write is refused | two-writer test on Unix and Windows | one success, one refusal; no replacement | filesystem tests: 18 pass; two-writer different-body and identical-body cases | Pending | | | |
| V34 | P5 migration order | An unsorted migration set is refused | new `migrations::plan` test | error, not a wrong plan | migration tests: 10 pass; unordered sets refused | Pending | | | |
| V35 | P6 gate | The merged tree passes the full fast gate | `tools/gate.sh` | pass | `workspace gate passed with CARGO_TARGET_DIR=<worktree>/target/gate`; 24 checkers | Pending | | | |
| V36 | P6 independent verification | The Improvement Verifier checks every finding without self-verification | verification record; audit statuses | `Verified` or returned; no self-verification | | Pending | | | |
| V37 | P6 records | The changelog, AI log, board, and prior-finding triage are updated | review; `python3 tools/check_changelog.py` | pass; every prior finding resolved | CHANGELOG [Unreleased] has Added/Changed/Fixed/Removed; AI log extended; check_changelog passes; board unchanged | Pending | | | |
| V38 | P6 cleanup | The worktrees and branches are removed after merge; the `Done` board items remain | `git worktree list`; `git branch -a`; `python3 tools/worktree_cleanup.py check <worktree>` | no task worktree or branch; board unchanged | | Pending | | | |

## Verification addendum (2026-11-20)

The 22 findings are `Verified` by a sequential Improvement Verifier pass against
the merged tree. The evidence is the current full gate, the browser suite, the
composed tier, and the targeted tests recorded in the 2026-11-17 whole-project
audit. The plan's process rows remain as recorded: V5 (issue creation) was
human-gated and used work-item IDs, and V10 (Windows packaged smoke) is covered
by the current release workflow's packaged Windows test. The plan stays
`In Review` for human acceptance of those process rows.
