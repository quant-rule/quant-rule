# Execution Plan: 1m Compression-Breakout Strategy

- Date: 2026-11-04 (UTC)
- Status: **Verified — no edge; validation not opened**
- Issue: [#375](https://github.com/quant-rule/quant-rule/issues/375)
- Model baseline: `MODEL-2026-11-04-04..05` (accepted 2026-11-04)
- Strategy contract: [`docs/config/2026-11-04/strategy-v9-compression.md`](../../config/2026-11-04/strategy-v9-compression.md)
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-04/375-compression-strategy.md`](../../ai-transparency-log/2026-11-04/375-compression-strategy.md)
- Claim type: strategy evidence after costs; no portfolio or profitability claim.

## 1. Intent

Test the accepted 1m compression-expansion model as a cost-aware strategy. The
model predicts a 4 ATR forward range; the strategy takes the event direction at
the next open and brackets it with a 1 ATR stop and a 4 ATR target, with a
20-bar time stop.

## 2. Current state

- The model baseline passes `model_dev` and `model_validation` at 1m, 4 ATR over
  20 bars with matched baselines, controls, and replication.
- The model event publishes its breakout direction; the model claim is
  expansion, not direction.
- The v8 event-composite strategy cannot take a dynamic side from one event, so
  a new `compression-breakout` strategy mode is required.
- The existing strategy input exposes events, states, levels, bar identity, and
  context; ATR-based risk is resolved by execution, not by the strategy.

## 3. Scope

Included:

- A `compression-breakout` strategy mode with dynamic side.
- A v9 strategy config with the event, stop, target, and time stop.
- Strategy development on BTCUSD, ETHUSD, XAUUSD, 2018–2022, one-year folds.
- One-shot strategy validation on 2024.
- Cost-aware execution at 1m.

Excluded:

- Model changes beyond the accepted baseline.
- Portfolio work.
- Extra entry/exit filters or threshold tuning.
- 4h and higher timeframes.

## 4. Assumptions and uncertainty

- The model's 4 ATR range claim may not translate into a directional edge; the
  strategy can fail even though the model baseline passed.
- 1m costs are material; the strategy must clear commission and slippage.
- The 2024 validation year is one-shot and may be a different regime.
- The 20-bar time stop is a strategy exit, not part of the model claim.

## 5. Decisions

| Decision | Choice |
|---|---|
| Event | 1m compression breakout, type 3, subtype 7, source 5 |
| Entry | Market at the next 1m open, side = event direction |
| Stop | 1 ATR from the fill |
| Target | 4 ATR (reward-to-risk 4.0) |
| Time stop | 20 bars |
| Fees | 4 bps per side |
| Slippage | 2 bps |
| Development | `strategy_dev` 2018–2022, one-year folds |
| Validation | `strategy_validation` 2024, one-shot |
| Green | Positive after costs, max drawdown ≤10%, ≥30 trades, profit factor >1, stable across instruments |
| Portfolio | None until a strategy baseline is accepted |

## 6. Work breakdown

| Phase | Work | Paths | Role |
|---|---|---|---|
| 0 | Plan, contract, AI log | `docs/execution-plans/`, `docs/config/`, `docs/ai-transparency-log/` | Orchestrator |
| 1 | Strategy mode and rule | `crates/quantrule-backtest/src/strategy/compression.rs`, `config.rs`, `runner/` | Quant Developer |
| 2 | v9 config parser and explain | `crates/quantrule-shell/src/config/strategy.rs`, `command/strategy_explain.rs` | Quant Developer |
| 3 | Tests | `crates/quantrule-backtest/src/strategy/`, `crates/quantrule-shell/src/config/tests.rs` | Test Engineer |
| 4 | Configs and runs | `config/compression/`, `data/improvement/` | Quant Developer |
| 5 | Dev and validation records | `docs/improvement/2026-11-04/` | Result Analyst / Validator |

## 7. Verification

- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test
  --workspace`, `cargo doc --workspace --no-deps`.
- Strategy unit tests: dynamic side, time stop, no event, missing context,
  contiguous guard, determinism.
- Config parser tests and the explain printed output.
- Strategy development metrics and one-shot validation; reproducibility check.
- Registry/validation/link/term checks.

## 8. Risks and recovery

- The strategy may fail after 1m costs; the model baseline remains valid.
- The new strategy mode is a boundary change; the strategy input stays narrow
  and the rule reads only the published event.
- Local backtest artifacts are not committed; the committed record names the
  data versions, configs, costs, and results.

## 9. Stop conditions

- Development fails the green criteria: record no edge; do not open validation.
- Validation fails: development-only; stop.
- A required decision outside this plan: stop and ask.

## 10. Result

The strategy implementation and tests are complete. The `strategy_dev` matrix
(15 one-year folds) failed: every fold halted at the 10% drawdown limit, the
compounded returns were BTC −40.97%, ETH −31.80%, XAU −41.17%, and the pooled
profit factor was 0.481. The 2024 validation partition was not opened; no
strategy baseline was accepted. The model baseline remains accepted.
