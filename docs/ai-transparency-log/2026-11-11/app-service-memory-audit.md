# AI Transparency Log — Application Service and Research Memory Audit

- Date: 2026-11-11 (UTC)
- Work item: `app-service-memory-audit`
- Audit: [`docs/audits/2026-11-11/app-service-memory-audit.md`](../../audits/2026-11-11/app-service-memory-audit.md)
- Baseline: `main` at `8c4f2ca` (2026-11-11)
- Tool: OpenCode agent (DeepSeek V4.1 Flash)
- Branch: `docs/audit-v090-app-service-memory`
- Claim type: program audit; no model, strategy, or edge claim.

## Intent and boundary

Perform the read-only Audit stage of the Audit Workflow on the v0.9.0
application service and local research memory. The audit covers the service and
memory code, the desktop command and memory boundary, the records that describe
them, and the test gate used to verify them. It does not change code, tests,
configs, issues, or pull requests, and it does not test a trading hypothesis. No
Quant Validator record is required because no model or strategy behavior was
changed or evaluated.

## Work performed

- Read the audit workflow, the memory contract, the app-service and memory
  implementation, the desktop command boundary, the memory panel, the v0.9.0
  release records, and the related execution plans.
- Ran the app crate gate: `cargo fmt`, `cargo clippy -p quantrule-app`,
  `cargo test -p quantrule-app` (103 unit tests and one doctest), and
  `cargo doc -p quantrule-app`.
- Ran the full workspace test suite. The first run failed in the model boundary
  tests because the shared Cargo target directory had reused a test binary
  compiled in the removed `readme-refresh` worktree; the compile-time
  `CARGO_MANIFEST_DIR` pointed at that worktree. `cargo clean -p quantrule-model`
  refreshed the binary; the model boundary tests and the full workspace then
  passed with 1,476 passed, 0 failed, and 8 ignored. This became AUD-03.
- Ran the repository checkers: links, terms, structure, file size, execution
  plans, workflows, changelog, releases, command surface, legacy, and claims.
  All passed.
- Wrote a scratch probe crate at `/tmp/opencode/qr-audit-probe`, outside the
  repository, with seven tests for the suspected behaviors:
  foreign-database adoption, foreign-database restore, workspace-root
  duplication, workspace-name overwrite, unreadable integer import, inverted
  timestamps/null metric values, and `update_note` ordering. All seven probes
  reproduced the behaviors recorded in the audit.
- Published the audit document with nine findings: three Major and six Minor.
  No code, test, config, issue, or PR was modified.

## Native-window follow-up (2026-11-11)

The human accepted the audit and decided that the desktop UI is a first-class
surface: the GUI and the shell are the same program, with feature parity and one
shared source, and the shell is primarily for agents. The audit was extended to
the native Tauri window under that decision.

- Launched the v0.9.0 release binary (`apps/desktop/src-tauri/target/release/quantrule-desktop`)
  under Xvfb with a temporary `HOME`/`XDG_DATA_HOME`, software rendering, and a
  scratch Python/XTest driver.
- Verified the native window renders and the real IPC path works: opened a
  fixture workspace, summarised a dataset (260 M1 candles), validated a run,
  computed the model (EURUSD, 4 timeframes, 585 events), opened the memory,
  recorded a run, and wrote a logical export.
- Reproduced the native run-summary export failure: the default path points into
  an output root the app does not create; the error hides the model table and
  the export controls. The memory record contains zero metrics, claims, and
  notes; the model counts are not stored.
- Found the GUI/shell parity gap: the shell has the backtest, strategy,
  execution, portfolio, qualification, analytics, candle, event, report, and
  fetch use cases; the GUI has workspace, dataset, config, model, export, and
  memory only. The desktop README and Settings page state the pipeline commands
  remain on the CLI.
- Wrote [`native-tauri-window-audit.md`](../../audits/2026-11-11/native-tauri-window-audit.md)
  with three Major and five Minor findings. The finding IDs continue after the
  first audit (AUD-2026-11-11-11 through -18); the first audit was renumbered to
  AUD-02 through -10 to avoid colliding with the existing simulation data loss
  finding AUD-2026-11-11-01.
- Ran the frontend and desktop checks: `npm run check` (0 errors, 0 warnings),
  `npm test` (7 passed), `npm run test:e2e` (1 passed, fixture backend),
  `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` (4 passed), and
  the desktop fmt/clippy checks. No source, test, config, issue, or PR was
  modified.

