# SIM08-BULL-DIV — Strategy Research Hypothesis Note

- Date: 2026-11-11 (UTC)
- Status: Proposed; not pre-registered, not a strategy
- Source run: `SIM-08` (`data/simulation/2026-11-10/SIM-08-divergence/`)
- Model: `M1.3`, registry `2026-11-10.2`
- Partition used: `strategy_dev` 2018–2022
- Label window: 2018–2019; check window: 2020–2022
- Claim type: hypothesis note from a screen; no edge claim
- Human approval: pending

## 1. Rule under consideration

- **Event:** `divergence.rsi_regular_bullish` (code 4, role `signal`).
- **Entry:** at the next bar open after the event's `available_bar_index`.
- **Exit for the screen:** at the close 20 bars after entry.
- **Cost:** 6 bps round trip.
- **Side:** long only. The bearish mirror was negative and is not part of the
  hypothesis.

The screen has no stop, target, sizing, or fill model. Those are declared in the
Strategy Research pre-registration, not here.

## 2. Evidence

The check is an event study against an all-bars control on 2020–2022:

| Measure | Bullish divergence long | All-bars control |
|---|---:|---:|
| Non-overlapping events | 466 | 4,998 |
| Mean net 20-bar return, 6 bps | +0.0786% | −0.0276% |
| Hit rate | 54.1% | 48.7% |
| Positive half-years | 4 of 6 | — |

Half-years: 2020H1 +0.0979%, 2020H2 +0.1177%, 2021H1 −0.1394%, 2021H2
+0.2741%, 2022H1 −0.1320%, 2022H2 +0.2254%.

The label sample (32 windows, 100 marks) did not show a stable EMA-stack,
RSI-zone, ATR, or wick split; the event itself is the candidate. The bearish
divergence short was negative on the same check and is rejected.

Evidence: `data/simulation/2026-11-10/SIM-08-divergence/check-bull-bars/check.md`
and `check-bear/check.md`.

## 3. Limitations

- The effect is small: +0.0786% net over 20 bars after 6 bps.
- The screen has no stop, target, sizing, fill model, drawdown limit, or trade
  floor. It is not a backtest.
- The check is one window (2020–2022), not validation. 2024 remains closed.
- Ten simulation runs and several checks were tried; the survivor is a
  multiple-testing candidate. The pre-registration must declare the correction
  and the primary-cell count.
- The event's own claim is in the registry; the hypothesis does not add a new
  measurement or a fitted field.
- The label sample is 2018–2019; the check is 2020–2022. Both are
  `strategy_dev`; no validation or holdout data was read.

## 4. Required next steps

1. Human accepts or rejects the hypothesis note.
2. If accepted, the Quant Researcher writes a Strategy Research pre-registration
   with the exact rule, stop, target, time stop, costs, partitions, green
   criteria, multiple-testing correction, and iteration cap.
3. The Quant Validator runs the feasibility preflight before any backtest.
4. The Quant Developer implements the strategy config only after the
   pre-registration is approved; no code is written from this note.
5. The strategy tier remains subject to the model-baseline gate and the normal
   validation rules.

Do not open 2024, do not tune the 20-bar horizon after seeing the check, and do
not run another threshold search on the same event.
