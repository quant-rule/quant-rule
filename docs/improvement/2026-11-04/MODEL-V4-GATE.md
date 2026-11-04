# MODEL-V4-GATE — Combined Model v4 Decision

- Date: 2026-11-04 (UTC)
- Development run: [`MODEL-2026-11-04-01`](MODEL-2026-11-04-01-model-v4-dev.md)
- Validation run: none — not opened
- Protocol: [`docs/model/2026-11-04/path-validation-v4.md`](../../model/2026-11-04/path-validation-v4.md)
- Local artifacts: `data/improvement/2026-11-04/MODEL-2026-11-04-01/`
- Gate: **no edge**
- Claim type: model evidence; no strategy or edge claim

## Decision

No tested signal-role v4 event family passes the path gate on `model_dev`.

- 12 primary family-timeframe cells; all grade C.
- No cell passes the corrected lift bound and the state-conditioned incremental
  bound together.
- No cell passes the random, back-lag, and shuffled controls together.
- No family replicates across two instruments on a positive cell.
- The `adx20` sensitivity rows agree with `adx25` for H2, H3, and H4; H1 changes
  as expected and does not become incremental.

The validation partition is not opened. No strategy baseline may be built on
these events. No threshold, horizon, timeframe, or subset is selected after the
result.

## What this means

The v4 model computes its readings, states, levels, events, progression, and
charts. The failure is predictive validity for the tested event families, not a
calculation defect. The result agrees with the v3 gate and the earlier pilots:
on this data, the tested events are descriptive, not predictive.

The v4 result contract, configurable events, 1h ceiling, chart-ready result, and
path harness remain the implemented model surface. A future model thesis must be
a new, pre-registered hypothesis over candles-only, rule-based events; another
threshold or timeframe variant on the same sample is not a new hypothesis.

## Next step

The Improvement Workflow stops at “no edge”. The alternatives are a genuinely
new market thesis, a new dataset or regime, or stopping the loop. Strategy and
portfolio work do not start from this result.

## Limitations

- The gate measures a 20-bar ATR path outcome; it does not model costs,
  slippage, funding, or execution.
- Yearly folds are pooled per instrument; regime changes inside a partition are
  not separated.
- XAUUSD carries market-hours gaps; the path includes them.
- Same-session harness, review, and decision; no independent Quant Validator
  record.
