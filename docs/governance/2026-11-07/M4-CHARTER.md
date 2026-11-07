# M4.x Model Programme Charter — Location-Conditioned Directional Events

- Date: 2026-11-07 (UTC)
- Status: **Approved** (human, 2026-11-07) — build, count gate, and development
  swarm authorized; validation and the strategy tier not authorized
- Programme ID: `PROGRAM-2026-11-07-01` (planned; not created)
- Owner: human
- Coordinator: OpenCode session `m4-location-programme`
- Plan: [`m4-location-programme.md`](../../execution-plans/2026-11-07/m4-location-programme.md)
- Predecessors: [`M2-CHARTER.md`](../2026-11-05/M2-CHARTER.md),
  [`M2-CLOSEOUT.md`](../../improvement/2026-11-05/M2-CLOSEOUT.md),
  [`M3-CHARTER.md`](../2026-11-05/M3-CHARTER.md),
  [`M3-PREFLIGHT.md`](../../improvement/2026-11-05/M3-PREFLIGHT.md),
  [`DIRECTIONAL-EVENT-PROGRAM-CLOSEOUT.md`](../../improvement/2026-11-04/DIRECTIONAL-EVENT-PROGRAM-CLOSEOUT.md)
- Contract: [`research-mandate.md`](../2026-11-04/research-mandate.md),
  [`run-contract.md`](../../workflows/run-contract.md),
  [`model-research.md`](../../workflows/model-research.md),
  [`research-swarm.md`](../../workflows/research-swarm.md)
- Claim type: pre-registered model programme; no strategy or edge claim

## 1. Premise

Directional event information is **conditional on location relative to a
confirmed major level**. The same event at a confirmed major level has positive
forward edge after costs; the same event away from structure does not. Location,
not the event alone, is the missing condition.

This is a new premise. M2 tested an unconditional directional vote and averaged
every location into one number. M3 conditioned the event on the published
volatility/trend state and still averaged every location. The 2026-11-04
directional event-family programme tested five families unconditionally at 15m
and all five graded C. None of those programmes conditioned on where the event
occurred relative to confirmed structure.

## 2. Economic rationale

Confirmed major pivots are where resting liquidity, stop orders, and
market-maker inventory concentrate. A directional event at a confirmed major
level is a liquidity interaction — absorption, a stop run, or a breakout — and
has a counterparty flow that can push price after the event. The same event
mid-range has no such flow: it is a momentum reading without a liquidity
mechanism, and it should decay toward the unconditional base rate.

The asymmetry is structural, not a parameter. The level is published by the
model from confirmed pivots; the condition is a context reading, not a new
indicator. The counterparty is the resting order book around prior swing
extremes. The effect should persist while swing levels remain reference points
for stops and inventory.

The M2 result is consistent with this premise: the unconditional vote blended
event bars at levels with event bars mid-range and produced no stable lift. The
M3 result is also consistent: the state gate reduced the sample without
conditioning on location. The 15m family result is the direct baseline: the same
mechanisms were tested without a location condition and all graded C.

## 3. Screening record

| Item | Value |
|---|---|
| Premise | Directional event edge is conditional on proximity to a confirmed major level. |
| Family / role | Five registered `signal` events; the gate reads the `level_touches` `context` reading. |
| Instruments | XAU, BTC, S&P 500, EURUSD. |
| Timeframe | 15m (`900` seconds). |
| Development partition | `model_dev`, 2018-01-01..2020-12-31. |
| Validation partition | `model_validation`, 2021-01-01..2022-12-31; one-shot, not approved here. |
| Expected sample | The 2026-11-04 15m families produced 4,118–60,328 events per cell; the level gate is expected to reduce that. The count-only preflight decides power before any outcome is read. |
| Data admissibility | Price-only cells; no volume, participation, or flow reading. All four instruments are admissible on `model_dev`; the BTC/ETH volume defect does not apply. |
| New vs. variant | New condition (location). M2 was unconditional; M3 was state-conditioned; the 15m programme was unconditional per family. The event, timeframe, target, stop, horizon, and cost are deliberately reused for comparability, but the condition is not a state, threshold, or parameter variant. |
| Prior records reviewed | M2 closeout and development swarm; M3 charter and preflight; the five 15m family candidates and their closeout; the accepted compression baselines; the registry claims; the indicator verification evidence. |

The screening decision is **propose for a charter**. The human accepts or rejects
the premise at the gate in section 14.

## 4. Tier and frozen inputs

