# Simulation Batch 2 — SIM-04, SIM-05, SIM-06

- Date: 2026-11-11 (UTC)
- Status: Current; Batch 2 complete; human acceptance pending
- Model: `M1.3`, registry `2026-11-10.2`
- Partition: `strategy_dev` 2018–2022
- Label window: 2018–2019
- Check window: 2020–2022
- Claim type: exploratory hindsight labeling and descriptive checks; no edge claim

## 1. Runs

| Run | Premise | Label sample | Check | Verdict |
|---|---|---|---:|---|
| SIM-04 | A level break that closes back inside reverses; the reclaim is the trade | 32 windows, 116 marks | 419 long / 442 short filtered; 2,092 / 2,101 control | `no_pattern` |
| SIM-05 | A compression breakout that retests and holds continues | 32 windows, 122 marks | 137 long / 138 short filtered; 241 / 250 control | `no_pattern` |
| SIM-06 | In an aligned trend with ADX rising, the first pullback continues | 32 windows, 100 marks | 687 long / 729 short; filter equals control | `no_pattern` |

## 2. SIM-04 — failed breakout / reclaim

The label sample suggested a rejection shape: good short marks had a larger
upper wick (0.29 vs. 0.11) and a smaller body (0.48 vs. 0.65); good long marks
closed lower in the bar (0.32 vs. 0.56). The check used a one-bar confirmation
and a close back inside the level.

| Side | Filtered n | Filtered mean net | Filtered hit | Control n | Control mean | Control hit | Positive half-years |
|---|---:|---:|---:|---:|---:|---:|---:|
| Long | 419 | +0.0225% | 50.4% | 2,092 | −0.0289% | 51.9% | 1 |
| Short | 442 | −0.1468% | 43.2% | 2,101 | −0.0959% | 47.9% | 1 |

The long side is slightly better than its control but its hit rate is below the
control and only one half-year is positive. The short side is worse than its
control. No pattern.

## 3. SIM-05 — breakout retest hold

The label sample had no clean context split. The check used the event's
direction and an RVOL filter of at least 1.0.

| Side | Filtered n | Filtered mean net | Filtered hit | Control n | Control mean | Control hit | Positive half-years |
|---|---:|---:|---:|---:|---:|---:|---:|
| Long | 137 | −0.1294% | 40.1% | 241 | −0.1664% | 38.6% | 2 |
| Short | 138 | +0.0486% | 42.0% | 250 | −0.0561% | 39.6% | 3 |

The short side beats its control but the hit rate is 42.0%, below the
pre-registered 50% gate. The long side is negative. No pattern.

## 4. SIM-06 — trend pullback

The event already encodes the EMA-stack and ADX agreement, so the EMA-stack
filter used in the check is identical to the event at the entry bar. The
filtered set and the control are the same set.

| Side | n | Mean net | Hit rate |
|---|---:|---:|---:|
| Long | 687 | −0.0092% | — |
| Short | 729 | −0.0163% | — |

Both sides are negative after 6 bps. The event does not produce a positive
screen. No pattern.

## 5. Findings

1. Six runs are complete: SIM-01, SIM-02, SIM-04, SIM-05, and SIM-06 are
   `no_pattern`; SIM-03 is `no_exit_rescue`.
2. The entry families tested so far do not produce a positive 20-bar screen
   after 6 bps on 2020–2022. The best result is SIM-04 long (+0.0225%) and
   SIM-05 short (+0.0486%); neither meets the hit-rate and half-year gates.
3. The context filters either do not separate the label marks or do not
   generalise. The 2020–2022 check is doing its job: it rejects the label
   sample's apparent shapes.
4. The exit is not the missing edge (SIM-03). The remaining work is a different
   entry premise, not another threshold on the same families.

## 6. Limits

- The marks and the good/bad labels use future bars by design.
- The oracle is a heuristic; the charts were rendered with the zoom panel but
  the AI review was a context-table review, not a mark-by-mark correction.
- The checks are screens, not backtests. No validation (2024) or holdout (2025)
  data was read.
- SIM-06's filter is the event itself; the check is a direction-only event
  study, not a filter comparison.

## 7. Next

Batch 3 (SIM-07 compression first thrust, SIM-08 divergence at a confirmed
pivot, SIM-09 climax/absorption, SIM-10 opening-range sweep) remains. The
evidence now points to a stop-and-redesign decision rather than more runs on the
same entry families. The human owns that decision.

No strategy code, config, or baseline change is made from Batch 2.
