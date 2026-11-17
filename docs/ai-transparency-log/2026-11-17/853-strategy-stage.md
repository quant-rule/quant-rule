# AI Transparency Log — Strategy Stage Vertical Slice

- Date: 2026-11-17 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #853, `853-strategy-stage`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)
- ADR: [`docs/adr/2026-11-16/01-stage-execution-and-evidence-boundary.md`](../../adr/2026-11-16/01-stage-execution-and-evidence-boundary.md)
- Depends on: #850, #851, and #852

## Intent and boundary

Deliver the strategy stage vertical slice: a profile-scoped lookup from a model
artifact id to its succeeded upstream stage, a strategy executor that reuses the
existing application use case, the `stage run strategy` and `stage explain`
shell commands, and the Workbench Strategy tab with shared client parity and an
end-to-end test.

The AI did not add portfolio compute, promotion, a research ledger, or an
evidence run. It did not change the model, strategy, execution, or portfolio
contract. The strategy stage is diagnostic: it writes no run row, canonical
digest, baseline, or research record.

## Verification and evidence

- `cargo fmt --all -- --check` — pass.
- `cargo clippy --workspace --all-targets -- -D warnings` — pass.
- `cargo test --workspace --no-fail-fast` — pass; no target reported a failure.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` — pass.
- `bash tools/checkers.sh` — pass.
- `python3 tools/check_links.py` — pass.
- `python3 tools/check_changelog.py` — pass.
- `python3 tools/check_command_surface.py` — 36 implemented, 10 planned.
- `python3 tools/check_client_docs.py` — pass.
- `python3 tools/check_structure.py` — pass.
- `python3 tools/check_doc_length.py` — pass.
- `python3 tools/check_file_size.py` — pass.
- `bun run check` in `apps/web` — pass.
- `bun test` in `apps/web` — pass.
- `bun run build` in `apps/web` — pass.
- `bunx playwright test` — pass, including the strategy-stage flow.

The executor is tested with the in-memory stage client, data client, and object
store. The composed tier and a live evidence-service lookup are not exercised by
the fast gate.

## Learning and corrections

- The strategy submission keeps the API input
  `{kind:"model_artifact", model_artifact_id}`. The server resolves the upstream
  model stage through a new internal route,
  `GET /internal/v1/stages/by-artifact/{kind}/{artifact_id}`, and derives the
  dataset from the upstream provenance. A supplied `data_version` that disagrees
  returns `stage.invalid`; an unknown artifact returns `stage.not_found`.
- The store lookup is profile-scoped and visible only for a succeeded stage of
  the requested kind. The memory, Postgres, and SQLite stores were extended and
  the shared conformance suite covers the wrong kind, another profile, and a
  missing artifact.
- The executor materializes the verified upstream model files and the strategy
  config bundle into a private directory, calls
  `quantrule_app::stages::strategy::run` in `spawn_blocking`, and returns exactly
  `decisions.ndjson`, `summary.txt`, and `provenance.json`. A
  `compression_bracket` strategy maps `UnsupportedEngine` to a clear failure
  because its side is knowable only after a fill.
- The divergence test found the meaningful difference: a strategy that enters
  and is rejected by execution stays flat in the coupled path, while the
  standalone stage holds its simulated position. The standalone decision at the
  later bar is `hold`/`long`; the coupled decision is `enter`/`flat`. The test
  reads the coupled `rejections.csv` (`non_contiguous_bar`) and asserts both
  sequences; the coupled behavior was not changed.
- `stage explain` downloads the published `decisions.ndjson`, selects the record
  whose `bar_index` matches `--bar`, and prints its fields. It never recomputes a
  decision. The browser uses the same parse-and-select helpers.
- The shared client-parity fixture now carries a strategy submission, a strategy
  stage detail, the decision artifact bytes, and the explain record. The shell
  and browser issue their own request sequences (29 and 27) and build the same
  normalized view.

## Residual risk

- The executor is exercised with in-memory clients; a live internal lookup and a
  live config-bundle read belong to the composed tier.
- The divergence test asserts one deterministic non-contiguous rejection. Other
  rejection reasons (zero volume, missing ATR, position already open) produce the
  same class of divergence because the stage cannot read any execution feedback,
  but they are not separately enumerated.
- A stage artifact is never evidence. The existing stage/run separation remains
  the boundary proof; promotion and research are later issues.
