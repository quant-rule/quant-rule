# Audit Remediation — Application Service, Research Memory, and Native Window

- Date: 2026-11-11 (UTC)
- Plan: `audit-app-service-native-remediation`
- Issue: — (audit remediation; no issue number assigned)
- Audits: [`app-service-memory-audit.md`](../../audits/2026-11-11/app-service-memory-audit.md), [`native-tauri-window-audit.md`](../../audits/2026-11-11/native-tauri-window-audit.md)
- Status: In Progress
- Claim type: Behaviour Claim
- Owner: Human
- Workflow: Execution Plan; Audit Workflow remediation

## Intent

Fix every finding of the two 2026-11-11 audits and implement the human decision
that the desktop UI is a first-class surface. The GUI and the shell must be one
program with feature parity and one shared source; neither duplicates the other.
The shell is primarily for agents. The remediation covers data integrity, path
identity, the research memory, the native window, the test gate, and the
records that describe the surface.

## Current state

- Baseline: `main` at `8c4f2ca`; the v0.9.0 release binary matches the audited
  source under `apps/` and `crates/`.
- The app-service audit has findings AUD-2026-11-11-02 through -10.
- The native-window audit has findings AUD-2026-11-11-11 through -18.
- The native window was driven under Xvfb; the core workspace, dataset, config,
  model, memory-open, record, and export paths work. The default run-summary
  export fails and hides the model view; the memory stores no model evidence;
  the GUI lacks the pipeline stages; the shell lacks the memory commands.
- The first audit's earlier IDs were renumbered to avoid the simulation
  data-loss finding AUD-2026-11-11-01.

## Scope

Included:

- Memory and application-service integrity: foreign-database refusal, canonical
  workspace/config paths, stable workspace names, import validation, note
  timestamp validation, atomic logical export.
- Desktop behaviour: run timestamps, run-scoped notes, run-summary export
  default and error handling, startup error handling, native path scope,
  file/folder pickers or inline path validation.
- Research evidence: record the model summary with the run; expose metric,
  claim, and note CRUD in the shared service and both surfaces.
- One-program architecture: move the pipeline use cases behind a shared layer
  and expose them from the shell and the native window without duplication.
- Native verification: add an automated native IPC E2E or a documented
  equivalent gate.
- Records and documentation: the one-program ADR, `AGENTS.md`, architecture
  docs, module docs, plan/contract statuses, and the per-worktree target rule.
- Regression tests for every finding, with a fail-before/pass-after guard or a
  written reason why one is impossible.

Excluded:

- No model, strategy, execution, portfolio, or analytics behaviour change; no
  new signal, parameter, baseline, or edge claim.
- No cloud, account, sync, Windows, or macOS work.
- No new market data or trading hypothesis.
- No change to the v0.9.0 release tag or assets; a future release supersedes it.
- The `fetch` network source is a human decision under Phase 3c: either the GUI
  exposes the same explicit fetch or source acquisition remains shell-only with
  a recorded exception. The local-first "no network path" claim must not be
  weakened silently.

## Assumptions and uncertainty

- The user approved the audit findings, the one-program decision, and this plan;
  implementation may proceed within the plan's scope.
- The architecture extraction (AUD-11) is the largest item. It may be delivered
  in a separate PR/sub-plan after the integrity, native, and records phases. The
  plan's verification table keeps it `Pending` until that sub-plan lands; the
  full plan is not `Verified` while it is open.
- `tauri-plugin-dialog` is a new dependency. Phase 2 may use inline path
  validation (option B) first; the dialog plugin is used only if the human
  approves the dependency in the Phase 2 review.
- The native E2E requires `tauri-driver`, `WebKitWebDriver`, and the Tauri
  `test` feature. If the environment cannot run it, the plan records a dated
  exception and keeps the launch smoke plus a native command-bridge test.
- The shared target directory is a process rule, not a product behaviour; the
  fix is a documented per-worktree target in the plan template and gate
  instructions.
- The memory's model-summary shape is additive; no v1 schema migration is
  required if the summary is stored in the existing `summary_json` or as typed
  metrics. A schema change would require a new migration and a human decision.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Foreign database | Require an empty database or the expected `meta` schema before migrating; restore validates the backup before copying | The stored identity is the only proof of a QuantRule memory |
| Workspace identity | Canonicalize the root and run-config path before recording | One directory must be one workspace |
| Workspace name | Derive from the root directory or an explicit workspace name; never overwrite with a run name | The workspace is stable; the run is not |
| Import validation | Share one record validator between the store and import | The import boundary must not be weaker than the store |
| Logical export | Temp file in the same directory, then rename | Never destroy a good export on a failed write |
| Run export default | Create the output root or default to an existing path; keep the model visible on error | The primary action must work on first use |
| Native path scope | Validate against the workspace and memory roots; use dialogs when available | The webview must not have arbitrary filesystem access |
| Native E2E | `tauri-driver` + `WebKitWebDriver` for the core flow | The native surface needs an automated gate |
| Memory evidence | Record the model summary and expose metric/claim/note CRUD in both surfaces | The memory must store the evidence the UI computes |
| One-program architecture | Extract the pipeline use cases behind a shared layer; both adapters are thin | The user decision requires one source and parity |
| Fetch | Human decision in Phase 3c | The GUI network boundary is a product/security choice |
| Dialog dependency | Inline validation first; `tauri-plugin-dialog` only with explicit approval | New dependencies are a human decision |

