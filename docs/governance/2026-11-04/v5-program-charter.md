# V5 Model Program — Program Charter

- Date: 2026-11-04 (UTC)
- Program ID: `PROGRAM-2026-11-04-04`
- Status: **Approved** (human, 2026-11-04); Phase 1 executed — **no edge**, program stopped
- Owner: human
- Contract: [`research-mandate.md`](research-mandate.md)
- Plan: [`docs/execution-plans/2026-11-04/v5-model-program.md`](../../execution-plans/2026-11-04/v5-model-program.md) (approved 2026-11-04)
- Pre-registration: [`docs/improvement/2026-11-04/V5-SWING-1H-PREREG.md`](../../improvement/2026-11-04/V5-SWING-1H-PREREG.md)

## 1. Premise

The v4 event layer has no directional edge at a 20-bar horizon on 5m/15m/1h, but
those tests used a 1 ATR target against a 1 ATR stop, where the round-trip cost
is a large fraction of the expected move. The v5 premise is that the same
directional information is still present at a **multi-day** horizon, where a
4 ATR target has room to absorb cost and where trend and breakout persistence
has time to express itself.

Phase 1 tests that horizon explanation on the existing model. It does not add a
reading, change an event, or change the model code.

## 2. Economic rationale

Two effects can make a short-horizon null coexist with a longer-horizon edge:

- **Cost amortization.** Commission and slippage are per trade, not per bar. A
  4 ATR 1h move is several times the round-trip cost, while a 1 ATR 15m move is
  not.
- **Slow information diffusion.** Order flow, positioning, and risk transfer
  persist over days. A trend or a confirmed pivot break is a claim about that
  persistence; a 20-hour window may simply be too short for it to pay.

The counterparty is the liquidity provider and the mean-reverting short-horizon
trader. If the effect is real, it must survive a 2R payoff, the barrier controls,
and instrument replication.

## 3. Tier and frozen inputs

| Field | Value |
|---|---|
| Tier | `model` |
| Changed | the evaluation horizon and payoff only; no model code or config change |
| Frozen | data engine, model code, policy, strategy, execution, portfolio, analytics |
| Data versions | `dukascopy-bid-{btcusd,ethusd,xauusd}-m1-...`; existing v4 model pack |
| Costs | 4 bps commission per operation, 2 bps slippage, no funding (strategy tier only) |
| Policy | `config/model-gate/shared/policy-*.json`; model gate is pre-cost |

Phase 1 reuses the accepted v4 model pack at 1h. Regenerating the pack is not
part of the hypothesis.

## 4. Cells

Phase 1 has three primary cells, all at 1h, all on the same barrier:

| Cell | Hypothesis | Family | Primary? |
|---|---|---|---|
| `CELL-01` | A 1h trend-continuation trigger predicts a 4 ATR move before a 2 ATR stop | type 3 / subtype 6 / source 16, state `trend` | yes |
| `CELL-02` | A 1h major-pivot break predicts continuation | type 1 / subtype 3 / source 8, state `none` | yes |
| `CELL-03` | A 1h EMA50 test predicts continuation | type 1 / subtype 1 / source 2, state `ema_side` | yes |

Exact configs: `config/v5-swing/1h/families.json`; matrix:
`config/model-v4/dev/matrix.json` (`adx25` primary, `adx20` sensitivity).

A cell is not changed after its result is seen. A failed cell is retired, not
retried with a different horizon, target, stop, or state.

## 5. Partitions and data budget

| Partition | Window | Use |
|---|---|---|
| `model_dev` | 2018-01-01..2020-12-31 | Phase 1 gate and selection |
| `model_validation` | 2021-01-01..2022-12-31 | One-shot confirmation, only if a dev cell passes |

The validation partition has been used by the accepted compression model
baselines on 2026-11-04. It has **not** been used for a directional event gate at
a 100-bar horizon. That reuse is disclosed. Access is one-shot for this
hypothesis.

## 6. Correction

| Field | Value |
|---|---|
| Method | `bonferroni` |
| Primary cells | 3 |
| Family-wise alpha | 0.05 |
| Notes | `z = 2.394` on development; `z = 1.96` on the single validation cell |

The `adx20` rows are a pre-registered sensitivity check, not extra hypotheses,
and do not enter the correction.

## 7. Green criteria

A Phase 1 cell passes only when all hold on `model_dev`:

- `n >= 200` valid events and `J >= 20` clusters;
- event lift over the direction-matched 1h base `>= +0.05` (economic threshold);
- lift LCB `> 0` at `z = 2.394`;
- the random control passes (`p <= 0.05`);
- the back-lag control passes (100-bar lag);
- the shuffled-direction control passes (`p <= 0.05`);
- at least two instruments have `n >= 30` and `hit_rate > base`;
- the `adx20` sensitivity agrees: `lift > 0` for the trend cell, identical event
  set for the pivot-break and EMA50 cells.

The harness grade (`A`/`B`) is necessary but not sufficient: the `+0.05` lift
threshold is the pre-registered promotion rule. A grade A/B cell below `+0.05`
is recorded as `development_only` and the program stops.

A validation pass is a **candidate model baseline pending human acceptance**. It
is not an accepted baseline and not a strategy go.

## 8. Concurrency and ownership

| Field | Value |
|---|---|
| Max parallel cells | 1 (the three cells share one harness run) |
| Worktree root | `<repository>--v5-swing-model` |
| Cell ownership | one branch and one worktree for the program |
| Shared read-only files | v4 model pack, source CSVs, engine configs |
| Shared write policy | the program writes only `config/v5-swing/`, its docs, and `data/improvement/<date>/V5-SWING-*` |

## 9. Stop rule

The program stops when any of the following holds:

- the 1h pre-flight sample is below 200 for any cell;
- no primary cell passes the development gate;
- the validation cell fails;
- the iteration cap (one dev run, one validation run) is reached;
- the human stops it.

A stopped program does not open the next tier. Phase 2 (a new-information v5
model) is a separate premise with its own charter and pre-registration; it starts
only after Phase 1 fails and the human approves it.

## 10. Validation access

| Field | Value |
|---|---|
| Access | `none` — the development gate produced no passing cell |
| Approved by | not applicable |
| Approved on | not applicable |
| Promoted cells | none |
| Conditions | the development gate must pass before access is requested |

## 11. Human approval

- Approved by: human
- Approved on: 2026-11-04
- Scope: the premise, three cells, barrier, correction, partitions, and stop rule
  in this charter and the pre-registration
- Not authorized: a different event, timeframe, target, stop, horizon, filter,
  cost, or a second validation access

## 12. Closeout

| Field | Value |
|---|---|
| Verdict | `no_edge` |
| Accepted baseline | none |
| Registry | [`docs/improvement/2026-11-04/V5-SWING-1H-GATE.md`](../../improvement/2026-11-04/V5-SWING-1H-GATE.md) |
| Evidence | `data/improvement/2026-11-04/MODEL-2026-11-04-18/` (local) |
| Cleanup | worktree and branch removed after preservation |

Phase 1 stopped on 2026-11-04. The three cells are retired, the validation
partition is unopened, and no strategy or portfolio work started. Phase 2 needs a
new charter and human approval.
