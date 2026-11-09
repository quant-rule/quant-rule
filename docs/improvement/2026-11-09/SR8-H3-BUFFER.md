# SR8-H3-BUFFER — 2023 BTC Buffer Test

- Date: 2026-11-09 (UTC)
- Status: **Buffer ineligible — 2023 1h coverage 61.3%, below the 95% gate; 2024 not opened**
- Charter: [`SR8-CHARTER.md`](../../governance/2026-11-09/SR8-CHARTER.md)
- Plan: [`sr8-h3-buffer.md`](../../execution-plans/2026-11-09/sr8-h3-buffer.md)
- Predecessor: [`SR7-H3-CRYPTO.md`](SR7-H3-CRYPTO.md)
- Run record: `data/improvement/2026-11-09/SR8-H3-BUFFER/`
- Claim type: strategy robustness buffer; no candidate or edge claim

## 1. Intent

The 2014–2017 BTC source returned no candles, so the approved fallback was a
one-shot 2023 buffer test of the frozen H3 BTC long/short rule. The test asks
whether the H3 sign survives a year that was excluded from every formal
partition. It is a robustness gate, not a candidate or validation.

## 2. Data quality

| Check | Value | Gate | Result |
|---|---:|---:|---|
| 1m source rows | 487,479 | — | — |
| Gaps over 60s | 19,699 | — | — |
| 1h model bars | 5,354 | — | — |
| 1h missing bars | 3,378 | — | — |
| 1h coverage | 61.3% | ≥95% | fail |

The 2023 source is incomplete. The buffer is data-ineligible.

## 3. Results

| Cell | Trades | Net P&L | PF | Max DD | Halt |
|---|---:|---:|---:|---:|---|
| H3 long 6 bps | 0 | 0.00 | — | 0.00% | no |
| H3 short 6 bps | 4 | −$98.78 | 0.572 | 1.84% | no |
| H3 long 9 bps | 0 | 0.00 | — | 0.00% | no |
| H3 short 9 bps | 4 | −$108.71 | 0.544 | 1.90% | no |

Both sides are below the 5-trade buffer floor. The short side is negative at
both costs. The long side has no trades, so it is not a test of the long rule.

## 4. Criteria

| Criterion | Result |
|---|---|
| 2023 1h coverage ≥95% | fail (61.3%) |
| Each side ≥5 trades | fail (0 long / 4 short) |
| Net > 0 at 6 bps | fail short / no long sample |
| PF > 1.0 at 6 bps | fail short / no long sample |
| Every cell DD ≤5% | pass (1.84%/1.90%) |
| No halt | pass |
| 9 bps PF > 0 | fail short / no long sample |

## 5. What the result says

- **The buffer is data-ineligible, not a clean H3 failure.** The 2023 data has
  19,699 gaps and only 61.3% 1h coverage.
- **The long rule has no 2023 trades.** The long side is untested in the buffer.
- **The short side has four trades and loses at both costs.** The sample is too
  small and the data too incomplete to close the short rule.
- **The H3 BTC family remains underpowered.** The 2023 buffer cannot resolve it.
- **2024 remains closed.** No validation access.

## 6. Goal gap

No buffer result is usable, so there is no new return evidence. The H3 BTC lead
remains the 2018–2022 result: 39 trades and about 0.078%/month, far below the
10%/month target. The sample problem is still the bottleneck.

## 7. Recommendation

Do not open 2024 and do not reuse 2023. The next step is a **data-source
decision**: a complete BTC 2023 or 2014–2017 1m source. If no complete source is
available, the stack-fade family stays underpowered and no further strategy
variant is warranted.

## 8. Limitations

- The 2023 data is incomplete; the buffer result is descriptive only.
- The 2014–2017 source is unavailable.
- The buffer is consumed once and cannot be reused for this hypothesis.
- Quant Developer and Quant Validator are sequential same-agent passes, not
  independent agents.

## 9. Links

- [Charter](../../governance/2026-11-09/SR8-CHARTER.md)
- [Plan](../../execution-plans/2026-11-09/sr8-h3-buffer.md)
- [Predecessor](SR7-H3-CRYPTO.md)
- [AI transparency log](../../ai-transparency-log/2026-11-09/sr8-h3-buffer.md)
- Local artifacts: `data/improvement/2026-11-09/SR8-H3-BUFFER/`
