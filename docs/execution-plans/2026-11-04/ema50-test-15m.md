# Execution Plan: EMA50 Test 15m

- Date: 2026-11-04 (UTC)
- Status: **Verified — no edge; stopped at the model development gate**
- Issue: none; the human brief in this session is the plan of record
- Branch: `feat/ema50-test-15m`
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-04/ema50-test-15m.md`](../../ai-transparency-log/2026-11-04/ema50-test-15m.md)
- Claim type: pre-registered strategy-hypothesis test; no validation and no edge
  claim.

## 1. Intent

Test one 15m event family — the pure EMA50 level test — for a cost-aware
directional edge on top of the accepted v4 model. The test is a falsification
attempt, not a tuning exercise. A failed gate records "no edge" and stops the
brief.

The event is a **level** event (type 1), **test** subtype (1), **EMA50** source
(2), on 15m. It is published on the bar it occurs, and break/cross precedence on
the same reference removes break and cross bars from the test family.

## 2. Current state

| Item | Observed |
|---|---|
| Phase 0 merge | `time_stop_bars` present in `strategy/event.rs`; `state_applicable` present in `path_validation.py`; dev model pack present |
| Model pack | `data/model-v4/dev/**` and `data/model-v4/validation/**`, 9 and 6 cells, 1m/5m/15m/1h readings and events |
| Model baseline | `MODEL-2026-11-04-04..05` (1m 4 ATR compression breakout), accepted for range expansion, not direction |
| Strategy baseline | none; `STRATEGY-2026-11-04-01` recorded no edge |
| Event volume | 2,729–4,768 EMA50 test events per 15m bundle on `dev` adx25; ample for the event gate |
| Strategy schema | `quantrule.strategy/v8`, `event-composite`, supports `time_stop_bars` and `direction: 0` |
| Execution/portfolio | `quantrule.execution/v2`, `quantrule.portfolio/v1`; market-next-open, 2 bps slippage, 4 bps commission per operation |

## 3. Scope

Included:

- `config/ema50-test-15m/**` strategy, execution, portfolio, model-gate families,
  and generated per-fold run configs.
- `tools/improvement/build_ema50-test-15m_matrix.py`, a copy of the v9 builder
  with only paths and names changed.
- Model gate dev (`MODEL-2026-11-04-14`) and, if it passes, model validation
  (`MODEL-2026-11-04-19`).
- Strategy dev (`STRATEGY-2026-11-04-05`) and, if green, strategy validation 2024
  (`STRATEGY-2026-11-04-10`).
- The candidate document and run records.

Excluded:

- Any Rust change; any model, execution, portfolio, policy, or analytics change.
- Any timeframe above 1h; any compression variant.
- Any change to the event, target, stop, horizon, filters, or costs after a
  result is seen.
- Any tuning loop, extra cell, or second hypothesis.
- Pushes, pull requests, merges, and deletions.

## 4. Hypothesis

A pure test of EMA50 — the candle range intersects the EMA50 ±0.2 ATR band
without a break or cross taking precedence — that closes on one side marks a
dynamic support/resistance hold. The next 20 15m bars reach +2 ATR in the close
direction before −1 ATR more often than the EMA50-side base rate.

## 5. Pre-registered configuration

| Item | Value |
|---|---|
| Entry | one event condition: 15m, type 1, subtype 1, source 2, direction 0 (either side) |
| Ladder | `15m`, `1h` |
| Window | 1 bar |
| Time stop | 20 bars |
| Stop | 1.0 ATR |
| Target | 2.0 R (2.0 ATR) |
| Execution | market next open, stop first on ties, 2 bps slippage, zero-volume reject, end-of-data leave open |
| Portfolio | 10,000 USD, fixed notional 10,000, 1x, one position, shorts allowed, 4 bps commission per operation, no funding, 10% drawdown halt |

## 6. Gates and acceptance criteria

| Gate | Pass rule |
|---|---|
| Model dev | `selection` not null, config `adx25`, grade A or B |
| Model validation | `selection` not null and grade A or B, one-shot |
| Strategy dev 2018–2022 | `passed: true` and every criterion true |
| Strategy validation 2024 | `passed: true` and every criterion true, one-shot |

Green criteria: average instrument return > 0 after costs; worst fold drawdown
≤ 0.10; total trades ≥ 30; pooled profit factor > 1.0; positive instruments ≥ 2
of 3; no fold halts. Costs: 4 bps commission per operation, 2 bps slippage, no
funding.

Additional stop conditions: Phase 0 missing; model pack missing; valid event
count < 200 or clusters < 20; grade C or no selection; any gate failure; any test
or determinism failure; any need to change the event, target, stop, horizon,
timeframe, filter, or cost; any need for a Rust change; any conflict with
another session.

## 7. Work breakdown

| Phase | Work | Paths | Role |
|---|---|---|---|
| 1 | Plan, config, builder | `docs/execution-plans/`, `config/ema50-test-15m/`, `tools/improvement/build_ema50-test-15m_matrix.py` | Quant Developer |
| 2 | Model dev gate | `data/improvement/2026-11-04/MODEL-2026-11-04-14/` | Quant Developer, Test Engineer |
| 3 | Model validation gate | `data/improvement/2026-11-04/MODEL-2026-11-04-19/` | Quant Developer, Test Engineer |
| 4 | Strategy dev 2018–2022 | `config/ema50-test-15m/strategy/dev/`, `data/ema50-test-15m/dev/`, `data/improvement/2026-11-04/STRATEGY-2026-11-04-05/` | Quant Developer |
| 5 | Strategy validation 2024 | `config/ema50-test-15m/strategy/validation/`, `data/ema50-test-15m/validation/`, `data/improvement/2026-11-04/STRATEGY-2026-11-04-10/` | Quant Developer |
| 6 | Candidate document, log, commit | `docs/improvement/2026-11-04/`, `docs/ai-transparency-log/2026-11-04/` | Quant Developer |

## 8. Verification

| Check | Command | Expected |
|---|---|---|
| Phase 0 | the three preflight probes | all present |
| Model dev | `path_validation.py` on the dev matrix, both ADX settings | selection with `adx25`, grade A/B |
| Model validation | `path_validation.py` on the validation matrix, `--config adx25`, one-shot | selection, grade A/B |
| Strategy dev | 15 backtests, then `strategy_v9_evaluate.py` | `passed: true` |
| Strategy validation | 3 backtests, then the evaluator | `passed: true` |
| Determinism | rerun one strategy cell and compare artifact digests | byte-identical |

Limitations: one-year folds reset the account; the 15m test family publishes on
every touching bar, so consecutive events are correlated and clusters are the
honest unit; a pass is a candidate, not an edge, and requires an independent
Quant Validator and human acceptance.

## 9. Risks and recovery

- A gate may fail on the first attempt; that is the expected outcome for most
  hypotheses and is recorded as no edge, not retried with different settings.
- The model gate is run without `--config` so both ADX settings are evaluated;
  passing only the primary config would hide a sensitivity failure.
- Strategy state is isolated per fold; the run store is append-only and every run
  ID is unique, so a failed stage leaves earlier evidence intact.
- Recovery from a failed gate is a new hypothesis with a new human decision, not
  a change to this one.

## 10. Stop conditions

Any gate failure, any event-count or cluster shortfall, any test/determinism
failure, or any request to change a pre-registered parameter stops the brief and
records the result. No validation partition is opened after a failed prior gate.

## 11. Outcome

| Gate | Result |
|---|---|
| Phase 0 | Pass |
| Model dev `MODEL-2026-11-04-14` | **Fail**: `selection` null, `adx25` grade C, n 33,318, clusters 2,763, lift −0.0020, incremental −0.0066 |
| Model validation `MODEL-2026-11-04-19` | Not run: dev gate failed |
| Strategy dev `STRATEGY-2026-11-04-05` | Not run |
| Strategy validation `STRATEGY-2026-11-04-10` | Not run |

The development gate failed on `lift`, `lift_lcb`, `incremental_lift`,
`incremental_lcb`, `random`, `back_lag`, and `shuffle`. The event count and
cluster gates passed. A byte-identical rerun reproduced both artifact digests.
Verdict: no edge. The candidate document is
[`docs/improvement/2026-11-04/EMA50-TEST-15M-CANDIDATE.md`](../../improvement/2026-11-04/EMA50-TEST-15M-CANDIDATE.md).

