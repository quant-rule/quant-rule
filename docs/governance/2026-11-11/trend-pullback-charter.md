# Trend-Pullback Model Gate — Program Charter

- Date: 2026-11-11 (UTC)
- Program ID: `PROGRAM-2026-11-11-01`
- Status: **Approved — pre-registered; development gate only**
- Tier: model
- Model: `M1.3`; the level-event contract is unchanged from `M1.2`
- Predecessors: [`PROGRAM-2026-11-04-02`](../../improvement/2026-11-04/PROGRAM-2026-11-04-02.md)
  (cross-timeframe 1h trend agreement; no edge) and
  [`SR9`](../../improvement/2026-11-09/SR9-15M-STRUCTURE.md) (15m structure;
  underpowered)
- Claim type: model research; no directional or edge claim.

## 1. Premise

A 15m test of the 200 EMA that agrees with the last closed 1h trend state
(`ema_stack` in the event direction and `adx_regime == 1`) reaches +2 ATR before
−1 ATR more often than the same 15m EMA200 test without the 1h filter, and adds
beyond the 1h state base.

The claim is conditional, not predictive. The 1h state describes the current
situation; the test measures whether the next 15m path differs conditional on
that state.

## 2. Economic rationale

A 1h trend is a slow, observable regime. A 15m pullback into the 200 EMA is a
timing trigger at a known level inside that regime. The pullback entry is a
different mechanism from the continuation and breakout events tested by
`PROGRAM-2026-11-04-02`; it enters at a better price and uses a tighter stop, so
the payoff geometry can improve even if the hit rate is unchanged.

## 3. Distinction from prior work

`PROGRAM-2026-11-04-02` tested a 15m trend-continuation event, a major pivot
break, and an EMA50 test, all conditioned on the same 1h state. All three cells
failed the development gate; the best conditional lift was +1.13pp and the
back-lag control showed the event was late. The EMA200 pullback was not tested.
This program tests one new trigger against the same controls; it is not a retune
of the prior events.

## 4. Tier and frozen inputs

| Field | Value |
|---|---|
| Tier | model |
| Changed | model measurement (no model code change; the existing EMA200 level event is used) |
| Frozen | data, policy, strategy, execution, portfolio, analytics |
| Data | Dukascopy BTCUSD, ETHUSD, XAUUSD 1m, aggregated to 15m and 1h; 2018–2020 |
| Costs | 6 bps round-trip declared for the later strategy tier; the model gate uses barrier paths and applies no execution costs |
| Account | Not used; no execution or portfolio work |
| Validation | `model_validation` 2021–2022; not opened |

## 5. Cells

| Cell | Hypothesis | Primary? |
|---|---|---|
| `CELL-01-EMA200` | A 15m EMA200 test that agrees with the 1h state reaches +2 ATR before −1 ATR more often than the same test alone and adds beyond the 1h state base. | yes |
| `CTRL-EMA50` | The same test at the 50 EMA; a descriptive control for the level choice. | no |
| `CTRL-TREND` | The 15m trend-continuation event with the same 1h filter; a descriptive control for the prior program. | no |

The two controls are pre-registered and descriptive. They cannot promote a cell,
change the primary verdict, or justify a new hypothesis after the result.

## 6. Partitions and data budget

| Partition | Window | Use |
|---|---|---|
| Development | `model_dev` 2018-01-01..2020-12-31 | Gate |
| Validation | `model_validation` 2021-01-01..2022-12-31 | One-shot; closed |

2023 is excluded. The validation partition has not been used for this premise.
Each cell records the actual 15m and 1h coverage. The existing Dukascopy BTCUSD
and ETHUSD series are below 95% raw 24×7 coverage; a passing development gate is
therefore **not** a promotion to strategy research until the coverage question
is resolved or a human-approved data exception is recorded. XAUUSD session-adjusted
coverage is 92.8–95.2% across 2018–2020.

## 7. Correction

| Field | Value |
|---|---|
| Method | None; one primary cell |
| Primary cells | 1 |
| Family-wise alpha | 0.05 |
| Controls | EMA50 and trend continuation; descriptive, cannot promote |

The program is a new premise, not a continuation of the prior program's cell
family. The historical cross-timeframe tests remain recorded; this program does
not reopen their validation windows.

## 8. Green criteria

The primary cell must pass all of:

1. **G1 sample:** at least 200 filtered events and 20 clusters.
2. **G2 conditional lift:** filtered hit rate − unfiltered event hit rate ≥ 5pp.
3. **G3 conditional LCB:** the lower confidence bound of G2 is above zero.
4. **G4 incremental lift:** filtered hit rate − 1h state base ≥ 2pp.
5. **G5 incremental LCB:** the lower confidence bound of G4 is above zero.
6. **G6 random:** the filtered hit rate beats at least 95% of 200 seeded random
   draws from the same 1h state pool.
7. **G7 back-lag:** the 20-bar-earlier control does not beat the event by more
   than 1pp, or the conditional lift exceeds the back-lag lift by at least 1pp.
8. **G8 shuffle:** the filtered hit rate beats at least 95% of 200 seeded
   direction-shuffled draws.
9. **G9 replication:** at least two instruments have n ≥ 30 and a filtered hit
   rate above their own 1h state base.
10. **G10 sensitivity:** the `adx20` sensitivity config has a positive
    conditional lift and a positive incremental lift.

The pre-registered z uses one primary cell. The gate is a filter, not proof of an
edge.

## 9. Stop rule

If the primary cell fails, stop the trend-pullback line. Do not re-register a
variant with a different EMA period, ADX threshold, target, stop, horizon,
timeframe, filter, or cost. A failed gate is the result.

If the primary cell passes, the next step is a separate strategy design and a
separate charter. The 2024 validation partition remains closed until the
strategy development gate passes and the human approves the validation access.

## 10. Concurrency and ownership

| Field | Value |
|---|---|
| Max parallel cells | 1 |
| Worktree | `quant-rule--trend-pullback-model-gate` |
| Branch | `feat/trend-pullback-model-gate` |
| Quant roles | Quant Researcher (hypothesis and protocol), Quant Developer (configs and run), Quant Validator (not invoked; sequential review only) |

The run is implemented and reviewed sequentially by the same agent; no claim of
independent validation is made. A passing cell requires an independent Quant
Validator record before any strategy promotion.

## 11. Records

| Record | Path |
|---|---|
| Charter | `docs/governance/2026-11-11/trend-pullback-charter.md` |
| Program config | `config/research/PROGRAM-2026-11-11-01/` |
| Local run | `data/improvement/2026-11-11/PROGRAM-2026-11-11-01/` |
| Committed summary | `docs/improvement/2026-11-11/PROGRAM-2026-11-11-01.md` |
| AI transparency | `docs/ai-transparency-log/2026-11-11/trend-pullback-model-gate.md` |
| Index | `docs/improvement/index.md` |
| Baselines | `docs/improvement/baselines.md` |

## 12. Approval

The human approved the model-gate test in the instruction “APPROVED OPEN A NEW
WORKTREE AND PROCEED” on 2026-11-11 (UTC). This charter fixes the cells,
partition, criteria, and stop rule before the first result is read.
