# Execution Plan: 5m Compression Bracket Strategy

- Date: 2026-11-04 (UTC)
- Status: **Verified — 5m model gate no edge; strategy not run**
- Issue: [#379](https://github.com/quant-rule/quant-rule/issues/379)
- Prior 1m bracket gate: [`STRATEGY-BRACKET-GATE`](../../improvement/2026-11-04/STRATEGY-BRACKET-GATE.md) — no edge
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-04/379-compression-bracket-5m.md`](../../ai-transparency-log/2026-11-04/379-compression-bracket-5m.md)
- Claim type: model validation evidence and strategy evidence after costs; no
  portfolio or profitability claim.

## 1. Intent

Test whether the compression-range bracket becomes cost-viable on 5m, where one
ATR is about five times larger relative to the fixed commission and the trade
count is lower. The 1m bracket failed `strategy_dev` because the 4 bps commission
was about 5.5× the gross edge; this plan changes the timeframe, not the rule.

## 2. Current state

- The 1m compression-range event is an accepted model baseline.
- The 1m bracket failed `strategy_dev`: 15/15 folds halted, pooled PF 0.592,
  aggregate gross P&L +$2,887 versus $15,753 commission.
- The v10 bracket strategy, stop-entry/OCO execution, and 5m model support are
  implemented and merged.
- The 5m range event is already computed by the model bundle; the gate harness
  needs a `--timeframe` selector.
- The strategy configs and matrix are 1m-specific; 5m configs are not
  pre-registered yet.

## 3. Scope

Included:

- A pre-registered 5m compression-range next-bar gate.
- A 5m strategy config and matrix that reuse the v10 rule unchanged.
- `strategy_dev` 2018–2022 and one-shot 2024 validation only if development
  passes.
- A third-use disclosure for `model_validation` 2021–2022.

Excluded:

- Changes to the range event definition, the bracket rule, the costs, or the
  target/stop.
- 1m threshold tuning.
- Portfolio work.
- 4h and higher timeframes.

## 4. Assumptions and uncertainty

- 5m ATR is about five times 1m ATR, so the fixed commission is a smaller
  fraction of one risk unit.
- 5m has about one fifth of the bars, so the trade count and total commission
  are lower.
- The 5m range event may be less predictive than 1m; the model gate decides.
- The 5m bracket may still fail if the 4 ATR target rarely fills before the
  1 ATR stop.
- `model_validation` 2021–2022 has already been used twice; the third use is
  disclosed and raises residual snooping risk.

## 5. Decisions

| Decision | Choice |
|---|---|
| Event | `signal.compression_range`, type 3/subtype 8/source 5, direction 0 |
| Event timeframe | 5m |
| Coiling window | 5 bars |
| Outcome | next 5m bar touches `range_high` or `range_low` |
| Baseline | matched non-coiling 5-bar window completion |
| Model dev | `model_dev` 2018–2020 |
| Model validation | `model_validation` 2021–2022, third use disclosed |
| Strategy ladder | 5m execution, 15m context |
| Entry | two stop entries at the range boundaries, next 5m bar only |
| OCO | first fill cancels the other leg |
| Ambiguity | both boundaries touched with the open inside → no entry |
| Stop | 1 ATR from the fill |
| Target | 4 ATR from the fill |
| Time stop | 20 bars (100 minutes) |
| Fee | 4 bps per side |
| Slippage | 2 bps |
| Strategy dev | `strategy_dev` 2018–2022, one-year folds |
| Strategy validation | `strategy_validation` 2024, one-shot |
| Green | average return > 0; no fold halts; ≥30 trades; pooled PF > 1; positive on ≥2 of 3 |
| Portfolio | None until a strategy baseline is accepted |

## 6. Work breakdown

| Phase | Work | Paths | Role |
|---|---|---|---|
| 0 | Plan, protocol, contract, AI log | `docs/execution-plans/`, `docs/model/`, `docs/config/`, `docs/ai-transparency-log/` | Orchestrator |
| 1 | 5m gate selector and matrix | `tools/improvement/range_validation.py`, `config/compression-range-5m/` | Quant Developer |
| 2 | 5m model bundles and gate | `data/compression-range/` | Quant Developer |
| 3 | 5m strategy configs and matrix | `config/compression-bracket-5m/`, `tools/improvement/build_bracket_matrix.py` | Quant Developer |
| 4 | Strategy run records | `data/improvement/`, `docs/improvement/` | Result Analyst / Validator |

## 7. Verification

- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test
  --workspace`, `cargo doc --workspace --no-deps`.
- Range harness tests for timeframe selection and the 5m gate.
- 5m matrix determinism and a byte-identical rerun of one fold.
- Strategy explain output for the 5m config.
- Registry, validation-claim, link, and term checks.

## 8. Risks and recovery

- The 5m gate may fail or be inconclusive; record it and stop.
- The 5m strategy may still be cost-dominated; record no edge and stop.
- The third use of `model_validation` weakens the out-of-sample claim; disclose
  it in the gate and decision.
- The 5m bars are derived from the same 1m source; gaps and boundary alignment
  follow the data engine.

## 9. Stop conditions

- The 5m model gate fails or is inconclusive: record the result; do not run the
  strategy.
- Development fails: record no edge; do not open 2024 validation.
- Validation fails: record development-only; stop.
- A required decision outside this plan: stop and ask.

## 10. Result

The 5m model gate failed on the pre-registered back-lag control. The cell had
84,627 events, a +8.13pp lift, a +7.75pp corrected lower bound, random p 0.005,
and 3/3 replication, but the one-bar-lagged rule already captured +3.33pp,
leaving an incremental +4.80pp, 0.20pp below the +5pp gate.

The validation partition was not opened, the 5m strategy was not run, and no 5m
baseline was accepted. The accepted 1m range baseline and the 1m no-edge result
are unchanged.
