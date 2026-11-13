# Native Tauri Window Audit — v0.9.0

- Date: 2026-11-11 (UTC)
- Audit ID: `AUDIT-2026-11-11-native-tauri-window`
- Status: Published; findings in `Open`
- Baseline: `main` at `8c4f2ca`; the audited binary is the v0.9.0 release
  binary at `apps/desktop/src-tauri/target/release/quantrule-desktop`. The
  source paths under `apps/` and `crates/` are identical between `v0.9.0` and
  the baseline (`git diff v0.9.0..8c4f2ca -- apps crates` is empty).
- Scope: the native Tauri 2 window, its Rust command boundary, the Svelte
  frontend, the real IPC path, the memory panel, the run-summary export, the
  capability/CSP configuration, and the GUI/shell parity under the human
  decision of 2026-11-11.
- Workflow: [`audit-workflow.md`](../../workflows/audit-workflow.md)
- Claim type: program audit; no model, strategy, edge, or profitability claim.
- Related audit: [`app-service-memory-audit.md`](app-service-memory-audit.md)
- Human decision (2026-11-11): the desktop UI is a first-class surface from now
  on, equal to the shell. The GUI and the shell are the same program with
  feature parity and one shared source; neither duplicates the other. The human
  can run both; the shell is primarily for agents.

## Summary

| Severity | Count |
|---|---:|
| Critical | 0 |
| Major | 3 |
| Minor | 5 |
| Info | 0 |

The native window is real and works: it launched from the release binary under
Xvfb, rendered the Svelte UI, opened a workspace, summarised a dataset,
validated a run, computed the model over four timeframes, opened the research
memory, recorded a run, and wrote a logical export. The audit did not find a
crash, a missing command, an argument-name mismatch, or a network path.

The audit did find three Major gaps against the human decision and five Minor
defects:

1. the GUI and shell are not one program: the backtest, strategy, execution,
   portfolio, qualification, analytics, candle, event, report, and fetch use
   cases live in the shell, while the GUI can only reach workspace, dataset,
   config, model, export, and memory; the shared service is partial;
2. the memory does not capture the evidence the run panel computes: the model
   counts are discarded, and the typed metric/claim/note CRUD is not reachable
   from the GUI or the shell;
3. the native run-summary export uses a default path inside a directory the app
   does not create, so the first export fails and the error hides the model
   result and the export controls.

