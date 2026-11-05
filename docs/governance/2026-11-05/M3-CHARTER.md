# M3.x Model Programme Charter — State-Conditioned Event Edge

- Date: 2026-11-05 (UTC)
- Status: **Approved** (human, 2026-11-06) — P8b, validation, and the strategy
  tier remain unauthorized
- Programme ID: `PROGRAM-2026-11-05-04` (planned; not created)
- Owner: human
- Coordinator: OpenCode session `docs/476-m3-charter`
- Plan: [`476-m3-charter.md`](../../execution-plans/2026-11-05/476-m3-charter.md)
- Predecessor: [`M2-CHARTER.md`](M2-CHARTER.md); [`M2-CLOSEOUT.md`](../../improvement/2026-11-05/M2-CLOSEOUT.md)
- Contract: [`research-mandate.md`](../2026-11-04/research-mandate.md); [`run-contract.md`](../../workflows/run-contract.md)
- Claim type: pre-registered model programme; no edge claim

## 1. Premise

Directional event information is **conditional on the published market state**.
A trend-continuation event has positive directional edge only in a trend state;
a VWAP reclaim or rejection only in a coiling state; a value-area breakout only
in an expanding state; a climax reversal only in a high-range/low-participation
state; and an absorption rejection only in a settling state.

An unconditional event vote averages states with different mechanisms, base
rates, and sometimes opposite signs into zero. Evaluating each event inside its
pre-registered state, with the base matched on the same state, produces lift
after costs on `model_dev`.

This is a new premise, not the M2 hypothesis with a different parameter. M2
tested an unconditional vote; M3 tests whether the state is the missing
condition. The M2 evaluator names the state-conditioned base as the next
model-gate refinement; M3.1–M3.5 are that refinement.

## 2. Economic rationale

Different market states are governed by different participants:

| State | Mechanism | Event family | Expected direction |
|---|---|---|---|
| Trend (`adx_regime=1`, aligned EMA stack) | Momentum, stop flow, trend persistence | `signal.trend_continuation` | Continue the trend |
| Coiling (`context_code=2`) | Liquidity provision, mean reversion around a reference price | `signal.vwap_reclaim`, `signal.vwap_rejection` | Revert to VWAP |
| Expanding (`context_code=1`) | Breakout continuation after a volatility transition | `signal.profile_va_breakout` | Continue the break |
| High-range/low-participation (`context_code=4`) | Exhaustion on low participation | `signal.climax_reversal` | Fade the climax |
| Settling (`context_code=3`) | Absorption and replenishment | `signal.absorption_rejection` | Reject the absorption extreme |

The state is a causal, published measurement: `ATR14 / SMA20(ATR14)`, `RVOL20`,
ADX, the EMA stack, and the DI state. It is not a fitted classifier, a
regression, a percentile gate, or a new indicator. The event definitions and
their directional claims are already in the registry.

The economic asymmetry is persistence in trends and expansion, and reversion in
coiling, exhaustion, and absorption. M2's unconditional vote blended these
mechanisms. The state gate isolates each mechanism and tests whether it adds
information beyond the state's own base rate.

## 3. Tier and frozen inputs

| Field | Value |
|---|---|
| Tier | `model` |
| Changed | model spec, active events, state-conditioned evaluation |
| Frozen | data, policy, strategy, execution, portfolio, analytics |
| Base model | `M1.0`; `quantrule.model/v4`; `quantrule.model-result/v4` |
| Spec schema | `quantrule.model-spec/v2` (unchanged) |
| Timeframes | Count-only switch between 1h (`3600`) and 15m (`900`) |
| Instruments | XAU, BTC, S&P 500, EURUSD; volume cells use XAU, S&P 500, EURUSD |
| Development | `model_dev`, 2018-01-01..2020-12-31 |
| Validation | `model_validation`, 2021-01-01..2022-12-31; one-shot, not approved here |
| Excluded | 2023 from every partition; 2024 and 2025 from the model programme |
| Strategy | Closed until an M3.x baseline is accepted |