## Verification and evidence

- `cargo fmt --all -- --check`: pass.
- `cargo clippy -p quantrule-app --all-targets -- -D warnings`: pass.
- `cargo test -p quantrule-app`: 103 passed, 0 failed; one doctest passed.
- `RUSTDOCFLAGS='-D warnings' cargo doc -p quantrule-app --no-deps`: pass.
- `cargo test --workspace --no-fail-fast`: first run failed on the stale model
  test binary; after `cargo clean -p quantrule-model`, 1,476 passed, 0 failed,
  8 ignored.
- `python3 tools/check_links.py`: 839 Markdown files, no broken relative links.
- `python3 tools/check_terms.py`: 540 files, no banned terms.
- `python3 tools/check_structure.py`, `check_file_size.py`,
  `check_execution_plans.py`, `check_workflows.py`, `check_changelog.py`,
  `check_releases.py`, `check_command_surface.py`, `check_legacy.py`, and
  `check_claims.py`: pass.
- Scratch probe `cargo test --test probe`: 7 passed, 0 failed.
- Native follow-up: `npm run check` 0 errors/0 warnings; `npm test` 7 passed;
  `npm run test:e2e` 1 passed (fixture backend); desktop
  `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` 4 passed;
  desktop fmt and clippy pass.
- Native follow-up: the release binary rendered the window; `open_workspace`
  returned 1 dataset and 1 run config; `dataset_summary` returned 260 M1
  candles; `check_run` returned native-audit/EURUSD/test:v1; `run_model`
  returned EURUSD/4 timeframes/585 events; `memory_status` returned v1;
  `record_run` wrote one workspace/dataset/config/run; `memory_export` wrote a
  logical export with zero metrics, claims, and notes.
- Native follow-up: `export_run_summary` with the default path failed with
  `io error: .../config/../target/native-audit/run-summary.json: No existe el
  fichero o el directorio`; the UI hid the model and export controls. The
  service unit test with an existing parent passes.

The audit document records the exact commands, code references, probe names,
and limitations. The probes are evidence, not committed tests; the Planner must
add repository regression tests for each confirmed finding.

## Learning

- A SQLite `user_version` of 0 is not proof of a fresh database. The memory's
  stored `meta.schema` identity is the only safe discriminator, and it must be
  checked before the migration creates it. A wrong restore file currently
  reports success with an empty memory, which is the highest-impact finding.
- The application service canonicalizes dataset paths but not the workspace
  root or the run config path. The memory's unique-root rule protects the
  string, not the directory, so one workspace can split into several rows.
- The project's shared `CARGO_TARGET_DIR` is a reproducibility hazard across
  worktrees. The audit's first `cargo test --workspace` failed on a correct tree
  because a cached test binary carried a removed worktree's absolute path. The
  release evidence is reproducible only after a clean step; a per-worktree
  target is the cleaner rule.
- The desktop memory panel records `started_at == finished_at` and shows
  workspace-wide notes under a selected run. These are presentation/recording
  defects, not engine defects, but they affect the evidence the memory exists
  to preserve.
- The v0.9.0 release closed the code and release records but left the normative
  agent/architecture docs, the app module doc, and two shipped record statuses
  on the pre-desktop surface. The human must decide whether the desktop UI is a
  first-class surface or whether the memory needs a shell command; the audit
  recommends the former.
- The native window is not a mock: the real Tauri IPC path opened a workspace,
  ran the model, wrote the memory, and exported. The browser E2E with the
  fixture backend did not exercise that path. A first-class desktop surface
  needs a native E2E, not only a launch smoke.
- The native run-summary default path is a real first-use failure: the output
  root is not created, the export error replaces the model view, and the user
  must re-run the model to retry. The UI should keep the result visible and
  create or choose a valid parent.
- The memory is currently a run registry: the model counts are discarded, and
  the typed metric/claim/note CRUD is not reachable from the GUI or the shell.
  Under the one-program decision, this is a parity gap, not a future feature.
- The shell and GUI are not the same program yet: the pipeline stages live in
  the shell and the memory lives in the GUI. The shared application service is
  partial. The next step is a Requirements & Design / Architecture plan, not a
  direct Tauri-command addition that would duplicate the shell.
- The audit is one sequential pass by one agent. It is not an independent
  review, and a clean audit is not an approval or a release gate.
