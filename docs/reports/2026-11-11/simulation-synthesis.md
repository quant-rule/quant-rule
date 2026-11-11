# Simulation Campaign — Synthesis

- Date: 2026-11-11 (UTC)
- Status: Current; ten-run campaign complete; human acceptance pending
- Model: `M1.3`, registry `2026-11-10.2`
- Partition: `strategy_dev` 2018–2022; label 2018–2019; check 2020–2022
- Claim type: exploratory campaign; one candidate screen; no edge claim

## 1. Result

| Run | Proposition | Verdict |
|---|---|---|
| SIM-01 | Meta-label the human marks | `no_pattern` |
| SIM-02 | Level-tap rejection | `no_pattern` |
| SIM-03 | Exit lab | `no_exit_rescue` |
| SIM-04 | Failed breakout / reclaim | `no_pattern` |
| SIM-05 | Breakout retest hold | `no_pattern` |
| SIM-06 | Trend pullback | `no_pattern` |
| SIM-07 | Compression first thrust | `no_pattern` |
| SIM-08 | Divergence at a confirmed pivot | **`candidate_screen`** (bull long); `no_pattern` (bear short) |
| SIM-09 | Climax / absorption reversal | `underpowered` |
| SIM-10 | Opening-range sweep | `blocked` (no session events) |

## 2. The candidate

`divergence.rsi_regular_bullish` (code 4, role `signal`), long only:

- entry at the next bar open after the event's `available_bar_index`;
- exit at the close 20 bars later;
- 6 bps round trip;
- 2020–2022: n=466, mean net +0.0786%, hit 54.1%, versus the all-bars control
  −0.0276% and 48.7%; 4 of 6 half-years positive.

The effect is small. The screen has no stop, target, sizing, fill model,
drawdown limit, or trade floor. It is one survivor from ten runs; the
multiple-testing correction is mandatory. The hypothesis note is
[`sim08-bull-divergence-hypothesis.md`](sim08-bull-divergence-hypothesis.md).

## 3. What the campaign established

- The human marks are well chosen in 2018–2019, but their context split does not
  generalise to 2020–2022 (SIM-01).
- The Fibonacci test plus a rejection wick is not positive after costs
  (SIM-02); the exit does not rescue the entry (SIM-03).
- Failed breaks, retest holds, trend pullbacks, and compression first thrusts do
  not produce a positive 20-bar screen on 2020–2022 (SIM-04, SIM-05, SIM-06,
  SIM-07).
- The climax and absorption events are too rare for the sample floor (SIM-09).
- The opening-range event is unavailable in the recorded M1.3 bundle (SIM-10).
- The only survivor is the bullish RSI divergence long. It is a screen, not an
  edge.

## 4. Limits

- Hindsight labels and forward-path checks; no fill model.
- The checks use a 6 bps proxy and a 20-bar horizon; they are not backtests.
- Ten runs were tried; the survivor may be chance.
- No validation (2024) or holdout (2025) data was read.
- SIM-09 is underpowered; SIM-10 is blocked.

## 5. Next gate

1. The human accepts or rejects the SIM08-BULL-DIV hypothesis note.
2. If accepted, the Quant Researcher writes a Strategy Research pre-registration
   with the exact rule, stop, target, time stop, costs, partitions, green
   criteria, multiple-testing correction, and iteration cap.
3. The Quant Validator runs the feasibility preflight.
4. The Quant Developer implements the strategy config only after approval.
5. The strategy tier remains subject to the model-baseline gate and the normal
   validation rules. Do not open 2024 from this report.

## 6. Records

- [`simulation-batch-1.md`](simulation-batch-1.md)
- [`simulation-batch-2.md`](simulation-batch-2.md)
- [`simulation-batch-3.md`](simulation-batch-3.md)
- [`sim08-bull-divergence-hypothesis.md`](sim08-bull-divergence-hypothesis.md)
- Local artifacts: `data/simulation/2026-11-10/SIM-01..SIM-10/`
