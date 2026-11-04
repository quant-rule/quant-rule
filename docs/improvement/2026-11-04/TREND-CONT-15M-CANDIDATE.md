# Trend Continuation 15m — Candidate Record

- Date: 2026-11-04 (UTC)
- Brief: [`docs/execution-plans/2026-11-04/trend-cont-15m.md`](../../execution-plans/2026-11-04/trend-cont-15m.md)
- Branch: `feat/trend-cont-15m`
- Claim type: development gate evidence for one pre-registered strategy
  hypothesis; no edge and no baseline
- Verdict: **no edge — stopped at the model development gate**

## Hypothesis

The first bar of the trend-continuation conjunction — ADX in its trend regime
and rising, DI has a direction, the EMA stack agrees, and the MACD state agrees
— predicts a +2 ATR move in the event direction before −1 ATR within 20 bars
more often than the trend-state base rate. If it does, the event is entered in
the event direction with a 1 ATR stop, a 2 ATR target, and a 20-bar time stop,
after 4 bps commission per operation and 2 bps slippage.

Prior evidence is disclosed: the v4 gate tested the same family (H1) at 1 ATR
target / 1 ATR stop and it failed on 5m, 15m, and 1h. This brief was a retest
with a new pre-registered payoff (2 ATR / 1 ATR); nothing else changed.

Event: type 3 (signal), subtype 6 (trend continuation), source 16 (ADX), 15m,
direction ±1, emitted once at conjunction onset and available on that bar.

## Gate table

| Gate | Partition | Run ID | Result | Verdict |
|---|---|---|---|---|
| Model development | `model_dev` 2018–2020 | `MODEL-2026-11-04-17` | 4,118 events, 2,895 clusters, lift +2.61pp, incremental LCB −0.35pp, back lag +21.28pp, grade C, `selection: null` | fail — stop |
| Model validation | `model_validation` 2021–2022 | not opened | — | not run |
| Strategy development | `strategy_dev` 2018–2022 | not created | — | not run |
| Strategy validation | `strategy_validation` 2024 | not created | — | not run |

Only `MODEL-2026-11-04-17` was created. The other three run records were not
created because their gates never opened; a run folder with no run would be an
invalid record.

## Model development detail

Primary `adx25` cell, 15m, 20-bar horizon, 2 ATR target / 1 ATR stop:

| n | Clusters | Hit | Base | Lift | Lift LCB | State base | Incremental | Incremental LCB | Rand p | Back lag | Shuffle p | Rep | Grade |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 4,118 | 2,895 | 0.3366 | 0.3105 | +0.0261 | +0.0063 | 0.3203 | +0.0163 | −0.0035 | 0.0050 | +0.2128 | 0.0050 | 3 | C |

`adx20` sensitivity cell: 5,180 events, hit 0.3440, base 0.3100, lift +0.0340,
incremental +0.0215, incremental LCB +0.0036, back lag +0.1634.

Per instrument, `adx25`:

| Instrument | n | Hit | Base | Lift |
|---|---:|---:|---:|---:|
| BTCUSD | 1,546 | 0.3519 | 0.3188 | +0.0331 |
| ETHUSD | 1,268 | 0.3470 | 0.3159 | +0.0311 |
| XAUUSD | 1,304 | 0.3083 | 0.2953 | +0.0130 |

The unconditional lift is positive and replicated, but the event is not
incremental over the trend state and the back-lag control is much stronger than
the matched base. The harness selects no candidate.

## Strategy table

No strategy run. The model gate is a precondition for `strategy_dev`, and the
pre-registered stop condition ends the brief at the first failed gate.

| Matrix | Partition | Folds | Run ID | Average return | Worst DD | Trades | Pooled PF | Positive instruments | Verdict |
|---|---|---|---|---|---|---|---:|---:|---|
| not run | `strategy_dev` | 2018–2022 | not created | — | — | — | — | — | not run |
| not run | `strategy_validation` | 2024 | not created | — | — | — | — | — | not run |

## Verdict

**No edge.** The 2 ATR / 1 ATR / 20-bar payoff did not turn the trend
continuation family into a candidate. The development cell is grade C: its
incremental lower bound is below zero and the back-lag control exceeds its
matched base. The model validation partition is not opened, no strategy run is
performed, and no baseline is accepted.

This is a stop, not a tuning result. No target, stop, horizon, timeframe, or
filter was changed after the result.

## Limitations

- The model gate measures barrier hits, not strategy returns; costs, fills, and
  portfolio accounting were never reached.
- One-year folds are pooled per instrument; the entry is the event close.
- XAUUSD has market-hours gaps; the 20-bar window includes them.
- The `sensitivity` check is reported as `event_set_changed` because the family
  id is `trend_continuation_15m`; the harness's H1-specific sensitivity rule is
  keyed to `h1_trend_continuation`. The verdict does not depend on that check:
  `incremental_lcb` and `back_lag` fail independently.
- Same-session implementation, review, and decision; no independent Quant
  Validator record and no human acceptance.

## Next human decision

Retire the trend-continuation 2 ATR hypothesis as tested, or pre-register a
different family or payoff as a new brief. No further variation of this brief
is permitted.
