# MR5 Model Research Charter — Mechanism and Path Programme

- Date: 2026-11-08 (UTC)
- Status: **Approved** (human, 2026-11-08); development `no_edge`; validation
  not opened
- Programme ID: `PROGRAM-2026-11-08-01` (planned)
- Owner: human
- Coordinator: OpenCode `mr5-platform`
- Plan: [`mr5-programme.md`](../../execution-plans/2026-11-08/mr5-programme.md)
- Power worksheet: [`MR5-POWER.md`](../../improvement/2026-11-08/MR5-POWER.md)
- Data review: [`MR5-DATA-REVIEW.md`](../../improvement/2026-11-08/MR5-DATA-REVIEW.md)
- Naming: [`model-families.md`](../../model/2026-11-05/model-families.md)
- Contract: [`model-research.md`](../../workflows/model-research.md),
  [`research-swarm.md`](../../workflows/research-swarm.md),
  [`run-contract.md`](../../workflows/run-contract.md)
- Claim type: pre-registered model research programme; no edge claim

## 1. Premise

The model is `M1.0`. `MR5` is a **model research family**, not a model version.
The programme tests whether a small set of **event-sequence, confluence, and
path** mechanisms carry forward information that the earlier single-bar event
votes did not.

The earlier directional programmes (M2, M3, M4, the 15m family programme) tested
single-bar events or state/location conditions. They found no directional
model baseline. The next step is not more indicators. It is a different signal
form: an event followed by a confirming or failing event, a confluence of
independent published levels, and a path/expectancy measurement.

The family is a finite, pre-registered set. It is not a parameter search.

## 2. Economic rationale

| Mechanism | Counterparty / why it can persist |
|---|---|
| Failed breakout / stop run | Resting stops above and below confirmed pivots; a failed break is the liquidity event that traps breakout participants |
| Break-retest hold | Breakout traders wait for a retest; the hold is the confirmation that the level flipped |
| Failed retest | A retest that fails is absorption against the breakout; the trapped flow reverses |
| Level flip | A broken level becomes the other side's reference; the retest is a new decision point |
| Confluence | Independent reference types (VWAP, value area, pivot) concentrate liquidity; a signal there has a larger counterparty than a mid-range signal |
| Freshness | A newly confirmed level has untested liquidity; repeated tests consume it |
| Two-family confirmation | Trend and participation are different measurements; their agreement is not a second vote of the same reading |
| Gap plus level | An overnight gap through a level is a repricing event; the first test decides whether the gap holds |
| Compression magnitude | The accepted range baselines predict expansion; the magnitude and the participation profile decide whether a bracket is worth its costs |
| Path expectancy | A hit-rate gate can hide an asymmetric path; MFE/MAE and time-to-target decide whether an exit menu can monetize the signal |

None of these needs a fitted model, a statistic in the signal path, or data
outside OHLCV.

## 3. Tier and frozen inputs

| Field | Value |
|---|---|
| Tier | `model` |
| Changed | Model research configuration: the per-cell model config and the condition/evaluation file |
| Frozen | Data, policy, strategy, execution, portfolio, analytics |
| Model | `M1.0`; `quantrule.model/v4`; `quantrule.model-result/v4` |
| Spec | `quantrule.model-spec/v2` |
| Timeframe | 15m (`900`) |
| Instruments | XAU, BTC, S&P 500, EURUSD; price-only cells may add ETH if the data review is accepted |
| Development | `model_dev` 2018-01-01..2020-12-31 |
| Validation | `model_validation` 2021-01-01..2022-12-31; one-shot, **not approved here** |
| Costs | 4 bps commission, 2 bps slippage, 0 funding |
| Strategy | closed until an accepted model baseline |

The model engine does not change. Each cell has its own `model.json`,
`condition.json`, and specs. The model config variants are research
configuration, not a new model version.

## 4. Cells

The grid is **10 hypotheses × 3 configuration variants = 30 cells**.

- `C01` is the base variant.
- `C02` is the tighter variant.
- `C03` is the looser variant.
- The **five primary base cells** are `MR5.1-C01`–`MR5.5-C01`.
- The other 25 cells are exploratory. They are reported but not ranked as
  candidates and receive no correction credit.

