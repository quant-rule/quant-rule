# M2.x Model Programme Charter

- Date: 2026-11-05 (UTC)
- Status: **Approved** (human, 2026-11-05)
- Plan: [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](../../execution-plans/2026-11-05/410-m2-model-programme.md)
- Program: [`config/research/PROGRAM-2026-11-05-03/program.json`](../../../config/research/PROGRAM-2026-11-05-03/program.json)
- Coverage: `python3 tools/research/coverage.py --program config/research/PROGRAM-2026-11-05-03/program.json`
- Claim type: pre-registered model programme; no edge claim

## 1. Premise

An M2.x model version that combines OHLCV readings and event votes across
volume, participation, flow, structure, cross-asset, and session context
produces a forward directional prediction with lift over a direction-matched
base, after costs, on `model_dev`.

The economic rationale is that each information area measures a different
market mechanism. A vote that requires agreement across independent areas
should filter noise, while the area cells test whether one mechanism carries
information on its own. The exploratory cells guarantee coverage; the five
primary cells are the only confirmatory cells.

## 2. Tier and frozen inputs

| Item | Value |
|---|---|
| Tier | `model` |
| Changed | model spec, active events, prediction, reading overrides |
| Frozen | data, policy, strategy, execution, portfolio, analytics |
| Timeframe | 1h (`3600` seconds) only |
| Instruments | XAU, BTC, S&P 500, EURUSD |
| Panel leader | BTC |
| Data | `XAUUSD`, `BTCUSD`, `ESANDP500`, `EURUSD` M1 2018–2024 |
| Development | `model_dev`, 2018-01-01..2020-12-31 |
| Validation | `model_validation`, 2021-01-01..2022-12-31 |
| Excluded | 2023 from every partition; 2024 and 2025 from the model programme |
| Strategy tier | Closed until an M2.x baseline is accepted |

The panel aligns the four instruments on common 1h close timestamps. No forward
fill is used. A timestamp absent from one instrument is absent from the panel.
ETH is excluded because it is too correlated with BTC to add an independent
panel vote.

## 3. Model contract

| Item | Value |
|---|---|
| Spec schema | `quantrule.model-spec/v2` |
| Prediction | `kind: direction`; each signal event adds its own direction code |
| Event thresholds | Fixed by the model contract; not tuned |
| Reading overrides | Only the reading windows and periods listed per primary cell |
| Active events | The registry events of the cell's areas |
| Conditions | Diagnostic only; they do not change the vote |
| Panel | Required for area 9, M2.3, and M2.5 |
| Session | Required for area 10, M2.4, and M2.5 |

Only `signal` events vote. `context` and `diagnostic` events may be active and
may appear in conditions; they never enter the vote.

The primary cells use a curated one-family-per-area vote rather than every
signal event. This follows the no-double-counting rule: correlated events are not a
second vote. The exploratory cells may activate every signal event in their
area because they are coverage cells and are not ranked.

## 4. Exploratory cells

The ten area cells are exploratory. They are reported in full, are not ranked,
receive no multiple-testing credit, and never open validation.

| Cell | Area | Indicators | Active events | Prediction events | Panel | Session |
|---|---|---:|---:|---:|---|---|
| EXPL-01 | 1 Volume-weighted price | 1 | 10 | 6 | – | – |
| EXPL-02 | 2 Volume by price | 1 | 11 | 4 | – | – |
| EXPL-03 | 3 Participation | 4 | 13 | 10 | – | – |
| EXPL-04 | 4 Volume-price flow | 4 | 21 | 18 | – | – |
| EXPL-05 | 5 Volatility structure | 5 | 12 | 2 | – | – |
| EXPL-06 | 6 Structure | 11 | 28 | 16 | – | – |
| EXPL-07 | 7 Candle anatomy and gaps | 3 | 17 | 1 | – | – |
| EXPL-08 | 8 Path and duration | 7 | 10 | 3 | – | – |
| EXPL-09 | 9 Cross-asset | 5 | 11 | 6 | yes | – |
| EXPL-10 | 10 Calendar and session | 5 | 11 | 2 | – | yes |

Area 7 has no signal-role event. Its cell uses `signal.rsi_threshold` as a
pre-registered benchmark vote and keeps every candle/gap event active; the
benchmark event is listed in the cell's `coverage.json`, not counted as an area
event. `calendar_bucket` is reading-only and has no event; it is exercised as a
session reading in area 10.

The retained indicators are assigned to areas: `atr` to area 5; `ema`, `rsi`,
`macd`, `adx`, `dmi`, and `pivots` to area 6; `context` and `progression` to
area 8; `rvol` to area 3. The canonical map is
`config/research/PROGRAM-2026-11-05-03/areas.json`.

## 5. Primary cells

The five primary cells are the only confirmatory cells. They are ranked and
corrected.

