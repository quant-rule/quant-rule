# AI Transparency Log — Model Stage Vertical Slice

- Date: 2026-11-17 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #852, `852-model-stage`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)
- ADR: [`docs/adr/2026-11-16/01-stage-execution-and-evidence-boundary.md`](../../adr/2026-11-16/01-stage-execution-and-evidence-boundary.md)
- Depends on: #850 and #851

## Intent and boundary

Deliver the model stage vertical slice: a real model executor that reuses the
content-addressed model bundle, a verified stage artifact download, the seven
model-stage shell commands, and the Workbench Data/Model browser tabs with
shared client parity and an end-to-end test.

The AI did not add strategy or portfolio compute, `stage explain`,
`run submit --model-artifact`, a research ledger, operations screens, or release
work; those remain later issues. It did not change the model, strategy,
execution, or portfolio contract. The model stage is diagnostic: it writes no
run row, canonical digest, baseline, or research record.

## Verification and evidence

- `cargo fmt --all -- --check` — pass.
- `cargo clippy --workspace --all-targets -- -D warnings` — pass.
- `cargo test --workspace --no-fail-fast` — pass; 2,231 tests passed and no
  target reported a failure.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` — pass.
- `bash tools/checkers.sh` — 24 checkers and their tests passed.
- `python3 tools/check_links.py` — 1,002 Markdown files, no broken links.
- `python3 tools/check_changelog.py` — pass.
- `python3 tools/check_command_surface.py` — 34 implemented, 12 planned.
- `python3 tools/check_client_docs.py` — pass.
- `python3 tools/check_structure.py` — pass.
- `python3 tools/check_doc_length.py` — pass.
- `python3 tools/check_file_size.py` — pass.
- `bun run check` in `apps/web` — 0 errors and 0 warnings.
- `bun test` in `apps/web` — 131 passed.
- `bun run build` in `apps/web` — pass.
- `bunx playwright test` — 44 passed, including the 2 model-stage flows and the stage profile-scope flow.

The model executor is tested with the in-memory model client and object store;
the worker's fake-executor tests remain the control-plane proof. The composed
tier and a live model-service call are not exercised by the fast gate.

## Learning and corrections

- The model executor copies the exact bundle bytes under the stage blob
  namespace. It verifies the manifest schema, dataset, config hash, sorted
  artifact set, and every byte before returning files; the stage worker then
  writes its own stage manifest last. The manifest `model_version` is the model
  registry version, not the run configuration's `model_id`, so the executor does
  not compare it with the record's `model_version`.
- The stage artifact download verifies the record's size and SHA-256 on every
  read. The shared name checker now returns a plain message so the run route can
  keep `run.invalid` and the stage route can return `stage.invalid`.
- `stage events` reads and filters `events.csv`; it reads `report.json` only for
  the four coverage fields, which live under the real envelope's
  `flat_report.timeframes`. Both parsers have a regression case for that
  envelope, not only the shared fixture. The shell converts a timeframe code to
  seconds and otherwise leaves the published values untouched.
- The Workbench Data tab reuses the existing Datasets panel; the Model tab uses
  a stage store, a shared stage view module, and verified downloads. The stage
  store is part of the profile-scope boundary, so a profile switch clears and
  reloads it with the other four stores; an end-to-end case proves the previous
  stage row does not survive the switch. The browser never computes a model
  value.
- The client-parity fixture now carries one deterministic stage response and the
  real-envelope `events.csv` / `report.json` bytes. The shell and browser issue
  their own request sequences (25 and 23) and build the same normalized view.

## Residual risk

- The model stage reuses the model service's bundle; a stale or corrupt object
  is rejected by the byte/hash check, but the fast gate uses a fake client and
  object store.
- `P2-08` names a qualification/baseline rejection that cannot be exercised
  until the research ledger exists (P6). The implemented boundary test proves the
  stage row and artifact are invisible through the run store and run artifact
  route.
- The Workbench Model tab is covered by a mocked Playwright flow; no live
  gateway or engine runs in the browser tier.