| Field | Value |
|---|---|
| Tier | `model` |
| Changed | Model evaluation condition (the location gate) and the five model specs. |
| Frozen | Data, policy, strategy, execution, portfolio, analytics. |
| Base model | `M1.0`; `quantrule.model/v4`; `quantrule.model-result/v4`. |
| Spec schema | `quantrule.model-spec/v2` (unchanged). |
| Event contract | The registry events and their definitions are unchanged. |
| Data | M1 CSVs, 2018-01-01..2024-12-31, one-year folds on `model_dev`. |
| Costs | 4 bps commission, 2 bps slippage, 0 funding. |
| Policy | `POL1.0`, frozen. |
| Strategy | No strategy; the tier remains closed. |
| Validation | `model_validation` one-shot, not authorized by this charter. |

The model itself does not change. The five specs activate one registered `signal`
event each. The location gate is an evaluation condition read from the model's
published `readings.csv`; the Rust model never reads the gate file.

## 5. Data and admissibility

The four M1 sources are the same versions used by M2 and M3:

| Instrument | Source |
|---|---|
| XAU | `data/sources/XAUUSD_M1_2018-01-01_2024-12-31.csv` |
| BTC | `data/sources/BTCUSD_M1_2018-01-01_2024-12-31.csv` |
| S&P 500 | `data/sources/ESANDP500_M1_2018-01-01_2024-12-31.csv` |
| EURUSD | `data/sources/EURUSD_M1_2018-01-01_2024-12-31.csv` |

The M4 run configs set the data version to
`m4-<instrument>-m1-2018-01-01_<year>-12-31`. The source files are read-only.
2023 is excluded from every fold. The model programme does not read 2024 or
2025.

No cell uses volume, participation, or flow. The volume-admissibility rule is
therefore not a gate for this programme; the BTC volume defect is irrelevant to
a price-only level test. If a later variant adds a volume reading, it needs a
new charter and the M3 admissibility rule.

## 6. Condition gate and evaluation

### 6.1 Gate definition

The condition is the published `level_touches` reading at the prediction bar:

```json
{
  "schema": "quantrule.model-state-gate/v1",
  "all": [
    {"column": "level_touches", "min": 1}
  ]
}
```

`level_touches` is the `level_strength` context reading already published in
`readings.csv`. The registry defines it as the count of confirmed major levels
within `model.level.atr` of the bar's high or low over
`model.level.lookback`. It is null while structure is unavailable. The
condition set is:

- `level_touches >= 1`: the event is at confirmed structure;
- `level_touches == 0`: the event is away from structure;
- null: unavailable; the gate does not match.

The gate reads only the prediction bar's published value. It never reads a later
bar, a future level, or an unconfirmed pivot. The Rust model run does not read
the gate file; the boundary test in section 6.4 proves it.

The existing `quantrule.model-state-gate/v1` schema is extended with `min`; the
M3 `values` form remains valid and unchanged. The evaluator's historical
`state_*` output field names are retained so the M3 artifacts stay comparable;
this charter calls the measurement a **condition gate**, and the M4 result
records it as `state_gate` with the `level_touches` clause.

### 6.2 Evaluation

| Measurement | Definition |
|---|---|
| Condition base | The barrier hit rate on all valid bars in the same condition (`level_touches >= 1`). |
| Unconditional base | The barrier hit rate on all valid bars in the partition. |
| Condition lift | Prediction hit rate in the condition minus the condition base. |
| Away lift | Prediction hit rate with `level_touches == 0` minus the away base; diagnostic only. |
| Location increment | `condition_lift - unconditional_lift`. |
| Controls | Random entries, direction shuffle, and back-lag drawn or matched inside the same condition. |
| Replication | At least two admissible instruments with `n >= 30` and positive condition lift. |

The condition filter is applied before the outcome is joined. The evaluator is
evaluation-only and never feeds a rule. A positive away lift does not rescue a
cell: the location increment must be positive, or the premise is falsified.

### 6.3 Count-only preflight

The preflight reads counts only and never joins an outcome:

1. Run the five primary specs at 15m with the raw model.
2. Count `prediction != 0` rows whose `level_touches >= 1`, per cell, instrument,
   and year.
3. Count clusters with the 20-bar non-overlapping rule.
4. Require **every** primary cell to reach `n >= 200` and `clusters >= 20`.
5. Otherwise stop as `underpowered`; no development outcome is read.

There is no timeframe switch. 15m is the pre-registered timeframe; a failure at
15m retires the premise rather than selecting another timeframe after a count.

### 6.4 Evaluator changes and tests

