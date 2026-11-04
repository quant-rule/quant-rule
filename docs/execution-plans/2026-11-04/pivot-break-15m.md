# Execution Plan: Major Pivot Break 15m

- Date: 2026-11-04 (UTC)
- Status: **Verified — stopped at model_dev; no edge**
- Issue: none assigned; task branch `feat/pivot-break-15m`
- Claim type: model barrier-path evidence first; strategy evidence only if the
  model gates pass. No validation or edge claim.
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-04/pivot-break-15m.md`](../../ai-transparency-log/2026-11-04/pivot-break-15m.md)

## 1. Intent

Test one 15m event family for a cost-aware directional edge on top of the
accepted v4 model. The event is a close beyond a confirmed major swing high or
low by at least 0.2 ATR. The hypothesis is that the next 20 15m bars reach
+2 ATR in the break direction before -1 ATR more often than the unconditional
base rate.

The brief stops at the first failed gate. A failed gate is recorded as
`no edge`; the event, timeframe, target, stop, horizon, filters, and costs are
not changed after seeing any result.

## 2. Current state

- Phase 0 is merged to `main`: the v8 `time_stop_bars` field, the
  `state_applicable` path in `tools/improvement/path_validation.py`, and the
  v4 model pack are present.
- The accepted v4 model vends `events.csv` and `readings.csv` per
  instrument-year for the `adx20` and `adx25` configs.
- The model publishes level events: type 1, subtype 3 (break), source 8 (major
  pivot), 15m. The event is available on the bar it occurs and references a
  confirmed pivot, so it cannot appear before confirmation.
- The v8 event-composite strategy supports a zero-direction event, an ATR stop,
  a reward-to-risk target, and a time stop. No Rust change is expected.

## 3. Scope

Included:

- `config/pivot-break-15m/**`: strategy, execution, portfolio, model families.
- `tools/improvement/build_pivot-break-15m_matrix.py`: the dev/validation
  matrix builder, copied from the v9 builder with the brief's substitutions.
- Model gate `model_dev` (`MODEL-2026-11-04-13`) and, only if dev passes,
  one-shot `model_validation` (`MODEL-2026-11-04-18`).
- Strategy `strategy_dev` (`STRATEGY-2026-11-04-04`) and, only if dev is green,
  one-shot `strategy_validation` 2024 (`STRATEGY-2026-11-04-09`).
- Run records, candidate document, plan, and AI log.

Excluded:

- Model, execution, portfolio, policy, or analytics code and shared config.
- Timeframes above 1h, compression variants, extra filters, and threshold
  tuning.
- Portfolio runs, pushes, pull requests, merges, and cleanup.
- Any claim of validation or an edge. The final candidate needs an independent
  Quant Validator and human acceptance.

## 4. Assumptions and uncertainty

- The pivot-break family may have enough events and clusters on `model_dev`;
  the gate rejects fewer than 200 events or fewer than 20 clusters.
- The event may not beat its matched barrier base, random entries, direction
  shuffles, or the back-lag control. The gate may fail at `model_dev`.
- The model's barrier hit-rate claim may not survive the 15m cost and bracket
  path in a strategy backtest.
- The 2024 partition is one-shot and may be a different regime.
- The event is confirmed at the pivot, but the break bar itself uses only the
  close of that bar; no future data enters the signal path.

## 5. Decisions

| Decision | Choice |
|---|---|
| Event | 15m, type 1, subtype 3, source 8, direction ±1 |
| Entry | Next 15m open, side = event direction |
| Stop | 1 ATR |
| Target | 2 ATR (reward-to-risk 2.0) |
| Horizon | 20 bars |
| Time stop | 20 bars |
| Fee | 4 bps commission per operation, 0 funding |
| Slippage | 2 bps |
| Model configs | `adx20` and `adx25` on dev; `adx25` only on validation |
| Development | `model_dev` 2018–2020; `strategy_dev` 2018–2022, one-year folds |
| Validation | `model_validation` 2021–2022; `strategy_validation` 2024 |
| Instruments | BTCUSD, ETHUSD, XAUUSD |
| Model gate | selection not null, config `adx25`, grade A or B |
| Strategy gate | passed true and every criterion true |

Conditional pre-approvals in the brief: open model validation once if dev
passes; treat a passing model validation as the accepted model baseline for
this brief; open strategy validation 2024 once if strategy dev is green. A
strategy baseline is not pre-accepted.

## 6. Work breakdown

| Phase | Work | Paths | Role |
|---|---|---|---|
| 0 | Plan, configs, matrix builder | `docs/execution-plans/`, `config/pivot-break-15m/`, `tools/improvement/` | Quant Developer |
| 1 | Model dev gate | `data/improvement/2026-11-04/MODEL-2026-11-04-13/` | Quant Developer |
| 2 | Model validation gate (conditional) | `data/improvement/2026-11-04/MODEL-2026-11-04-18/` | Quant Developer |
| 3 | Strategy dev backtests and evaluation | `data/pivot-break-15m/dev/`, `STRATEGY-2026-11-04-04/` | Quant Developer / Result Analyst |
| 4 | Strategy 2024 validation (conditional) | `data/pivot-break-15m/validation/`, `STRATEGY-2026-11-04-09/` | Quant Developer / Result Analyst |
| 5 | Candidate document and commit | `docs/improvement/2026-11-04/PIVOT-BREAK-15M-CANDIDATE.md` | Orchestrator |

## 7. Verification

- Model dev command exactly as pre-registered, without `--config`, so both
  `adx20` and `adx25` are evaluated.
- Model validation command with `--config adx25`, `--primary-cells 1`, and
  `--skip-sensitivity`, once.
- `cargo build --release -p quantrule-shell` before the strategy backtests.
- Fifteen dev folds and three 2024 folds; the evaluator with
  `--max-drawdown 0.10 --min-trades 30 --min-profit-factor 1.0
  --require-no-halt`.
- Run artifacts are hashed into `results.json`; the manifest is written before
  the run and never edited afterwards.
- A failed gate stops the brief; no later gate is opened.

## 8. Risks and recovery

- A model-gate failure is a clean stop, not a model defect. The shared model
  pack is not modified.
- A strategy failure leaves the model baseline intact; it is a strategy
  `no edge`, not a model invalidation.
- Backtest artifacts remain local under `data/`; the committed record names the
  configs, costs, partitions, and observed results.
- The 2024 result is one-shot. A failure is not retried with a changed
  threshold.

## 9. Stop conditions

- Phase 0 marker missing, model pack missing, or data missing.
- Fewer than 200 events or fewer than 20 clusters at the model gate.
- Gate grade C or no selection; validation failure.
- Strategy dev not green; 2024 failure.
- Any test, determinism, or tooling failure.
- Any need to change the event, target, stop, horizon, timeframe, filter, or
  cost; any need for a Rust change; any conflict with another session.

## 10. Result

The model-dev gate failed and the brief stopped there. The pre-registered
command was run without `--config`, so both `adx20` and `adx25` were evaluated:

```text
python3 tools/improvement/path_validation.py \
  --matrix config/model-v4/dev/matrix.json \
  --families config/pivot-break-15m/model/families-dev.json \
  --out data/improvement/2026-11-04/MODEL-2026-11-04-13/artifacts \
  --timeframes 15m --horizon 20 --target-atr 2.0 --stop-atr 1.0 \
  --primary-cells 5 --seed 0
```

Observed (`adx25`; `adx20` identical): 14,650 events, 4,425 clusters, hit rate
0.3164, base rate 0.3089, lift +0.0074, lift LCB −0.0027, random p 0.0299,
back-lag lift +0.0033, shuffle p 0.3582, replication 3, grade C. Failed checks:
`lift`, `lift_lcb`, `shuffle`. The harness selection is null.

A byte-identical rerun with the same seed matched both artifact digests
(`path_validation.json` `1528ffb7…`, `path_validation.md` `fef3f480…`). The
`model_validation`, `strategy_dev`, and `strategy_validation` gates were not
opened. The candidate is recorded as no edge in
[`docs/improvement/2026-11-04/PIVOT-BREAK-15M-CANDIDATE.md`](../../improvement/2026-11-04/PIVOT-BREAK-15M-CANDIDATE.md).
