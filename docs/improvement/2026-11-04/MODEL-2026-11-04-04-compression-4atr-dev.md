# MODEL-2026-11-04-04 — 4 ATR Compression Development

- Date: 2026-11-04 (UTC)
- Tier: model
- Changed: model
- Frozen: strategy, portfolio
- Partition: `model_dev` (2018-01-01..2020-12-31)
- Code commit: `91af224` plus the uncommitted harder-cell working tree
- Manifest: `data/improvement/2026-11-04/MODEL-2026-11-04-04/manifest.json`
- Protocol: [`docs/model/2026-11-04/compression-harder.md`](../../model/2026-11-04/compression-harder.md)
- Local artifacts: `data/improvement/2026-11-04/MODEL-2026-11-04-04/artifacts/`
- Verdict: **pass — freeze 1m, 4 ATR, 20 bars**
- Claim type: model evidence; no strategy or edge claim

## Results

| Cell | n | J | Hit | Coiling base | Placebo base | Lift | Lift LCB | Rand p | Back lag | Rep | Pass |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 1m | 23,257 | 21,451 | 0.7943 | 0.7260 | 0.7455 | +0.0683 | +0.0622 | 0.0050 | −0.1331 | 3 | yes |
| 5m | 3,798 | 3,566 | 0.6901 | 0.5993 | 0.6878 | +0.0908 | +0.0737 | 0.0050 | −0.1451 | 3 | no: placebo |

Per instrument, 1m:

| Instrument | n | Hit | Coiling base | Lift |
|---|---|---:|---:|---:|
| BTCUSD | 10,417 | 0.8618 | 0.8104 | +0.0514 |
| ETHUSD | 7,481 | 0.7560 | 0.6662 | +0.0898 |
| XAUUSD | 5,359 | 0.7164 | 0.6452 | +0.0711 |

## Review

The 1m cell passes every condition: +6.83pp lift, +6.22pp corrected lower bound,
random p 0.005, placebo +4.88pp, back-lag −13.31pp, and three-instrument
replication. The 5m cell fails the placebo: the transition reaches 68.78% and
the event 69.01%, so the breakout adds only +0.23pp there. The 1m cell is frozen
for validation; the 5m cell is not promoted.

## Decision

Open `model_validation` once with the frozen 1m, 4 ATR, 20-bar cell. No strategy
or portfolio work before human baseline acceptance.

## Limitations

- The 1m timeframe carries high transaction costs; this is a model claim.
- The same M1 data underlies prior results; the 4 ATR outcomes were unseen, but
  residual snooping risk remains.
- Same-session review, not an independent Quant Validator record.