| Cell | Areas | Indicators | Active events | Prediction events | Min votes | Reading variant |
|---|---|---:|---:|---:|---:|---|
| M2.1 | 1–4 | 10 | 55 | 8 | 2 | VWAP 24, profile 96, RVOL 5/50, flow 20, MFI 14 |
| M2.2 | 5–7 | 19 | 56 | 4 | 2 | ATR slow 100, Keltner 20, Donchian 20, profile 96 |
| M2.3 | 9 | 5 | 11 | 2 | 1 | VWAP 24, profile 96 |
| M2.4 | 8, 10 | 12 | 21 | 2 | 1 | ATR slow 100, range 20, path 20 |
| M2.5 | 1–10 | 46 | 143 | 17 | 3 | combined: VWAP 24, profile 96, RVOL 5/50, ATR slow 100 |

The primary vote is one directional family per area:

| Cell | Vote families |
|---|---|
| M2.1 | VWAP, profile POC, RVOL, OBV divergences |
| M2.2 | ATR-term and RSI divergences |
| M2.3 | Relative-strength divergences |
| M2.4 | Overnight continuation and fade |
| M2.5 | VWAP, profile POC, RVOL, OBV, ATR-term, RSI, compression breakout, relative strength, overnight |

A primary cell's `active_events` contains every event of its areas so the run
publishes the full area surface; only the listed vote families enter the
prediction.

## 6. Coverage

The checker proves the committed matrix:

```text
ok PROGRAM-2026-11-05-03 cells=15 indicators=46 events=143
```

| Check | Result |
|---|---|
| Exploratory cells | 10 |
| Primary cells | 5 |
| Instruments per cell | 4 |
| Specs | 60 |
| Registry indicators covered | 46 / 46 |
| Registry events covered | 143 / 143 |
| Prediction events are active signal events | yes |
| Panel present for cross-asset cells | yes |
| Session present for session cells | yes |

The `areas.json` map makes the area membership explicit. The checker fails if an
area cell misses an event, a primary cell carries a benchmark, a prediction
event is not a signal, or a panel/session source is wrong.

## 7. Outcome, costs, and green criteria

| Item | Value |
|---|---|
| Horizon | 20 bars |
| Outcome | Barrier: +2 ATR target before −1 ATR stop; stop-first tie |
| Commission | 4 bps |
| Slippage | 2 bps |
| Cost hurdle | 6 bps |
| Min predictions | 200 |
| Min clusters | 20 |
| Min lift | 0.02 |
| Lift LCB | > 0 at the corrected alpha |
| Random p | <= 0.01 |
| Shuffle p | <= 0.01 |
| Back-lag | `back_lag_lift <= 0.01` or `lift - back_lag_lift >= 0.01` |
| Replication | at least 2 instruments with `n >= 30` and positive lift |

The evaluator is
[`tools/improvement/model_prediction.py`](../../../tools/improvement/model_prediction.py)
and its contract is
[`docs/analytics/2026-11-05/model-prediction.md`](../../analytics/2026-11-05/model-prediction.md).
It applies the split bounds and purges a prediction whose outcome crosses
`split_end`.

## 8. Correction and validation

- **Correction:** Bonferroni over the five primary cells; corrected alpha
  `0.05 / 5 = 0.01`.
- **Exploratory cells:** not ranked, no correction credit, no validation.
- **Validation:** one-shot `model_validation` for promoted primary cells only.
  The human approves the access and the promoted cells before the run.
- **Iteration cap:** 1. No post-hoc variant after a result.
- **Strategy:** `S2.x` remains closed until an M2.x baseline is accepted.

## 9. Stop rule

Stop the programme when:

- no primary cell passes the development gate;
- a validation cell fails;
- the iteration cap is reached;
- a cell cannot run because a shared input or tool is missing;
- the human stops it.

A stopped programme does not re-register a failed hypothesis with a different
event, timeframe, horizon, target, stop, filter, or cost. A new programme needs
a new premise and a new approved charter.

## 10. Human approval

Approve all of the following before P4/P5:

| Decision | Proposed |
|---|---|
| Premise and economic rationale | Section 1 |
| Ten exploratory cells and five primary cells | Sections 4–5 |
| One-family-per-area primary vote | Section 5 |
| 1h single timeframe; XAU/BTC/SP500/EURUSD panel | Section 2 |
| 20-bar horizon, +2 ATR / −1 ATR, 6 bps cost | Section 7 |
| Bonferroni over five primary cells | Section 8 |
| Green criteria and minimum sample | Section 7 |
| One-shot `model_validation` access | Section 8 |
| Strategy tier closed until an M2.x baseline | Section 2 |
| Stop rule and iteration cap | Section 9 |

Not authorized by this charter:

- running the development swarm before the P4 runner exists;
- opening `model_validation`;
- changing an event, timeframe, horizon, target, stop, filter, or cost after a
  result;
- any strategy, execution, portfolio, or live-capital work.

## 11. Implementation note

The program manifest pre-registers 15 cells and 60 specs. The cell command names
`tools/research/model_cell.py`, which **P4 adds**. Until then the manifest is a
pre-registration and is not runnable. The shell command in a cell is:

```text
quantrule report model --config <run.json> --spec <spec-<instrument>.json>
```

followed by `tools/improvement/model_prediction.py` over the four instrument
bundles. P4 implements that orchestration; P5 runs it on `model_dev`.
