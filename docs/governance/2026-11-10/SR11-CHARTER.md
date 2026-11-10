# SR11 Charter — 15m Trend-Aligned EMA/Fibonacci Entry

- Date: 2026-11-10 (UTC)
- Program ID: `PROGRAM-2026-11-10-02`
- Status: **Completed — no candidate; EMA/Fibonacci line stopped; 2024 closed**
- Tier: strategy
- Model: `M1.2` (development-only exception)
- Predecessor: [`SR10`](../../improvement/2026-11-10/SR10.md) — stopped early; no candidate
- Result: [`SR11`](../../improvement/2026-11-10/SR11.md) — all four cells failed; stopped
- Claim type: strategy research; no edge claim.

## 1. Premise

The SR10 entry was too permissive: the 3-bar event window allowed repeated
entries in the same cluster, and ten of the nineteen 2018 giveback exits were
stops. A same-bar EMA/Fibonacci entry with confirmed trend alignment should
remove the countertrend cluster and improve the cost-adjusted result.

This is a new hypothesis, not an SR10 variant. The SR10 result is not validation
evidence for SR11.

## 2. Entry rule

Long entry, all conditions on the same 15m bar:

- `level.fib_test` direction +1 (up-leg retracement);
- `level.test` source 1 (fast EMA), direction 0;
- `ema_stack = +1`;
- `structure_state = +1`.

Short entry mirrors: Fibonacci direction −1, EMA fast test, `ema_stack = −1`,
`structure_state = −1`.

The event window is **1 bar**. The rule is a deterministic conjunction of two
events and two published states.

## 3. Exits

| Cell | Side | Exit |
|---|---|---|
| `SR11.1` | long | `structure_state = −1` |
| `SR11.2` | short | `structure_state = +1` |
| `SR11.3` | long | `signal.macd_histogram_slope_flip` direction −1 |
| `SR11.4` | short | `signal.macd_histogram_slope_flip` direction +1 |

All cells use a 1 ATR stop, a 3 ATR target, and a 50-bar time stop. The giveback
exit is not a primary cell; it remains an SR10 result.

## 4. Frozen inputs

| Field | Value |
|---|---|
| Instrument | Dukascopy BTCUSD 15m (formal `strategy_dev`) |
| Robustness | Binance BTCUSDT 15m, descriptive only |
| Development | `strategy_dev` 2018-01-01..2022-12-31 |
| Validation | 2024; closed |
| 2023 | excluded |
| Costs | 6 bps base; 9 bps sensitivity |
| Account | $10,000 per cell; fixed notional; 1x; one position |
| Drawdown | 5% per cell, hard halt |
| Time stop | 50 bars |

## 5. Criteria

Each primary cell:

- at least 30 trades over 2018–2022;
- PF > 1.1 at 6 bps;
- PF > 1.0 at 9 bps;
- max drawdown ≤ 5%;
- no halt.

The structure cells are the controls for the force-loss cells. The force-loss
cells are variants, not replacements selected after a result. A cell that passes
does not promote the break-only control.

## 6. Multiple testing and stop rule

Four primary cells; eight planned trials (four base, four sensitivity). Bonferroni
correction at family alpha 0.05. The SR10 no-filter and giveback results are
descriptive and are not counted as trials.

Stop when a cell is underpowered, fails a criterion, or the data coverage is
inadequate. A failed hypothesis is not re-registered with a different threshold,
stop, target, timeframe, or filter.

## 7. Human approval

- Premise: **Approved**
- Model-baseline exception: **Approved**
- Development access: **Approved**
- Validation access: **Closed**
- Approved by: human, 2026-11-10 (“approve proceed”)

## 8. Result

The full matrix ran: 40 year-cells (four cells × 2018–2022 × 6/9 bps), all
completed. Every cell failed PF, drawdown, and no-halt criteria. At 6 bps:

| Cell | Trades | PF | Payoff | Win rate | Max DD | Halted |
|---|---:|---:|---:|---:|---:|---:|
| SR11.1 long structure | 123 | 0.680 | 2.31 | 22.8% | 5.91% | 5/5 |
| SR11.2 short structure | 86 | 0.692 | 2.14 | 24.4% | 7.03% | 5/5 |
| SR11.3 long force | 174 | 0.597 | 1.61 | 27.0% | 5.33% | 5/5 |
| SR11.4 short force | 111 | 0.453 | 0.98 | 31.5% | 7.86% | 5/5 |

Every cell's win rate is below the breakeven rate implied by its realized payoff.
The hypothesis is stopped and is not re-registered with a different parameter.
The 2024 validation partition was not opened.

The raw-edge diagnostic removed the exits and the halt: 20-bar net was −0.0502%
long and −0.0188% short at 6 bps, the non-overlapping intervals include zero, and
the full signal did not beat the event-only, states-only, or unconditional
controls. The EMA/Fibonacci line is stopped. Full report:
[`sr11-raw-edge.md`](../../reports/2026-11-10/sr11-raw-edge.md).

## 9. Artifacts

| Artifact | Path |
|---|---|
| Strategy configs | `config/sr11/strategy/` |
| Run record | `data/improvement/2026-11-10/SR11/` |
| Result record | `docs/improvement/2026-11-10/SR11.md` |
| AI log | `docs/ai-transparency-log/2026-11-10/sr11-ema-fib.md` |
