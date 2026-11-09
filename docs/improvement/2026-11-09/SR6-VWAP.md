# SR6-VWAP — 15m VWAP Rejection with 1h Stack Context

- Date: 2026-11-09 (UTC)
- Status: **No candidate — V1 fails net P&L, profit factor, and the 5% drawdown gate; 2024 not opened**
- Charter: [`SR6-CHARTER.md`](../../governance/2026-11-09/SR6-CHARTER.md)
- Plan: [`sr6-vwap.md`](../../execution-plans/2026-11-09/sr6-vwap.md)
- Predecessor: [`SR5-ABSORPTION.md`](SR5-ABSORPTION.md)
- Run record: `data/improvement/2026-11-09/SR6-VWAP/`
- Claim type: strategy research; no edge or profitability claim

## 1. Cells

The event is `signal.vwap_rejection` (type 3, subtype 14, source 19), a 15m
rejection at the session VWAP. The 1h EMA stack must agree for the primary
variant. Stop 1 ATR, target 3 ATR, time stop 20 bars.

| Variant | Rule | Primary? |
|---|---|---|
| V1 stack | VWAP rejection plus 1h `ema_stack` equal to the event direction | yes |
| V2 stack+ADX | V1 plus 1h `adx_regime = 1` | control |
| V3 event | VWAP rejection only | control |

Long and short are separate; 3 rules × 2 sides × 5 years = 30 base cells.

## 2. Count gate

The count gate passed easily: V1 has **4,822 long / 4,281 short** A1 events over
2018–2022, distributed across all five years. The event-only control has 12,928
long / 12,774 short events.

## 3. Results

| Variant | Trades | Net P&L | PF | Max cell DD | Halts | Positive years |
|---|---:|---:|---:|---:|---:|---:|
| V1 stack long | 229 | −242.14 | 0.966 | 6.37% | 5 | 2/5 |
| V1 stack short | 154 | −719.73 | 0.853 | 5.61% | 5 | 1/5 |
| V2 stack+ADX long | 259 | +367.61 | 1.044 | 6.85% | 5 | 3/5 |
| V2 stack+ADX short | 118 | +392.66 | 1.090 | 5.71% | 5 | 3/5 |
| V3 event long | 288 | −230.88 | 0.972 | 6.56% | 5 | 1/5 |
| V3 event short | 233 | −1,321.34 | 0.808 | 5.26% | 5 | 1/5 |

Every cell halted in every year. The 5% halt is bar-close and overshoots; the
observed maxima are above 5%.

## 4. Criteria

| Criterion | V1 long | V1 short |
|---|---|---|
| Count gate ≥30/side | pass | pass |
| ≥30 trades | pass (229) | pass (154) |
| Net > 0 at 6 bps | fail | fail |
| PF > 1.10 | fail | fail |
| Every cell DD ≤5% | fail (6.37%) | fail (5.61%) |
| ≥3/5 positive years | fail (2/5) | fail (1/5) |
| 9 bps PF > 1.0 | not run | not run |
| 2024 validation | not opened | not opened |

No side passes. V2 and V3 are also below PF 1.10 or above the drawdown ceiling;
they do not rescue V1.

## 5. What the result says

- **The event is frequent but not tradeable in this design.** 229/154 V1 trades
  are enough sample; the 1 ATR / 3 ATR payoff does not produce a cost-aware
  follow-through at 15m.
- **Every variant halts every year.** The 5% ceiling is reached repeatedly and
  the bar-close halt overshoots to 5.26–6.85%.
- **The 1h stack does not improve the event-only control.** V1 long is negative
  and V3 long is also negative; V1 short is negative and V3 short is worse.
- **The 1h ADX control is the least bad variant.** V2 is positive at 6 bps but
  PF is only 1.044/1.090 and every cell breaches drawdown; it is not a
  candidate.
- **The parser needed a fix.** The event-composite strategy hardcoded subtypes
  `1..=7`; `signal.vwap_rejection` is subtype 14. The validator now accepts
  registered signal subtypes and still rejects diagnostics, with regression
  tests.

## 6. Goal gap

No development result is positive and drawdown-compliant, so there is no return
to compare with the 10%/month target. The result is a clean rejection of this
event/payoff design, not a small edge that needs tuning.

## 7. Recommendation

Stop this programme. Do not run the 9 bps sensitivity or 2024. The VWAP
rejection event and the 1 ATR / 3 ATR design are retired together. The next
premise needs a different event, timeframe, or payoff mechanism, a new approved
charter, and its own count and feasibility gates.

## 8. Limitations

- One instrument, five years, 15m entry.
- The 9 bps sensitivity and 2024 validation were not opened.
- The 5% halt is bar-close and overshoots.
- The result is one stop/target design; a different design is a new premise.
- Quant Developer and Quant Validator are sequential same-agent passes, not
  independent agents.

## 9. Links

- [Charter](../../governance/2026-11-09/SR6-CHARTER.md)
- [Plan](../../execution-plans/2026-11-09/sr6-vwap.md)
- [Predecessor](SR5-ABSORPTION.md)
- [AI transparency log](../../ai-transparency-log/2026-11-09/sr6-vwap.md)
- Local artifacts: `data/improvement/2026-11-09/SR6-VWAP/`
