# AI transparency log — Strategy boundary and v4 loader (P2.9, AUD-32/33)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 3
  (Execution) for `AUD-2026-11-17-32` and `AUD-2026-11-17-33`.
- Claim type: Behaviour Claim.
- Intent: keep diagnostic and unregistered events out of the strategy snapshot,
  remove the raw model `ContextBar` from the strategy view, and make the run
  loader enforce the documented execution/fill and v4 checks.
- Boundary: branch `fix/audit-strategy-boundary`, based on current `main`
  `7ae3a984`. No push, PR, merge, or board change. The Quant Validator pass is
  sequential by the same agent, not independent.

## What changed

- `EventStrategyInput` filters every projected event through the compiled
  registry: only a registered `signal` or `context` event reaches the snapshot;
  an unregistered or `diagnostic` identity is dropped.
- The model-bundle reader (`stages/strategy/bundle.rs`) applies the same filter
  while parsing `events.csv`, so the file-driven strategy path cannot bypass
  the model-run path's boundary.
- `EventTimeframeView` no longer carries `Option<ContextBar>`, and the
  model-bundle reader no longer parses the raw `atr_ratio`, `rvol_ratio`, or
  `context_code` columns into a `BarRow`. The model's context remains a
  published model reading; it is not part of the strategy snapshot.
- `RunConfig::validate_cross` rejects a strategy execution timeframe that
  differs from `execution.fill_timeframe`.
- The coupled `submit_validated_pipeline` rejects `quantrule.run/v3` with
  `ConfigInvalid`; a v3 config remains valid for a model-only stage.
- Tests: a constructed snapshot test filters a diagnostic and an unregistered
  event and keeps the signal; the app loader test mutates a v4 strategy to 5m
  against a 1m fill and expects the loader error; the evidence route test
  submits a v3 config and expects rejection.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-backtest --lib strategy::` | 44 passed, 0 failed |
| `cargo test -p quantrule-app --lib config::tests` | 27 passed, 0 failed |
| `cargo test -p quantrule-evidence-service --test promoted_runs` | 3 passed, 0 failed |
| `cargo fmt --all -- --check` | pass |
| `tools/gate.sh` | pass: 2,324 passed, 0 failed, 16 ignored; 25 checkers |

## Quant validation (sequential, not independent)

- **Expected invariant:** a strategy may read only registered `signal`/`context`
  events and states, confirmed levels, bar identity, and the as-of context; a
  diagnostic never enters a decision (`AGENTS.md` §2-3). The loader enforces the
  documented v4 strategy/fill check (`docs/config/2026-11-13/run-v4-strategy.md`).
- **Observed before:** `EventStrategyInput` carried every event in the window and
  exposed `ContextBar`; the boundary test scanned source strings in `input.rs`
  only. The loader checked the strategy timeframes against the data ladder but
  not against the fill timeframe, and accepted v3 on the coupled route.
- **Observed after:** the snapshot test gives a diagnostic event
  (`signal.co_movement_cross`), an unregistered identity, and a signal event;
  only the signal survives. A 5m strategy against a 1m fill is rejected at
  `RunConfig::load`; a v3 bundle is rejected by `run submit`.
- **Default output:** a strategy that used only registered signal/context events
  sees the same events; a diagnostic event was already rejected by the condition
  builder, so the new filter removes a value the engine could not use. No model
  value or accepted strategy baseline changes.
- **Residual risk:** `EventTimeframeView` fields remain public, so a caller that
  hand-builds a view can still put a diagnostic `EventView` in it; the strategy
  condition builder still rejects a diagnostic condition, and the two real
  constructors filter. A future constructor must use the registry filter or the
  boundary test will not catch it. The v3 rejection is a route check, not a
  change to the model-only stage path.
- **Independence:** the Quant Researcher/Developer/Validator roles were
  performed sequentially by the same agent. This is not an independent
  validation and is not claimed as one.

## Learning

- The role check in the condition builder protected a decision but not the
  snapshot. Filtering at the projection makes the boundary a property of the
  input, not only of the rule.
- The file-driven strategy path is a second constructor; applying the same
  registry filter there was necessary for the invariant to hold for both
  clients.
- `ContextBar` is a model diagnostic, not strategy input. Removing it from the
  snapshot is clearer than documenting it as presentation-only.
