# AI Transparency Log — Application Service Foundation

- Date: 2026-11-11 (UTC)
- Work item: `606-app-service`
- Issue: [#606](https://github.com/quant-rule/quant-rule/issues/606)
- Plan: [`docs/execution-plans/2026-11-11/588-tauri-platform.md`](../../execution-plans/2026-11-11/588-tauri-platform.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash)
- Branch: `arch/606-app-service`
- Claim type: ADR (Architecture Decision)

## Intent and boundary

Create the shared application-service foundation so the command shell and the
future desktop UI call one workspace, dataset, configuration, model-run, and
export boundary. This change is structural: it moves file and path handling out
of the shell and does not change a reading, event, strategy, fill, or accounting
value.

## Work performed

- Added `crates/quantrule-app` and moved the strict config loader, CSV source
  reader, UTC date windows, content fingerprint, and model pipeline from the
  shell crate.
- Added the `Workspace` boundary: root validation, discovery, sorted run-config
  listing, and sorted dataset listing.
- Added `Dataset` and `DatasetSummary`: strict M1 loading with retained count and
  first and last open time.
- Added `AppService`: run validation, model-run outcome, model counts, and
  deterministic run-summary export.
- Added explicit `AppError` variants for config, source, date, pipeline,
  dataset, workspace, and I/O failures.
- Routed the shell `run check` command through `AppService::open_for_config` and
  the service's validated `RunSummary`.
- Added `quantrule.run-summary/v1` JSON export with sorted keys and stable list
  order.
- Updated the architecture guard for the new `quantrule-app` edge, the legacy
  allowlist for the moved config tests, the README, the living architecture
  documents, the course sources, the changelog, and this log.

## Verification

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo test --workspace --no-fail-fast`: 1,452 passed, 0 failed, 8 ignored.
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`: pass.
- `python3 tools/check_command_surface.py`: pass.
- `python3 tools/check_links.py`: 818 Markdown files, no broken links.
- `python3 tools/check_terms.py`: pass.
- `python3 tools/check_legacy.py`: pass after moving the config-test allowlist.
- `python3 tools/check_structure.py`: pass; five workspace crates.
- `python3 tools/check_file_size.py`: pass; largest source file 969 lines.
- Registry, validation, verification matrix, verification, event emitters,
  partitions, workflows, changelog, releases, claims, and execution plans: pass.
- `run check` against the current example and the fixture run: pass.

## Learning

- Moving a `pub(crate)` config surface into a library makes the shell's internal
  imports public API. The change was mechanical but broad; the architecture
  guard now records the new edge explicitly.
- The config tests depended on the shell crate's fixture directory. Their paths
  now point at `crates/quantrule-shell/tests/fixtures`, and the legacy allowlist
  follows the moved file.
- `run check` previously validated any config path. The service now discovers
  the owning workspace, with the config's directory as the standalone fallback,
  so that behavior is preserved.
- This phase does not yet move backtest, strategy, execution, portfolio, or
  analytics orchestration into the service. Those commands still call the same
  application-service config and pipeline modules directly; the UI phase will
  extend the service rather than duplicate them.
- No Quant Validator record is required: the signal path and strategy output are
  unchanged, and no golden fixture was modified.
