# SR12 Charter — Failed Break/Reclaim and Band Fade

- Date: 2026-11-10 (UTC)
- Program ID: `PROGRAM-2026-11-10-03`
- Status: **Completed — H1 engine-blocked; H2 no candidate; 2024 closed**
- Tier: strategy
- Model: `M1.2` (development-only exception)
- Predecessor: [`SR11`](../../improvement/2026-11-10/SR11.md) — stopped; no raw edge
- Claim type: strategy research; no edge claim.

## 1. Premise

SR11 showed that the EMA/Fibonacci tap had no raw forward edge and that the
`ema_stack`/`structure_state` filter did not add value. Two mechanically
different premises are screened together:

- **H1 — failed break and reclaim.** A break of a confirmed major pivot that is
  reclaimed is a failed breakout; the reclaim should continue away from the
  pivot.
- **H2 — band fade.** A test of a multi-touch horizontal band is followed by a
  reaction away from the band; a short time stop captures the reaction.

The two hypotheses are tested in one pre-registered batch with a shared control
and a declared multiple-testing correction. They are not variants of SR11.

## 2. H1 rule

Confirmed major pivot (`source 8`):

- long: `level.break` direction −1 (close below the pivot by `break_atr` for two
  bars) and `level.cross` direction +1 (close back above) within a 2-bar window;
- short: `level.break` direction +1 and `level.cross` direction −1.

The event-composite engine does not enforce event order inside the window. The
signal is the conjunction within two bars. This limitation is recorded; a
sequence-aware engine is a follow-up only if the raw edge is positive.

Exits: structure flip (`structure_state = −1` long, `+1` short) or a 20-bar time
stop. Risk: 1 ATR stop, 2 ATR target.

Cells: H1.1 long structure, H1.2 short structure, H1.3 long time, H1.4 short
time.

## 3. H2 rule

Horizontal band (`source 57`):

- long: `level.band_test` direction +1 (support test; close at or above the band
  center);
- short: `level.band_test` direction −1 (resistance test).

Exits: time stop 8 bars or 12 bars, with a protective 1 ATR stop and 2 ATR
target.

Cells: H2.1 long 8, H2.2 short 8, H2.3 long 12, H2.4 short 12.

## 4. Frozen inputs

| Field | Value |
|---|---|
| Instrument | Dukascopy BTCUSD 15m |
| Development | `strategy_dev` 2018-01-01..2022-12-31 |
| Validation | 2024; closed |
| 2023 | excluded |
| Model ladder | 15m only; 1m source retained |
| Costs | 6 bps base; 9 bps sensitivity |
| Account | $10,000 per cell; 1x; one position |
| Drawdown | 5% per cell, hard halt |

Sizing is frozen. The $1,000 / 0.1% question is a portfolio-tier study and is not
part of this strategy batch.

## 5. Stage 1 — raw-edge screen

Before the full matrix, screen every cell over 2018–2022 with no stop, target,
halt, or position state:

- entry at the next bar open;
- forward returns and MFE/MAE at 1, 3, 5, 10, and 20 bars;
- H1 barrier 2:1 over 20 bars; H2 barrier 2:1 over 12 bars;
- controls: H1 break-only and cross-only; H2 all bars;
- 6 bps round trip.

A cell stops before the full matrix when its non-overlapping mean R point
estimate is negative and the 95% interval excludes a positive edge. A stopped
cell counts as a trial in the correction.

## 6. Stage 2 — full matrix

Survivors only. Eight primary cells at 6 bps, then 9 bps sensitivity. Criteria
per cell:

- at least 30 trades over 2018–2022;
- PF > 1.1 at 6 bps;
- PF > 1.0 at 9 bps;
- max drawdown ≤ 5%;
- no halt.

The portfolio stays frozen at 1x, $10,000, and the 5% halt. If the halt binds,
it is recorded; the strategy is not retuned. Sizing is a separate portfolio-tier
study after a strategy candidate exists.

## 7. Multiple testing and stop rule

Eight primary cells; sixteen planned runs with the 9 bps sensitivity. Bonferroni
correction at family alpha 0.05: 0.05 / 8 = 0.00625 for the primary cells. The
raw-edge screen is a filter; its stopped cells remain in the family.

Stop when a cell is underpowered, fails a criterion, or the data coverage is
inadequate. A failed hypothesis is not re-registered with a different threshold,
stop, target, timeframe, or filter.

## 8. Human approval

- Premise: **Approved**
- Model-baseline exception: **Approved**
- Development access: **Approved**
- Validation access: **Closed**
- Approved by: human, 2026-11-10 (“okay understood you can run the corrected
  plan”)

## 9. Result

H1 was blocked before the matrix: the compiled strategy registry does not expose
major-pivot sources, and the engine rejects opposite-direction entry events. Its
VWAP screen had negative 20-bar net returns and did not beat the cross-only
control.

H2 ran all 40 year-cells. Every year-cell halted. The long 12-bar cell was the
closest at PF 1.072 (6 bps) and 1.022 (9 bps), but it still failed PF > 1.1,
max drawdown ≤5%, and no halt. The short cells were negative. No candidate.
Full record: [`SR12`](../../improvement/2026-11-10/SR12.md).

A follow-up portfolio-tier sizing diagnostic on the long 12-bar cell found a
full-sample PF of 0.81–0.87 at a 6 bps effective cost for 0.5x and 0.2x notional.
The 1x PF 1.072 was a halt-truncated artifact. The strategy line is stopped.
Charter: [`SR12-SIZING.md`](SR12-SIZING.md).

## 10. Artifacts

| Artifact | Path |
|---|---|
| Edge specs | `config/sr12/edge/` |
| Screen output | `data/improvement/2026-11-10/SR12/screen/` |
| Strategy configs | `config/sr12/strategy/` |
| Result record | `docs/improvement/2026-11-10/SR12.md` |
| AI log | `docs/ai-transparency-log/2026-11-10/sr12-event-screen.md` |
