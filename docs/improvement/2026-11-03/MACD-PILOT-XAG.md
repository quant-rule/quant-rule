# MACD-PILOT-XAG — XAGUSD 1h

- Date: 2026-11-03 (UTC)
- Issue: [#361](https://github.com/quant-rule/quant-rule/issues/361)
- Plan: [`docs/execution-plans/2026-11-03/361-macd-pilot.md`](../../execution-plans/2026-11-03/361-macd-pilot.md)
- Tool: `tools/improvement/macd_pilot.py`
- Local artifacts: `data/macd-pilot/`
- Verdict: **stop — primary MACD cross failed development**
- Claim type: model prediction evidence; no strategy or edge claim

## Question

After a MACD line/signal cross on XAGUSD 1h, is the next-hour return more often
in the cross direction than the unconditional base rate?

## Data

XAGUSD 1h. Development 2018–2020, validation 2021–2022. Five one-year folds,
pooled per partition. MACD is active in the model config. No strategy.

## Primary result

MACD line/signal cross, next-1h return sign:

| Partition | n | Hit | Base | Lift | 95% LCB |
|---|---:|---:|---:|---:|---:|
| Development | 809 | 0.4586 | 0.5030 | −0.0444 | 0.4243 |
| Validation | 739 | 0.4831 | 0.4926 | −0.0095 | 0.4471 |

The primary event is below the base rate on both partitions. It does not
predict the next-hour direction.

## Secondary events

| Partition | Event | n | Hit | Base | Lift | 95% LCB |
|---|---|---|---:|---:|---:|---:|
| Development | MACD zero cross | 392 | 0.4541 | 0.5030 | −0.0489 | 0.4048 |
| Development | Histogram slope flip | 2,862 | 0.4752 | 0.5030 | −0.0278 | 0.4569 |
| Validation | MACD zero cross | 346 | 0.4509 | 0.4926 | −0.0418 | 0.3984 |
| Validation | Histogram slope flip | 2,646 | 0.4841 | 0.4926 | −0.0085 | 0.4651 |

No secondary event is promoted.

## Decision

Stop. MACD alone fails on the fresh asset. The RSI+MACD conjunction is not run;
adding RSI back would be an attempt to rescue a failed MACD event. No strategy is
run.

## Limitations

- One asset, one timeframe, one period; a pilot, not a model baseline.
- Sign outcomes only; no costs or magnitude.
- Same-session analysis, not an independent Quant Validator record.
