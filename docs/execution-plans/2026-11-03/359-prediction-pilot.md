# Execution Plan: Single-Timeframe RSI/MACD Prediction Pilot

- Date: 2026-11-03 (UTC)
- Status: **Stopped after the RSI step** — the primary RSI 30/70 reversion event failed development and validation; MACD was not added.
- Issue: [#359](https://github.com/quant-rule/quant-rule/issues/359)
- Prior evidence: [`MODEL-GATE`](../../improvement/2026-11-03/MODEL-GATE.md)
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-03/359-prediction-pilot.md`](../../ai-transparency-log/2026-11-03/359-prediction-pilot.md)
- Claim type: model prediction evidence; no strategy or edge claim

## 1. Intent

The v3 gate found no predictive edge on BTCUSD, ETHUSD, and XAUUSD. This pilot
starts a new, deliberately small prediction study on a fresh asset. The question
is not whether a strategy is profitable; it is whether one indicator event
predicts the next-hour direction better than the unconditional base rate.

The sequence is additive: RSI first, then MACD, then the conjunction. Structure
and progression sources are added only after direction and momentum have a
result. No axis score is aggregated.

## 2. Current state

- The v3 gate already tested RSI and MACD on BTC/ETH/XAU 2018–2022; all cells
  were C except one non-replicated validation divergence.
- The model publishes RSI threshold events, RSI zone states, MACD states, MACD
  line/signal cross, histogram slope flip, and zero cross.
- The claim-validation harness reads `readings.csv` and `events.csv` and grades
  forward outcomes; it can be extended with event filters.
- EURUSD M1 2018–2024 is available and was not used in the gate.

## 3. Scope

Included:

- EURUSD 1h, 2018–2022; development 2018–2020, validation 2021–2022.
- One-year model folds because the M1 window is capped at 600,000 candles.
- RSI 30/70 crossing as the primary event; RSI zone as a secondary state.
- Next-1h return sign as the primary target; horizons 3, 5, and 10 as secondary.
- MACD line/signal cross, then the RSI+MACD conjunction, only after the RSI
  result.
- Model prediction only; no strategy, costs, execution, or portfolio.

Excluded:

- Strategy or portfolio changes.
- Threshold tuning after seeing the result.
- Structure/progression sources in the first step; they require pivots and leg
  data and follow only if direction/momentum shows something.
- Reusing BTC/ETH/XAU 2018–2022 as "new" evidence.

## 4. Decisions

| Decision | Choice |
|---|---|
| Asset | EURUSD |
| Timeframe | 1h |
| Development | 2018-01-01..2020-12-31 |
| Validation | 2021-01-01..2022-12-31 |
| Primary horizon | 1 bar (1 hour) |
| Primary event | RSI crossing 30 or 70 |
| Secondary event | RSI zone (<30 or >70) |
| Target | sign of next-1h return |
| Axis treatment | Source tag, not an aggregated score |
| Strategy | None |

## 5. Acceptance criteria

- Five one-year EURUSD 1h model bundles exist: 2018, 2019, 2020, 2021, 2022.
- The pilot harness filters RSI 30/70 threshold events and RSI zone states.
- The development result reports n, base rate, hit rate, lift, and 95% lower
  bound for the primary event.
- The validation result is reported once, without tuning.
- A decision records whether MACD is added or the pilot stops.
- No strategy is run.

## 6. Work breakdown

| Step | Work | Role | Path |
|---|---|---|---|
| 1 | Plan and configs | Orchestrator, Quant Researcher | `docs/execution-plans/`, `config/prediction-pilot/` |
| 2 | Model bundles | Quant Developer | `data/prediction-pilot/` |
| 3 | RSI prediction harness | Quant Developer, Test | `tools/improvement/prediction_pilot.py` |
| 4 | Development result | Quant Validator sequential pass | `data/prediction-pilot/RSI-DEV/` |
| 5 | Validation result | Quant Validator sequential pass | `data/prediction-pilot/RSI-VAL/` |
| 6 | Decision | Improvement Workflow, human | `docs/improvement/2026-11-03/` |

## 7. Verification

| Check | Command | Expected |
|---|---|---|
| Format | `cargo fmt --all -- --check` | pass |
| Lints | `cargo clippy --all-targets -- -D warnings` | pass |
| Tests | `cargo test --workspace` | pass |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | pass |
| Registry | `python3 tools/check_registry.py` | pass |
| Tooling | `python3 -m unittest discover -s tools/improvement` | pass |
| Links/terms | `python3 tools/check_links.py`, `python3 tools/check_terms.py` | pass |
| Pilot | `prediction_pilot.py` over the five bundles | a dev and a validation result |

## 8. Risks and recovery

- **Small sample.** RSI 30/70 crossings on 1h EURUSD may be too few to detect a
  small edge. Report the confidence bound and call an inconclusive result
  inconclusive; do not claim no edge from an underpowered sample.
- **Data snooping.** The primary event and horizon are fixed before the run. No
  threshold or horizon change after seeing validation.
- **EURUSD is a new regime.** A positive result may not transfer to crypto; it is
  a pilot, not a baseline.
- **M1 window cap.** One-year folds stay within 600,000 candles; the harness
  pools the folds per instrument.

## 9. Stop conditions

- The model bundle misses or corrupts the 1h series: stop and fix the pipeline.
- RSI 30/70 has no development signal: record no edge for the primary event; do
  not add MACD to rescue it.
- Validation contradicts development: stop; do not tune.
- The pilot needs a new indicator, timeframe, or asset outside the approved
  scope: ask the human.
