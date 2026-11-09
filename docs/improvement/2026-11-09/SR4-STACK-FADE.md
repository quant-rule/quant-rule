# SR4-STACK-FADE — Multi-Instrument Stack-Aligned 1h Fade

- Date: 2026-11-09 (UTC)
- Status: **No candidate — instrument-specific; S1 fails the 2-of-3 instrument and 5% drawdown gates; 2024 not opened**
- Charter: [`SR4-CHARTER.md`](../../governance/2026-11-09/SR4-CHARTER.md)
- Plan: [`sr4-stack-fade.md`](../../execution-plans/2026-11-09/sr4-stack-fade.md)
- Predecessor: [`SR3-FEATURES.md`](../2026-11-08/SR3-FEATURES.md)
- Run record: `data/improvement/2026-11-09/SR4-STACK-FADE/`
- Claim type: strategy research; no edge or profitability claim

## 1. Cells

Six variants (three rules × two sides) × three instruments × five years = 90
base cells. S1 is the primary rule; S2 and S3 are controls. The event is
`signal.compression_breakout`; the fade side is opposite the event. Stop 1 ATR,
target 3 ATR, time stop 20 bars.

| Variant | Rule | Primary? |
|---|---|---|
| S1 stack | Event direction equals the 1h EMA stack; no ADX or RVOL filter | yes |
| S2 stack+RVOL | S1 plus `rvol_regime = -1` | control |
| S3 stack+ADX | S1 plus `adx_regime = 0` | control |

The instruments are BTCUSD, EURUSD, and XAUUSD; long and short are separate.

## 2. Results

Pooled by variant:

| Variant | Trades | Net P&L | PF | Max cell DD | Positive instruments | Positive years |
|---|---:|---:|---:|---:|---:|---:|
| S1 stack long | 113 | +1,150.76 | 1.341 | 5.19% | 1/3 | 4/5 |
| S1 stack short | 111 | +411.97 | 1.164 | 5.11% | 1/3 | 3/5 |
| S2 stack+RVOL long | 54 | +1,383.51 | 1.717 | 5.0035% | 1/3 | 3/5 |
| S2 stack+RVOL short | 48 | +915.32 | 1.901 | 3.18% | 1/3 | 3/5 |
| S3 stack+ADX long | 64 | +1,108.01 | 1.534 | 4.41% | 1/3 | 4/5 |
| S3 stack+ADX short | 75 | +429.46 | 1.235 | 4.17% | 1/3 | 3/5 |

By instrument, S1 stack long: BTCUSD 25 trades +1,944.83 PF 2.01; EURUSD 60
trades −461.36 PF 0.42; XAUUSD 28 trades −332.70 PF 0.49. S1 stack short:
BTCUSD 26 trades +908.63 PF 1.78; EURUSD 49 trades −261.67 PF 0.57; XAUUSD 36
trades −234.99 PF 0.68.

A reproducibility check reran S1 stack long on BTCUSD and EURUSD 2018:
`portfolio/metrics.json`, `portfolio/trades.csv`, and
`strategy/decisions.ndjson` are byte-identical.

## 3. Criteria

| Criterion | S1 long | S1 short |
|---|---|---|
| Pooled net > 0 | pass | pass |
| Pooled PF > 1.10 | pass | pass |
| ≥ 30 pooled trades | pass (113) | pass (111) |
| ≥ 2 of 3 instruments positive | fail (1/3) | fail (1/3) |
| ≥ 3 of 5 pooled years positive | pass (4/5) | pass (3/5) |
| Every cell drawdown ≤ 5% | fail (5.19%) | fail (5.11%) |
| 9 bps PF > 1.0 | not run | not run |
| 2024 validation | not opened | not opened |

No side passes. S2 and S3 are also BTC-only; they do not rescue the
multi-instrument premise.

## 4. What the result says

- **The larger sample falsified the premise.** The stack-aligned fade is
  BTC-specific in this sample. EURUSD and XAUUSD are negative on every variant
  and side.
- **The controls agree.** S2 and S3 are also BTC-only; adding the RVOL or ADX
  filter does not create a multi-instrument edge.
- **The BTC cells still breach the drawdown ceiling.** S1 long is 5.19% in 2018
  and 5.0035% in 2021; S1 short is 5.11% in 2019. The halt is bar-close.
- **2019 remains negative** for S1 long and short; 2020–2022 carry the result.
- **Cost survival is untested.** The 9 bps sensitivity was not run because the
  base gate failed; this is not evidence of cost survival.
- **No candidate and no 2024 access.** The programme stops; S1 is not
  re-registered with a new filter.

## 5. Goal gap

The best BTC-only S1 long result is 25 trades and +$1,944.83 on $50,000 over
five years, about 3.9% total or **0.065%/month**. The 10%/month target is 303.5×
over five years; the BTC-only result is roughly 292× short. The pooled
multi-instrument result is positive only because BTC dominates; it is not a
robust edge and does not close the gap.

## 6. Recommendation

Stop this programme. Do not open 2024 and do not run the 9 bps sensitivity. The
next research question is a new premise, not a variant of S1: either explain why
the stack-aligned fade is BTC-specific (microstructure, trend persistence, or
the 2019 regime) or test a different event family on BTCUSD. A new charter is
required before the first run.

## 7. Limitations

- 1h-only, five years, 25–60 trades per instrument for S1.
- The result is instrument-specific; no pooled multi-instrument edge.
- The 5% halt overshoots on BTC.
- The 9 bps sensitivity and 2024 validation were not opened.
- Quant Developer and Quant Validator are sequential same-agent passes, not
  independent agents.

## 8. Links

- [Charter](../../governance/2026-11-09/SR4-CHARTER.md)
- [Plan](../../execution-plans/2026-11-09/sr4-stack-fade.md)
- [Predecessor](../2026-11-08/SR3-FEATURES.md)
- [AI transparency log](../../ai-transparency-log/2026-11-09/sr4-stack-fade.md)
- Local artifacts: `data/improvement/2026-11-09/SR4-STACK-FADE/`
