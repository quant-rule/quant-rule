# AI Transparency Log — P0 Legacy Inventory

- Date: 2026-11-05 (UTC)
- Tool: OpenCode / DeepSeek V4.1 Flash
- Work item: [#468](https://github.com/quant-rule/quant-rule/issues/468)
- Branch: `docs/468-legacy-inventory`

## Intent and boundary

Freeze `main` at `32e6fb2` and inventory the legacy artifacts before deletion.
This entry covers repository analysis and documentation only. No source, test,
config, or model behavior is changed.

## Verification and evidence

- Inventory written to `docs/registry/history.md`.
- Execution plan written to `docs/execution-plans/2026-11-05/468-legacy-inventory.md`.
- `git diff --check`, link check, and term check to be run before the PR.

## Learning

The old architecture is not one module: it is a coupled surface of model axes,
shell commands, config parsers, tests, and docs. Deleting one layer without the
inventory would leave broken references and ambiguous current-vs-history
documentation. The ADRs and run records stay as history; only the current
surface is deleted.
