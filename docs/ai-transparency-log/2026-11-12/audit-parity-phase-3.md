# AI Transparency Log — Audit Parity Phase 3 Plan

- Date: 2026-11-12 (UTC)
- Work item: `audit-parity-phase-3`
- Plan: [`docs/execution-plans/2026-11-12/audit-parity-phase-3.md`](../../execution-plans/2026-11-12/audit-parity-phase-3.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash)
- Branch: `feat/audit-parity-phase-3`
- Claim type: Behaviour Claim
- Status: P3.1 implemented; P3.2–P3.5 pending

## Intent and boundary

Plan the remaining Phase 3 work for the two 2026-11-11 audits and the
one-program decision. The plan slices memory CRUD, native path scope and
pickers, the native WebDriver E2E, the shared pipeline use cases, and the
GUI/fetch decision. No implementation or code change was made in this planning
pass.

## Work performed

- Re-read the merged Phase 1/2 state at `688409d`, the parent remediation plan,
  the one-program ADR, and the remaining audit findings.
- Wrote the Phase 3 plan with five slices, traceability from AUD-11/12/14/16/18
  and the `fetch` decision, a verification table, risks, and stop conditions.
- Created the task worktree `quant-rule--audit-parity-phase-3` on
  `feat/audit-parity-phase-3` from `main` at `688409d`.

## Verification and evidence

- No tests were run for code because no code changed.
- The plan is checked by `tools/check_execution_plans.py` and the repository
  link/term checks before it is presented.
- The plan is `Proposed`; the first implementation slice is P3.1 (memory write
  path).

## Learning

- The remaining work is not one change. The pipeline extraction is a multi-stage
  refactor; the memory write path and the native boundary are independently
  reviewable.
- The shell `memory` command changes the public command surface and needs the
  help/README/command-surface update in the same slice.
- The native dialog plugin and the WebDriver environment are dependency/gate
  decisions; the plan names them explicitly.
- The `fetch` network boundary must stay a human decision; it cannot ride along
  with GUI parity silently.

## Implementation — P3.1 (memory write path)

- Added the shell `memory` command family: status, record, workspace/run lists,
  metric/claim/note CRUD, logical export/import, SQLite backup/restore. `record`
  computes the model and stores its counts through `record_run_with_model`.
- Added desktop Tauri commands for metric, claim, and note writes and the
  Memory panel forms. The forms reload the run evidence after each write.
- Updated the command surface to 16 implemented commands: help, README,
  command-surface registry, and a new research-memory tutorial.
- Added shell parser tests, a shell CLI memory-status test, a shell CRUD
  round-trip test, desktop command round-trip tests, and browser E2E coverage
  for adding a metric, claim, and note.

Verification: shell CLI 19 passed; desktop backend 4 + 2 bridge tests; frontend
10 tests; browser E2E 1; `svelte-check` clean; `tools/gate.sh` passed with 1,492
workspace tests, 0 failed, 8 ignored.

Remaining Phase 3: P3.2 path scope/pickers, P3.3 native WebDriver E2E, P3.4
shared pipeline use cases, P3.5 GUI pipeline parity and the `fetch` decision.

## Implementation — P3.2 path scope

- Added a backend-owned desktop scope: the opened workspace root and memory
  database are set by `open_workspace` and `memory_status`; every dataset,
  config, model, export, memory write, export/import, and backup/restore path is
  checked against those roots. Out-of-scope paths are rejected before I/O.
- Added the out-of-scope regression test for a dataset and a memory export.
- The dialog-backed file/folder pickers are not added: `tauri-plugin-dialog` is
  a new dependency and needs explicit approval. The typed-path inputs remain
  with backend scope validation as the fallback.

Verification: desktop backend 5 tests + 2 bridge; workspace 1,492 passed,
0 failed, 8 ignored; `tools/gate.sh` passed. P3.2 V2 is `Pass`; V3 is `Pending`
on the dialog dependency.

## Implementation — P3.2 native pickers

- The human approved `tauri-plugin-dialog`; added the Rust plugin, the
  `dialog:default` capability, and the npm package.
- Added `apps/desktop/src/lib/paths.ts` with directory, open-file, and save-file
  helpers; the browser fixture returns null, so the typed path stays the
  fallback.
- Wired native Browse buttons into Settings (workspace root and memory
  database), Memory (export, import, backup, restore), and Run (summary export).
  The buttons render only inside the Tauri webview; selected paths still pass
  the backend scope check.
- Added Settings- and Memory-panel unit tests that mock the picker helpers and
  assert the bound workspace, memory, export, and import fields receive the
  selected paths. Updated the Run-panel test mock for the `isTauri` guard.

Verification: `svelte-check` 0 errors/0 warnings; frontend 13 tests; browser E2E
1; desktop backend 5 tests + 2 bridge. P3.2 V3 is `Pass`.
