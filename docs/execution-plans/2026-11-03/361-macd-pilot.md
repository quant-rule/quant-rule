# Execution Plan: XAGUSD MACD-Only Prediction Pilot

- Date: 2026-11-03 (UTC)
- Status: **Stopped after the MACD step** — the primary line/signal cross failed development and validation; the conjunction was not run.
- Issue: [#361](https://github.com/quant-rule/quant-rule/issues/361)
- Prior step: [`PREDICTION-PILOT-RSI`](../../improvement/2026-11-03/PREDICTION-PILOT-RSI.md)
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-03/361-macd-pilot.md`](../../ai-transparency-log/2026-11-03/361-macd-pilot.md)
- Claim type: model prediction evidence; no strategy or edge claim

## 1. Intent

The RSI 30/70 reversion step failed on EURUSD. This step tests MACD alone on a
fresh asset, so the result is not a rescue of the RSI event and not another look
at the gate's BTC/ETH/XAU data.

The question is prediction-only: after a MACD line/signal cross on XAGUSD 1h,
is the next-hour return more often in the cross direction than the base rate?

## 2. Scope

Included:

- XAGUSD 1h, 2018–2022; development 2018–2020, validation 2021–2022.
- Primary: MACD line/signal cross, next-1h return sign.
- Secondary: MACD zero cross and histogram slope flip.
- Five one-year folds, pooled per partition.
- No strategy, costs, execution, or portfolio.

Excluded:

- RSI, the RSI+MACD conjunction, and any threshold or horizon tuning.
- BTC/ETH/XAU or EURUSD re-use.
- Structure/progression sources.
- Strategy or portfolio changes.

## 3. Decisions

| Decision | Choice |
|---|---|
| Asset | XAGUSD |
| Timeframe | 1h |
| Development | 2018-01-01..2020-12-31 |
| Validation | 2021-01-01..2022-12-31 |
| Primary event | MACD line/signal cross |
| Primary horizon | 1 bar (1 hour) |
| Target | sign of next-1h return |
| Secondary events | MACD zero cross, histogram slope flip |
| Strategy | None |

## 4. Acceptance criteria

- Five XAGUSD 1h model bundles exist (2018–2022).
- The harness filters MACD events by type/subtype/source.
- Development and validation report n, base rate, hit rate, lift, and LCB for the
  primary event.
- The secondary events are reported but not promoted.
- A decision records stop or the RSI+MACD conjunction.

## 5. Work breakdown

| Step | Work | Role | Path |
|---|---|---|---|
| 1 | Plan and configs | Orchestrator, Quant Researcher | `docs/execution-plans/`, `config/macd-pilot/` |
| 2 | Model bundles | Quant Developer | `data/macd-pilot/` |
| 3 | MACD harness and tests | Quant Developer, Test | `tools/improvement/macd_pilot.py` |
| 4 | Development result | Quant Validator sequential pass | `data/macd-pilot/MACD/` |
| 5 | Validation result | Quant Validator sequential pass | same |
| 6 | Decision | Improvement Workflow, human | `docs/improvement/2026-11-03/` |

## 6. Verification

- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `cargo doc`, registry, tooling tests, links, and terms.
- Primary n >= 100 in development; report LCB and lift.
- Validation is one-shot and not used for tuning.

## 7. Risks

- XAGUSD 1h MACD crosses may be fewer than RSI crossings; report the bound and
  call an underpowered result inconclusive.
- A positive result is a pilot on one asset, not a model baseline.
- The validation partition is one-shot for this hypothesis.

## 8. Stop conditions

- Development primary fails: stop; do not run the conjunction.
- Validation contradicts development: stop; do not tune.
- The model bundle misses the 1h series: stop and fix the pipeline.
