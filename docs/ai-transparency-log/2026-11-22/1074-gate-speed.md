# AI Transparency Log — Gate Speed and Retired Research Guard

- Date: 2026-11-22 (UTC)
- Issue: [#1074](https://github.com/quant-rule/quant-rule/issues/1074)
- Plan: [`docs/execution-plans/2026-11-22/1074-gate-speed.md`](../../execution-plans/2026-11-22/1074-gate-speed.md)
- Tool: OpenCode agent

## Intent and boundary

Reduce the compile work a pull request needs by classifying the change scope,
while keeping the full gate for engine/shared changes, `main`, and releases.
Make historical research programs non-runnable without rewriting their audit
records. No product, model, strategy, execution, portfolio, or client behavior
change.

## Work

- Added `qr-governance gate plan` with a pure path classifier and CLI tests.
- Added `tools/gate.sh --changed` and `--explain`.
- Added the `retired`/`retirement_note` fields and refusal to the research
  runner; marked the 11 historical program manifests retired.
- Added the change-scope workflow, execution plan, and gate documentation.

## Verification

- `cargo test -p quantrule-governance` (classifier, CLI, research guard).
- `tools/gate.sh --explain` on docs/governance/web/full ranges.
- CI shadow run before conditional jobs.
- Full gate on `main` and release unchanged.
- CI-only `CARGO_INCREMENTAL=0` and `CARGO_PROFILE_DEV_DEBUG=0` are enabled and
  measured on the next full and scoped runs.

## Learning

- The checker binary is the only unconditional Rust compile; the plan keeps it
  once per PR by combining planning and checking.
- Historical command arrays are evidence, not live runners; the retired flag
  makes that explicit.
