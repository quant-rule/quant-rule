# MODEL-2026-11-04-05 — 4 ATR Compression Validation

- Date: 2026-11-04 (UTC)
- Tier: model
- Changed: model
- Frozen: strategy, portfolio
- Partition: `model_validation` (2021-01-01..2022-12-31)
- Code commit: `91af224` plus the uncommitted harder-cell working tree
- Manifest: `data/improvement/2026-11-04/MODEL-2026-11-04-05/manifest.json`
- Protocol: [`docs/model/2026-11-04/compression-harder.md`](../../model/2026-11-04/compression-harder.md)
- Baseline: `MODEL-2026-11-04-04` (frozen development candidate)
- Local artifacts: `data/improvement/2026-11-04/MODEL-2026-11-04-05/artifacts/`
- Verdict: **accepted — model baseline**
- Claim type: model evidence; no strategy or edge claim

## Results

| Cell | n | J | Hit | All base | Coiling base | Placebo base | Lift | Lift LCB | Rand p | Back lag | Rep | Pass |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 1m | 12,397 | 11,686 | 0.7582 | 0.6859 | 0.6950 | 0.7294 | +0.0632 | +0.0556 | 0.0050 | −0.1354 | 3 | yes |

Per instrument:

| Instrument | n | Hit | Coiling base | Lift |
|---|---|---:|---:|---:|
| BTCUSD | 4,673 | 0.7965 | 0.7290 | +0.0675 |
| ETHUSD | 4,819 | 0.7759 | 0.7136 | +0.0623 |
| XAUUSD | 2,905 | 0.6675 | 0.6095 | +0.0580 |

## Review

The frozen 1m, 4 ATR, 20-bar cell passes validation: +6.32pp lift, +5.56pp
corrected lower bound, random p 0.005, placebo +3.44pp, back-lag −13.54pp, and
three-instrument replication. Development was +6.83pp lift with a +6.22pp
corrected bound. The partitions agree in sign and magnitude. Every instrument is
positive on both partitions.

This is the first model claim in the program that passes both partitions with
the pre-registered baselines, controls, and correction. The human accepted it as
the model baseline on 2026-11-04.

## Decision

Accepted model baseline (human, 2026-11-04). No strategy, execution, or portfolio
work starts from the model result alone; a strategy pre-registration is next. The 1m timeframe carries high costs;
the model claim is expansion, not a cost-adjusted strategy result.

## Limitations

- The 1m timeframe is cost-sensitive; this is not a profitability claim.
- The same M1 data underlies prior results; the 4 ATR outcomes were unseen, but
  residual snooping risk remains.
- Same-session review, not an independent Quant Validator record.