The Minor findings are an unscoped native path boundary, a non-atomic logical
export, the absent native IPC E2E, a panic on window-startup failure, and the
absence of native file/folder pickers.

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| AUD-2026-11-11-11 | Major | architecture/parity | The GUI and shell are not one program: the pipeline stages are shell-only, the memory is GUI-only, and the shared application service covers only workspace, dataset, config, model, export, and memory | `apps/desktop/README.md:18-19`; `apps/desktop/src/lib/SettingsPanel.svelte:42`; `apps/desktop/src-tauri/src/lib.rs:9-29`; `crates/quantrule-app/src/service.rs:32-243`; `crates/quantrule-shell/src/{backtest,strategy,execution,portfolio,qualification,analytics}`; `docs/architecture/command-surface.md:50-56`; [`607-desktop-ui.md`](../../execution-plans/2026-11-11/607-desktop-ui.md) excludes the stages "until the service owns them" | The released desktop is not a first-class equivalent of the shell; adding the missing GUI stages would duplicate the shell orchestration or move it without a shared contract; the two surfaces can drift | A. Move the shell's orchestration into `quantrule-app` use cases and make both adapters thin; B. have the GUI spawn the CLI; C. add the use cases directly to the Tauri commands | A | A use-case inventory maps every shell command to an `AppService` method; both adapters call only the service; a parity test runs one scenario from both surfaces and compares the output; no Tauri command contains domain orchestration | Open | [`command-surface.md`](../../architecture/command-surface.md); [`607-desktop-ui.md`](../../execution-plans/2026-11-11/607-desktop-ui.md); human decision |
| AUD-2026-11-11-12 | Major | memory/evidence | The memory records the config summary only; the model counts are discarded, and metric/claim/note CRUD is not reachable from the GUI or the shell | `apps/desktop/src/lib/RunPanel.svelte:9-26`; `crates/quantrule-app/src/service.rs:159-222,338-364`; `crates/quantrule-app/src/memory/store.rs:275-483`; `apps/desktop/src-tauri/src/commands.rs:349-370`; native DB/export: 0 metrics, 0 claims, 0 notes; `docs/architecture/command-surface.md` has no memory command | The memory is a run registry, not the research evidence store; the model result shown in the UI is lost on navigation and never recorded; the user cannot add metrics, claims, or notes from either surface; the service CRUD is unreachable outside tests | A. Include the model summary in the recorded run and expose metric/claim/note CRUD in the service, shell, and GUI; B. keep the config-only record and add manual CRUD; C. document that the memory stores configs only | A | A native/browser test runs the model, records it, and reads back the model counts and a metric/claim/note; the shell produces the same record; the export contains the evidence | Open | [`research-memory-v1.md`](../../config/2026-11-11/research-memory-v1.md); [`607-desktop-ui.md`](../../execution-plans/2026-11-11/607-desktop-ui.md) |
| AUD-2026-11-11-13 | Major | desktop/run | The default "Export summary" path is inside a directory the app does not create; the first export fails and the error hides the model result and the export controls | `apps/desktop/src/lib/RunPanel.svelte:20,28-45,60-105`; `crates/quantrule-app/src/export.rs:58-67`; `crates/quantrule-app/src/config/run.rs:117`; native screenshot p14: `io error: .../config/../target/native-audit/run-summary.json: No existe el fichero o el directorio`; the model table and export input disappear under the error branch | The promised export fails on first use for a fresh output root; the model result is replaced by an error; retrying requires a full model re-run; the default path is non-canonical | A. Create the output root or the export parent before writing; B. default to an existing directory or a native save dialog; C. keep the model visible on error and allow retry | A + B + C | A test with a non-existent output root exports successfully; a forced write error leaves the model and export controls visible; a retry succeeds; the default path is canonical | Open | — |
| AUD-2026-11-11-14 | Minor | security/native | The native command boundary accepts arbitrary filesystem paths; the capability is `core:default` and does not scope the custom commands | `apps/desktop/src-tauri/src/commands.rs:238-425` (`open_workspace(root)`, `dataset_summary(path)`, `memory_import(db, from)`, `memory_export(db, out)`, `memory_backup(db, out)`, `memory_restore(backup, target)`, `export_run_summary(config, out)`); `apps/desktop/src-tauri/capabilities/default.json` `"permissions": ["core:default"]`; no path validation in the command layer | A compromised webview script, for example through a frontend dependency, can read, overwrite, or create files anywhere the user can; the CSP blocks network but not local file access | A. Validate every path against the workspace root and an explicit memory directory; B. use the Tauri fs plugin with scoped permissions and native dialogs; C. document the local-only trust boundary and accept the risk | A + B | A command with a path outside the allowed roots is rejected; allowed paths still work; the capability scope is documented | Open | — |
| AUD-2026-11-11-15 | Minor | store/export | The logical memory export replaces an existing file in place and is not atomic | `crates/quantrule-app/src/memory/transfer.rs:149-155` (`fs::write`); the docs say "replacing an existing file"; the native export wrote the JSON document to the selected path | A failed or partial write truncates a previous good export; the import then rejects the file; a backup workflow can lose its only logical copy | A. Write a temp file in the same directory and rename on success; B. require a new target path like backup/restore; C. document the risk | A | A simulated write failure leaves the previous file intact; a successful export is complete and importable | Open | [`app-service-memory-audit.md`](app-service-memory-audit.md) |
| AUD-2026-11-11-16 | Minor | tests/native | The native IPC path has no automated E2E; the Playwright E2E uses the browser fixture backend, and the native check is a launch smoke | `apps/desktop/e2e/app.spec.ts:3`; `apps/desktop/src/lib/api.ts:11-28` (`mockMode`/`mockInvoke`); `apps/desktop/README.md:32-40` (Xvfb launch smoke); [`607-desktop-ui.md`](../../execution-plans/2026-11-11/607-desktop-ui.md) defers WebDriver; [`release-readiness.md`](../../reports/2026-11-11/release-readiness.md) records the gap | A broken command registration, argument name, or native-only UI path can ship; the fixture E2E cannot prove the real IPC surface | A. Add a `tauri-driver` + `WebKitWebDriver` E2E for the core flow; B. add a native integration test for the command bridge; C. keep the smoke and document | A | The native E2E launches the built app, opens a fixture workspace, runs the model, records a run, and asserts the UI; it fails on an unregistered command or renamed argument | Open | — |
| AUD-2026-11-11-17 | Minor | desktop/startup | The desktop library panics with `.expect` when the window fails to start; the message does not say what to do | `apps/desktop/src-tauri/src/lib.rs:31`; `apps/desktop/src-tauri/src/main.rs:6-7`; the `run()` function is library code called by the binary | A missing display, WebKitGTK, or GTK failure crashes without an actionable message; the library-code rule is violated in the adapter | A. Return `Result` and handle it in `main` with an actionable message and a non-zero exit; B. move the panic to `main` and name the missing dependency; C. keep | A | A simulated startup error returns an error and exits non-zero; no panic in `quantrule_desktop_lib` | Open | `AGENTS.md` §3 |
| AUD-2026-11-11-18 | Minor | desktop/UX | The native UI has no file or folder pickers; every workspace, memory, export, import, backup, and restore path is typed | `apps/desktop/src/lib/SettingsPanel.svelte:19-33`; `apps/desktop/src/lib/MemoryPanel.svelte:306-322`; `apps/desktop/src/lib/RunPanel.svelte:94-99`; no dialog plugin in `apps/desktop/src-tauri/Cargo.toml` or `capabilities/default.json`; [`607-desktop-ui.md`](../../execution-plans/2026-11-11/607-desktop-ui.md) chose text inputs "No extra dialog plugin; deterministic and testable" | Path entry is error-prone; the default export failure is one symptom; users must know filesystem paths; the first-class desktop surface still behaves like a shell with buttons | A. Add `tauri-plugin-dialog` for folder/file pickers and validate the selection; B. add inline validation and recent paths; C. keep the text inputs and document | A + B | A UI test with a mocked dialog selects a workspace, memory, and export path; the command receives it; invalid paths are rejected inline | Open | — |