## Acceptance criteria

- A foreign SQLite file is refused by `open`/`restore`, left unchanged, and a
  valid backup still restores.
- `/repo` and `/repo/.` record one workspace; a relative and absolute config
  path record one config; the workspace name is stable across runs.
- Import rejects every record the store rejects and leaves the target empty on
  failure; a valid export still round-trips.
- `update_note` rejects `updated_at < created_at`.
- The logical export is atomic; a failed write leaves the previous file intact.
- The desktop records a non-zero duration or a null finish, never two equal
  timestamps for an unknown duration.
- The run evidence panel shows only the selected run's notes, or labels workspace
  notes separately.
- The default run-summary export succeeds with a non-existent output root; a
  failed export leaves the model and export controls visible.
- The desktop startup error is actionable and non-panicking.
- Native commands reject paths outside the workspace/memory roots, or the
  dialog-selected path is validated.
- The memory records the model summary and supports metric/claim/note CRUD from
  the shared service and both surfaces.
- The GUI and shell expose the same pipeline and memory use cases from one
  source; no Tauri command contains duplicated domain orchestration.
- A native IPC E2E (or the recorded equivalent gate) drives the core flow.
- The one-program ADR, `AGENTS.md`, architecture docs, module docs, and plan/
  contract statuses match the shipped behaviour.
- The per-worktree target rule is documented and the gate is reproducible.
- `cargo fmt`, `cargo clippy --all-targets`, `cargo test`, `cargo doc`, the
  desktop checks, and the repository checkers pass.

## Traceability

| Finding | Work item | Verification |
|---|---|---|
| AUD-2026-11-11-02 | P1 foreign-database refusal and restore validation | V1 |
| AUD-2026-11-11-03 | P1 canonical workspace/config path | V2 |
| AUD-2026-11-11-04 | P4 per-worktree target rule and clean gate | V3 |
| AUD-2026-11-11-05 | P1 stable workspace name | V4 |
| AUD-2026-11-11-06 | P1 shared import validator | V5 |
| AUD-2026-11-11-07 | P1 update_note timestamp validation | V6 |
| AUD-2026-11-11-08 | P2 run timing in the desktop | V7 |
| AUD-2026-11-11-09 | P2 run-scoped notes | V8 |
| AUD-2026-11-11-10 | P4 surface decision, ADR, docs, statuses | V9 |
| AUD-2026-11-11-11 | P3c shared pipeline use cases and GUI/shell parity | V10 |
| AUD-2026-11-11-12 | P3a model summary and memory CRUD in both surfaces | V11 |
| AUD-2026-11-11-13 | P2 run-summary export default and error | V12 |
| AUD-2026-11-11-14 | P2 native path scope | V13 |
| AUD-2026-11-11-15 | P1 atomic logical export | V14 |
| AUD-2026-11-11-16 | P3b native IPC E2E | V15 |
| AUD-2026-11-11-17 | P2 desktop startup error | V16 |
| AUD-2026-11-11-18 | P2 file/folder pickers or inline validation | V17 |

## Work breakdown

