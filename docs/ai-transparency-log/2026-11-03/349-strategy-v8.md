# AI Transparency Log — Strategy v8 Event Composite

- Date: 2026-11-03 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Plan: [`docs/execution-plans/2026-11-03/349-strategy-v8.md`](../../execution-plans/2026-11-03/349-strategy-v8.md)

## Intent and boundary

The human asked to continue until the model and strategy are fully implemented.
The strategy work is a code and test implementation, not a profitability claim.
The model baseline is still not accepted, so no strategy result is a trading
claim.

## Implemented

- Added v3 states to `ModelBar`.
- Added the v8 event/state snapshot input.
- Added bounded event/state conditions, entry conjunction, exit disjunction,
  direction validation, and the ATR/reward-risk intent.
- Added runner integration, config parsing, `policy check`, and `strategy
  explain`.
- Added verification configs for a full backtest.

## Verification observed

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --all-targets -- -D warnings`: pass.
- `cargo test --workspace`: pass.
- `python3 tools/check_registry.py`: 10 indicators, 16 events, 55 parameters.
- `python3 tools/check_validation.py`: 25 claims.
- A v8 BTCUSD 2018-01 backtest ran end to end; the DI-cross configuration
  produced 7 executed trades and a negative return. This is a harness result,
  not an edge claim.

## Learning

The v8 input deliberately excludes axes and raw indicators. The first condition
attempt used a higher-timeframe trend-continuation event that did not fire in
the January sample; the verification config was changed to a 15m DI cross to
exercise the path. No threshold was selected from the result.