The base model is frozen. M3.1–M3.5 add no Rust indicator, event, reading, or
state. They add a state condition to the **evaluation** of the raw event
prediction.

## 4. Data and volume admissibility

The four M1 sources are the same versions used by M2:

| Instrument | Source |
|---|---|
| XAU | `data/sources/XAUUSD_M1_2018-01-01_2024-12-31.csv` |
| BTC | `data/sources/BTCUSD_M1_2018-01-01_2024-12-31.csv` |
| S&P 500 | `data/sources/ESANDP500_M1_2018-01-01_2024-12-31.csv` |
| EURUSD | `data/sources/EURUSD_M1_2018-01-01_2024-12-31.csv` |

The V5 Phase 2 audit found BTC and ETH volume is not admissible on 2018–2020:
the monthly median volume moves by 600×–1,800× within a year, a unit/contract
change rather than market volume. The volume family was therefore untested, not
rejected. M3.1–M3.5 use the following fixed admissibility rule:

> An instrument-year is volume-admissible when the ratio of the largest to the
> smallest monthly median M1 volume is at most 12× **and** no month-to-month
> median step is greater than 10×. The rule is evaluated on the development
> partition before any outcome; it is a data-quality gate, not a signal.

Observed on `model_dev`:

| Instrument | 2018 | 2019 | 2020 | Admissible |
|---|---:|---:|---:|---|
| XAU | 4.2× | 2.6× | 2.2× | yes |
| S&P 500 | 4.3× | 3.5× | 11.7× | yes; the March 2020 step is 8×, below the 10× unit-change threshold |
| EURUSD | 1.3× | 3.8× | 9.1× | yes |
| BTC | 1,881× | 682× | 1,135× | no |

Consequences:

- `M3.1` (trend) uses no volume reading and keeps all four instruments.
- `M3.2`–`M3.5` use volume or `context_code` (which contains RVOL) and use XAU,
  S&P 500, and EURUSD only. EURUSD volume is tick volume, not real volume; it
  passes the unit-stability rule but is a weaker participation measure.
- BTC is not re-admitted by changing the threshold after a result.
- The exploratory area cells keep all four instruments for coverage; their
  volume cells are descriptive, not confirmatory.

A repaired BTC/ETH volume source is a separate data work item. It is not part of
this charter.

## 5. Model contract and state-conditioned evaluation

### 5.1 Raw prediction

The model spec is unchanged. A primary cell activates one event family and
publishes its raw directional prediction at the event's `available_bar_index`:

```json
{
  "schema": "quantrule.model-spec/v2",
  "id": "M3.2",
  "family": "M3",
  "instrument": "XAU",
  "timeframes": [900],
  "readings": {"extended": {"vwap_window": 24}},
  "active_events": ["signal.vwap_reclaim", "signal.vwap_rejection"],
  "prediction": {
    "kind": "direction",
    "events": ["signal.vwap_reclaim", "signal.vwap_rejection"],
    "min_votes": 1
  }
}
```

Only a `signal` event votes. Each primary cell uses one event family; the state
is a condition, not a second vote. `context` readings and states may gate a
signal; a `diagnostic` never enters the gate.

### 5.2 State gate

The state gate is a separate file in the cell directory. It is read by the
evaluator, not by the Rust model:

```json
{
  "schema": "quantrule.model-state-gate/v1",
  "all": [
    {"column": "context_code", "values": [2]}
  ]
}
```

`column` is one of the per-bar columns in `readings.csv`:
`context_code`, `adx_regime`, `adx_slope`, `ema_stack`, `di_state`, `rsi_zone`,
or `macd_state`. `values` is a non-empty integer list.
`all` requires every clause to match at the prediction bar. A null state does
not match. The gate reads only the same bar's published state; it never reads a
later bar.

For `M3.1` the gate is:

```json
{
  "schema": "quantrule.model-state-gate/v1",
  "all": [
    {"column": "adx_regime", "values": [1]},
    {"column": "ema_stack", "values": [-1, 1]}
  ]
}
```