## Detail

### AUD-2026-11-11-11 — the GUI and shell are not one program

- **Expected invariant:** under the human decision, the GUI and shell are the
  same program with feature parity and one shared source. A capability is
  implemented once in the shared application service and exposed by both
  adapters; the shell is primarily for agents, not a second implementation.
- **Observed:** the shell command surface covers data inspection (`candles`,
  `candles-tolerant`, `timeframes`, `fetch`), the event batch (`events`), the
  model report (`report model`), strategy (`strategy explain`, `strategy run`),
  execution (`execution run`), portfolio (`portfolio replay`), the coupled run
  (`backtest run`), qualification (`qualify`), analytics (`analytics
  evaluate`), and `run check`
  (`docs/architecture/command-surface.md:50-56`). The desktop has six sections
  and 16 Tauri commands: workspace, dataset summary, check run, model run,
  export summary, memory status/workspaces/runs/metrics/claims/notes, record
  run, export, import, backup, and restore (`lib.rs:9-29`). It has no section or
  command for the pipeline stages. `apps/desktop/README.md:18-19` says the
  desktop "does not run backtests, strategies, execution, portfolio, or
  analytics. Those remain CLI operations until the application service owns
  them." The Settings page says the same (`SettingsPanel.svelte:42`).
- **Root cause:** the `quantrule-app` service exposes only workspace, dataset,
  configuration, model-run, export, and memory (`service.rs:32-243`). The shell
  owns the orchestration for the other stages in
  `crates/quantrule-shell/src/{backtest,strategy,execution,portfolio,qualification,analytics}`;
  it calls the app config and pipeline modules directly. The 607 plan excluded
  the GUI stage orchestration "until the service owns them"
  (`607-desktop-ui.md`). No extraction happened before the release.
- **Impact:** the released desktop is not a first-class equivalent of the
  shell. A user cannot run a backtest, strategy, execution, portfolio,
  qualification, analytics evaluation, fetch, candle inspection, event batch,
  or model report from the GUI. Adding those to the GUI by writing Tauri
  commands would duplicate the shell logic; spawning the CLI would violate the
  desktop boundary ("does not spawn the CLI") and the one-program requirement.
  The two surfaces can drift, and an agent using the shell and a human using
  the GUI can see different capabilities.
- **Affected contracts/docs:** `apps/desktop/README.md`;
  `docs/architecture/README.md` (application-service paragraph);
  `docs/architecture/command-surface.md`; the 607 desktop plan; the v0.9.0
  release scope; `AGENTS.md` §2.
