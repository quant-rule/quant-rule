# SR3-FEATURES — Feature-Filtered BTCUSD Fade

- Date: 2026-11-08 (UTC)
- Status: **No candidate — positive 1h subsets but underpowered; 15m entry negative; 2024 not opened**
- Programme: [`sr2-feature-programme.md`](../../execution-plans/2026-11-08/sr2-feature-programme.md)
- Phase A: [`SR2-BTC-FEATURES.md`](SR2-BTC-FEATURES.md)
- Model change: [`rvol-regime.md`](../../model/2026-11-08/rvol-regime.md)
- Run record: `data/improvement/2026-11-08/SR3-FEATURES/`
- Claim type: strategy research; no edge or profitability claim

## 1. Cells

All cells are BTCUSD, `strategy_dev` 2018–2022, 6 bps base, 5% per-cell
drawdown, 1 ATR stop / 3 ATR target, 20-bar time stop. H1/H3/H4 also ran at
9 bps; H2 did not, because its base failed. The entry event is
`signal.compression_breakout`; the fade side is opposite the event.

| Cell | Timeframe | Event | 1h EMA stack | 1h ADX | 1h RVOL |
|---|---|---|---|---|---|
| H1 long | 1h | −1 | −1 | weak | low |
| H1 short | 1h | +1 | +1 | weak | low |
| H2 long | 15m | −1 | −1 | weak | low |
| H2 short | 15m | +1 | +1 | weak | low |
| H3 long | 1h | −1 | −1 | weak | any |
| H3 short | 1h | +1 | +1 | weak | any |
| H4 long | 1h | −1 | −1 | any | low |
| H4 short | 1h | +1 | +1 | any | low |

H1 and H2 are primary; H3 and H4 are controls that isolate the ADX and RVOL
conditions. H2 uses the 1h state as context for the 15m entry.

## 2. Results

| Cell | Trades | Net 6 bps | PF 6 | Max DD 6 | Net 9 bps | PF 9 | Max DD 9 | Positive folds |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| H1 long | 13 | +1,126 | 2.04 | 4.41% | +1,092 | 1.99 | 4.51% | 3/5 |
| H1 short | 9 | +1,129 | 5.22 | 1.39% | +1,106 | 4.98 | 1.44% | 3/5 |
| H2 long | 63 | −178 | 0.91 | 4.77% | not run | — | — | 2/5 |
| H2 short | 59 | −1,002 | 0.48 | 4.84% | not run | — | — | 0/5 |
| H3 long | 18 | +1,620 | 2.23 | 4.41% | +1,572 | 2.17 | 4.51% | 4/5 |
| H3 short | 21 | +723 | 1.73 | 4.17% | +670 | 1.65 | 4.28% | 3/5 |
| H4 long | 17 | +1,881 | 2.47 | 5.0035% | +1,836 | 2.41 | 5.0401% | 3/5 |
| H4 short | 12 | +1,270 | 3.84 | 3.18% | +1,240 | 3.69 | 3.28% | 3/5 |

H4 long halts once and its observed drawdown is above the 5% ceiling. Every
other cell stays at or below 4.84% (H2 short).

A reproducibility check reran H3 long 2018 and H4 long 2021 with the final
binary: `portfolio/metrics.json`, `portfolio/trades.csv`, and
`strategy/decisions.ndjson` are byte-identical.

## 3. Criteria

| Criterion | H1 | H2 | H3 | H4 |
|---|---|---|---|---|
| Complete 5 cells | pass | pass | pass | pass |
| Return > 0 | pass | fail | pass | pass |
| PF > 1.10 | pass | fail | pass | pass |
| Drawdown ≤ 5% | pass | pass | pass | fail (H4 long 5.0035%) |
| ≥ 30 trades per side | fail (13/9) | pass (63/59) | fail (18/21) | fail (17/12) |
| ≥ 4/5 positive folds | fail (3/3) | fail (2/0) | fail (4/3) | fail (3/3) |
| 9 bps PF > 1.0 | pass | not run | pass | pass |
| 2024 validation | not opened | not opened | not opened | not opened |

No cell is a candidate. H2 fails on return and PF; H1/H3/H4 are positive but
underpowered and fail the fold and trade-count floors. H4 long also breaches
the 5% ceiling.

## 4. What the result says

- **The 1h filters survive costs.** H1, H3, and H4 remain positive at 9 bps;
  the positive sign is not a cost artifact.
- **The filters are not additive.** H3 (no ADX) and H4 (no RVOL) are competitive
  with H1 (both filters); no control dominates on every side. The stack
  alignment is the common positive condition; the extra ADX and RVOL filters
  mostly reduce the sample.
- **The 15m entry fails.** H2 has 122 trades and is negative on both sides; the
  1h context does not rescue the 15m breakout. The user's desired 1h
  regime/15m entry architecture is not supported by this rule.
- **The 1h sample is tiny.** 9–21 trades per side over five years is far below
  the 30-trade floor and far below a robust sample. The positive PF is not
  enough evidence.
- **2019 remains negative** for the 1h short cells; the 1h long cells have zero
  or one 2019 trade (H3 long −$81), so 2019 is not a clean control. The sample
  is still regime-dependent.
- **The 5% ceiling is not strict.** H4 long observes 5.0035% at 6 bps and
  5.0401% at 9 bps; the bar-close halt overshoots. A strict 5% envelope needs a
  lower soft halt or intra-bar control.
- **Phase D is not triggered.** The 1h single-model cells are better than the
  15m multi-timeframe cells. There is no evidence that multiple model
  configurations are the missing ingredient; a `ModelSet` would be speculative.

## 5. Goal gap

The best pooled cells are H4: 29 trades, +$3,151 at 6 bps, +$3,075 at 9 bps.
Over five years and $50,000 that is about 6.3%/6.2% total, or **0.10%/month**.
The 10%/month target is 303.5× over five years; the current result is roughly
286× short. The 15m version has more trades but loses money. The 1h version
does not have enough trades to close the gap; more filtering makes it smaller.

## 6. Recommendation

Do not freeze or open 2024. The feature programme produced a coherent
hypothesis and a positive 1h subset, but the sample is too small and the 15m
entry is negative. The next research question is whether the **stack-aligned 1h
fade** survives on a larger sample: add EURUSD/XAUUSD as robustness controls, or
test a different 15m entry design with the 1h context. That is a new
pre-registered hypothesis, not a tuning of this one.

## 7. Limitations

- One instrument, five years, 9–21 trades per side.
- The 2019 regime loss remains.
- The 5% halt is bar-close and can overshoot.
- The model state `rvol_regime` is a new context feature; the accepted model
  baseline's event values are unchanged, but the state has no standalone
  validation claim.
- The 15m cells were not run at 9 bps because the base failed.
- The Quant Developer and Quant Validator passes are sequential same-agent
  passes, not independent agents.

## 8. Links

- [Phase A feature attribution](SR2-BTC-FEATURES.md)
- [Programme plan](../../execution-plans/2026-11-08/sr2-feature-programme.md)
- [Model change](../../model/2026-11-08/rvol-regime.md)
- [Audit](SR2-BTC-AUDIT.md)
- [AI transparency log](../../ai-transparency-log/2026-11-08/sr2-feature-programme.md)
- Local artifacts: `data/improvement/2026-11-08/SR3-FEATURES/`
