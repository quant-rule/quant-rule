# AI Transparency Log — P3 Four-Axis Removal

- Date: 2026-11-05 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: [#471](https://github.com/quant-rule/quant-rule/issues/471)
- Plan: [`471-four-axis-removal.md`](../../execution-plans/2026-11-05/471-four-axis-removal.md)
- Scope: P3a-1, the behavior-neutral module move

## Intent and boundary

The human approved the P3 boundary: remove the axis scores, dashboard, state,
consensus, aggregate, shell command surface, analytics crate, and backtest axis
ladder; keep readings, events, levels, progression, context, panel, sessions,
spec, timeframe, run, execution, portfolio, and the generic bracket/event
engines.

P3a-1 moves the retained level registry and progression logic out of `axes/`
before the score modules are deleted. The old `axes::structure::registry` and
`axes::progression` paths are temporary compatibility re-exports; P3a-2 removes
them.

No model formula, event, state, or output changed in this slice.

## Changes

- Move `axes/structure/registry.rs` to `levels.rs`.
- Move `axes/progression/{mod,leg,target,tests}.rs` to `progression/`.
- Add `pub mod levels` and `pub mod progression` in `lib.rs`.
- Update internal imports to the new modules.
- Keep temporary re-exports at the old paths so the old report/dashboard code
  still compiles during the cutover.

## Verification and evidence

| Check | Observed result |
|---|---|
| `cargo check -p quantrule-model --all-targets` | Pass |
| `cargo test -p quantrule-model` | 593 unit tests, 35 doctests, and the integration suite pass |
| Behavior | No formula, event, state, or output change |

## Learning

- Moving the retained modules first keeps the tree compiling while the score
  modules are still referenced by the dashboard/report code.
- The compatibility re-exports are intentionally short-lived; leaving them in
  the final P3 tree would keep the old `axes` path alive.
- The registered 143 events do not depend on the axis state changes; those
  events use `LEGACY_UNREGISTERED` code 0. The model core cutover can remove
  them without removing a registered event.

## Remaining P3 work

- P3a-2: rewrite `TimeframeModel`, `ModelBar`, `TimeframeSnapshot`, and
  `ModelRun` without axis fields; remove the legacy axis state events.
- P3b: remove the old shell commands and rewrite `report model`/`events` around
  the flat model.
- P3c: remove `quantrule-analytics` and the backtest axis ladder/context.
- P3d: delete old modules, tests, fixtures, and docs; update the M1.0 contract.
