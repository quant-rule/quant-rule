# Execution Plan: 1m Compression Bracket Strategy

- Date: 2026-11-04 (UTC)
- Status: **Verified — model baseline accepted; 1m strategy no edge; validation not opened**
- Issue: [#377](https://github.com/quant-rule/quant-rule/issues/377)
- Accepted model baseline: `MODEL-2026-11-04-04..05` (1m, 4 ATR, 20 bars)
- Prior strategy gate: [`STRATEGY-V9-GATE`](../../improvement/2026-11-04/STRATEGY-V9-GATE.md) — no edge
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-04/377-compression-bracket.md`](../../ai-transparency-log/2026-11-04/377-compression-bracket.md)
- Claim type: model validation evidence and strategy evidence after costs; no
  portfolio or profitability claim.

## 1. Intent

Test whether the accepted 1m compression-expansion baseline can be traded without
predicting the breakout direction. The model publishes both compression
boundaries before the breakout; execution places a two-sided stop-entry bracket
at those boundaries with OCO cancellation; the first fill is bracketed by a
1 ATR stop, a 4 ATR target, and a 20-bar time stop.

The directional v9 strategy failed `strategy_dev` on every fold. This plan
replaces the directional entry, not the model. The model's claim remains
expansion, not direction.

## 2. Current state

- The 1m 4 ATR compression-breakout event passed `model_dev` and
  `model_validation`; it is the accepted model baseline.
- The current compression event fires after the breakout and publishes only the
  broken boundary. A bracket needs both boundaries before the side is known.
- Execution supports market-next-open entries, a position stop/target bracket,
  and stop-loss exits. It does not support two-sided stop entries or OCO.
- The strategy snapshot drops event values, so a bracket cannot read the range
  high and low.
- The v9 strategy mode takes a dynamic side from the breakout event; it has no
  bracket entry.

## 3. Scope

Included:

- A neutral `signal.compression_range` model event (type 3, subtype 8, source 5)
  that publishes the compression range high and low at the last coiling bar.
- A pre-registered `compression_range` model gate on 1m.
- Two-sided stop-entry orders with first-fill OCO cancellation, expiry, and a
  deterministic same-bar ambiguity rule.
- A `compression-bracket` strategy mode and `quantrule.strategy/v10` config.
- Strategy development on BTCUSD, ETHUSD, XAUUSD, 2018–2022, one-year folds.
- One-shot 2024 strategy validation only if development passes.

Excluded:

- Changes to the accepted breakout event or its thresholds.
- Direction prediction, filters, or post-result threshold tuning.
- Portfolio work or leverage.
- 4h and higher timeframes.

## 4. Assumptions and uncertainty

- A completed coiling range is more likely than a matched non-coiling range to be
  touched on the next bar; the model gate tests this.
- A boundary touch may be a false breakout; the 1 ATR stop bounds it.
- The 4 ATR target may rarely fill before the stop; the strategy can fail after
  costs even if the model event passes.
- The 1m costs are material.
- `model_validation` 2021–2022 was already used for the accepted breakout
  baseline. Reusing it for the range event is a second use of the same partition
  and is disclosed in the protocol and decision record. The event and outcome
  are different, but the residual snooping risk is higher.

## 5. Decisions

| Decision | Choice |
|---|---|
| Range event | type 3, subtype 8, source 5, direction 0 |
| Range values | `primary_value = range_high`, `reference_value = range_low` |
| Event bar | last bar of the `K = 5` coiling window |
| Entry | two stop entries: buy stop at range high, sell stop at range low |
| Validity | next bar only (`valid_for_bars = 1`) |
| OCO | first fill cancels the other order |
| Same-bar ambiguity | if both boundaries are touched and no gap resolves one side, neither fills |
| Gap | a stop-entry fills at the open when the open is beyond its trigger |
| Stop | 1 ATR from the fill |
| Target | 4 ATR from the fill (reward-to-risk 4.0) |
| Time stop | 20 bars |
| Fee | 4 bps per side |
| Slippage | 2 bps |
| Development | `strategy_dev` 2018–2022, one-year folds |
| Validation | `strategy_validation` 2024, one-shot |
| Green | average instrument return > 0; no fold halts; ≥30 trades; pooled PF > 1; positive on ≥2 of 3 instruments |
| Portfolio | None until a strategy baseline is accepted |

## 6. Model gate

The event claim is: a completed coiling window is followed by a next-bar touch of
at least one published boundary more often than a matched non-coiling window
completion. The outcome is direction-free:

```text
touch = high[b+1] >= range_high or low[b+1] <= range_low
```

The matched baseline is every 5-bar window completion that is not all-coiling.
Controls are random non-event completions and a one-bar back-lag. The full
protocol is [`docs/model/2026-11-04/compression-range.md`](../../model/2026-11-04/compression-range.md).

## 7. Work breakdown

| Phase | Work | Paths | Role |
|---|---|---|---|
| 0 | Plan, protocols, AI log | `docs/execution-plans/`, `docs/model/`, `docs/config/`, `docs/ai-transparency-log/` | Orchestrator |
| 1 | Range event and registry | `crates/quantrule-model/src/events/compression.rs`, `input.rs`, `timeframe.rs`, `docs/registry/` | Quant Developer |
| 2 | Event values in the strategy snapshot | `crates/quantrule-backtest/src/strategy/event_input.rs`, `runner/` | Quant Developer |
| 3 | Stop-entry/OCO execution | `crates/quantrule-backtest/src/execution/` | Quant Developer |
| 4 | Bracket strategy and config | `crates/quantrule-backtest/src/strategy/`, `crates/quantrule-shell/src/config/` | Quant Developer |
| 5 | Tests | model, execution, strategy, shell config | Test Engineer |
| 6 | Model gate | `tools/improvement/`, `config/compression-range/` | Quant Developer |
| 7 | Strategy development and validation | `config/compression-bracket/`, `data/improvement/` | Result Analyst / Validator |

## 8. Verification

- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test
  --workspace`, `cargo doc --workspace --no-deps`.
- Range event: golden, causality, determinism, empty/single/short/gap/zero-volume
  and equal-boundary tests.
- Execution: stop-entry fill at the trigger and at a gap, OCO cancellation,
  expiry, same-bar ambiguity, zero volume, non-contiguous bar, end of data,
  position cap, and determinism.
- Strategy: bracket decision from a range event, no event, ambiguous/absent
  values, time stop, contiguous guard, determinism.
- Shell: v10 config parser, explain output, and command rendering.
- Model gate and strategy matrix, with a byte-identical rerun.

## 9. Risks and recovery

- The range event may pass the gate while the bracket strategy fails after
  costs; that is a strategy-tier failure, not a model invalidation.
- The 2021–2022 model validation is a second use; the record discloses it.
- The OCO ambiguity rule is conservative and may reduce fills; it is fixed
  before the run.
- The new execution path is isolated from market orders; existing behavior is
  covered by regression tests.

## 10. Stop conditions

- The range-event gate fails or is inconclusive: record the result; do not run
  the strategy.
- Development fails: record no edge; do not open 2024 validation.
- Validation fails: record development-only; stop.
- A required decision outside this plan: stop and ask.

## 11. Model gate result

Both partitions pass every pre-registered range-event criterion.

| Partition | n | J | Hit | Matched base | Lift | Lift LCB | Rand p | Back lag | Rep |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `model_dev` | 346,120 | 60,605 | 0.6474 | 0.5442 | +0.1032 | +0.1011 | 0.0050 | +0.0531 | 3 |
| `model_validation` | 247,989 | 43,937 | 0.5802 | 0.4984 | +0.0818 | +0.0795 | 0.0050 | +0.0291 | 3 |

The range event is an accepted model baseline (human, 2026-11-04). The
strategy matrix, configs, and evaluator are implemented and tested; the
strategy tier proceeds.

## 12. Strategy result

The 1m bracket failed `strategy_dev`. All 15 one-year folds halted at the 10%
drawdown limit; compounded returns were BTC −41.23%, ETH −26.03%, XAU −41.18%;
the pooled profit factor was 0.592 and no instrument was positive. The aggregate
gross P&L was +$2,887 against $15,753 of commission, so the 4 bps cost was about
5.5× the gross edge. The 2024 validation partition was not opened; no strategy
baseline was accepted.

The model baseline remains accepted. The next structural hypothesis is the same
bracket rule on 5m, which needs a new 5m range gate and a new pre-registration.