| # | Cell | Mechanism | Prediction event | Condition | C02 / C03 |
|---|---|---|---|---|---|
| 1 | `MR5.1-C01` | Failed breakout / stop run | `level.break` | `level.cross` within 3 bars, same level, opposite direction, close back on the original side | within 2 / 5 |
| 2 | `MR5.2-C01` | Break-retest hold | `level.break` | `level.test` within 5 bars, same level, close on the breakout side | within 3 / 8 |
| 3 | `MR5.3-C01` | Failed retest | `level.break` | `level.test` within 5 bars, same level, close back on the original side | within 3 / 8 |
| 4 | `MR5.4-C01` | Level flip | `level.break` | `level.test` within 8 bars, same level, opposite direction | within 5 / 12 |
| 5 | `MR5.5-C01` | Confluence | `signal.rsi_threshold` | at least 2 of VWAP, POC, last pivot high, last pivot low within 0.50 ATR | 0.35 / 0.75 ATR |
| 6 | `MR5.6-C01` | Event freshness | `signal.di_cross` | `level_touches >= 1` and `level_age <= 24` | 12 / 48 |
| 7 | `MR5.7-C01` | Two-family confirmation | `signal.rvol_expansion` | `signal.trend_continuation` within 5 bars, same direction | within 3 / 8 |
| 8 | `MR5.8-C01` | Gap plus level | `signal.gap_continuation` | `level.gap_test` within 5 bars | within 3 / 8 |
| 9 | `MR5.9-C01` | Compression magnitude | `signal.compression_breakout` | `rvol_ratio >= 1.0` | 1.2 / 0.8 |
| 10 | `MR5.10-C01` | Path expectancy | `signal.di_cross` | `atr >= 1.0` (all volatile bars) | 0.8 / 1.2 |

Notes:

- The condition may reference context events (for example `level.gap_test`); the
  prediction itself uses only registered `signal` events.
- `MR5.5` uses `signal.rsi_threshold` as the prediction event and the confluence
  as the condition. It is one signal event, not a multi-event vote.
- `MR5.7` uses `signal.rvol_expansion` as the prediction and
  `signal.trend_continuation` as the condition. The two events are independent
  definitions; the condition is not a second vote.
- `MR5.9` and `MR5.10` are path/range theses. Their primary gate is the same
  hit-rate gate; the path report is the additional evidence.

The exact condition and model-config overrides are fixed in the MR5 `grid.json`
before the first run.

## 5. Data and admissibility

The four core M1 sources are XAU, BTC, S&P 500, EURUSD. ETHUSD is already in
`data/partitions.json` and may be added for the price-only cells to increase the
cross-sectional sample. The volume family is excluded: BTC/ETH volume is not
admissible on the development partition. The Data Steward review records the
validation reuse and the fresh-instrument options.

The `model_validation` partition is not fresh; it was used by the accepted
compression baselines. The human must accept the residual data-snooping risk
before validation access.

## 6. Configuration hierarchy

```text
M1.0                      model
MR5.1-C01                 hypothesis 1, configuration 01
PROGRAM-2026-11-08-01     programme
```

Each cell directory contains:

```text
model.json               the model config for this variant
condition.json           the sequence/confluence/column condition
spec-<instrument>-900.json
coverage.json
```

The builder writes the grid from `grid.json`; it refuses to overwrite an
existing programme. The runner copies the cell model config into each prepared
run config, so the model config participates in provenance without changing the
shared run configs.

## 7. Evaluation and green criteria

| Item | Value |
|---|---|
| Outcome | Barrier: +2 ATR target before −1 ATR stop; stop-first tie |
| Horizon | 20 bars |
| Cost hurdle | 6 bps |
| Condition base | Direction-matched base on bars where the condition matches |
| Unconditional base | Direction-matched base on all valid bars |
| Condition increment | `condition_lift - unconditional_lift >= +0.01` |
| Minimum condition-matched predictions | 200 |
| Minimum clusters | 20 |
| Minimum condition lift | +0.02 |
| Lift LCB | > 0 at corrected alpha |
| Random p | <= corrected alpha |
| Shuffle p | <= corrected alpha |
| Back-lag | `back_lag_lift <= 0.01` or `condition_lift - back_lag_lift >= 0.01` |
| Replication | at least 2 instruments with `n >= 30` and positive condition lift |
| Promotion | grade A only; grade B is `development_only` |
| Path report | MFE/MAE, time-to-target, time-to-stop, expected R under the pre-registered menu |

Grade A requires sample, condition lift, LCB, controls, back-lag, condition
increment, and replication. Grade B is all but replication. Grade C has a
positive condition lift with one or more failed gates. Grade D fails the sample
or has a non-positive condition lift.

