# AI Transparency Log — Spec Schema and Evaluator (#423)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: model-spec schema, validation, and evaluator
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Status: resolved spec, validation, evaluator, and JSON parser implemented and
  tested; the CLI and artifacts are #424

## Intent and boundary

Implement `quantrule.model-spec/v1` as a bounded measurement document: a
timeframe ladder, an evaluation timeframe, and named boolean conditions over the
published events and discrete states. The spec configures measurement only; it
carries no strategy, position, cash, or P&L field.

The shell parses JSON and resolves registry event identifiers. The model receives
numeric codes and evaluates the spec. This keeps the model crate free of a JSON
dependency and keeps the registry resolution at the boundary that owns I/O.

## Implemented work

| Item | Detail |
|---|---|
| `crates/quantrule-model/src/spec.rs` | Resolved spec types, validation, as-of selection, event/state evaluator |
| Extended batch | The evaluator reads `events()` and `extended_events()` together |
| `TimeframeModel::states_at` | Cheap state accessor; no full bar assembly per condition |
| `crates/quantrule-shell/src/spec.rs` | JSON parser, embedded registry resolution, strict unknown-field rejection |
| `docs/model/2026-11-05/model-spec-v1.md` | v1 contract |
| Fixture | `crates/quantrule-shell/tests/fixtures/model_spec_v1.json` |

Limits: depth 16, 256 nodes per condition, 64 conditions, 10,000-bar event
window. v1 defers reading overrides, event-family switches, and panel/session
conditions.

## Verification and evidence

Observed on 2026-11-05:

| Command | Result |
|---|---|
| `cargo test -p quantrule-model --lib spec::` | 9 tests pass |
| `cargo test -p quantrule-shell --bin quantrule spec::` | 2 tests pass |
| `cargo clippy -p quantrule-shell --all-targets -- -D warnings` | clean |
| `cargo fmt --check` | clean |

The shell parser is declared `#[cfg(test)]` until #424 wires `report model
--spec`; the module is tested but not yet reachable from a command. The log says
so rather than implying the CLI exists.

## Learning and rejected suggestions

- The evaluator originally read only `TimeframeModel::events()`; a test with a
  MAR-regime condition failed because the extended batch is separate. The
  evaluator now merges both batches and sorts the merged availability list.
- The state predicate originally assembled the complete `ModelBar` for every
  condition bar. `states_at` avoids recomputing direction, momentum, structure,
  progression, and context for a state read.
- The parser resolves event IDs from the embedded registry rather than accepting
  numeric codes in the document; a reviewer can read a spec without the registry
  open.
- v1 does not expose reading overrides. Adding them silently would make the
  document a second model config; the contract defers them to v2.