- **Dependencies:** the human decision; a use-case inventory; a new
  architecture/requirements plan; possibly an ADR. The memory CRUD finding
  (AUD-12) uses the same extraction.
- **Effort:** L (multi-stage extraction, both adapters, tests, docs).
- **Options:**
  - A. Move the shell's orchestration into `quantrule-app` use cases (or a
    shared use-case crate the app service depends on); the shell and the Tauri
    commands become thin adapters; add the missing GUI sections and the shell
    commands for the same use cases. The app service owns file access, the
    engines stay pure, and both adapters render the same values.
  - B. Keep the shell orchestration and have the GUI spawn the CLI. This
    contradicts the desktop boundary, the no-duplication requirement, and the
    one-program decision; it also makes the GUI depend on the shell binary.
  - C. Add the use cases directly to the Tauri commands. This duplicates the
    shell logic and guarantees drift; rejected by the human decision.
- **Recommendation:** A. It is the only option that satisfies "same source,
  no duplication, feature parity" and preserves the one-way pipeline. The
  extraction is a substantial architecture change and needs a
  Requirements & Design or Refactor plan with a human approval gate.
- **Verification:** a use-case inventory maps every shell command to an
  `AppService` method and a GUI action. Both adapters call only the service; a
  parity test runs the same fixture scenario through the shell and the native
  GUI and compares the output. No Tauri command parses a config or computes a
  domain value. The architecture checker records the new edges.
- **Links:** [`command-surface.md`](../../architecture/command-surface.md);
  [`607-desktop-ui.md`](../../execution-plans/2026-11-11/607-desktop-ui.md);
  human decision in the related audit.

### AUD-2026-11-11-12 — the memory does not capture the evidence the UI computes

- **Expected invariant:** the research memory stores the run and the evidence
  the researcher needs to compare and accept it: the model result counts, the
  metrics, the claims, and the notes. Both surfaces can reach the write path;
  the memory is not a read-only run registry.
- **Observed:** the native run panel computed EURUSD, four timeframes, and 585
  events, with per-timeframe bars/events/levels (screenshot p13). The model
  summary is a returned value (`service.rs:338-364`) held in the `RunPanel`
  local state (`RunPanel.svelte:9-26`); navigating away destroys it. The
  `record_run` command stores the config `RunSummary` only
  (`service.rs:159-222`); the native DB record's `summary_json` contains the
  config composition and no model counts. The memory store has typed CRUD for
  metrics, claims, and notes (`store.rs:275-483`), but the desktop commands only
  list them (`commands.rs:349-370`); there is no create, update, or delete
  command. The shell has no memory command in the command surface
  (`command-surface.md:50-56`). The native DB and the logical export both
  contained zero metrics, zero claims, and zero notes.
- **Root cause:** the memory phase implemented the CRUD behind `quantrule-app`
  and the desktop phase exposed only the run record and the read views; the
  model summary was never added to the recorded evidence. The 607 plan says
  "view metrics/claims/notes", not write them (`607-desktop-ui.md`).
- **Impact:** the memory cannot answer the research question it exists for:
  what did this run measure, and what did the researcher claim about it? The
  model evidence is lost on navigation; the user cannot add a metric, a claim,
  or a note from either surface; the service CRUD is unreachable outside tests.
  Under the human decision, this is a feature-parity gap, not a future
  extension.
- **Affected contracts/docs:** `research-memory-v1.md`; `service.rs`
  `record_run`; the desktop Memory panel and Tauri commands; the command
  surface; the 607 plan.
- **Dependencies:** the shared-use-case extraction in AUD-11; a decision on
  whether the model summary is stored as metrics, a typed summary, or both.
- **Effort:** M–L.
- **Options:**
  - A. Include the model summary in the recorded run (a `record_model_run`
    method or an extended `record_run`) and expose metric/claim/note CRUD in
    `AppService`, the shell, and the GUI. The model counts can become typed
    metrics; the export carries them.
  - B. Keep the config-only run record and add manual metric/claim/note CRUD;
    document that the model result is not stored. This is smaller but leaves
    the evidence gap.
  - C. Add the write surface only in the GUI. This fails parity; rejected.
- **Recommendation:** A. The memory contract already defines the record types;
  the missing piece is the write path and the model-summary handoff.
- **Verification:** a native/browser test runs the model, records it, reads back
  the run and its model counts, adds a metric, a claim, and a note, and exports
  them; the shell produces the same record for the same fixture; the export
  contains the evidence. The memory tests keep the null/integer rules.