The path report is descriptive for the exploratory round. If a primary cell is
promoted, the path report is part of the candidate packet, not a replacement for
the hit-rate gate.

## 8. Correction and power

- **Correction:** Bonferroni over the five primary base cells; family-wise alpha
  0.05, cell alpha 0.01 (two-sided).
- **Power:** the MDE worksheet shows the low-frequency cells are already at or
  above a 2pp MDE at five cells. A ten-cell confirmation is not supported on the
  current sample; it needs a fresh instrument/partition that raises the cluster
  count.
- **Exploratory credit:** none. The 25 exploratory cells are hypothesis
  generation.
- **Iteration cap:** 1.
- **Selection:** the primary base cells are frozen before the exploratory grid
  is read. A better exploratory variant is a hypothesis for a new programme; it
  is not promoted into this confirmation.

## 9. Validation

- `model_validation` is **not approved** by this charter.
- After the development round, the human promotes at most the five primary base
  cells.
- The one-shot validation uses the same frozen model config, condition,
  instruments, outcome, costs, and criteria, with `primary_cells=1` and alpha
  0.05.
- A validation pass is a candidate baseline pending human acceptance, not an
  edge claim.
- The reuse of 2021–2022 is disclosed; the Quant Validator records the residual
  risk.

## 10. Stop rule

Stop the programme when:

- no primary base cell reaches grade A on `model_dev`;
- a promoted validation cell fails;
- the count gate cannot reach 200 condition-matched predictions and 20 clusters
  in every primary base cell;
- a shared input, coverage check, or evaluator prerequisite is missing;
- a threshold, cost, split, or criterion changes after a result is seen;
- the human stops it.

A stopped programme retires the MR5 premise. It is not re-registered with a
different event, timeframe, horizon, target, stop, filter, cost, instrument set,
condition, or threshold. A new programme needs a new premise and a new charter.

## 11. Risks and limitations

| Risk | Disposition |
|---|---|
| The condition is too rare and the primary cells are underpowered | Count-only gate stops before an outcome; the condition variants show whether the rarity is structural |
| The 30-cell grid is a search | The five primary base cells are frozen; the 25 exploratory cells receive no credit; a non-base variant cannot be promoted |
| The validation partition is reused | Disclosed; the Quant Validator records the residual risk; a fresh instrument is a Data Lifecycle work item |
| The path report is over-read | It is descriptive; the hit-rate gate remains the primary decision |
| A model config variant becomes a hidden parameter search | The three variants are pre-registered with an economic direction (tighter/looser); no post-hoc change |
| Same-agent QD/QV review | Disclosed; separate agents are used where the harness allows; no claim of independent review otherwise |
| No strategy or live result | The model gate is a filter, not a profitability claim |

## 12. Human approval

The human approved this charter on 2026-11-08. The development grid ran and
stopped at `no_edge`; validation remains closed.

| Decision | Proposed |
|---|---|
| Naming | `M1.0` model; `MR5.x` research family; `-Cnn` configuration |
| Premise and ten hypotheses | Sections 1–2 and 4 |
| 10 × 3 grid; five primary base cells | Section 4 |
| Condition and path evaluators; per-cell model config | Sections 4 and 6 |
| Five-cell Bonferroni correction | Section 8 |
| Data: four core instruments; ETH optional; price-only | Section 5 and the Data Steward review |
| Validation not approved now | Section 9 |
| Stop rule and iteration cap | Section 10 |

On approval, this charter authorizes the grid build, the count gate, and the
30-cell exploratory round. It does not authorize validation, the strategy tier,
or a model engine change.

- Approved by: Human
- Approved on: 2026-11-08 (UTC)
- Scope: the premise, hypotheses, grid, correction, budget, and stop rule
- Not authorized: `model_validation`; the strategy, execution, or portfolio
  tier; a post-hoc variant; a model engine change
- Development result: `no_edge`; no candidate; validation not opened

## 13. What this charter does not authorize

- A 31st cell, a different condition, or a different config after the grid is
  frozen.
- Reading a forward outcome before the count gate passes.
- Opening `model_validation` without the human promotion and separate access.
- Changing the model, events, spec parser, evaluator semantics beyond the
  documented condition/path modules, shell, or engine.
- Starting the strategy, execution, or portfolio tier.
- Treating an exploratory cell as evidence or a baseline.
