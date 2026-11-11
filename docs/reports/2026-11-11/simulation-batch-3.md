# Simulation Batch 3 — SIM-07, SIM-08, SIM-09, SIM-10

- Date: 2026-11-11 (UTC)
- Status: Current; Batch 3 complete; human acceptance pending
- Model: `M1.3`, registry `2026-11-10.2`
- Partition: `strategy_dev` 2018–2022
- Label window: 2018–2019
- Check window: 2020–2022
- Claim type: exploratory hindsight labeling and descriptive checks; no edge claim

## 1. Runs

| Run | Premise | Label sample | Check | Verdict |
|---|---|---|---:|---|
| SIM-07 | The first compression expansion continues | 32 windows, 122 marks | 208 long / 184 short filtered; 497 / 479 control | `no_pattern` |
| SIM-08 | A regular RSI divergence at a confirmed pivot reverses | 32 windows, 100 marks | 466 bullish long; 509 bearish short | **`candidate_screen`** (bull long); `no_pattern` (bear short) |
| SIM-09 | A volume climax or absorption reversal at VWAP/POC reverses | 19 windows, 82 marks | 5/12 climax; 1/0 absorption | `underpowered` |
| SIM-10 | The opening-range sweep and reclaim reverses | — | — | `blocked` (no session events) |

## 2. SIM-07 — compression first thrust

The long side was negative; the short side beat its control but did not reach
the hit-rate gate.

| Side | Filtered n | Filtered mean net | Filtered hit | Control mean | Control hit | Positive half-years |
|---|---:|---:|---:|---:|---:|---:|
| Long | 208 | −0.1180% | 37.5% | −0.1609% | 38.4% | 2 |
| Short | 184 | +0.0516% | 42.4% | −0.1060% | 39.9% | 3 |

No pattern.

## 3. SIM-08 — divergence at a confirmed pivot

This is the only survivor screen in the ten-run campaign.

| Side | n | Mean net 20-bar, 6 bps | Hit rate | Control mean | Control hit | Positive half-years |
|---|---:|---:|---:|---:|---:|---:|
| Bullish divergence, long | 466 | **+0.0786%** | **54.1%** | −0.0276% | 48.7% | 4 of 6 |
| Bearish divergence, short | 509 | −0.0684% | 50.5% | — | — | 2 of 6 |

The bullish event is `divergence.rsi_regular_bullish` (code 4, role `signal`).
The screen enters at the next bar open after the event's `available_bar_index`,
exits at the close 20 bars later, and subtracts 6 bps. The label sample did not
show a stable EMA, RSI-zone, ATR, or wick split; the event itself is the
candidate.

This is not an edge. The effect is small, there is no stop, target, sizing, or
fill model, and ten runs were tried. The hypothesis note is
[`sim08-bull-divergence-hypothesis.md`](sim08-bull-divergence-hypothesis.md);
the next step is a Strategy Research pre-registration, not strategy code.

## 4. SIM-09 — climax / absorption reversal

The events are too rare. The label window produced 19 windows from 19
candidates; the check produced 5 long / 12 short climax events and 1 long / 0
short absorption events. The 30-event floor is not lowered after seeing the
counts. No rule claim.

## 5. SIM-10 — opening-range sweep

Blocked. The M1.3 report has no session block, so
`signal.opening_range_sweep` (144) and its control (145) are absent in every
year 2018–2022. The run did not substitute `level.break` or synthesize the
event. A session-enabled report is a separate Requirements & Design / Model
Change task.

## 6. Campaign summary

| Verdict | Runs |
|---|---|
| `no_pattern` | SIM-01, SIM-02, SIM-04, SIM-05, SIM-06, SIM-07 |
| `no_exit_rescue` | SIM-03 |
| `candidate_screen` | SIM-08 bullish divergence long |
| `underpowered` | SIM-09 |
| `blocked` | SIM-10 |

The campaign found one candidate screen and no confirmed edge. The candidate is
small (+0.0786% net over 20 bars after 6 bps), was selected from ten runs, and
has not been tested with a stop, target, sizing, or validation data. The
multiple-testing correction is mandatory in any pre-registration.

## 7. Limits

- The marks and the good/bad labels use future bars by design.
- The checks are screens, not backtests. They do not model fills, slippage
  beyond the 6 bps proxy, position sizing, drawdown, or halts.
- No validation (2024) or holdout (2025) data was read.
- SIM-09 is underpowered; SIM-10 is blocked.
- The survivor is one of many checks; it may be chance.

## 8. Next

The human decides the model gate first. The divergence-at-pivot model programme
(`PROGRAM-2026-11-04-01`) is not accepted, so the strategy tier is closed for
this family. Open a Model Research pre-registration or retire the family. The
SIM-08 check used 2020–2022, including the 2021–2022 model-validation window;
the observation is a recorded limitation for any later model validation.

Do not tune the 20-bar horizon, do not run another threshold search on the same
event, and do not open 2024 from this report. No strategy code, config, or
baseline change is made from Batch 3.
