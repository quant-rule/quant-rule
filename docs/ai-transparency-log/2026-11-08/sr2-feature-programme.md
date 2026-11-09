# AI Transparency Log — SR2 Feature Programme

- Date: 2026-11-08 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: `sr2-feature-programme`
- Plan: [`sr2-feature-programme.md`](../../execution-plans/2026-11-08/sr2-feature-programme.md)
- Records: [`SR2-BTC-FEATURES.md`](../../improvement/2026-11-08/SR2-BTC-FEATURES.md), [`SR3-FEATURES.md`](../../improvement/2026-11-08/SR3-FEATURES.md)
- Status: complete; no candidate; 2024 not opened
- Claim type: research programme; no edge or profitability claim

## Intent and boundary

The human approved all phases: reverse-engineer the best SR2-A BTCUSD trades
into features, promote the surviving features into the model, and test a
BTC-only 1h-context / 15m-entry strategy. This log covers the charts, feature
attribution, the `rvol_regime` model state, and the Phase C cells. It does not
cover 2024 or a multi-model `ModelSet`.

## Implementation

- Added `tools/improvement/sr2_feature_audit.py` and five tests. It extracts
  139 trade feature rows and renders five 1,000-candle 1h PNGs with volume,
  EMA 10/50/200, entry/exit, and a feature strip.
- Added the `rvol_regime` context state to the model: low below 1.4, normal
  1.4–2.0, high at or above 2.0. It is published in `readings.csv` and
  `report.json`, read by the strategy bundle, and exposed as a strategy state.
- Added eight strategy cells: H1 (1h aligned + weak ADX + low RVOL), H2 (15m
  entry, 1h context), H3 (1h without ADX), H4 (1h without RVOL). Long and short
  are separate cells.
- Ran 40 preflight cells at 6 bps, 40 base cells, and 30 9 bps sensitivity cells
  for H1/H3/H4. H2 failed the base, so it was not run at 9 bps.
- Recorded the Phase A and Phase C documents and the local artifacts.

## Verification and evidence

| Check | Observed |
|---|---|
| Charts | 5 / 5 PNGs |
| Feature rows | 139 |
| Preflight | 40 / 40 pass at 6 bps |
| Base cells | 40 / 40 complete |
| 9 bps cells | 30 / 30 complete for H1/H3/H4 |
| H1 6 bps | 22 trades, +2,255, PF 2.04/5.22 |
| H3 6 bps | 39 trades, +2,343, PF 2.23/1.73 |
| H4 6 bps | 29 trades, +3,151, PF 2.47/3.84 |
| H2 6 bps | 122 trades, −1,180, PF 0.91/0.48 |
| H4 long drawdown | 5.0035% at 6 bps; 5.0401% at 9 bps |
| Unit tests | model, model-spec parser, strategy parser, feature audit |
| Rust tests | `cargo test` pass |
| Lints | `cargo clippy --all-targets -- -D warnings` pass |
| Python tests | 131 pass |
| Reproducibility | H3 long 2018 and H4 long 2021 rerun byte-identical (metrics, trades, decisions) |
| Record | research analysis `SR3-FEATURES`; no v2 manifest; 2024 not opened |

## Learning

- The top-five pattern is real in the sample: low RVOL, weak ADX, and event
  direction aligned with the EMA stack. The stack alignment is the common
  positive condition; the ADX and RVOL filters are not additive in the small
  sample.
- The 15m entry with 1h context is negative. More trades do not help when the
  15m breakout lacks the edge.
- The 1h filtered cells are positive at 6 and 9 bps but have only 9–21 trades
  per side over five years. They are underpowered and still regime-dependent.
- The 5% halt overshoots to 5.04% in H4 long; a strict 5% ceiling needs a lower
  soft halt or intra-bar control.
- Phase D is not justified: the 1h single-model cells outperform the 15m
  multi-timeframe cells. A `ModelSet` would be speculative.

## Rejected or corrected

- Did not open 2024.
- Did not tune the filters, target, stop, or timeframe after seeing the result.
- Did not treat the top-five pattern as evidence; it is hypothesis generation.
- Did not build a multi-model engine without evidence.
- Did not call the 5% drawdown run compliant; H4 long observes 5.04%.
- Did not present the Phase C folder as a v2 run; it has no manifest and is
  recorded as a research analysis.
- Did not commit machine-specific run scripts; the reproduction commands and
  logs are in the run record instead.
- Did not claim independent Quant Developer or Quant Validator review; the
  passes were sequential same-agent passes.