### 5.3 State-matched evaluation

`tools/improvement/model_prediction.py` gains a state-conditioned mode:

| Field | Rule |
|---|---|
| State filter | Keep a prediction only when every gate clause matches at the prediction bar |
| State base | For each direction, the base rate is computed on all valid bars in the **same state**, not the unconditional partition |
| State controls | Random entries, direction shuffle, and back-lag are drawn or matched inside the same state |
| State increment | `state_lift - unconditional_lift`; the unconditional lift uses the same raw predictions and the unconditional base |
| Replication | Each admissible instrument's state-matched `n >= 30` and positive lift; at least 2 instruments |
| Output | `state_gate`, `state_metrics`, `unconditional_metrics`, `state_increment`, and the existing metrics |

The state filter is applied before the outcome is joined. The state at bar `t`
is published at `t`; the gate cannot leak the future. The evaluator remains
evaluation-only and never feeds a rule.

### 5.4 Determinism and tests

P8b must add:

- A causality test: appending, changing, or removing later bars does not change
  a past state-filtered prediction or state-matched base.
- A determinism test: the same inputs, seed, and state gate produce
  byte-identical JSON and Markdown.
- Edge cases: absent column, null state, empty state, all-state, state present
  on one instrument only, and a state gate that removes every prediction.
- A boundary test proving the Rust model run does not read the gate file.

The model run itself must not change. The gate is a validation-boundary change,
documented in `docs/analytics/` and the M3 programme record.

## 6. Exploratory cells

The ten area cells are exploratory. They are reported in full, are not ranked,
receive no correction credit, and never open validation. They provide coverage
and the unconditional area benchmark.

| Cell | Area | Indicators | Events | Instruments | State gate |
|---|---|---:|---:|---|---|
| EXPL-01 | 1 Volume-weighted price | 1 | 10 | 4 | none |
| EXPL-02 | 2 Volume by price | 1 | 11 | 4 | none |
| EXPL-03 | 3 Participation | 4 | 13 | 4 | none |
| EXPL-04 | 4 Volume-price flow | 4 | 21 | 4 | none |
| EXPL-05 | 5 Volatility structure | 5 | 12 | 4 | none |
| EXPL-06 | 6 Structure | 11 | 28 | 4 | none |
| EXPL-07 | 7 Candle anatomy and gaps | 3 | 17 | 4 | none |
| EXPL-08 | 8 Path and duration | 7 | 10 | 4 | none |
| EXPL-09 | 9 Cross-asset | 5 | 11 | 4 | none |
| EXPL-10 | 10 Calendar and session | 5 | 11 | 4 | none |

The event lists, prediction lists, and reading overrides are the **verbatim M2
area cells** in `config/research/PROGRAM-2026-11-05-03/cells/EXPL-01..10`, with
`family` changed to `M3` and both timeframe specs (`3600` and `900`) present.
The area map is copied to the M3 programme. Area 7 keeps the
`signal.rsi_threshold` benchmark event; `calendar_bucket` remains reading-only.

The exploratory cells are not evidence of an edge. They exist so the M3
programme has a complete flat-model coverage surface and an unconditional
comparison for each area.

## 7. Primary cells

The five primary cells are the only confirmatory cells. Each uses one event
family, one state gate, and a state-matched base.

| Cell | State gate | Event(s) | Instruments | Hypothesis |
|---|---|---|---|---|
| `M3.1` | `adx_regime=1` **and** `ema_stack in {-1,+1}` | `signal.trend_continuation` | XAU, BTC, S&P 500, EURUSD | Trend continuation has positive directional edge in a trend state |
| `M3.2` | `context_code=2` (coiling) | `signal.vwap_reclaim`, `signal.vwap_rejection` | XAU, S&P 500, EURUSD | VWAP mean reversion has positive directional edge in a coiling state |
| `M3.3` | `context_code=1` (expanding) | `signal.profile_va_breakout` | XAU, S&P 500, EURUSD | A value-area breakout continues in an expanding state |
| `M3.4` | `context_code=4` (high-range/low-participation) | `signal.climax_reversal` | XAU, S&P 500, EURUSD | A climax reversal fades the extreme in an exhaustion state |
| `M3.5` | `context_code=3` (settling) | `signal.absorption_rejection` | XAU, S&P 500, EURUSD | An absorption rejection reverses in a settling state |