- **Links:** [`research-memory-v1.md`](../../config/2026-11-11/research-memory-v1.md);
  [`607-desktop-ui.md`](../../execution-plans/2026-11-11/607-desktop-ui.md).

### AUD-2026-11-11-13 — the default run-summary export fails and hides the model

- **Expected invariant:** the primary export action succeeds on a fresh run, or
  it offers a valid path; a failed export is a recoverable error that leaves the
  model result visible.
- **Observed:** after a successful model run, `RunPanel.svelte:20` sets
  `exportPath = ${model.run.output_root}/run-summary.json`. The output root is
  `config.output_root` (`service.rs:296`), built with `base.join`
  (`config/run.rs:117`); in the native run it was
  `/tmp/opencode/qr-native/workspace/config/../target/native-audit`. That
  directory did not exist. `export_run_summary` calls
  `write_run_summary_file`, which uses `fs::File::create` and does not create
  the parent (`export.rs:58-67`). The native click produced
  `io error: .../config/../target/native-audit/run-summary.json: No existe el
  fichero o el directorio (os error 2)`. The template's `{#if error}` branch
  (`RunPanel.svelte:60-105`) then hid the model table, the export input, and
  the Export summary button; the user could not retry without re-running the
  model.
- **Impact:** the first export fails for any run whose output root does not
  already exist. The model result disappears and the only recovery is another
  full model run. The default path contains `..`, so the user sees an
  unresolved path in the error.
- **Affected contracts/docs:** `RunPanel` docs; `export_run_summary` docs;
  `RunSummary.output_root` ("The resolved output root"); the engine-artifact
  path convention.
- **Dependencies:** whether the output root is owned by a stage command or by
  the UI; a save-dialog decision (AUD-18).
- **Effort:** S.
- **Options:**
  - A. Create the output root or the export parent before writing, and write
    through a temp file plus rename.
  - B. Default the export path to an existing directory (for example the
    workspace root) or use a native save dialog.
  - C. Keep the model result visible on an export error and show the error in a
    separate status area, so the user can correct the path and retry.
- **Recommendation:** A + B + C. The default should be valid, the write should
  be safe, and the error should be non-destructive.
- **Verification:** a test runs the model with a non-existent output root and
  exports successfully; a forced write error leaves the model table and export
  controls visible; a corrected path exports without a re-run; the default path
  is canonical.
- **Links:** —.

### AUD-2026-11-11-14 — the native path boundary is unscoped

- **Expected invariant:** a first-class desktop app with a webview limits
  filesystem access to the workspace, the memory directory, and paths the user
  explicitly chooses; the Tauri capability model expresses that scope.
- **Observed:** the Tauri commands accept raw `String` paths and pass them to
  the service or `std::fs`: `open_workspace(root)`, `dataset_summary(path)`,
  `check_run(config)`, `run_model(config)`, `export_run_summary(config, out)`,
  `memory_status(db)`, `memory_import(db, from)`, `memory_export(db, out)`,
  `memory_backup(db, out)`, `memory_restore(backup, target)`
  (`commands.rs:238-425`). The command layer has no containment check. The
  capability file grants only `core:default`
  (`capabilities/default.json`), which does not scope the custom commands. The
  CSP is `default-src 'self'; connect-src ipc: http://ipc.localhost;
  style-src 'self' 'unsafe-inline'` (`tauri.conf.json`), so remote content is
  blocked, but local file access is not.
- **Impact:** a compromised webview script (for example through a malicious
  frontend dependency) can read, overwrite, or create files anywhere the user
  can. The desktop is now first-class, so the trust boundary should be
  explicit. This is a local supply-chain risk, not a network exposure.
- **Affected contracts/docs:** the Tauri command contract; the capability file;
  the architecture boundary; `AGENTS.md` security review rule.
- **Dependencies:** the workspace/memory path model; whether the UI uses native
  dialogs (AUD-18).
- **Effort:** S–M.
- **Options:**
  - A. Validate every path against the opened workspace root and the configured
    memory directory; reject a path outside them, except a path the user chose
    through a native dialog.
  - B. Move filesystem access to the Tauri fs plugin with scoped permissions
    and native dialogs; the commands receive opaque handles or validated paths.
  - C. Document the local-only trust boundary and accept the risk.
- **Recommendation:** A + B. Keep the command boundary explicit and use the
  platform's scope mechanism rather than relying on the CSP alone.