| Phase | Work | Role | Paths |
|---|---|---|---|
| P1 | Foreign-database refusal, restore validation, canonical root/config, stable workspace name, shared import validation, update_note, atomic export | Implementation / Test | `crates/quantrule-app/src/`, `crates/quantrule-app/tests/` or module tests |
| P2 | Desktop run timing, run-scoped notes, run-export default/error, path scope, startup error, pickers/validation | Implementation / Test | `apps/desktop/src/`, `apps/desktop/src-tauri/src/`, tests |
| P3a | Model summary recording and memory metric/claim/note CRUD in the service, shell, and GUI | Implementation / Quant Developer (no signal change) | `crates/quantrule-app/src/service.rs`, `crates/quantrule-shell/src/command/`, `apps/desktop/src/` |
| P3b | Native IPC E2E or a documented equivalent gate | Test | `apps/desktop/e2e/`, `src-tauri/`, CI |
| P3c | Shared pipeline use cases; shell and GUI thin adapters; fetch decision; parity test | Architect / Implementation / Test | `crates/quantrule-app/`, `crates/quantrule-shell/`, `apps/desktop/` |
| P4 | One-program ADR, `AGENTS.md`, architecture docs, module docs, plan/contract statuses, target rule | Architect / Implementation | `docs/adr/`, `docs/architecture/`, `docs/execution-plans/`, `AGENTS.md` |

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Foreign database | Foreign DB refused; restore leaves no target; valid backup restores | `cargo test -p quantrule-app memory::` and the new tests | pass | 28 memory tests pass; foreign DB and restore refused | Pass | Agent | 2026-11-11 | — |
| V2 | Canonical identity | `/repo` and `/repo/.` one workspace; relative/absolute one config | new service/memory tests | pass | workspace canonical and service tests pass | Pass | Agent | 2026-11-11 | — |
| V3 | Target isolation | A fresh worktree target passes without a clean; the shared target hazard is documented | gate instructions + `cargo test --workspace` | pass | tools/gate.sh passed with a per-worktree target | Pass | Agent | 2026-11-11 | — |
| V4 | Workspace name | Two runs keep the root-derived name | new service test | pass | root-name service test passes | Pass | Agent | 2026-11-11 | — |
| V5 | Import validation | Each malformed record is rejected; valid export round-trips | new memory tests | pass | import invariant tests pass | Pass | Agent | 2026-11-11 | — |
| V6 | Note timestamp | Update before `created_at` is rejected | new memory test | pass | note timestamp test passes | Pass | Agent | 2026-11-11 | — |
| V7 | Run timing | Non-zero duration or null finish | frontend test | pass | session test passes; 10 frontend tests | Pass | Agent | 2026-11-11 | — |
| V8 | Run-scoped notes | Only the selected run's notes under its evidence | frontend/native test | pass | notes_for_run test, command, and browser E2E pass | Pass | Agent | 2026-11-11 | — |
| V9 | Records/docs | No stale surface phrases; statuses match; ADR present | `check_links`, `check_terms`, `check_workflows`, `check_execution_plans` | pass | ADR and records updated; checkers pass | Pass | Agent | 2026-11-11 | — |
| V10 | One-program parity | Use-case inventory; both adapters call one source; parity test | architecture test + `check_structure` | pass | Pending | Pending | Agent | 2026-11-11 | Phase 3c |
| V11 | Memory evidence | Model summary and CRUD readable/exported | app/shell/desktop tests | pass | model summary recorded; metric/claim/note CRUD remains | Pending | Agent | 2026-11-11 | Phase 3a |
| V12 | Run export | Default export succeeds; error leaves model visible | frontend test + native check | pass | export parent test and RunPanel test pass | Pass | Agent | 2026-11-11 | — |
| V13 | Path scope | Out-of-root path rejected; allowed paths work | desktop command tests | pass | Pending | Pending | Agent | 2026-11-11 | — |
| V14 | Atomic export | Failed write leaves the previous export | memory test | pass | atomic export tests pass | Pass | Agent | 2026-11-11 | — |
| V15 | Native E2E | Core native flow passes | native E2E or recorded equivalent | pass | bridge guard passes; full native E2E remains | Pending | Agent | 2026-11-11 | Phase 3b |
| V16 | Startup error | Actionable error, no panic | desktop unit test | pass | desktop builds/tests; main handles the error | Pass | Agent | 2026-11-11 | — |
| V17 | Pickers | Dialog/inline validation test | frontend test | pass | Pending | Pending | Agent | 2026-11-11 | — |

## Risks and recovery

| Risk | Impact | Control | Recovery |
|---|---|---|---|
| Architecture extraction is large | Partial parity, drift | Phase 3c sub-plan, one use case at a time, parity test | Stop the phase; keep the thin adapters; no duplicate logic |
| Native E2E environment missing | The native gate cannot run | Try `tauri-driver`/`WebKitWebDriver`; otherwise record an exception | Keep the launch smoke and command-bridge test; add the E2E when available |
| New dialog dependency | Build/review cost | Inline validation first; request approval before adding | Revert the dependency; keep text inputs with validation |
| Path scoping breaks a legitimate workflow | User friction | Allow workspace, memory, and dialog-selected paths | Add an explicit path grant; never allow the whole filesystem silently |
| Memory summary shape changes | Schema/provenance risk | Use the existing `summary_json` or typed metrics; no v1 rewrite | Add a v2 migration only with human approval |
| Fetch in the GUI | Local-first/network promise | Phase 3c human decision; do not weaken silently | Keep fetch shell-only with a recorded exception |
| Docs drift | Agents follow the wrong surface | P4 update in the same change; checkers | Revert the stale text; update the index |

## Stop conditions

Stop and ask the human if:

- the one-program extraction requires a new public crate, a new dependency, or a
  module-boundary change beyond this plan;
- the memory summary needs a schema migration or the `summary_json` contract
  changes;
- the GUI `fetch` decision would add a network path to the desktop;
- a native E2E cannot be built in the available environment and the human must
  accept a recorded exception;
- any finding cannot be fixed without changing model, strategy, execution,
  portfolio, or analytics behaviour;
- a test would need to be weakened, skipped, or removed to pass.

## Verification

The full gate for this plan is `cargo fmt`, `cargo clippy --workspace
--all-targets`, `cargo test --workspace`, `cargo doc`, the desktop checks
(`npm run check`, `npm test`, `npm run test:e2e`, `cargo test/clippy` for
`src-tauri`), the repository checkers, and the native E2E or its recorded
equivalent. The plan is not `Verified` while any V row is `Pending`; a completed
row records the actual command and observed result.