Each primary cell:

- uses `kind: direction`, `min_votes: 1`, and only `signal` events;
- uses the raw event direction, not a new threshold or a tuned side;
- has no `conditions` block and no panel/session source;
- has one `state.json` and one spec per instrument per timeframe;
- is compared with its own **unconditional** prediction stream, so the state
  increment is measured on the same events and bars.

`signal.compression_breakout` is **not** a primary vote. Its registry claim is a
forward range, not direction. M3.3 tests the directional value-area breakout,
which has a directional claim. The compression family remains an accepted
non-directional baseline and an exploratory coverage event.

`signal.trend_continuation`'s registry claim is a +1 ATR / -1 ATR barrier. The
M3 gate uses the common +2 ATR / -1 ATR barrier for comparability with M2. A
pass at +2/-1 is stronger than the event's claim; a fail does not falsify the
narrower +1/-1 claim.

## 8. Exact config

### 8.1 Programme manifest

The runnable manifest is created in P8b after approval. Its fields are fixed
here:

| Field | Value |
|---|---|
| Path | `config/research/PROGRAM-2026-11-05-04/program.json` |
| Schema | `quantrule.research-program/v1` |
| Programme ID | `PROGRAM-2026-11-05-04` |
| Tier / changed | `model` / `model` |
| Frozen | `strategy`, `portfolio` |
| Partitions | `model_dev` / `model_validation` |
| Costs | 4 bps commission, 2 bps slippage, 0 funding |
| Correction | `bonferroni`, 5 primary cells |
| Max parallel | 5 |
| Iteration cap | 1 |
| Validation access | `one_shot`; not approved by this charter |
| Stop rule | Section 12 |

### 8.2 Cell paths and command

| Cell | Config directory | State file |
|---|---|---|
| `M3.1` | `config/research/PROGRAM-2026-11-05-04/cells/M3.1` | `state.json` |
| `M3.2` | `config/research/PROGRAM-2026-11-05-04/cells/M3.2` | `state.json` |
| `M3.3` | `config/research/PROGRAM-2026-11-05-04/cells/M3.3` | `state.json` |
| `M3.4` | `config/research/PROGRAM-2026-11-05-04/cells/M3.4` | `state.json` |
| `M3.5` | `config/research/PROGRAM-2026-11-05-04/cells/M3.5` | `state.json` |

Each cell contains `coverage.json`, `state.json`, and `spec-<instrument>.json`
for each admissible instrument at each timeframe. The command is:

```bash
python3 tools/research/model_cell.py \
  --cell-dir {config} \
  --phase {phase} \
  --out {out} \
  --horizon 20 \
  --target-atr 2.0 \
  --stop-atr 1.0 \
  --cost-bps 6 \
  --primary-cells 5 \
  --state-file {config}/state.json \
  --seed 0
```

The runner passes the state gate to the evaluator; the model run receives the
spec only. The `--state-file` flag is added in P8b. The exploratory cells omit
it.

### 8.3 Reading overrides

| Cell | Reading overrides |
|---|---|
| `M3.1` | Defaults: EMA 20/50/200, ADX 14/25/3, MACD 12/26/9 |
| `M3.2` | `extended.vwap_window=24` |
| `M3.3` | `extended.profile_window=96` |
| `M3.4` | Defaults: climax volume 3.0×, range 1.5 ATR |
| `M3.5` | Defaults: absorption volume 2.0×, range 0.6 ATR, level 0.25 ATR |