- **Verification:** a command with a path outside the allowed roots is rejected
  with a deterministic error; allowed workspace/memory paths still work; the
  capability file and the command docs state the scope.
- **Links:** —.

### AUD-2026-11-11-15 — the logical export is not atomic

- **Expected invariant:** an export that is used as a backup either writes to a
  new path or replaces the old file only after the new document is complete.
- **Observed:** `export_to_file` (`transfer.rs:149-155`) calls `fs::write` on
  the target. The function doc says it replaces an existing file. If the write
  fails partway (disk full, I/O error, process kill), the previous export is
  truncated or replaced with invalid JSON; the import then rejects it.
- **Impact:** a backup workflow can lose its only good logical export. The
  SQLite `backup` path is safer (`VACUUM INTO`, target must not exist), but the
  logical export is the human-readable copy.
- **Affected contracts/docs:** `research-memory-v1.md` §Export and import;
  `export_to_file` docs.
- **Dependencies:** the UI export flow (AUD-13) and the native path boundary
  (AUD-14).
- **Effort:** XS.
- **Options:**
  - A. Write to a temporary file in the same directory, flush/close it, and
    rename it over the target; remove the temporary file on failure.
  - B. Require a new target path, as `backup`/`restore` do; the UI asks for a
    fresh path.
  - C. Document the overwrite risk.
- **Recommendation:** A. It preserves the existing file and the familiar
  "replace" behavior.
- **Verification:** a simulated write failure leaves the previous file intact;
  a successful export is complete and imports into an empty memory.
- **Links:** [`app-service-memory-audit.md`](app-service-memory-audit.md).

### AUD-2026-11-11-16 — the native IPC path has no automated E2E

- **Expected invariant:** a first-class desktop surface has an automated test
  that drives the real Tauri IPC, not only a mocked browser backend and a
  window-launch smoke.
