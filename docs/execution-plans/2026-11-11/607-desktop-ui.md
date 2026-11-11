# P3.4 — Tauri and Svelte Desktop UI

- Date: 2026-11-11 (UTC)
- Issue: [#607](https://github.com/quant-rule/quant-rule/issues/607)
- Parent plan: [`588-tauri-platform.md`](588-tauri-platform.md)
- Status: In Progress
- Claim type: Problem Claim
- Owner: Human
- Workflow: Execution Plan

## Intent

Build the local desktop surface over the application service. A solo researcher
opens a workspace, inspects datasets and run configurations, validates and runs
the model, records the run in the local research memory, and exports or backs up
the result. The app has no account, no network path, and no cloud runtime.

## Current state

`crates/quantrule-app` exposes workspace, dataset, configuration, model-run,
export, and memory operations. `crates/quantrule-shell` is the current command
surface. No desktop app exists. Node 22, npm 10, GTK 3, WebKitGTK 4.1, libsoup 3,
librsvg, appindicator, `patchelf`, and Xvfb are installed.

## Scope

Included:

- `apps/desktop` with a Tauri 2 Rust backend and a Svelte 5 + TypeScript + Vite
  frontend.
- Workspace open, dataset list and summary, run-config list, run validation,
  model run, and run-summary export.
- Research-memory initialize, record run, list runs, view metrics/claims/notes,
  export/import, and backup/restore.
- Settings for the workspace root and memory path, stored locally.
- Frontend unit tests, Playwright browser E2E, Tauri build, and an Xvfb launch
  smoke test.
- README, changelog, and AI transparency log.

Excluded:

- Backtest, strategy, execution, portfolio, qualification, and analytics
  orchestration in the UI. Those remain CLI operations until the service owns
  them.
- Accounts, sync, cloud, Windows, and macOS.
- Linux installer packaging and the download page (Phase 3.5).
- A new database schema.

## Assumptions and uncertainty

- The Tauri crate and npm packages are new dependencies; the approved plan names
  Tauri 2 and Svelte 5, and the human approved the native toolchain install.
- Tauri builds under Xvfb for a smoke launch, but interactive window testing is
  limited to the local machine.
- Playwright drives the Vite build with a mocked Tauri `invoke`; the Tauri smoke
  test proves the native shell launches. A WebDriver E2E is deferred to Phase 3.6.
- The app calls only application-service operations. It does not spawn the CLI.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| App location | `apps/desktop` | Matches the architecture description and keeps the Rust workspace focused. |
| Rust workspace | Standalone Tauri crate, excluded from the root workspace | Keeps the engine gate independent of native GUI libraries; the desktop job installs them. |
| Frontend | Svelte 5 runes + TypeScript + Vite | Approved stack; small, local, testable. |
| IPC | Typed Tauri commands returning serializable views | One backend boundary; the UI holds no path or config logic. |
| File selection | Text inputs for workspace, config, and memory paths | No extra dialog plugin; deterministic and testable. |
| Browser E2E | Playwright with a mocked `invoke` | The frontend flows are testable without a display; native launch is separate. |
| Time | The frontend supplies Unix seconds for recording | The application service never reads the clock. |

## Acceptance criteria

- `npm test` passes the frontend unit tests.
- `npm run test:e2e` passes the Playwright browser flow.
- `cargo test` passes for the Tauri crate.
- `npm run tauri build -- --no-bundle` succeeds.
- `xvfb-run` launches the built binary and it reaches the main window without an
  error.
- The UI workspace, dataset, run-check, model-run, memory, export, backup, and
  restore flows use only `quantrule-app`.
- The run-check view shows the same name, instrument, data version, and ladders
  as the CLI `run check` for the same config.
- No account, network request, or cloud endpoint exists in the app.

## Work breakdown

| Step | Work | Role | Paths |
|---:|---|---|---|
| 1 | Scaffold the Tauri crate and npm package | Implementation | `apps/desktop/` |
| 2 | Backend commands over the application service | Implementation | `src-tauri/src/` |
| 3 | Svelte shell, navigation, and typed API | Implementation | `src/` |
| 4 | Workspace, data, config, run, memory, and settings panels | Implementation | `src/lib/` |
| 5 | Unit, E2E, and native smoke tests | Test | `src/**/*.test.ts`, `e2e/` |
| 6 | CI desktop job, docs, changelog, AI log | Implementation | `.github/workflows/`, `docs/` |

## Verification table

Commands are run from `apps/desktop` unless noted.

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Scaffold | Tauri and frontend dependencies install | `npm ci` | pass | Pending | Pending | — | — | |
| V2 | Frontend | Unit tests pass | `npm test` | pass | Pending | Pending | — | — | |
| V3 | Build | Frontend production build | `npm run build` | pass | Pending | Pending | — | — | |
| V4 | Backend | Tauri crate tests pass | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | Pending | Pending | — | — | |
| V5 | Native build | Desktop binary builds | `npm run tauri build -- --no-bundle` | pass | Pending | Pending | — | — | |
| V6 | Native launch | Window opens under Xvfb | `xvfb-run -a timeout 20 ./src-tauri/target/release/quantrule-desktop` | window starts | Pending | Pending | — | — | |
| V7 | Browser E2E | Core flow passes with mocked IPC | `npm run test:e2e` | pass | Pending | Pending | — | — | |
| V8 | CLI parity | Run-check view matches CLI | compare UI and `run check` fields | match | Pending | Pending | — | — | |
| V9 | Offline | No account or network path | source scan | none | Pending | Pending | — | — | |
| V10 | Gate | Full Rust and repository gate | root `cargo test`; checkers | pass | Pending | Pending | — | — | |

## Risks and recovery

| Risk | Control | Recovery |
|---|---|---|
| Tauri build fails on Fedora | Early `cargo check` and `tauri build` | Stop; the approved fallback is a local web app with a new ADR. |
| Playwright cannot drive the native window | Browser E2E plus Xvfb smoke | Add WebDriver in Phase 3.6. |
| Native libraries missing in CI | Desktop job installs them explicitly | Keep the engine job separate. |
| UI duplicates domain logic | Backend returns service views only | Move logic into `quantrule-app`. |
| Large generated assets | Do not commit `node_modules`, `dist`, or `target` | `.gitignore` and diff review. |

## Stop conditions

Stop and ask if:

- a Tauri or frontend dependency outside the approved stack is required;
- the app needs a network endpoint or account;
- the UI would need to spawn the CLI or reimplement a service operation;
- the native build cannot be made to run on this machine;
- backtest orchestration must move into the service before the UI can be useful.
