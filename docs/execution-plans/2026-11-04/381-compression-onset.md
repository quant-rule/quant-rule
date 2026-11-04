# Execution Plan: Fresh-Onset Compression Event

- Date: 2026-11-04 (UTC)
- Status: **Verified — model baseline accepted; 5m onset strategy no edge; validation not opened**
- Issue: [#381](https://github.com/quant-rule/quant-rule/issues/381)
- Prior 5m gate: [`COMPRESSION-RANGE-5M-GATE`](../../improvement/2026-11-04/COMPRESSION-RANGE-5M-GATE.md) — back-lag fail
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-04/381-compression-onset.md`](../../ai-transparency-log/2026-11-04/381-compression-onset.md)
- Claim type: model validation evidence and strategy evidence after costs; no
  portfolio or profitability claim.

## 1. Intent

Test whether selecting only the first completed coiling window after a
non-coiling bar removes the rolling-window back-lag that failed the 5m range
gate. The 5m rolling event had a strong +8.13pp lift but a +4.80pp incremental
edge over the one-bar lag, 0.20pp below the +5pp gate.

## 2. Current state

- The 1m range event is an accepted model baseline; the 1m bracket is no edge.
- The 5m rolling range gate failed on the back-lag control.
- The model event, bracket execution, and v10 strategy are implemented.
- The onset event and 5m gate configs are not implemented yet.

## 3. Scope

Included:

- A neutral `signal.compression_range_onset` event (type 3, subtype 9) that
  fires only at the first completed coiling window after a known non-coiling
  bar.
- A pre-registered 5m onset next-bar gate.
- A pre-registered 5m onset bracket strategy, run only if the model gate passes.
- 5m strategy development and one-shot validation if development passes.

Excluded:

- Changes to the rolling range event, the bracket rule, costs, or risk.
- 1m threshold tuning.
- Portfolio work.
- 4h and higher timeframes.

## 4. Assumptions and uncertainty

- The onset selection removes the overlap; the back-lag difference should clear
  +5pp if the onset timing carries the edge.
- The onset sample is smaller than the rolling sample; it must still clear the
  sample and cluster thresholds.
- The 5m bracket may still be cost-dominated.
- `model_validation` 2021–2022 is a third use; disclose it.

## 5. Decisions

| Decision | Choice |
|---|---|
| Event | `signal.compression_range_onset`, type 3/subtype 9/source 5 |
| Timeframe | 5m |
| Coiling window | 5 bars |
| Onset condition | window all-coiling, prior bar known non-coiling |
| Outcome | next 5m bar touches `range_high` or `range_low` |
| Baseline | matched non-coiling 5-bar completion |
| Model dev | `model_dev` 2018–2020 |
| Model validation | `model_validation` 2021–2022, third use disclosed |
| Strategy ladder | 5m execution, 15m context |
| Entry | two stop entries at the range boundaries, next 5m bar only |
| OCO | first fill cancels the other leg |
| Ambiguity | both boundaries touched with the open inside → no entry |
| Stop | 1 ATR from the fill |
| Target | 4 ATR from the fill |
| Time stop | 20 bars |
| Fee | 4 bps per side |
| Slippage | 2 bps |
| Strategy dev | `strategy_dev` 2018–2022, one-year folds |
| Strategy validation | `strategy_validation` 2024, one-shot |
| Green | average return > 0; no fold halts; ≥30 trades; pooled PF > 1; positive on ≥2 of 3 |

## 6. Work breakdown

| Phase | Work | Paths | Role |
|---|---|---|---|
| 0 | Plan, protocol, contract, AI log | `docs/execution-plans/`, `docs/model/`, `docs/config/`, `docs/ai-transparency-log/` | Orchestrator |
| 1 | Onset event, registry, config, tests | `crates/quantrule-model/`, `crates/quantrule-shell/src/config/` | Quant Developer |
| 2 | Onset gate harness and configs | `tools/improvement/`, `config/compression-range-onset*/` | Quant Developer |
| 3 | 5m model bundles and gate | `data/compression-range-onset/` | Quant Developer |
| 4 | 5m onset strategy configs | `config/compression-bracket-onset-5m/` | Quant Developer |
| 5 | Strategy run records | `data/improvement/`, `docs/improvement/` | Result Analyst / Validator |

## 7. Verification

- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test
  --workspace`, `cargo doc --workspace --no-deps`.
- Onset event: causality, determinism, onset suppression, edge cases.
- 5m gate and strategy explain output.
- Registry, validation-claim, link, and term checks.

## 8. Risks and recovery

- The onset gate may fail or be underpowered; record it and stop.
- The 5m bracket may still be cost-dominated; record no edge and stop.
- The third use of `model_validation` weakens the out-of-sample claim; disclose
  it.

## 9. Stop conditions

- The onset gate fails or is inconclusive: record it; do not run the strategy.
- Development fails: record no edge; do not open 2024 validation.
- Validation fails: record development-only; stop.
- A required decision outside this plan: stop and ask.

## 10. Model gate result

Both partitions pass.

| Partition | n | J | Hit | Matched base | Lift | Lift LCB | Rand p | Back lag | Rep |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `model_dev` | 18,324 | 12,521 | 0.5706 | 0.4760 | +0.0946 | +0.0874 | 0.0050 | +0.0365 | 3 |
| `model_validation` | 14,303 | 9,708 | 0.5373 | 0.4651 | +0.0722 | +0.0640 | 0.0050 | −0.0065 | 3 |

The 5m fresh-onset event is an accepted model baseline (human, 2026-11-04). The
5m onset bracket configs and matrix are pre-registered; the strategy tier
proceeds.

## 11. Strategy result

The 5m onset bracket failed `strategy_dev`. All 15 one-year folds halted at the
10% drawdown limit; compounded returns were BTC −37.47%, ETH −24.97%, XAU
−40.96%; the pooled profit factor was 0.654 and no instrument was positive. The
aggregate gross P&L was −$2,286 against $9,960 of commission, so the 4 ATR
target / 1 ATR stop payoff has no directional edge before costs. The 2024
validation partition was not opened; no strategy baseline was accepted.