- **Observed:** the Playwright test (`e2e/app.spec.ts:3`) drives the Vite build;
  `api.ts:11-28` routes to `mockInvoke` when Tauri is absent or
  `VITE_MOCK_IPC=1`. The README's native check is `xvfb-run -a timeout 15
  ./src-tauri/target/release/quantrule-desktop` (`README.md:32-40`), which only
  proves the window stays open. The 607 plan defers WebDriver to Phase 3.6
  (`607-desktop-ui.md`), and the release readiness records the gap as a
  residual risk.
- **Impact:** a command registration mistake, an argument-name change, or a
  native-only UI path can ship without a failing test. The browser E2E cannot
  prove the real IPC surface; the audit had to drive the native window with a
  scratch XTest script to see it work.
- **Affected contracts/docs:** the desktop verification plan; the release
  readiness residual risks; the 607 plan.
- **Dependencies:** the Tauri `test` feature and a WebDriver setup; the
  standalone desktop workspace.
- **Effort:** M.
- **Options:**
  - A. Add a `tauri-driver` + `WebKitWebDriver` E2E that launches the built app
    and runs the core local workflow against a fixture workspace, asserting the
    real UI and IPC.
  - B. Add a native integration test for the command bridge (the commands are
    already unit-tested; the bridge is not).
  - C. Keep the browser E2E and the launch smoke, and document the residual
    risk.
- **Recommendation:** A. The native window is now a first-class surface; the
  test should follow it.
- **Verification:** the native E2E starts the built app, opens a fixture
  workspace, summarises a dataset, validates and runs the model, records the
  run, exports, and asserts the UI; it fails if a command is unregistered or an
  argument is renamed.
- **Links:** [`607-desktop-ui.md`](../../execution-plans/2026-11-11/607-desktop-ui.md).

### AUD-2026-11-11-17 — the desktop library panics on startup failure

- **Expected invariant:** library code returns an error; a binary entry point
  may panic only when the failure is unrecoverable and the message says what to
  do next (`AGENTS.md` §3).
- **Observed:** `lib.rs:31` ends `run()` with
  `.expect("the QuantRule desktop window failed to start")`. `main.rs:6-7`
  calls `quantrule_desktop_lib::run()` with no result. A missing display,
  WebKitGTK, or GTK failure panics from the library function; the message does
  not name the missing dependency or the next step.
- **Impact:** a headless or unsupported environment crashes without guidance;
  the adapter violates the library-code rule.
- **Affected contracts/docs:** `AGENTS.md` §3; the desktop README's native
  dependency section.
- **Dependencies:** none.
- **Effort:** XS.
- **Options:**
  - A. Return `Result<(), tauri::Error>` from `run`, handle it in `main`, print
    an actionable message (for example "install WebKitGTK/GTK or start a
    display"), and exit non-zero.
  - B. Move the `expect` to `main` as a binary entry point and improve the
    message.
  - C. Keep the panic.
- **Recommendation:** A. It keeps the library testable and the binary
  responsible for the user-facing message.
- **Verification:** a simulated startup error returns an error and exits
  non-zero; no panic path remains in `quantrule_desktop_lib`.
- **Links:** `AGENTS.md` §3.

### AUD-2026-11-11-18 — the native UI has no file or folder pickers

- **Expected invariant:** a first-class desktop UI lets a human choose a
  workspace folder and a file path with the platform's picker, validates the
  choice, and remembers recent paths.
- **Observed:** the Settings workspace and memory fields are text inputs
  (`SettingsPanel.svelte:19-33`); the memory export, import, backup, and restore
  paths are text inputs (`MemoryPanel.svelte:306-322`); the run-summary export
  path is a text input (`RunPanel.svelte:94-99`). The Tauri crate has no dialog
  plugin (`src-tauri/Cargo.toml`), and the capability file has only
  `core:default`. The 607 plan chose text inputs deliberately: "No extra dialog
  plugin; deterministic and testable" (`607-desktop-ui.md`).
- **Impact:** path entry is error-prone; the default export failure (AUD-13) is
  one symptom; the user must know the filesystem; the first-class desktop
  surface still behaves like a shell with buttons.
- **Affected contracts/docs:** the 607 plan's file-selection decision; the
  desktop README; the settings and memory UI.
- **Dependencies:** the path-boundary decision (AUD-14); a new dependency
  (`tauri-plugin-dialog`) requires human approval.
- **Effort:** M.
- **Options:**
  - A. Add `tauri-plugin-dialog` for folder/file pickers and validate the
    selected path; keep the text field as a fallback.
  - B. Add inline path validation and recent-path suggestions without a new
    plugin.
  - C. Keep the text inputs and document the choice.
- **Recommendation:** A + B. The desktop is first-class; a picker is the
  expected interaction, and inline validation catches typos.
- **Verification:** a UI test with the dialog mocked selects a workspace,
  memory, and export path; the command receives it; an invalid path is rejected
  inline before the command runs.
- **Links:** [`607-desktop-ui.md`](../../execution-plans/2026-11-11/607-desktop-ui.md).

## Native verification performed

The native window was launched from the v0.9.0 release binary under Xvfb with a
temporary `HOME`/`XDG_DATA_HOME`, software rendering, and no window manager. A
scratch Python/XTest driver sent clicks and keys; screenshots were captured with
ImageMagick. The driver and screenshots are outside the repository at
`/tmp/opencode/qr-native`; the commands and code references below are
reproducible.

| Check | Observed |
|---|---|
| Native launch | The window rendered at 1200×800; no crash; the Svelte UI showed the Workspace section |
| `open_workspace` | The fixture workspace opened with 1 dataset and 1 run config |
| `dataset_summary` | `state_m1`: 260 M1 candles, first open `2025-01-01T00:00:00Z`, last open `2025-01-01T04:19:00Z` |
| `check_run` | `native-audit`, EURUSD, `test:v1`, ladders `1m, 5m, 15m, 1h`, execution 5m, M1.0, P1.0 |
| `run_model` | EURUSD, 4 timeframes, 585 events; 1m 260 bars/520 events/68 levels; 5m 52/62/21; 15m 17/3/5; 1h 4/0/0 |
| `memory_status` | `quantrule.research-memory/v1`, version 1 |
| `record_run` | Recorded run #1; the DB has 1 workspace, 1 dataset, 1 config, 1 run |
| `memory_export` | A logical export was written with 1 workspace, 1 dataset, 1 config, 1 run, 0 metrics, 0 claims, 0 notes |
| `export_run_summary` | The default path failed with `No existe el fichero o el directorio`; the model table and export controls were hidden by the error; the service test with an existing parent passes |
| Command registration | 16 `#[tauri::command]` functions and 16 frontend `call` names match; the argument names are snake_case and match the frontend |
| Frontend check | `npm run check`: 0 errors, 0 warnings |
| Frontend unit tests | `npm test`: 7 passed |
| Browser E2E | `npm run test:e2e`: 1 passed (fixture backend) |
| Desktop backend tests | `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`: 4 passed |
| Desktop fmt/clippy | `cargo fmt` and `cargo clippy --all-targets -- -D warnings` pass |
| Source/release identity | `git diff v0.9.0..8c4f2ca -- apps crates` is empty; the released binary matches the audited source |

