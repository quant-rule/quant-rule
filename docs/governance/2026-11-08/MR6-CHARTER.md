# MR6 Charter — Ordered Family Confirmation

- Date: 2026-11-08 (UTC)
- Status: **Approved** (human, 2026-11-08)
- Family: `MR6`; model `M1.0`; research family, not a model version
- Programme: `PROGRAM-2026-11-08-02`
- Plan: [`mr6-programme.md`](../../execution-plans/2026-11-08/mr6-programme.md)
- Power: [`MR6-POWER.md`](../../improvement/2026-11-08/MR6-POWER.md)
- Data review: [`MR6-DATA-REVIEW.md`](../../improvement/2026-11-08/MR6-DATA-REVIEW.md)
- Claim type: pre-registered model research; no strategy or edge claim

## 1. Premise

The order of independent confirmation matters. A trend or momentum event
followed within a short window by an independent participation event in the same
direction carries directional information. The reverse order does not. The
effect should survive a timeframe change if it is a real mechanism.

The premise is not a retry of `MR5.7`. MR5.7 tested one trend event and one
participation event. MR6 tests the family-level mechanism: any of seven
trend/momentum events followed by any of eight relative-volume participation
events, with the reverse order as the falsification control.

## 2. Economic rationale

A trend reading states that the auction has chosen a side. An independent
participation reading states that new volume is entering on that side. When
participation follows the trend, a second, non-price family is accepting the
move. When participation leads, the trend event is a lagging description of the
same volume. The ordered test separates acceptance from description. The level
arm adds location because participation at a confirmed level can be absorption
rather than acceptance.

## 3. Hypotheses and cells

| Cell | Mechanism | Timeframe | Role |
|---|---|---|---|
| `MR6.1-C01` | Trend/momentum → participation family | 15m | Primary |
| `MR6.2-C01` | Participation → trend/momentum (reverse order) | 15m | Primary control |
| `MR6.3-C01` | Trend/momentum → participation family | 5m | Primary |
| `MR6.4-C01` | Participation → trend/momentum (reverse order) | 5m | Primary control |
| `MR6.5-C01` | Trend/momentum → participation at a confirmed level | 15m | Primary |

Each hypothesis has `C01` base, `C02` tighter window, and `C03` looser window:
**5 hypotheses × 3 configuration variants = 15 cells**. The 10 non-primary cells
are window sensitivity checks, not candidates.

The reverse-order cells are controls. If `MR6.2-C01` or `MR6.4-C01` passes, the
order premise is falsified and the programme stops; the control is not promoted
as a candidate. `MR6.5` is the location-confound check.

## 4. Event families

Trend/momentum (`first` side; `second` side in the controls):

- `signal.ema_pair_cross`
- `signal.ema_stack_change`
- `signal.macd_line_signal_cross`
- `signal.macd_histogram_slope_flip`
- `signal.macd_zero_cross`
- `signal.di_cross`
- `signal.trend_continuation`

Participation (`second` side; `first` side in the controls):

- `signal.rvol_expansion`
- `signal.rvol_ratio`
- `signal.cmf_cross`
- `signal.mfi_threshold`
- `signal.climax_reversal`
- `signal.climax_continuation`
- `signal.absorption_break`
- `signal.absorption_rejection`

The cumulative raw-volume OBV and AD crosses are excluded before the run because
the BTC and ETH sources change volume units within a year; see the data review.
The family uses relative-volume and flow readings that are invariant to a
constant unit and distorted only in a bounded window after a unit change.

## 5. Condition

The sequence condition uses `first_events` and `second_events` lists. The base
window is five bars, `min_gap_bars = 1` (the confirming event is strictly later
than the first), and `direction = "same"`. `C02` uses three bars; `C03` uses
eight. `MR6.5` adds `level_touches_min = 1` on the confirming bar.

The condition is evaluation-only. The Rust model never reads it.

## 6. Data and protocol

| Item | Value |
|---|---|
| Model | `M1.0`; `quantrule.model/v4` |
| Instruments | XAU, BTC, S&P 500, EURUSD, ETH |
| Development partition | `model_dev`, 2018-01-01..2020-12-31 |
| Folds | One calendar year per run: 2018, 2019, 2020 |
| Timeframes | 15m for MR6.1/2/5; 5m for MR6.3/4 |
| Outcome | Barrier, +2 ATR target before −1 ATR stop; stop-first tie |
| Horizon | 20 bars |
| Costs | 4 bps commission + 2 bps slippage = 6 bps |
| Minimum predictions | 200 condition-matched, every primary base cell |
| Minimum clusters | 20, every primary base cell |
| Minimum lift | 0.02 |
| Lift LCB | > 0 at corrected alpha |
| Controls | random and shuffle p ≤ corrected alpha; back-lag pass; condition increment ≥ 0.01 |
| Replication | at least 2 instruments with n ≥ 30 and positive condition lift |
| Correction | Bonferroni over five primary cells; cell alpha 0.01 |
| Seeds | 0 random, 1 shuffle |
| Execution | sequential, one cell at a time |

## 7. Count gate and stop rule

The count gate runs on the five primary base cells before any outcome is read.
If any primary cell is below 200 predictions or 20 clusters, the programme stops
as `underpowered`; no outcome is inspected and no threshold is changed.

The programme stops when no primary base cell reaches grade A, the count gate
fails, a reverse-order control passes, a shared prerequisite is missing, or the
human stops it. A failed hypothesis is not re-registered with a different event,
timeframe, horizon, target, stop, filter, cost, instrument set, condition, or
threshold.

## 8. Not authorized

- `model_validation`; the strategy, execution, or portfolio tier.
- A post-hoc variant, event, threshold, window, or instrument change.
- A model-engine, indicator, event, or data-contract change.
- A claim of an edge from a development result.

## 9. Approval

- Approved by: Human
- Approved on: 2026-11-08 (UTC)
- Scope: the premise, families, grid, correction, count gate, budget, and stop
  rule
- Development result: `no_edge`; 0/5 primary base cells passed; reverse controls
  did not pass; validation not opened; see
  [`MR6-DEV-SWARM.md`](../../improvement/2026-11-08/MR6-DEV-SWARM.md)
