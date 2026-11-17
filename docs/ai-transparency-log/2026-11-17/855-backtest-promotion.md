# AI Transparency Log — Backtest Promotion

- Date: 2026-11-17 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #855, `855-backtest-promotion`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)
- ADR: [`docs/adr/2026-11-16/01-stage-execution-and-evidence-boundary.md`](../../adr/2026-11-16/01-stage-execution-and-evidence-boundary.md)
- Depends on: #850–#854

## Intent and boundary

Deliver backtest promotion: `run submit --model-artifact` reuses a succeeded
model stage as the frozen model input, records the artifact id on the run, and
keeps the full-run identity unchanged. Strategy, execution, and portfolio remain
recomputed by the coupled evidence path.

The AI did not add a research ledger, a baseline, or a new command. It did not
change the model, strategy, execution, or portfolio contracts. A stage artifact
is still never evidence by itself; only the coupled run is.

## Verification and evidence

- `cargo fmt --all -- --check` — pass.
- `cargo clippy --workspace --all-targets -- -D warnings` — pass.
- `cargo test --workspace --no-fail-fast` — pass; no target failed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` — pass.
- `bash tools/checkers.sh` — pass.
- `python3 tools/check_links.py` — pass.
- `python3 tools/check_terms.py` — pass.
- `python3 tools/check_doc_length.py` — pass.
- `python3 tools/check_changelog.py` — pass.
- `python3 tools/check_command_surface.py` — 37 implemented, 9 planned.
- `python3 tools/check_client_docs.py` — pass.
- `python3 tools/check_structure.py` — pass.
- `python3 tools/check_file_size.py` — pass.
- `bun run check` in `apps/web` — 0 errors, 0 warnings.
- `bun test` in `apps/web` — 138 passed.
- `bun run build` in `apps/web` — pass.
- `bunx playwright test` — 48 passed.

The fast gate uses in-memory evidence, stage, data, model, and object clients.
The composed tier and a live Postgres migration were not exercised locally.

## Learning and corrections

- The API and internal run types carry `model_artifact_id` as an optional
  top-level field. It is omitted from a full-run submission body and rendered as
  `none` by the shell and the browser.
- `derive_run_id` is unchanged. `derive_run_id_with_model` appends a tagged
  `model-artifact` component only when an artifact is supplied, so a promoted run
  is distinct from the same full run while the canonical digest can match.
- Evidence resolves a succeeded model stage by artifact id in the active profile,
  checks data version, config hash, and model version, and returns the stable
  `run.model_artifact_invalid` 400 for every mismatch. The run store persists the
  id in Postgres and SQLite through additive migrations.
- The worker uses the stage client only when the run names an artifact. It
  verifies the stage manifest and every blob, writes the bytes into the existing
  `Workspace`, and still calls `BacktestFromModelRequest`; the model service is
  not called. The promotion test asserts both the byte identity and the zero
  model-service calls.
- A changed-strategy test builds a second config and model stage with a different
  config hash, then compares the promoted run to that config's coupled baseline.
  This proves the strategy path is recomputed rather than reused.

## Residual risk

- The fast gate uses in-memory stores. The Postgres additive migration and the
  live internal stage lookup belong to the composed tier.
- A promoted run is idempotent by its promoted id; the full and promoted run are
  intentionally two distinct rows, and their canonical digests are compared, not
  their run ids.
- The model stage must still be visible to the active profile. Expired stage
  artifacts are rejected at submission, so promotion cannot outlive the stage TTL.