The evaluator change is limited to the condition-gate parser and matcher in
`tools/improvement/model_prediction.py`:

- add `level_touches` to the accepted gate columns;
- accept a `min` (and, for completeness, `max`) integer bound as an alternative
  to `values`; reject a clause that carries both;
- keep the existing `values` semantics and the `state_*` output fields.

Tests must cover:

- `level_touches` 0, 1, 2, and null; a null state never matches;
- a gate that removes every prediction;
- a gate that matches every bar;
- `values` and `min`/`max` in the same document, rejected;
- causality: appending, changing, or removing later bars does not change a past
  match or a past condition base;
- determinism: the same inputs and seed produce byte-identical JSON and
  Markdown;
- the boundary test: the Rust model run does not read the gate file.

The evaluator's documentation under `docs/analytics/2026-11-05/model-prediction.md`
is updated with the condition gate and the `level_touches` column. The M3
`values` contract is unchanged.

## 7. Cells

The programme has **five primary cells and no new exploratory cells**. M4 adds
no indicator or event; the M2 exploratory surface already covers 46/46
indicators and 143/143 events. Re-running that surface would be a variant of the
M2 programme that already recorded `no_edge`, so it is not re-registered. The
coverage check in section 11 inherits the M2 surface and verifies the five M4
primary cells.

| Cell | Event | Hypothesis | Config |
|---|---|---|---|
| `M4.1` | `signal.rsi_threshold` | An RSI threshold crossing at a confirmed major level has a directional path reaction; away from structure it does not. | `config/research/PROGRAM-2026-11-07-01/cells/M4.1` |
| `M4.2` | `signal.macd_histogram_slope_flip` | A MACD histogram slope flip at a confirmed major level accelerates in the flip direction; away from structure it does not. | `config/research/PROGRAM-2026-11-07-01/cells/M4.2` |
| `M4.3` | `signal.di_cross` | A DI cross at a confirmed major level continues in the cross direction; away from structure it does not. | `config/research/PROGRAM-2026-11-07-01/cells/M4.3` |
| `M4.4` | `signal.trend_continuation` | A trend continuation signal at a confirmed major level reaches +2 ATR before −1 ATR; away from structure it does not. | `config/research/PROGRAM-2026-11-07-01/cells/M4.4` |
| `M4.5` | `signal.ema_stack_change` | An EMA stack state change at a confirmed major level continues; away from structure it does not. | `config/research/PROGRAM-2026-11-07-01/cells/M4.5` |

Each cell:

- uses `kind: direction`, `min_votes: 1`, and one active `signal` event;
- uses the event's published direction; it does not tune a side;
- has no `conditions` block and no panel/session source;
- has one `state.json` and one spec per instrument at 15m;
- is compared with its own unconditional prediction stream, so the location
  increment is measured on the same events and bars.

`signal.ema_stack_change` is one registered event ID with one source; the model
counts it once per bar by its stable event code. The cell does not activate a
second EMA event. `signal.compression_breakout` is excluded: its registry claim
is a forward range, not direction.

## 8. Exact config

### 8.1 Programme manifest

The runnable manifest is created after approval. Its fields are fixed here:

| Field | Value |
|---|---|
| Path | `config/research/PROGRAM-2026-11-07-01/program.json` |
| Schema | `quantrule.research-program/v1` |
| Programme ID | `PROGRAM-2026-11-07-01` |
| Tier / changed | `model` / `model` |
| Frozen | `strategy`, `portfolio` |
| Partitions | `model_dev` / `model_validation` |
| Timeframe | 15m (`900`) |
| Instruments | XAU, BTC, S&P 500, EURUSD |
| Costs | 4 bps commission, 2 bps slippage, 0 funding |
| Correction | `bonferroni`, 5 primary cells |
| Max parallel | 5 |
| Iteration cap | 1 |
| Validation access | `one_shot`; not approved by this charter |
| Stop rule | Section 12 |

### 8.2 Cell paths and command

| Cell | Config directory | Gate |
|---|---|---|
| `M4.1` | `config/research/PROGRAM-2026-11-07-01/cells/M4.1` | `state.json` |
| `M4.2` | `config/research/PROGRAM-2026-11-07-01/cells/M4.2` | `state.json` |
| `M4.3` | `config/research/PROGRAM-2026-11-07-01/cells/M4.3` | `state.json` |
| `M4.4` | `config/research/PROGRAM-2026-11-07-01/cells/M4.4` | `state.json` |
| `M4.5` | `config/research/PROGRAM-2026-11-07-01/cells/M4.5` | `state.json` |

