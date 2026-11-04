# MODEL-2026-11-04-11 — Fresh-Onset 5m Validation

- Date: 2026-11-04 (UTC)
- Tier: model
- Changed: model
- Frozen: strategy, portfolio
- Partition: `model_validation` (2021-01-01..2022-12-31)
- Code commit: `2c85b41` plus the 381 working tree
- Manifest: `data/improvement/2026-11-04/MODEL-2026-11-04-11/manifest.json`
- Protocol: [`docs/model/2026-11-04/compression-range-onset-5m.md`](../../model/2026-11-04/compression-range-onset-5m.md)
- Baseline: `MODEL-2026-11-04-10` (frozen development event)
- Local artifacts: `data/improvement/2026-11-04/MODEL-2026-11-04-11/artifacts/`
- Verdict: **accepted — model baseline**
- Claim type: model evidence; no strategy, cost, or edge claim

## Third-use disclosure

`model_validation` 2021–2022 was already used for the accepted 1m 4 ATR
compression-breakout baseline and the accepted 1m compression-range baseline.
This run applies the same partition to a 5m fresh-onset event and outcome. The
third use is disclosed; residual data-snooping risk is higher than a first use.

## Results

| n | J | Hit | Matched base | Coiling base | Lift | Lift LCB | Rand p | Back lag | Rep | Pass |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 14,303 | 9,708 | 0.5373 | 0.4651 | 0.5319 | +0.0722 | +0.0640 | 0.0050 | −0.0065 | 3 | yes |

Per instrument:

| Instrument | n | Hit | Matched base | Lift |
|---|---:|---:|---:|---:|
| BTCUSD | 5,025 | 0.5373 | 0.4550 | +0.0823 |
| ETHUSD | 5,453 | 0.5491 | 0.4629 | +0.0862 |
| XAUUSD | 3,825 | 0.5205 | 0.4816 | +0.0389 |

## Review

Validation passes every criterion: +7.22pp lift, +6.40pp corrected lower bound,
random p 0.005, back-lag −0.65pp, and 3/3 instruments above their matched
baselines. The lift is smaller than development (+9.46pp) but the back-lag is
clean, so the onset timing survives out of sample.

The event definition and configuration were frozen before validation: 5m,
5 coiling bars, type 3 subtype 9 source 5, direction 0. No threshold changed.

## Decision

Accepted model baseline (human, 2026-11-04). The pre-registered 5m onset bracket
runs `strategy_dev` 2018–2022; the 2024 strategy validation is opened once only
if development passes.

## Limitations

- The outcome is a boundary touch, not a return or a cost-aware payoff.
- The validation partition is reused; see the disclosure above.
- Same-session review, not an independent Quant Validator record.
