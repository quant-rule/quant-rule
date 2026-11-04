# AI Transparency Log — 1m Compression-Breakout Strategy

- Date: 2026-11-04 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Plan: [`docs/execution-plans/2026-11-04/375-compression-strategy.md`](../../execution-plans/2026-11-04/375-compression-strategy.md)
- Issue: [#375](https://github.com/quant-rule/quant-rule/issues/375)

## Intent and boundary

Implement the human-approved strategy on the accepted 1m 4 ATR compression
baseline: next-open entry in the event direction, 1 ATR stop, 4 ATR target,
20-bar time stop, 4 bps fee, 2 bps slippage. The work is strategy-only; the
model, policy, execution, and analytics contracts are frozen except for the new
strategy config and the execution timeframe.

## Design decisions

- A new `compression-breakout` strategy mode is required because the v8 event
  composite requires a fixed entry direction; the compression event supplies
  the side.
- The strategy reads only the published event snapshot and the position given
  by the runner. Execution resolves the ATR stop and reward-to-risk target.
- The time stop is a strategy exit based on `bars_held`, not an execution
  feature.
- The config schema is `quantrule.strategy/v9`; v8 remains readable.
- The 1m execution config is separate from the 15m model-gate config so costs
  and fill timeframe are explicit.

## Process note

The model baseline was accepted before this strategy work began. The strategy
rule and costs were approved by the human in the prior step. No threshold was
selected from a backtest; the development matrix is the first run of the rule.

## Result

The directional strategy failed `strategy_dev`. All 15 one-year folds halted at
the 10% drawdown limit. Compounded returns: BTC −40.97%, ETH −31.80%, XAU
−41.17%; pooled profit factor 0.481; positive instruments 0/3. The only
positive fold (ETH 2018, +1.23%) still halted at 10.13%. The 2024 validation
partition was not opened; no strategy baseline was accepted.

## Verification

- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test
  --workspace`, and `cargo doc --workspace --no-deps` passed.
- The strategy unit tests, v9 config parser test, and evaluator tests passed;
  the improvement tooling test suite passed.
- The 15 backtests were run with 4 bps commission, 2 bps slippage, no funding.
- Registry, validation, link, and term checks passed.

## Learning

- The model's expansion claim is direction-neutral. Taking the event direction
  did not preserve the validated range edge after 1m costs.
- The failure is not a threshold miss: every instrument and every fold halted.
  A directional variant would be post-hoc tuning.
- The next hypothesis is a bracket / stop-entry volatility strategy, which
  needs a new execution feature and a new pre-registration.
