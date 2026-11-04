# MODEL-2026-11-04-08 — Compression-Range 5m Development

- Date: 2026-11-04 (UTC)
- Tier: model
- Changed: model
- Frozen: strategy, portfolio
- Partition: `model_dev` (2018-01-01..2020-12-31)
- Code commit: `669a67a` plus the 379 working tree
- Manifest: `data/improvement/2026-11-04/MODEL-2026-11-04-08/manifest.json`
- Protocol: [`docs/model/2026-11-04/compression-range-5m.md`](../../model/2026-11-04/compression-range-5m.md)
- Baseline: `MODEL-2026-11-04-06..07`
- Local artifacts: `data/improvement/2026-11-04/MODEL-2026-11-04-08/artifacts/`
- Verdict: **no edge — back-lag control failed**
- Claim type: model evidence; no strategy, cost, or edge claim

## Results

| n | J | Hit | Matched base | Lift | Lift LCB | Rand p | Back lag | Back-lag difference | Rep | Pass |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 84,627 | 11,234 | 0.5545 | 0.4732 | +0.0813 | +0.0775 | 0.0050 | +0.0333 | +0.0480 | 3 | no |

Per instrument:

| Instrument | n | Hit | Matched base | Lift |
|---|---:|---:|---:|---:|
| BTCUSD | 27,033 | 0.5470 | 0.4460 | +0.1010 |
| ETHUSD | 21,979 | 0.6291 | 0.5281 | +0.1011 |
| XAUUSD | 35,615 | 0.5142 | 0.4600 | +0.0542 |

## Review

The 5m cell meets the sample, lift, lower-bound, random-control, and replication
criteria. It fails only the pre-registered back-lag control: the lagged
structural rule already has a +3.33pp lift, and the event's incremental timing
edge is +4.80pp, 0.20pp short of the required +5pp.

The gate is not relaxed after the result. The 2024 validation partition is not
opened, the 5m strategy is not run, and no 5m baseline is accepted.

## Decision

No edge. The 5m model hypothesis stops here. A fresh-onset variant — only the
first completed coiling window after a non-coiling bar — is the next model
hypothesis; it needs a new definition and a new pre-registration.

## Limitations

- The outcome is a boundary touch, not a return or a cost-aware payoff.
- The 5m series is derived from the same 1m source.
- The result is a near-miss, not a demonstrated edge.
- Same-session review, not an independent Quant Validator record.
