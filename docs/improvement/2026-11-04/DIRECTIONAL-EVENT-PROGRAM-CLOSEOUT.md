# Directional Event-Family Program — Closeout

- Date: 2026-11-04 (UTC)
- Status: **Stopped — no edge at the model-development gate**
- Program: five 15m event families, 2 ATR target, 1 ATR stop, 20-bar horizon
- Tier reached: `model_dev` only; `model_validation`, `strategy_dev`, and
  `strategy_validation` were not opened
- Strategy baseline: none
- Model baselines: unchanged; `MODEL-2026-11-04-04..05` remains accepted

## 1. Question

Does any of five directional event families — major pivot break, EMA50 test, DI
cross, MACD histogram flip, or trend continuation — reach a 2 ATR target before
a 1 ATR stop within 20 15m bars more often than its matched baseline and
controls on `model_dev` 2018–2020?

## 2. Results

| Family | Run | n | Clusters | Hit | Base | Lift | Lift LCB | Controls | Grade |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| Major pivot break | `MODEL-2026-11-04-13` | 14,650 | 4,425 | 0.3164 | 0.3089 | +0.0074 | −0.0027 | random p 0.0299; back-lag +0.0033; shuffle p 0.3582 | C |
| EMA50 test | `MODEL-2026-11-04-14` | 33,318 | 2,763 | 0.3029 | 0.3049 | −0.0020 | −0.0140 | random p 0.78; back-lag +1.51pp; shuffle p 0.62 | C |
| DI cross | `MODEL-2026-11-04-15` | 17,851 | 3,563 | 0.3121 | 0.3073 | +0.0048 | −0.0038 | random p 0.0796; shuffle p 0.7612 | C |
| MACD histogram flip | `MODEL-2026-11-04-16` | 60,328 | 74 | 0.3151 | 0.3073 | +0.0078 | −0.0126 | random p 0.0050; back-lag −0.19pp; shuffle p 0.3532 | C |
| Trend continuation | `MODEL-2026-11-04-17` | 4,118 | 2,895 | 0.3366 | 0.3105 | +0.0261 | +0.0063 | state 0.3203; incremental LCB −0.0035; back-lag +0.2128; random p 0.0050 | C |

All five cells are grade C. None produced a `selection`; therefore no model
validation was opened.

### Why the two positive cells are still no edge

- **Trend continuation** has a positive unconditional lift, but the trend-state
  baseline is 0.3203 and the incremental LCB is −0.0035. The event adds nothing
  significant beyond the state it claims to mark. The back-lag control is
  +0.2128: the same direction 20 bars before the event already had a much
  higher hit rate, so the event is late confirmation, not an incremental
  trigger. The adx20 sensitivity also fails the back-lag control.
- **MACD histogram flip** has 60,328 events but only 74 clusters. The events are
  so overlapping that the effective sample is tiny; the lift LCB is −0.0126 and
  the shuffle control fails.

## 3. Decision

- The program is **stopped**. No threshold, timeframe, target, stop, horizon,
  filter, or cost was changed after the result.
- No model validation, strategy development, strategy validation, or portfolio
  run is opened for these five families.
- No strategy baseline is accepted.
- The accepted model baselines are direction-neutral and remain valid. This
  result does not invalidate them.
- The five families are retired. They are not re-pre-registered as variants.
  A new program requires a new premise and a new human-approved charter.

## 4. Evidence

- Run records (local, under `data/improvement/2026-11-04/`):
  `MODEL-2026-11-04-13`, `-14`, `-15`, `-16`, `-17`.
- Candidate records:
  [`PIVOT-BREAK-15M-CANDIDATE.md`](PIVOT-BREAK-15M-CANDIDATE.md),
  [`EMA50-TEST-15M-CANDIDATE.md`](EMA50-TEST-15M-CANDIDATE.md),
  [`DI-CROSS-15M-CANDIDATE.md`](DI-CROSS-15M-CANDIDATE.md),
  [`MACD-HIST-15M-CANDIDATE.md`](MACD-HIST-15M-CANDIDATE.md),
  [`TREND-CONT-15M-CANDIDATE.md`](TREND-CONT-15M-CANDIDATE.md).
- Source commits before closeout:
  - `feat/pivot-break-15m` at `4e550a1`
  - `feat/ema50-test-15m` at `27371f9`
  - `feat/di-cross-15m` at `29412a0`
  - `feat/macd-hist-15m` at `3356ccd`
  - `feat/trend-cont-15m` at `d5aba92`
- Determinism: each run was rerun and the artifacts matched byte-for-byte; the
  candidate records state the observed hashes.

## 5. What remains

- The platform: data engine, model engine, backtest, execution, portfolio,
  analytics, partitions, costs, and accepted model baselines.
- The research platform built after this closeout: program manifest, cell
  runner, batch runner, aggregator, and registry.
- The next program starts from a new premise and a human-approved charter, not
  from another variation of these five families.

## 6. Limitations

- Development partition only; the barrier path is model evidence, not strategy
  P&L after costs.
- Overlapping 20-bar outcomes; cluster counts are the honest sample.
- Same-session implementation and review; no independent Quant Validator
  record.
- The five run records are local, not committed. They are preserved in the
  main checkout's `data/improvement/2026-11-04/`.
