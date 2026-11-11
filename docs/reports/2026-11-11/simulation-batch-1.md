# Simulation Batch 1 — SIM-01, SIM-02, SIM-03

- Date: 2026-11-11 (UTC)
- Status: Current; Batch 1 complete; human acceptance pending
- Model: `M1.3`, registry `2026-11-10.2`
- Partition: `strategy_dev` 2018–2022
- Label window: 2018–2019
- Check window: 2020–2022
- Claim type: exploratory hindsight labeling and descriptive checks; no edge claim

## 1. Runs

| Run | Premise | Label sample | Check | Verdict |
|---|---|---|---:|---|
| SIM-01 | The human marks carry a context split (EMA stack + ATR ratio) that generalises | 810 human entries | 1,292 filtered / 4,999 control | `no_pattern` |
| SIM-02 | A Fibonacci test becomes a trade only with a rejection wick of at least 0.30 | 48 windows, 202 marks | 1,902 long / 1,692 short filtered; 4,583 control | `no_pattern` |
| SIM-03 | The exit, not the entry, is the weak part of the Fib trade | 101 entries | 3,328 event entries | `no_exit_rescue` |

## 2. SIM-01 — meta-label the human marks

The 810 human entries were mapped to the M1.3 bars by timestamp and labelled by
their 50-bar forward return. Long: 368 good, 120 bad; short: 246 good, 76 bad.
The best context cell with at least 30 marks was short, bear EMA stack, ATR
ratio >= 1.0: n=38, mean 50-bar return +3.45%, hit 86.8%.

The 2020–2022 context screen rejected it: filtered mean net −0.12% (n=1,292,
hit 44.6%) versus control −0.09% (n=4,999, hit 45.6%), one positive half-year.
The cell is the best of 16 and does not survive the check.

Evidence: `data/simulation/2026-11-10/SIM-01-meta-label/meta.md`,
`meta-check.json`, `meta-marks.csv`.

## 3. SIM-02 — level-tap rejection

The 48 windows came from 192 non-overlapping `level.fib_test` candidates. The
context contrast was the rejection wick: good long lower-wick ratio 0.37 versus
0.26 for bad; good short upper-wick ratio 0.34 versus 0.27. EMA stack, RSI zone,
ADX, and ATR ratio overlapped.

The check rejected the 0.30 wick filter:

| Side | Filtered n | Filtered mean net | Filtered hit | Control n | Control mean | Control hit | Positive half-years |
|---|---:|---:|---:|---:|---:|---:|---:|
| Long | 1,902 | −0.0165% | 47.7% | 4,583 | −0.0433% | 47.3% | 3 |
| Short | 1,692 | −0.0747% | 45.4% | 4,583 | −0.0767% | 46.5% | 1 |

The filter improves the long control slightly but stays negative after 6 bps and
does not beat the short control.

Evidence: `data/simulation/2026-11-10/SIM-02-level-rejection/review.md`,
`check.md`, `charts/`, `contact-sheet.png`.

## 4. SIM-03 — exit lab

On the label set, the time exit was the least bad for long (+1.08% mean, 66.0%
hit); fixed 2R and EMA20 trail were smaller; the swing trail gave back more. On
the 2020–2022 check every exit was negative:

| Side | Best check rule | Mean net | Hit rate |
|---|---|---:|---:|
| Long | time | −0.02% | 50.2% |
| Short | time | −0.10% | 46.0% |

The fixed 2R bracket and the swing trail were worse. The 1R partial was close to
the fixed bracket. No exit turns the Fib entry positive; the entry remains the
weak part.

Evidence: `data/simulation/2026-11-10/SIM-03-exit-lab/exit-lab.md`,
`exit-lab.json`.

## 5. Findings

1. The human marks are well chosen in 2018–2019; the forward-path labels confirm
   it. Their context split does not generalise to 2020–2022.
2. The Fibonacci test plus a rejection wick is not a positive screen after
   costs. The wick is the only entry-time feature that separated the oracle's
   good and bad marks, and it is not enough.
3. The exit is not the missing edge. The time stop is the least bad rule on the
   check, but it is still negative; the fixed bracket and swing trail are worse.
4. All three runs point to the entry as the weak part. The next batch should
   change the entry hypothesis, not the exit or the filter thresholds.

## 6. Limits

- The marks and the good/bad labels use future bars by design. They are labels,
  not signals.
- The oracle is a heuristic; the AI visual review was a context review, not a
  full mark-by-mark correction.
- The 400-bar context chart is too wide for the rejection wick; the zoom panel is
  now rendered for the next batch.
- The checks are screens, not backtests. No validation (2024) or holdout (2025)
  data was read.
- The sample sizes are large for the event checks but the human-mark cells are
  small; the best cell was the best of 16, and the check rejected it.

## 7. Next

Record the human Batch 1 review. Then either:

- continue with Batch 2 (SIM-04 failed breakout/sweep, SIM-05 breakout retest,
  SIM-06 trend pullback), or
- stop the simulation campaign if the entry-first conclusion is accepted.

No strategy code, config, or baseline change is made from Batch 1.
