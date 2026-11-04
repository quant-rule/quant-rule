# 5m Onset Barrier Path Protocol

- Date: 2026-11-04 (UTC)
- Status: **Executed; no directional edge; bracket program stopped**
- Issue: [#383](https://github.com/quant-rule/quant-rule/issues/383)
- Plan: [`docs/execution-plans/2026-11-04/383-barrier-path.md`](../../execution-plans/2026-11-04/383-barrier-path.md)
- Model baseline: `MODEL-2026-11-04-10..11` (accepted 5m fresh-onset range)
- Claim type: model/analytics diagnostic; no strategy or profitability claim.

## 1. Question

For the 5m fresh-onset event, does the path from the touched boundary reach a
+1, +2, +3, or +4 ATR target before a 1 ATR stop within 20 bars? Does the
momentum side or the fade side have a positive gross and net expectancy?

This is a path diagnostic, not a strategy. It decides whether a bracket at a
given target is worth a new pre-registered strategy.

## 2. Data

`model_dev` 2018-01-01..2020-12-31, BTCUSD, ETHUSD, XAUUSD, 5m, from the
accepted onset model bundles. No validation or holdout partition is used.

## 3. Event and entry population

The event is `signal.compression_range_onset` (type 3, subtype 9, source 5) at
the last bar of the first completed coiling window after a known non-coiling
bar.

For event bar `b`, the next bar `b+1` resolves the entry:

- Open at or above `range_high`: long entry at the open.
- Open at or below `range_low`: short entry at the open.
- Open inside the range and both boundaries touched: skip the event.
- Open inside the range and only the upper touched: long entry at `range_high`.
- Open inside the range and only the lower touched: short entry at `range_low`.
- Neither touched: no entry.

The entry is the momentum side. The fade side uses the same entry price and the
opposite direction. ATR is ATR14 at bar `b`.

## 4. Path

For each entry, the stop is 1 ATR from the entry. For target multiple `k` in
`{1, 2, 3, 4}`, the target is `k` ATR from the entry on the profitable side.
The path starts on the entry bar `b+1`:

- On the entry bar, the bar's high and low are evaluated from the entry; the
  stop wins a same-bar tie.
- On later bars, a gap at the open resolves before the intrabar range; the stop
  wins an intrabar tie.
- If neither level is reached within 20 bars, the outcome is a timeout marked at
  the close of the 20th bar.

The R multiple is `+k` for a target, `-1` for a stop, and the directional price
change divided by ATR for a timeout.

## 5. Costs

The gross R is the path result. The estimated cost is 12 bps of notional round
trip (4 bps commission per side plus 2 bps slippage per side), expressed in R as
`0.0012 * entry / ATR`. Net R is gross R minus cost R. The cost is an
approximation; the strategy tier remains the cost-aware test.

## 6. Estimator

For each side and target multiple:

- `n`, target hits, stop hits, timeouts, target rate, stop rate, timeout rate.
- Trade-weighted mean gross R, mean cost R, mean net R.
- Bundle-level means for the nine instrument-year bundles; the lower bound is
  `mean - 2.306 * stdev(bundle_means) / sqrt(9)` (95% t, 8 degrees of freedom).
- Per-instrument mean net R and event count.

The side/target is **promising** only when the net lower bound is above zero and
at least 2 of 3 instruments have positive mean net R.

## 7. Decision

If the momentum side is promising, recommend a stop-entry bracket at that
target. If the fade side is promising, recommend a limit-entry fade and note
that the execution engine does not yet support limit entries. If no side/target
is promising, record no directional edge and stop.

## 8. Boundaries

- The diagnostic is not a strategy result; it ignores account state and
  compounding.
- The bundle-level CI is descriptive, not a validated confidence guarantee.
- The cost estimate is an approximation in R.
- The run is a same-session analysis, not an independent Quant Validator record.

## 9. Result

No directional edge. No side/target is promising.

| Side | Target | n | Target rate | Stop rate | Gross R | Cost R | Net R | Net LCB | Positive |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| momentum | 1 | 10,349 | 47.62% | 51.59% | −0.0405 | 1.2116 | −1.2521 | −2.0575 | 0/3 |
| momentum | 2 | 10,349 | 31.54% | 63.80% | +0.0101 | 1.2116 | −1.2015 | −2.0297 | 0/3 |
| momentum | 3 | 10,349 | 21.02% | 69.06% | +0.0121 | 1.2116 | −1.1995 | −2.0272 | 0/3 |
| momentum | 4 | 10,349 | 14.55% | 71.27% | +0.0185 | 1.2116 | −1.1930 | −2.0356 | 0/3 |
| fade | 1 | 10,349 | 47.59% | 51.59% | −0.0393 | 1.2116 | −1.2509 | −2.0426 | 0/3 |
| fade | 2 | 10,349 | 29.76% | 63.25% | −0.0004 | 1.2116 | −1.2120 | −2.0137 | 0/3 |
| fade | 3 | 10,349 | 19.38% | 67.04% | +0.0277 | 1.2116 | −1.1839 | −1.9931 | 0/3 |
| fade | 4 | 10,349 | 12.26% | 68.54% | +0.0300 | 1.2116 | −1.1815 | −1.9914 | 0/3 |

The 1 ATR target hit rate is 47.6% against a 50% break-even; the 2 ATR target is
31.5% against 33.3%. The path is close to a symmetric random walk. The 12 bps
cost is about 1.21 R per trade, so no small gross edge survives.

The compression bracket program stops. No further target/stop, timeframe, or
filter variants are supported.

- Gate: [`BARRIER-PATH-GATE`](../../improvement/2026-11-04/BARRIER-PATH-GATE.md)
- Diagnostic: [`MODEL-2026-11-04-12`](../../improvement/2026-11-04/MODEL-2026-11-04-12-barrier-path.md)