Each cell contains `coverage.json`, `state.json`, and
`spec-<instrument>-900.json` for each instrument. The command is:

```bash
python3 tools/research/model_cell.py \
  --cell-dir {config} \
  --phase dev \
  --out {out} \
  --family M4 \
  --timeframe 900 \
  --runs-dir config/research/PROGRAM-2026-11-07-01/runs \
  --horizon 20 \
  --target-atr 2.0 \
  --stop-atr 1.0 \
  --cost-bps 6 \
  --primary-cells 5 \
  --seed 0 \
  --state-file {config}/state.json
```

The count preflight adds `--count-only` and writes `count.json`. The runner
passes the gate to the evaluator; the model run receives the spec only.

### 8.3 Spec example

```json
{
  "schema": "quantrule.model-spec/v2",
  "id": "M4.1",
  "family": "M4",
  "instrument": "XAU",
  "timeframes": [900],
  "readings": {},
  "active_events": ["signal.rsi_threshold"],
  "prediction": {
    "kind": "direction",
    "events": ["signal.rsi_threshold"],
    "min_votes": 1
  }
}
```

### 8.4 Run configs

The one-year run configs are copied from the M3 programme and re-labelled with
the M4 data version. The model, policy, execution, portfolio, period, and source
paths are unchanged. The runner merges the yearly prediction and readings
bundles per instrument, then runs one pooled evaluation per cell.

## 9. Outcome, costs, and green criteria

| Item | Value |
|---|---|
| Horizon | 20 bars |
| Outcome | Barrier: +2 ATR target before −1 ATR stop; stop-first tie |
| Commission | 4 bps |
| Slippage | 2 bps |
| Cost hurdle | 6 bps |
| Minimum condition-matched predictions | 200 |
| Minimum clusters | 20 |
| Minimum condition lift | +0.02 |
| Lift LCB | > 0 at corrected alpha 0.01 |
| Random p | <= 0.01 |
| Shuffle p | <= 0.01 |
| Back-lag | `back_lag_lift <= 0.01` or `condition_lift - back_lag_lift >= 0.01` |
| Location increment | `condition_lift - unconditional_lift >= +0.01` |
| Replication | At least 2 admissible instruments with `n >= 30` and positive condition lift |
| Promotion | Grade A only; grade B is `development_only` |

The condition base and all controls are computed inside the same condition. A
cell that is positive only because the location itself has a directional bias
fails the location-increment criterion.

Grade:

| Grade | Rule |
|---|---|
| A | Sample, condition lift, LCB, controls, back-lag, location increment, and replication pass |
| B | All of A except replication |
| C | Sample and positive condition lift pass; at least one of LCB, controls, back-lag, or location increment fails |
| D | Sample fails, condition lift is missing, or condition lift is not positive |

Only grade A promotes to validation. Grade B is recorded as
`development_only`. Grade C/D is `no_edge` for that cell. The human reviews every
cell.

## 10. Correction, validation, and iteration

- **Correction:** Bonferroni over the five primary cells; family-wise alpha
  0.05, cell alpha 0.01. The count preflight and the away-from-structure
  diagnostic are not hypotheses and receive no correction credit.
- **Validation:** one-shot `model_validation` 2021-01-01..2022-12-31, only for
  grade-A cells promoted by the human. The validation run uses the same frozen
  spec, gate, instrument set, outcome, costs, and criteria with
  `primary_cells=1` and alpha 0.05. A validation pass is a candidate baseline
  pending human acceptance, not an edge claim.
- **Second-use disclosure:** `model_validation` 2021–2022 was already used by
  the three accepted compression baselines. The M4 hypothesis is new, but the
  partition is not fresh. The human must accept or reject this residual
  data-snooping risk before validation access.
- **Iteration cap:** 1. No post-hoc variant after a result.
- **Strategy:** closed until a model baseline is accepted and a separate strategy
  charter is approved.

## 11. Coverage

| Check | Expected |
|---|---|
| Primary cells | 5 |
| Instruments | 4 per primary cell |
| Timeframe | 15m (`900`) |
| Registry indicators covered | 46 / 46 inherited from `PROGRAM-2026-11-05-03` |
| Registry events covered | 143 / 143 inherited from `PROGRAM-2026-11-05-03` |
| Primary prediction events are active `signal` events | yes |
| Primary gate column is a published reading | yes, `level_touches` |
| Inherited exploratory surface | `PROGRAM-2026-11-05-03`; not re-run |
| Specs | 4 instruments × 5 cells at 15m |