The exact defaults are the `ExtendedConfig::DEFAULT` values in the model
contract. The resolved values are recorded in `provenance.json`; no threshold is
tuned after a result.

### 8.4 Timeframe count switch

The primary timeframe is selected before any outcome:

1. Run the five primary specs at 1h and 15m with the raw model; do not join an
   outcome.
2. Count `prediction != 0` rows that match the cell's state gate, per instrument.
3. Use 1h if **every** primary cell has at least 200 state-matched predictions
   and 20 clusters; otherwise use 15m if every cell does; otherwise stop as
   `underpowered`.
4. Freeze the selected timeframe, the spec hashes, the state gates, and the
   instrument set in the run manifest.

The count stage inspects counts only. It does not compute a hit rate, base rate,
lift, or control. The 1h/15m choice is not a hypothesis and receives no
correction credit.

### 8.5 Coverage rebuild

P8b generalizes `tools/research/coverage.py` for M3:

- 10 exploratory + 5 primary cells, IDs `EXPL-01..10` and `M3.1..M3.5`;
- `family: M3` and schema `quantrule.model-spec/v2`;
- per-cell instrument sets (4 for exploratory, 4 or 3 for primary);
- both timeframe specs;
- `state.json` schema, columns, values, and instrument coverage;
- every registry indicator and event covered by at least one exploratory cell;
- every primary prediction event is an active `signal` event;
- every primary state column is a published `readings.csv` column.

The expected output is:

```text
ok PROGRAM-2026-11-05-04 cells=15 indicators=46 events=143
```

No cell runs before the coverage check passes.

## 9. Outcome, costs, and green criteria

| Item | Value |
|---|---|
| Horizon | 20 bars |
| Outcome | Barrier: +2 ATR target before −1 ATR stop; stop-first tie |
| Commission | 4 bps |
| Slippage | 2 bps |
| Cost hurdle | 6 bps |
| Minimum state-matched predictions | 200 |
| Minimum clusters | 20 |
| Minimum state-matched lift | +0.02 |
| Lift LCB | > 0 at corrected alpha 0.01 |
| Random p | <= 0.01 |
| Shuffle p | <= 0.01 |
| Back-lag | `back_lag_lift <= 0.01` or `state_lift - back_lag_lift >= 0.01` |
| State increment | `state_lift - unconditional_lift >= +0.01` |
| Replication | at least 2 admissible instruments with `n >= 30` and positive state lift |
| Promotion | grade A only; grade B is `development_only` |

The state-matched base and all controls are computed inside the same state. The
unconditional lift is the same raw prediction against the unconditional base.
A cell that is positive only because the state itself has a directional bias
fails the state-increment criterion.

Grade:

| Grade | Rule |
|---|---|
| A | Sample, state lift, LCB, controls, back-lag, state increment, and replication pass |
| B | All of A except replication |
| C | Sample and positive state lift pass; at least one of LCB, controls, back-lag, or state increment fails |
| D | Sample fails, state lift is missing, or state lift is not positive |

Only grade A promotes to validation. Grade B is recorded as `development_only`.
Grade C/D is `no_edge` for that cell. The human reviews every cell.

## 10. Correction, validation, and iteration

- **Correction:** Bonferroni over the 5 primary cells; family-wise alpha 0.05,
  cell alpha 0.01. The count-only timeframe switch, the exploratory cells, and
  the unconditional metrics are not hypotheses and receive no correction credit.
- **Validation:** one-shot `model_validation` 2021-01-01..2022-12-31, only for
  grade-A cells promoted by the human. The validation run uses the same frozen
  spec, state gate, instrument set, outcome, costs, and criteria with
  `primary_cells=1` and alpha 0.05. A validation pass is a candidate baseline
  pending human acceptance, not an edge claim.
- **Second-use disclosure:** `model_validation` 2021–2022 was already used by
  the accepted 1m compression baselines. The M3 hypothesis is new, but the
  partition is not fresh. The human must accept or reject this residual
  data-snooping risk before validation access.
