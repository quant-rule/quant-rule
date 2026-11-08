# MR7 Charter — Opening-Range Sweep/Reclaim

- Date: 2026-11-08 (UTC)
- Status: **Approved** (human, 2026-11-08)
- Family: `MR7`; model `M1.1`; research family, not a model version
- Programme: `PROGRAM-2026-11-08-03`
- Plan: [`mr7-programme.md`](../../execution-plans/2026-11-08/mr7-programme.md)
- Claim type: model change + data lifecycle + pre-registered model research; no strategy or edge claim

## 1. Premise

A sweep of the completed opening range that closes back inside is a stop-run and
reclaim at a session extreme. The reclaim carries a reversal edge that a generic
failed breakout does not, because the opening range is where early-session stops
cluster. A sweep that closes outside is the continuation control: if it also
reverses, the reclaim premise is falsified.

The mechanism is a one-bar event, not a two-event sequence. It is distinct from
`MR5.1` (a confirmed pivot or EMA break followed by a cross within three bars)
and from `MR6` (a trend/momentum family followed by a participation family).

## 2. Model change

Add two signal events using the existing opening-range readings; no new
indicator:

| Event | Code | Claim |
|---|---:|---|
| `signal.opening_range_sweep` | 144 | A sweep of the completed opening range that closes back inside predicts a reversal. |
| `signal.opening_range_sweep_control` | 145 | A sweep of the completed opening range that closes outside is the continuation control. |

Definition, after the opening range is complete:

```text
high sweep: H > ORH + sweep_atr*ATR and C < ORH  -> sweep, direction -1
low sweep:  L < ORL - sweep_atr*ATR and C > ORL  -> sweep, direction +1
high control: H > ORH + sweep_atr*ATR and C >= ORH -> control, direction +1
low control:  L < ORL - sweep_atr*ATR and C <= ORL -> control, direction -1
```

If both sides exceed the threshold, keep the side with the larger excursion; an
equal excursion emits no event. The event is available at the bar close. The
reference value is the opening-range high or low; the primary value is the
close. The event is only emitted inside the London session after the opening
range is complete.

`session.sweep_atr` is a new spec session field (default 0.0, non-negative). The
opening range length remains the existing `session.opening_range_bars` (default
2). The model identity moves to `M1.1`; the registry version moves to
`2026-11-08.1`; the event count moves from 143 to 145 and the validation claims
from 189 to 191.

## 3. Grid

Five hypotheses × three configurations = 15 cells; five primary base cells.

| Cell | Mechanism | Timeframe | Role |
|---|---|---|---|
| `MR7.1-C01` | 2-bar opening-range sweep/reclaim | 15m | Primary |
| `MR7.2-C01` | 2-bar opening-range sweep/reclaim | 1h | Primary |
| `MR7.3-C01` | 4-bar opening-range sweep/reclaim | 15m | Primary |
| `MR7.4-C01` | Sweep/reclaim with relative-volume expansion | 15m | Primary |
| `MR7.5-C01` | No-reclaim control (close outside) | 15m | Primary control |

Variants:

- `MR7.1`, `MR7.2`, `MR7.5`: `C01` sweep_atr 0.0, `C02` 0.1, `C03` 0.2.
- `MR7.3`: `C01` 4-bar OR and 0.0, `C02` 4-bar OR and 0.1, `C03` 4-bar OR and
  0.2.
- `MR7.4`: `C01` rvol_ratio ≥ 1.5, `C02` ≥ 1.2, `C03` ≥ 1.8; sweep_atr 0.0.

The control is not a candidate. If the control also shows a reversal edge, the
reclaim premise is falsified; if it shows continuation, the reclaim is a
distinct mechanism.

## 4. Data

| Item | Value |
|---|---|
| Existing instruments | XAU, BTC, S&P 500, EURUSD, ETH |
| Fresh instruments | COPPERCMDUSD, TSLAUSUSD (price-only) |
| Development partition | `model_dev`, 2018-01-01..2020-12-31 |
| Folds | One calendar year per run: 2018, 2019, 2020 |
| Timeframes | 15m for MR7.1/3/4/5; 1h for MR7.2 |
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

Fresh-instrument admission:

- `COPPERCMDUSD`: price-only; zero volume on most rows; volume, participation,
  and flow events are not admissible.
- `TSLAUSUSD`: price-only; volume units shift in 2020; volume, participation,
  and flow events are not admissible.
- The participation cell `MR7.4` uses only the five existing instruments.

## 5. Green criteria

A primary cell is a candidate only when all of the following hold:

1. At least 200 condition-matched predictions and 20 clusters.
2. Lift ≥ 0.02 and lift LCB > 0 at the corrected alpha.
3. Random and shuffle p ≤ the corrected alpha.
4. Back-lag and condition-increment rules pass.
5. At least two instruments replicate with n ≥ 30 and positive lift.
6. Grade A.
7. **Mean net R > 0** on the 2/1/20 menu after 6 bps.

Criterion 7 is new and is applied by the programme report, not by the model
grade alone. A hit-rate pass on a negative-expectancy base is not a candidate.

## 6. Count gate and stop rule

The count gate runs on the five primary base cells before any outcome is read.
If any primary cell is below 200 predictions or 20 clusters, the programme stops
as `underpowered`.

The programme stops when no primary base cell passes the full gate, the count
gate fails, the no-reclaim control also reverses, a shared prerequisite is
missing, or the human stops it. A failed hypothesis is not re-registered with a
different event, timeframe, horizon, target, stop, filter, cost, instrument set,
condition, or threshold.

## 7. Not authorized

- `model_validation`; the strategy, execution, or portfolio tier.
- A post-hoc variant, event, threshold, window, timeframe, or instrument change.
- A strategy, execution, or portfolio rule.
- A claim of an edge from the development result.

## 8. Approval

- Approved by: Human
- Approved on: 2026-11-08 (UTC)
- Scope: the model change, data admission, grid, correction, path gate, count
  gate, budget, and stop rule
- Development result: `no_edge`; 0/5 primary base cells passed the hit-rate
  and path gates; the control was positive but did not reverse; validation not
  opened; see [`MR7-DEV-SWARM.md`](../../improvement/2026-11-08/MR7-DEV-SWARM.md)