The coverage command is:

```bash
python3 tools/research/coverage.py \
  --program config/research/PROGRAM-2026-11-07-01/program.json \
  --inherit config/research/PROGRAM-2026-11-05-03/program.json
```

The expected output is `ok PROGRAM-2026-11-07-01 cells=5 indicators=46
events=143 inherited=PROGRAM-2026-11-05-03`. No cell runs before the coverage
check passes.

## 12. Concurrency and ownership

| Field | Value |
|---|---|
| Max parallel cells | 5 |
| Mode | Config-only cells; no code worktree per cell |
| Output | `data/research/PROGRAM-2026-11-07-01/<phase>/<CELL>/` |
| Shared read-only files | Model pack, source CSVs, engine configs, research tools |
| Shared write policy | None; every cell writes only its own output directory |
| Cell isolation | No cell reads another cell's result before recording its own |
| Validation | No cell opens validation; only the human promotes |

The coordinator creates the programme, validates the manifest, checks coverage,
runs the count gate, and aggregates. The coordinator does not tune, vote, or
promote. A cell agent does not change its config after seeing a result.

## 13. Stop rule

Stop the programme when:

- no primary cell reaches grade A on `model_dev`;
- a promoted validation cell fails;
- the count gate cannot reach 200 condition-matched predictions and 20 clusters
  in every primary cell;
- a shared input, coverage check, or evaluator prerequisite is missing;
- a threshold, cost, split, or criterion changes after a result is seen;
- the human stops it.

A stopped programme retires the location premise. It is not re-registered with a
different event, timeframe, horizon, target, stop, filter, cost, instrument set,
gate column, or gate threshold. A new programme needs a new premise and a new
approved charter.

## 14. Risks and limitations

| Risk | Disposition |
|---|---|
| The location gate is too rare and the programme is underpowered | The count-only preflight stops before any outcome; the premise is retired |
| The 15m counts from 2026-11-04 are indicative, not a power guarantee | The count gate is the authority; no outcome is read before it passes |
| `level_touches` is H/L-based while the event may be close-based | The gate is a structural location condition; the event's own definition is unchanged; the limitation is recorded |
| The away-from-structure subset is small | It is diagnostic; the location increment uses the unconditional base, as in M3 |
| Validation is not fresh | Prior compression uses disclosed; the human accepts or rejects the residual risk before access |
| The gate is an evaluator feature, not a Rust model output | The gate is documented and tested as a validation-boundary change; a future strategy must implement the same causal filter |
| Same-agent QD/QV review | Disclosed; not independent; the human remains the final authority |
| No strategy or live result | The model gate is a filter, not a profitability claim |

## 15. Human approval

The human approved this charter on 2026-11-07 (UTC). The approval authorizes the
build, the count-only preflight, and the development swarm described in the
plan. It does not authorize validation, the strategy tier, or a model code
change.

| Decision | Proposed |
|---|---|
| Premise and economic rationale | Sections 1–2 |
| Five primary cells and no new exploratory cells | Section 7 |
| Location gate `level_touches >= 1` and the condition-matched evaluation | Sections 6 and 9 |
| 15m timeframe and four instruments | Sections 3, 7, and 8 |
| 20-bar +2/−1 ATR outcome and 6 bps cost | Section 9 |
| Green criteria, location increment, grade A promotion | Section 9 |
| Bonferroni over 5 primary cells | Section 10 |
| Iteration cap 1 and stop rule | Sections 10 and 13 |
| Validation access not approved now; request after the development gate | Section 10 |
| Strategy tier closed until an accepted model baseline | Sections 4 and 10 |

On approval, the charter authorizes the build, the count-only preflight, and the
development swarm described in the plan. It does not authorize validation, the
strategy tier, or a model code change.

- Approved by: Human
- Approved on: 2026-11-07 (UTC)
- Scope: the premise, cells, gate, correction, budget, and stop rule above
- Not authorized: `model_validation`; the strategy, execution, or portfolio
  tier; a second validation access; a post-hoc variant

## 16. What this charter does not authorize

- A programme variant outside the five cells, gate, timeframe, instruments,
  outcome, costs, and correction approved above.
- Reading a forward outcome before the count-only gate passes.
- Opening `model_validation` without the human's promotion and separate access.
- Changing the model, events, spec parser, evaluator semantics beyond the gate
  described in section 6, shell, or engine.
- Starting the strategy, execution, or portfolio tier.
- Treating this charter as an edge claim or a baseline acceptance.