- **Iteration cap:** 1. No post-hoc variant after a result.
- **Strategy:** `S3.x` remains closed until an M3.x baseline is accepted.

## 11. Coverage

| Check | Expected |
|---|---|
| Exploratory cells | 10 |
| Primary cells | 5 |
| Instruments | 4 per exploratory cell; 4 or 3 per primary cell |
| Registry indicators covered | 46 / 46 |
| Registry events covered | 143 / 143 |
| Primary prediction events are active signal events | yes |
| Primary state columns exist in `readings.csv` | yes |
| Volume cells use only admissible instruments | yes |
| Specs | 4 instruments × 2 timeframes × 15 cells, minus excluded BTC specs |

The exploratory area map and event sets are copied from
`config/research/PROGRAM-2026-11-05-03/areas.json` and its ten area cells. The
primary cells are new and use six events total. Coverage is a property of the
exploratory surface; the primary cells are confirmatory subsets.

## 12. Stop rule

Stop the programme when:

- no primary cell reaches grade A on `model_dev`;
- a promoted validation cell fails;
- the count switch cannot find a timeframe where every primary cell has 200
  state-matched predictions and 20 clusters;
- a shared input, coverage check, or evaluator prerequisite is missing;
- the human stops it.

A stopped programme retires the state-conditioned premise. It is not
re-registered with a different state, event, timeframe, horizon, target, stop,
filter, cost, instrument set, or state-increment threshold. A new programme
needs a new premise and a new approved charter.

## 13. Risks and limitations

| Risk | Disposition |
|---|---|
| State conditioning reduces the sample below 200 | The count-only switch chooses 1h or 15m; otherwise the programme stops as underpowered |
| BTC/ETH volume is inadmissible | Volume cells exclude them; the trend cell keeps all four; a repaired source is a separate work item |
| `model_validation` is not fresh | Prior compression uses disclosed; the human accepts or rejects the residual risk before access |
| The state gate is an evaluation prototype, not a Rust model output | The gate is documented and tested as a validation-boundary change; a strategy must implement the same causal filter later |
| The primary cells use one event each | Deliberate, to avoid double counting; interactions remain untested |
| `signal.trend_continuation` claims +1/−1 ATR, the gate uses +2/−1 | A pass is stronger; a fail does not falsify the narrower claim |
| A state cell is positive only because the state is biased | The state-matched base and the state-increment criterion reject it |
| Same-agent QD/QV review | Disclosed; not independent |
| No strategy or live result | The model gate is a filter, not a profitability claim |

## 14. Human approval

Approve all of the following before P8b:

| Decision | Proposed |
|---|---|
| Premise and economic rationale | Sections 1–2 |
| 10 exploratory area cells + 5 primary state cells | Sections 6–7 |
| State gates and state-matched evaluation | Sections 5 and 7 |
| 1h/15m count-only switch | Section 8.4 |
| Volume admissibility rule and instrument sets | Section 4 |
| 20-bar +2/−1 ATR outcome and 6 bps cost | Section 9 |
| Green criteria, state increment, grade A promotion | Section 9 |
| Bonferroni over 5 primary cells | Section 10 |
| Iteration cap 1 and stop rule | Sections 10 and 12 |
| Validation access not approved now; request after the development gate | Section 10 |
| Strategy tier closed until an accepted M3.x baseline | Sections 3 and 10 |

- Approved by: Human
- Approved on: 2026-11-06 (UTC)
- Scope: the premise, cells, correction, budget, and stop rule above
- Not authorized: P8b, any run, `model_validation`, or the strategy tier

## 15. What this charter does not authorize

- Creating the programme manifest, cell specs, run configs, or coverage tool.
- Changing the model, events, spec parser, evaluator, coverage tool, or shell.
- Running a cell, computing a hit rate, or opening validation.
- Starting the strategy, execution, or portfolio tier.
- Treating this `Proposed` document as an approved protocol or an edge claim.

P8b starts only after the human approves this charter and separately authorizes
the programme build and run.