### Native confirmation of prior findings

- **AUD-2026-11-11-05 (workspace name):** the native `record_run` produced a
  workspace row with root `/tmp/opencode/qr-native/workspace` and name
  `native-audit`, the run name. The native UI showed the workspace as
  `native-audit`, not the directory name.
- **AUD-2026-11-11-08 (zero-duration run):** the native DB row has
  `started_at = finished_at = 1794434538`; the logical export repeats the
  equality. The native UI supplied both values from `Date.now()`.
- **AUD-2026-11-11-03 (path identity):** the native memory dataset row stores
  the canonical path `/tmp/.../data/sources/state_m1.csv`, while the run
  `summary_json` stores `/tmp/.../config/../data/sources/state_m1.csv`. The two
  strings identify one file; the summary is not fully resolved.
- No new ID is assigned for these; they remain the findings of the related
  audit.

## Limitations and unexamined areas

- One sequential audit pass by one agent. The audit is not an independent
  review and does not claim to be one.
- The native window was driven with a scratch XTest script, not WebDriver. The
  workspace, dataset, config, model, memory-open, record, and export buttons
  were exercised. The backup, restore, and import buttons were not successfully
  driven because the text-field/layout automation missed them; their service
  logic is covered by the memory tests, and the Tauri bridge is a direct call.
  The native E2E gap is recorded as AUD-16.
- The native window ran under Xvfb with software rendering and no window
  manager. The release's clean-container install, offline AppImage launch, and
  bundle rebuild were not repeated; they remain the release-readiness evidence.
- The audit did not run a backtest, strategy, execution, portfolio,
  qualification, or analytics evaluation. The parity finding is a capability
  and source inspection, not a strategy result.
- The screenshots and XTest driver are in `/tmp/opencode/qr-native` and are not
  committed. The code references and the native checks are reproducible; the
  Planner should add a native E2E rather than rely on the scratch driver.
- No model or strategy behavior changed, so no Quant Validator record is
  required. The memory is outside the signal path.
- The `run-native-2` record is a test fixture; no market, baseline, or edge
  claim is made.
- The human decision is recorded in the related audit and this document. An ADR
  is not yet written; the Planner/Architect should record the one-program,
  first-class-surface decision in `docs/adr/` as part of the remediation plan.
- The 607 desktop plan is still `In Review` and its "Excluded" list contains the
  pipeline stages that the human decision now requires. The plan must be
  revised or superseded; this audit does not change it.

## Recommended order

1. Write the shared-use-case plan for AUD-11 and AUD-12 under Requirements &
   Design / Architecture, with an ADR for the one-program decision. The plan
   moves the shell orchestration into `quantrule-app`, exposes the memory CRUD,
   and adds the GUI/CLI surfaces. No implementation before human approval.
2. Fix AUD-13 (default export path, parent creation, non-destructive error) and
   AUD-15 (atomic export) as a small, reviewable pair.
3. Fix AUD-14 (path scope) with the dialog work in AUD-18.
4. Add the native IPC E2E for AUD-16 and the startup error handling for AUD-17.
5. Revise or supersede the 607 plan and the release documentation to match the
   human decision and the new parity scope.

## Links

- Related audit: [`app-service-memory-audit.md`](app-service-memory-audit.md)
- Native desktop: `apps/desktop/README.md`
- Desktop plan: [`607-desktop-ui.md`](../../execution-plans/2026-11-11/607-desktop-ui.md)
- Parent Tauri plan: [`588-tauri-platform.md`](../../execution-plans/2026-11-11/588-tauri-platform.md)
- Command surface: [`command-surface.md`](../../architecture/command-surface.md)
- Memory contract: [`research-memory-v1.md`](../../config/2026-11-11/research-memory-v1.md)
- v0.9.0 release: [`v0.9.0.md`](../../releases/2026-11-11/v0.9.0.md)
- Release readiness: [`release-readiness.md`](../../reports/2026-11-11/release-readiness.md)
