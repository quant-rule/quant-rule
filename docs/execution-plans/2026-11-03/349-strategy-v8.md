# Execution Plan: Strategy v8 Event Composite

- Date: 2026-11-03 (UTC)
- Status: **Approved by the human's instruction to complete the model and strategy**
- Issue: [#349](https://github.com/quant-rule/quant-rule/issues/349)
- Epic: [#342](https://github.com/quant-rule/quant-rule/issues/342)
- Contract: removed in #463; this plan is the durable record
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-03/349-strategy-v8.md`](../../ai-transparency-log/2026-11-03/349-strategy-v8.md)

## 1. Intent

Implement the flat event-composite strategy. It reads the model's published
events, v3 discrete states, confirmed levels, bar identity, and its own position.
It does not read axes, raw indicators, statistics, cash, or P&L.

## 2. Scope

- Add the v8 bounded condition language and parser.
- Add the event/state snapshot strategy input.
- Add the v8 strategy decision path and runner integration.
- Add `policy check` and `strategy explain` support.
- Add causality, determinism, parser, boundary, and runner tests.
- Keep v3–v7 readable as history.

## 3. Non-goals

- No profitability claim.
- No accepted strategy baseline.
- No axis removal.
- No portfolio change.

## 4. Work breakdown

| Step | Work | Paths |
|---|---|---|
| 1 | Add v3 states to `ModelBar` | `crates/quantrule-model/src/timeframe.rs` |
| 2 | Add event/state snapshot input | `crates/quantrule-backtest/src/strategy/event_input.rs` |
| 3 | Add bounded conditions and strategy | `crates/quantrule-backtest/src/strategy/event.rs` |
| 4 | Add runner branch | `crates/quantrule-backtest/src/runner/mod.rs` |
| 5 | Add shell parser and explain | `crates/quantrule-shell/src/config/strategy.rs`, `command/strategy_explain.rs` |
| 6 | Add verification configs and docs | `config/verification/`, `docs/config/2026-11-03/` |

## 5. Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo doc --workspace --no-deps`
- `python3 tools/check_registry.py`
- `python3 tools/check_validation.py`
- `python3 tools/check_links.py`
- `python3 tools/check_terms.py`
- Run `policy check`, `strategy explain`, and a full `backtest` on the
  verification config.

## 6. Acceptance criteria

- A v8 config parses and rejects conflicting entry directions.
- The strategy input exposes only event/state/level/bar/position data.
- The runner reaches the v8 decision path.
- `policy check` and `strategy explain` print the v8 rule.
- A full backtest runs end to end.
- All checks pass.
