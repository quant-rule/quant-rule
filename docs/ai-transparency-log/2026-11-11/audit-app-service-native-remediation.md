# AI Transparency Log — Audit Remediation

- Date: 2026-11-11 (UTC), continued 2026-11-12
- Work item: `audit-app-service-native-remediation`
- Plan: [`docs/execution-plans/2026-11-11/audit-app-service-native-remediation.md`](../../execution-plans/2026-11-11/audit-app-service-native-remediation.md)
- Audits: [`app-service-memory-audit.md`](../../audits/2026-11-11/app-service-memory-audit.md), [`native-tauri-window-audit.md`](../../audits/2026-11-11/native-tauri-window-audit.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash)
- Branch: `fix/audit-app-service-native-remediation`
- Claim type: Behaviour Claim

## Intent and boundary

Implement the approved remediation plan for the two 2026-11-11 audits and the
human decision that the desktop UI is a first-class surface. The work fixes
program defects and the native-visible UI paths; it does not change model,
strategy, execution, portfolio, or analytics behavior. No Quant Validator
record is required for the completed changes: the signal path and engine
outputs are unchanged.

## Work performed

- **Memory integrity (AUD-02):** a database with user tables but no
  research-memory `meta` schema is refused before migration; `restore`
  validates the backup read-only before copying. The file is left unchanged and
  a foreign restore creates no target.
- **Path identity (AUD-03):** `Workspace::open` canonicalizes the root;
  `record_run` canonicalizes the config path and the run summary's data source;
  the workspace name comes from the root directory, not the run (AUD-05).
- **Import and note invariants (AUD-06, AUD-07):** import rejects non-numeric
  integer text, inverted run/dataset/note timestamps, null metrics with values,
  and non-positive metric IDs; `update_note` rejects `updated_at` before
  `created_at`.
- **Atomic logical export (AUD-15):** `export_to_file` writes a temp file in the
  target directory and renames on success; a failed write removes the temp and
  leaves the previous export.
- **Run export (AUD-13):** `write_run_summary_file` creates the missing parent;
  the desktop keeps the model table and export controls visible when an export
  fails.
- **Run timing (AUD-08):** the run panel stores the completed model and its
  start/finish; the memory panel records the real timestamps and a null finish
  for an unknown run.
- **Run-scoped notes (AUD-09):** the store and Tauri command expose
  `notes_for_run`; the memory panel uses it.
- **Model evidence (AUD-12 partial):** `record_run_with_model` stores the model
  counts in the run summary; the desktop passes the completed model to the
  memory record.
- **Startup error (AUD-17):** the desktop library returns `tauri::Error`; the
  binary prints an actionable message and exits non-zero instead of panicking.
- **Gate isolation (AUD-04):** `tools/gate.sh` runs the workspace gate with a
  target inside the worktree; the execution-plan workflow, template, and
  `AGENTS.md` require it.
- **Native bridge guard (AUD-16 partial):** a source-level test checks the Tauri
  handler list, command functions, frontend `call` names, and mock cases agree.
- **Records (AUD-10):** the one-program ADR, `AGENTS.md`, architecture README,
  command surface, module doc, plan statuses, run-v3 status, changelog, and audit
  remediation plan are updated. The desktop is recorded as first-class; the
  pipeline parity gap is explicit.

## Verification

- `cargo test --workspace --no-fail-fast`: 1,488 passed, 0 failed, 8 ignored.
- `cargo test -p quantrule-app`: 115 unit tests and one doctest passed.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`: pass.
- `npm run check`: 0 errors, 0 warnings; `npm test`: 10 passed;
  `npm run test:e2e`: 1 passed.
- Desktop backend: 4 tests passed; bridge source guard: 2 tests passed.
- `tools/gate.sh`: passed with `CARGO_TARGET_DIR` inside the worktree; all
  repository checkers passed.
- The native window itself was driven in the audit pass before these changes;
  the rebuilt native binary was not re-driven for the remediation. The native
  IPC E2E remains Phase 3b.

## Human acceptance

The human approved the remediation on 2026-11-12. This commit covers the
completed phase. AUD-11, the AUD-12 metric/claim/note CRUD, AUD-14, the full
AUD-16 native E2E, and AUD-18 remain Phase 3; the plan stays `In Progress`.

## Remaining Phase 3 work

- AUD-11: extract the pipeline use cases behind the shared application service
  and expose them from both surfaces. This is the large one-program parity
  change; the ADR and plan define it.
- AUD-12: expose metric/claim/note CRUD in the shell and the GUI; the model
  counts are now recorded, but the manual evidence write path is not.
- AUD-14: scope the native command paths to the workspace/memory roots and add
  the dialog-backed path grant.
- AUD-16: add the full native WebDriver E2E; the source-level bridge guard is in
  place.
- AUD-18: add native file/folder pickers (requires the dialog dependency) or
  inline validation.
- The `fetch` network-boundary parity decision remains a human decision.

## Learning

- The foreign-database guard must inspect `sqlite_master` before migration;
  `user_version = 0` is not identity.
- Canonicalizing the workspace root and config path removes the duplicate-row
  class without changing the run's behavior.
- The run-summary parent creation fixes the first-use export failure; keeping
  the model visible on error is a separate UI concern from the write.
- The model counts belong with the run record; `summary_json` is additive and
  needs no v1 schema migration.
- The one-program parity is not a small fix. Sharing the shell's command engine
  would invert the application-service boundary; the approved direction is the
  service extraction, delivered in Phase 3.
- The gate must use a per-worktree target. The source-level bridge guard is a
  useful complement to the native E2E, not a replacement.
