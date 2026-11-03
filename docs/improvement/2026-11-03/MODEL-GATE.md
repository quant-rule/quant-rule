# MODEL-GATE — Combined Model Decision

- Date: 2026-11-03 (UTC)
- Development run: [`MODEL-2026-11-03-01`](MODEL-2026-11-03-01-model-gate-dev.md)
- Validation run: [`MODEL-2026-11-03-02`](MODEL-2026-11-03-02-model-gate-validation.md)
- Protocol: [`docs/model/2026-11-03/claim-validation.md`](../../model/2026-11-03/claim-validation.md)
- Local artifacts: `data/improvement/2026-11-03/MODEL-GATE/`
- Gate: **no edge**
- Claim type: model evidence; no edge claim

## Decision

No signal-role model state or event passes the gate on both partitions.

- Development: 48 primary cells, all C.
- Validation: 47 C and one B (`divergence.regular_bullish` 5m).
- The validation-only B does not replicate the development result, which was C.
- Passing families on both partitions: none.

The model is descriptive, not predictive, on the tested data and horizons. No
strategy baseline may be built on these states or events. No threshold, horizon,
or subset is selected after the result.

## What this means

The v3 model correctly computes its readings, states, levels, events, and
progression. The failure is predictive validity, not a calculation defect. The
new MACD family, the level events, the divergence family, and the signal events
all fail the same test. The earlier axis evidence and this gate agree.

## Next step

The Improvement Workflow may stop at “no edge”. The alternatives are a genuinely
new market thesis, a new dataset or regime, or stopping the loop. Another
threshold or timeframe variant on the same sample is not a new hypothesis and is
not authorised by this result.

## Limitations

- The gate measures sign outcomes at fixed horizons; it does not model magnitude
  or trading costs.
- Yearly folds are pooled per instrument.
- The same-session harness, run, and review are sequential passes; no independent
  Quant Validator record exists.
