# MACD Histogram Flip 15m — Candidate Report

- Date: 2026-11-04 (UTC)
- Branch: `feat/macd-hist-15m`
- Status: Stopped at the model development gate
- Claim type: Development-only measurement; no validation, no edge

## Hypothesis

A MACD histogram slope flip marks momentum re-acceleration. The next 20 15m bars
reach +2 ATR in the flip direction before −1 ATR more often than the
unconditional base rate.

Event: type 3 (signal), subtype 4 (histogram slope flip), source 15 (MACD
histogram), 15m, direction ±1. The event is available on the bar it occurs.
This is distinct from the v4 gate's MACD line/signal cross (H2).

## Gate table

| Gate | Partition | Run | Result | Verdict |
|---|---|---|---|---|
| Model development | `model_dev` 2018–2020 | `MODEL-2026-11-04-16` | selection null; `adx25` grade C | fail |
| Model validation | `model_validation` 2021–2022 | not opened | — | not run |
| Strategy development | `strategy_dev` 2018–2022 | not opened | — | not run |
| Strategy validation | `strategy_validation` 2024 | not opened | — | not run |

### Model development detail

| Config | n | Clusters | Hit | Base | Lift | Lift LCB | Rand p | Back lag | Shuffle p | Rep | Grade |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| adx25 | 60,328 | 74 | 0.3151 | 0.3073 | +0.0078 | −0.0126 | 0.0050 | −0.0019 | 0.3532 | 3 | C |
| adx20 | 60,328 | 74 | 0.3151 | 0.3073 | +0.0078 | −0.0126 | 0.0050 | −0.0019 | 0.3532 | 3 | sensitivity |

Failed checks: `lift` (+0.78pp, below the 1.0pp minimum), `lift_lcb` (−1.26pp,
not above zero), and `shuffle` (p 0.3532). The pre-registered pass condition
required a non-null selection under `adx25` with grade A or B; the selection is
null and the grade is C.

Per instrument under `adx25`:

| Instrument | n | Hit | Base | Lift |
|---|---:|---:|---:|---:|
| BTCUSD | 22,977 | 0.3313 | 0.3166 | +0.0147 |
| ETHUSD | 18,086 | 0.3108 | 0.3092 | +0.0016 |
| XAUUSD | 19,265 | 0.2998 | 0.2944 | +0.0053 |

## Strategy table

No strategy backtest was run. The strategy config is pre-registered at
`config/macd-hist-15m/strategy/shared/`, but the model development gate failed,
so the strategy stage was not opened. There is no return, drawdown, trade count,
profit factor, or instrument table to report.

## Verdict

**No edge.** The MACD histogram slope flip on 15m does not clear the
pre-registered development gate. The effect is direction-insensitive under the
shuffle control, the lift is below the minimum, and its cluster-robust lower
bound is negative. The validation partition and the strategy partitions were not
opened.

The accepted v4 model baseline is not invalidated. This is a rejected event
family, not a model defect.

## Limitations

- The model gate is pre-cost; the strategy's 4 bps commission and 2 bps slippage
  would only worsen a gross lift that already fails.
- The 20-bar outcomes inside each cluster overlap; the cluster-robust standard
  error accounts for that, but the result is not a trade-level claim.
- The event is frequent (60,328 events over nine instrument-years), so the
  failure is not a small-sample artifact.
- `adx25` and `adx20` select the same event set, so the failure is not an
  ADX-threshold artifact.
- A byte-identical rerun with the same seed and inputs matched both artifact
  hashes, so the failure is not a nondeterministic draw.
- Same-session review and decision; no independent Quant Validator record.
- The gate was not rerun with a different seed, draw count, or parameter set.
  The only rerun was a byte-identical determinism check.

## Runs

| Run | Status | Location |
|---|---|---|
| `MODEL-2026-11-04-16` | executed, no edge | `data/improvement/2026-11-04/MODEL-2026-11-04-16/` |
| `MODEL-2026-11-04-21` | not opened | — |
| `STRATEGY-2026-11-04-07` | not opened | — |
| `STRATEGY-2026-11-04-12` | not opened | — |

## Next human decision

Accept the stop as `no edge`, or pre-register a different event-family
hypothesis. Do not change this brief's event, target, stop, horizon, timeframe,
filters, or costs after the result.
