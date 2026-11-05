# AI Transparency Log — P3 Four-Axis Removal

- Date: 2026-11-05 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: [#471](https://github.com/quant-rule/quant-rule/issues/471)
- Plan: [`471-four-axis-removal.md`](../../execution-plans/2026-11-05/471-four-axis-removal.md)
- Scope: P3a–P3d, the full four-axis removal

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

## P3a–P3d implementation

- Rewrote `TimeframeModel`, `ModelBar`, and `ModelRun` without axis, consensus,
  or snapshot fields.
- Removed the legacy axis state events from `SignalEventSeries`. They used code 0
  and were never registry events.
- Deleted `axes`, `dashboard`, `state`, `consensus`, `aggregate`, and the old
  report assembler/chart.
- Moved the retained level registry and progression series to flat modules.
- Rewrote the shell around data/fetch, `report model`, `events`, `policy check`,
  and `help`. Removed the old chart, store, research, lab, strategy, and backtest
  commands.
- Removed `quantrule-analytics`.
- Removed the backtest axis ladder, context, strategy walk, and runner; kept
  execution, portfolio, the bracket engine, and the event-composite engine.
- Removed the strategy and analytics files from `run.json`; the loader rejects
  them as unknown fields.
- Removed the `fetch` PNG chart path; fetch writes CSV only.
- Deleted the approved old model, config, and tutorial documents and redirected
  historical links to `docs/registry/history.md`.
- Updated the M1.0 contract, README, docs index, architecture kata, and
  characteristics.

## P3 verification and evidence

| Check | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `cargo test` | 977 passed, 0 failed, 7 ignored across 28 suites |
| `python3 tools/check_legacy.py` | 179 current files; no retired labels |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |
| `python3 tools/check_validation.py` | 188 claims |
| `python3 tools/check_links.py` | 538 Markdown files; no broken relative links |
| `python3 tools/check_terms.py` | 324 files; no banned terms |
| Signal golden fixture | 69 legacy axis-state rows removed; 49 registered rows remain and pass |
| `report model` fixture | Flat bundle; no chart artifacts; no axes or market_report field |
| `events` fixture | Deterministic JSON; no axis state events |

The Quant Developer and Quant Validator passes were sequential passes by the
same agent, not independent agents or independent model judgments. The
Validator reproduced the working tree before the commit; the PR and CI run are
the post-commit reproduction.

## P3 learning

- The axis state events were the only model output that depended on the axis
  series. Removing them loses no registry event, but it does change the signal
  golden fixture; the fixture now carries the registered rows only.
- The shell's report bundle was coupled to `MarketReport` and the chart
  renderer. A flat `FlatReport` projection made the report independent of the
  deleted dashboard and chart code.
- The backtest context and strategy walk were the last axis consumers. Removing
  them exposed the generic event engine's dependence on the context type; its
  input now takes explicit timeframe views, which is the S3.x shape.
- The `fetch` chart option could not survive the chart deletion. The command now
  writes CSV only; this is an intentional command-contract change recorded in
  the plan.
- Historical documents linked to deleted current contracts. Redirecting those
  links to the history index keeps the history readable without restoring the
  retired current docs.
