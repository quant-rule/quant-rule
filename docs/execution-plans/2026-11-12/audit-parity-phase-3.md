# Audit Parity — Phase 3

- Date: 2026-11-12 (UTC)
- Plan: `audit-parity-phase-3`
- Issue: — (audit remediation; no issue number assigned)
- Parent plan: [`audit-app-service-native-remediation.md`](../2026-11-11/audit-app-service-native-remediation.md)
- Audits: [`app-service-memory-audit.md`](../../audits/2026-11-11/app-service-memory-audit.md), [`native-tauri-window-audit.md`](../../audits/2026-11-11/native-tauri-window-audit.md)
- ADR: [`one-program-first-class-desktop.md`](../../adr/2026-11-11/one-program-first-class-desktop.md)
- Status: In Progress
- Claim type: Behaviour Claim
- Owner: Human
- Workflow: Execution Plan; Audit Workflow remediation

## Intent

Close the remaining Phase 3 findings from the two audits and implement the
one-program decision: the desktop UI and the shell are two first-class surfaces
of the same program, share one use-case layer, and have feature parity. The work
is sliced so each slice is reviewable, testable, and independently reversible.

## Current state

- `main` at `2c4c9fb` (PR #626 merged) contains Phase 3 P3.1.
- The app service owns workspace, dataset, configuration, model run, export,
  memory, and the model-summary record.
- The shell owns the pipeline orchestration for data inspection, events, report,
  strategy, execution, portfolio, backtest, qualification, and analytics.
- The desktop UI owns workspace, data, config, model, memory read/write/export,
  and settings; the P3.2 slice adds the backend path scope and native pickers.
- The native WebDriver E2E is absent; the bridge source guard and the native
  launch/drive evidence exist.
- Open findings: AUD-11, AUD-12 (model-summary portion), AUD-14, AUD-16,
  AUD-18; the `fetch` network-boundary decision is open.

## Scope

Included:

- P3.1 Memory write path: shell `memory` commands and desktop metric/claim/note
  CRUD over the existing service; model evidence is already recorded.
- P3.2 Native boundary: backend-owned path scope, dialog-backed path selection,
  and inline path validation.
- P3.3 Native E2E: `tauri-driver` + `WebKitWebDriver` core-flow test.
- P3.4 Shared pipeline use cases: extract one stage at a time behind the app
  service; the shell and desktop become thin adapters; a parity test.
- P3.5 GUI pipeline parity and the explicit `fetch` decision.

Excluded:

- No model, strategy, execution, portfolio, or analytics behavior change; no
  new signal, baseline, or edge claim.
- No cloud, account, sync, Windows, or macOS.
- No new market data.
- The `fetch` network boundary is a separate human decision; it is not added to
  the desktop silently.

## Assumptions and uncertainty

- Each slice lands as its own branch/PR; the parent remediation plan stays
  `In Progress` until every slice is merged.
- The shell `memory` commands are a stable agent interface; adding them changes
  the command surface count and requires README, help, and checker updates.
- `tauri-plugin-dialog` was explicitly approved and added in P3.2; inline
  validation remains the fallback for typed paths.
- The native E2E may be blocked by the environment; a recorded exception is
  allowed only with human approval.
- Moving the shell orchestration into the app service is the largest slice; the
  `request` argument structs must be split into explicit use-case parameters,
  and the shell keeps parsing/rendering/fetch.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Slice order | P3.1 memory, P3.2 boundary, P3.3 E2E, P3.4 pipeline, P3.5 GUI/fetch | Memory closes a Major finding quickly; the boundary is security; the extraction is last and largest |
| Memory write path | Shell `memory` command + GUI forms; the service owns the rules | Both surfaces, one source |
| Path scope | Backend-owned workspace/memory roots; dialog for new paths | The webview must not choose arbitrary file access |
| Dialog dependency | `tauri-plugin-dialog`, added in P3.2 after explicit approval | Dependency gate; native selection without a webview path guess |
| Native E2E | `tauri-driver` + `WebKitWebDriver` | Real IPC, not the fixture backend |
| Pipeline extraction | Move orchestration into the app service, one stage per slice | ADR direction; no shell-library inversion |
| Fetch | Human decision; default shell-only with a recorded exception | Network boundary |

## Acceptance criteria

- The shell and the desktop can add, update, delete, and list metrics, claims,
  and notes; the memory rules and validation are shared.
- The native commands reject paths outside the workspace/memory roots; a
  dialog-selected path is accepted; invalid paths are rejected inline.
- The native WebDriver E2E covers the core flow (workspace, data, config, model,
  memory record/evidence, export).
- Each pipeline stage has one use-case implementation in the shared layer; the
  shell and desktop call it; a parity test runs one scenario from both.
- The GUI exposes the pipeline stages or a recorded, human-approved exception.
- The `fetch` decision is recorded; no silent network path.
- The full gate, desktop checks, and native E2E pass; the plan is `Verified`
  only when every row is `Pass` or an accepted exception.

## Work breakdown

| Slice | Work | Role | Paths |
|---|---|---|---|
| P3.1 | Shell `memory` parser/runner and desktop metric/claim/note forms; command docs and tests | Implementation / Test | `crates/quantrule-shell/src/command/`, `apps/desktop/src/lib/MemoryPanel.svelte`, `apps/desktop/src-tauri/src/commands.rs` |
| P3.2 | Backend path scope, dialog pickers, inline validation | Implementation / Security / Test | `apps/desktop/src-tauri/`, `apps/desktop/src/lib/`, `capabilities/` |
| P3.3 | Native WebDriver E2E and CI job | Test | `apps/desktop/e2e/`, `.github/workflows/` |
| P3.4 | Shared pipeline use cases, one stage per PR; thin adapters; parity test | Architect / Implementation / Test | `crates/quantrule-app/`, `crates/quantrule-shell/`, `apps/desktop/` |
| P3.5 | GUI pipeline parity and `fetch` decision | Requirements / Implementation / Human | `apps/desktop/`, docs |

## Traceability

| Finding | Slice | Verification |
|---|---|---|
| AUD-2026-11-11-12 (CRUD) | P3.1 | V1 |
| AUD-2026-11-11-14 (path scope) | P3.2 | V2 |
| AUD-2026-11-11-18 (pickers) | P3.2 | V3 |
| AUD-2026-11-11-16 (native E2E) | P3.3 | V4 |
| AUD-2026-11-11-11 (one-program parity) | P3.4/P3.5 | V5, V6 |
| `fetch` boundary | P3.5 | V6 |

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | P3.1 memory write path | Shell and GUI CRUD round-trip; shared validation | `cargo test -p quantrule-app`; `cargo test -p quantrule-shell --test cli`; frontend tests | pass | shell CRUD round-trip, desktop CRUD tests, and the full gate pass | Pass | Agent | 2026-11-12 | 18 shell CLI tests + CRUD test; 10 frontend; 1 E2E |
| V2 | P3.2 path scope | Out-of-root rejected; allowed paths work | desktop command tests | pass | out-of-root dataset and memory-export paths rejected; workspace commands pass | Pass | Agent | 2026-11-12 | 5 desktop tests + 2 bridge |
| V3 | P3.2 pickers/validation | Dialog/inline path test | frontend test | pass | Settings, Memory, and Run use the native pickers and the fields update; typed paths remain scope-validated | Pass | Agent | 2026-11-12 | dialog plugin approved; 13 frontend tests |
| V4 | P3.3 native E2E | Core native flow passes | native WebDriver E2E | pass | Pending | Pending | Agent | — | — |
| V5 | P3.4 pipeline extraction | One use-case source; parity test | architecture/parity tests | pass | Pending | Pending | Agent | — | — |
| V6 | P3.5 GUI parity/fetch | Parity or accepted exception; fetch decision recorded | parity test + record | pass | Pending | Pending | Agent | — | — |

## Risks and recovery

| Risk | Impact | Control | Recovery |
|---|---|---|---|
| Shell `memory` command grows the command surface | Docs/checker drift | Update help, README, command surface, tests in the same slice | Revert the slice |
| Dialog dependency denied | Pickers deferred | Inline validation fallback | Keep text inputs with validation; record exception |
| Native E2E environment missing | Gate gap | Try `tauri-driver`/`WebKitWebDriver` | Record a human-approved exception; keep bridge guard |
| Pipeline extraction changes behavior | Regression risk | One stage per PR; golden/parity tests; no signal-path change | Revert the stage; keep the shell path |
| Fetch in GUI weakens local-first | Security/product risk | Explicit human decision; no silent addition | Keep shell-only with a recorded exception |

## Stop conditions

Stop and ask the human if:

- a slice needs a new dependency, a new public crate, or a module-boundary change
  beyond this plan;
- the memory CRUD would change the v1 schema or the export contract;
- the path scope would break a legitimate workflow;
- the native E2E cannot be built in the environment;
- the `fetch` decision would add a network path to the desktop;
- any change would alter model, strategy, execution, portfolio, or analytics
  behavior;
- a test would need to be weakened, skipped, or removed.
