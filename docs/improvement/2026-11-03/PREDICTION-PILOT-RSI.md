# PREDICTION-PILOT-RSI — EURUSD 1h

- Date: 2026-11-03 (UTC)
- Issue: [#359](https://github.com/quant-rule/quant-rule/issues/359)
- Plan: [`docs/execution-plans/2026-11-03/359-prediction-pilot.md`](../../execution-plans/2026-11-03/359-prediction-pilot.md)
- Tool: `tools/improvement/prediction_pilot.py`
- Local artifacts: `data/prediction-pilot/`
- Verdict: **stop — primary event failed development**
- Claim type: model prediction evidence; no strategy or edge claim

## Question

After RSI exits oversold (crosses 30 upward) or exits overbought (crosses 70
downward), is the next 1h return more often in the reversion direction than the
unconditional base rate?

## Data

EURUSD 1h. Development 2018–2020, validation 2021–2022. One-year model folds,
pooled per partition. MACD active but not used in this first test. The model gate
did not use EURUSD, so this is a new asset.

## Primary result

RSI 30/70 reversion event, next-1h return sign:

| Partition | n | Hit | Base | Lift | 95% LCB |
|---|---:|---:|---:|---:|---:|
| Development | 579 | 0.4663 | 0.5007 | −0.0344 | 0.4257 |
| Validation | 329 | 0.4590 | 0.4886 | −0.0297 | 0.4051 |

The primary event is below the base rate on both partitions. It does not
predict reversion; if anything, the crossing direction is contrarian, but the
result is not used to flip the sign after the fact.

## Secondary observation

The RSI **zone** state (all bars with RSI < 30 or > 70) is not the primary
pre-registered event. It shows a development-only positive at 3h:

| Partition | Horizon | n | Hit | Base | Lift | 95% LCB |
|---|---:|---:|---:|---:|---:|---:|
| Development | 3h | 2,002 | 0.5415 | 0.5031 | 0.0384 | 0.5196 |
| Validation | 3h | 1,224 | 0.5098 | 0.4913 | 0.0185 | 0.4818 |

The validation lower bound is below 0.50. This is not a confirmed prediction and
is not promoted. It is recorded as a secondary observation only.

## Decision

Stop. The primary RSI 30/70 reversion event failed development, so MACD is not
added to rescue it. No strategy is run. A new event definition or thesis is
required before the next step.

## Harness correction

The first harness run diluted the primary hit rate by counting non-reversion
30/70 crossings (30-down, 70-up) as misses. The event filter now keeps only the
reversion subset before counting. The corrected result is the one above. The
first run is not used as evidence.

## Limitations

- One asset, one timeframe, one period; a pilot, not a model baseline.
- Sign outcomes only; no costs or magnitude.
- Same-session analysis, not an independent Quant Validator record.
