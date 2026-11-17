# AI Transparency Log — Portfolio Stage Vertical Slice

- Date: 2026-11-17 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #854, `854-portfolio-stage`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)
- ADR: [`docs/adr/2026-11-16/01-stage-execution-and-evidence-boundary.md`](../../adr/2026-11-16/01-stage-execution-and-evidence-boundary.md)
- Depends on: #850–#853

## Intent and boundary

Deliver the portfolio stage vertical slice: a profile-scoped lookup from a
decisions artifact to its succeeded strategy stage, an executor that runs the
existing execution and portfolio use cases without the strategy engine, the
`stage run portfolio` shell command, and the Workbench Portfolio tab with shared
parity and invariant coverage.

The AI did not add promotion, research memory, or a backtest run. It did not
change model, strategy, execution, or portfolio rules. The stage artifact is
diagnostic: it writes no run row, canonical digest, baseline, or evidence
record, and it is never a backtest input.

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
- `bunx playwright test` — 48 passed, including the two portfolio flows.

The executor is tested with the in-memory stage/data clients and object store.
The composed tier and a live internal lookup are not exercised by the fast gate.

## Learning and corrections

- The portfolio submission keeps `{kind:"decisions_artifact",
  decisions_artifact_id}`. The route resolves the succeeded strategy stage in
  the active profile, rejects an unknown or non-strategy artifact with
  `stage.not_found`, and derives both `data_version` and `model_version`. A
  supplied `data_version` or model-version mismatch returns `stage.invalid`.
- The executor materializes the frozen decisions bundle, the run config, and the
  catalogued dataset in a private workspace. It materializes a named strategy
  file because `RunConfig::load` validates it, but it never runs the strategy
  engine. A v3 config without a strategy file also runs.
- The executor returns two prefixed bundles. Prefixing is required because both
  bundles publish `summary.txt` and `provenance.json`; a flat merge would drop
  one of each.
- The published-artifact invariant is checked from `portfolio/metrics.json`
  (`final_equity = initial_cash + net_pnl`) and the execution summary carries
  fees and slippage. A tight drawdown publishes one risk event and `halted=true`;
  the existing ledger test proves a halted ledger refuses a later `size_entry`.
- The portfolio tables are filtered by kind, the shared selection is cleared on
  a tab switch, and a Playwright case proves a strategy detail does not leak
  into the Portfolio tab.

## Residual risk

- The executor is exercised with in-memory clients; the live internal lookup,
  config read, and dataset object read belong to the composed tier.
- The stage replays fills and does not re-size entries against a halted ledger,
  because sizing belongs to the coupled runner. The halt gate is therefore
  proven by the existing ledger test, while the stage proves the published risk
  event and `halted` metric.
- A stage artifact is never evidence; the baseline and research surfaces are
  later issues.
