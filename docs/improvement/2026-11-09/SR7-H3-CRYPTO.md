# SR7-H3-CRYPTO — BTC+ETH H3 Cross-Section

- Date: 2026-11-09 (UTC)
- Status: **No candidate — H3 long fails trade-count/drawdown; H3 short fails PF/years/ETH/drawdown; 2024 not opened**
- Charter: [`SR7-CHARTER.md`](../../governance/2026-11-09/SR7-CHARTER.md)
- Plan: [`sr7-h3-crypto.md`](../../execution-plans/2026-11-09/sr7-h3-crypto.md)
- Predecessor: [`SR6-VWAP.md`](SR6-VWAP.md)
- Run record: `data/improvement/2026-11-09/SR7-H3-CRYPTO/`
- Claim type: strategy research; no edge or profitability claim

## 1. Premise

SR3 found the H3 1h fade positive on BTCUSD but underpowered (18 long / 21
short trades). SR4 showed it is not a EURUSD/XAUUSD behavior. This programme
tests whether H3 is a **crypto-major** behavior by adding ETHUSD, a correlated
major with the same microstructure.

H3: fade a 1h compression breakout when the event direction equals the 1h EMA
stack and 1h `adx_regime = 0` (weak trend), with no RVOL filter.

## 2. Count gate

| Instrument | H3 long | H3 short |
|---|---:|---:|
| BTCUSD | 20 | 23 |
| ETHUSD | 19 | 19 |
| Pooled | 39 | 42 |

The gate requires 30 per side pooled; **pass**.

## 3. Results

| Variant | Trades | Net P&L | PF | Max cell DD | Halts | Positive years | Positive instruments |
|---|---:|---:|---:|---:|---:|---:|---:|
| H3 long pooled | 27 | +$2,289.36 | 2.098 | 5.62% | 1 | 5/5 | 2/2 |
| H3 short pooled | 36 | +$147.02 | 1.067 | 5.64% | 1 | 1/5 | 1/2 |

By instrument:

| Variant | BTCUSD | ETHUSD |
|---|---|---|
| H3 long | 18 / +$1,619.50 / PF 2.232 / DD 4.41% | 9 / +$669.86 / PF 1.870 / DD 5.62% |
| H3 short | 21 / +$723.44 / PF 1.731 / DD 4.17% | 15 / −$576.42 / PF 0.527 / DD 5.64% |

## 4. Criteria

| Criterion | H3 long | H3 short |
|---|---|---|
| Count gate ≥30/side | pass | pass |
| ≥30 pooled trades | fail (27) | pass (36) |
| Net > 0 at 6 bps | pass | pass |
| PF > 1.10 | pass | fail (1.067) |
| Every cell DD ≤5% | fail (ETH 2020 5.62%) | fail (ETH 2021 5.64%) |
| ≥3/5 pooled years positive | pass (5/5) | fail (1/5) |
| Both instruments non-negative | pass | fail (ETH −$576) |
| 9 bps PF > 1.0 | not run | not run |
| 2024 validation | not opened | not opened |

No side passes.

## 5. What the result says

- **The BTC control reproduces SR3 exactly.** 18 long / 21 short, positive,
  inside the 5% ceiling. The data and strategy path are consistent.
- **ETH long is positive but not a confirmation.** 9 trades, PF 1.87, but a
  5.62% drawdown in 2020 and only 2/5 positive years.
- **ETH short is negative.** 15 trades, −$576, PF 0.53, and a 5.64% drawdown in
  2021. The cross-crypto premise fails on the short side.
- **H3 long is 5/5 positive years but underpowered.** 27 pooled trades and the
  ETH drawdown breach keep it out of the candidate set.
- **H3 short is not acceptable.** Pooled PF 1.067, 1/5 years, ETH negative, and
  a drawdown breach.
- **No 9 bps and no 2024.** The base gate failed.

## 6. Goal gap

H3 long pooled +$2,289.36 over 20 cells × $10,000 = $200,000 over five years is
about 1.14% total, or **0.019%/month**; per instrument-side the samples are
small. The 10%/month target remains far away. The result does not justify a
candidate or validation access.

## 7. Recommendation

Close the stack-fade family as a candidate. The BTC control is positive but
underpowered; ETH does not confirm it, and the short side is negative. The next
premise needs a different event, timeframe, or payoff mechanism, a new approved
charter, and its own count and feasibility gates. Do not re-register H3 with ETH
or a new filter.

## 8. Limitations

- One new instrument, five years, 9–21 trades per instrument-side.
- ETH is correlated with BTC; even a positive ETH result would not be
  independent evidence.
- The 5% halt is bar-close and overshoots to 5.62–5.64%.
- The 9 bps sensitivity and 2024 validation were not opened.
- Quant Developer and Quant Validator are sequential same-agent passes, not
  independent agents.

## 9. Links

- [Charter](../../governance/2026-11-09/SR7-CHARTER.md)
- [Plan](../../execution-plans/2026-11-09/sr7-h3-crypto.md)
- [Predecessor](SR6-VWAP.md)
- [AI transparency log](../../ai-transparency-log/2026-11-09/sr7-h3-crypto.md)
- Local artifacts: `data/improvement/2026-11-09/SR7-H3-CRYPTO/`
