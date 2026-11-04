# MODEL-2026-11-04-07 — Compression-Range Validation

- Date: 2026-11-04 (UTC)
- Tier: model
- Changed: model
- Frozen: strategy, portfolio
- Partition: `model_validation` (2021-01-01..2022-12-31)
- Code commit: `8e8a481` plus the 377 working tree
- Manifest: `data/improvement/2026-11-04/MODEL-2026-11-04-07/manifest.json`
- Protocol: [`docs/model/2026-11-04/compression-range.md`](../../model/2026-11-04/compression-range.md)
- Baseline: `MODEL-2026-11-04-06` (frozen development event)
- Local artifacts: `data/improvement/2026-11-04/MODEL-2026-11-04-07/artifacts/`
- Verdict: **accepted — model baseline**
- Claim type: model evidence; no strategy, cost, or edge claim

## Second-use disclosure

`model_validation` 2021–2022 was already used for `MODEL-2026-11-04-05`, the
accepted 1m 4 ATR compression-breakout baseline. This run applies the same
partition to a different event (`signal.compression_range`) and a different
outcome (next-bar boundary touch). The reuse is disclosed; residual
data-snooping risk is higher than a first use.

## Results

| n | J | Hit | Matched base | Coiling base | Lift | Lift LCB | Rand p | Back lag | Rep | Pass |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 247,989 | 43,937 | 0.5802 | 0.4984 | 0.5802 | +0.0818 | +0.0795 | 0.0050 | +0.0291 | 3 | yes |

Per instrument:

| Instrument | n | Hit | Matched base | Lift |
|---|---:|---:|---:|---:|
| BTCUSD | 78,494 | 0.5989 | 0.4908 | +0.1081 |
| ETHUSD | 93,123 | 0.5872 | 0.5063 | +0.0809 |
| XAUUSD | 76,372 | 0.5525 | 0.4966 | +0.0560 |

## Review

Validation passes every criterion: +8.18pp lift, +7.95pp corrected lower bound,
random p 0.005, back-lag +2.91pp (event lift − back-lag = +5.27pp), and 3/3
instruments above their matched baselines. The lift is smaller than development
(+10.32pp) but agrees in sign and clears the +5pp threshold. The matched
baseline is 49.84%, so the outcome is not saturated.

The event definition and configuration were frozen before validation: 1m,
5 coiling bars, type 3 subtype 8 source 5, direction 0. No threshold was changed
after the development result.

## Decision

Accepted model baseline (human, 2026-11-04). The approved bracket strategy runs
`strategy_dev` 2018–2022; the 2024 validation is opened once only if development
passes.

## Limitations

- The outcome is a boundary touch, not a return or a cost-aware payoff.
- The validation partition is reused; see the disclosure above.
- The event is neutral; it does not claim the expansion direction.
- Same-session review, not an independent Quant Validator record.
