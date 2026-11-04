# MODEL-2026-11-04-03 — Harder Compression-Expansion Cells

- Date: 2026-11-04 (UTC)
- Tier: model
- Changed: model
- Frozen: strategy, portfolio
- Partition: `model_dev` (2018-01-01..2020-12-31)
- Code commit: `91af224` plus the uncommitted harder-cell working tree
- Manifest: `data/improvement/2026-11-04/MODEL-2026-11-04-03/manifest.json`
- Protocol: [`docs/model/2026-11-04/compression-harder.md`](../../model/2026-11-04/compression-harder.md)
- Local artifacts: `data/improvement/2026-11-04/MODEL-2026-11-04-03/artifacts/`
  and the nine development bundles under `data/model-v4/compression/dev/`
- Verdict: **no edge under the family rule; 1m is a positive predictive lead**
- Claim type: model evidence; no strategy, cost, or edge claim

## Hypothesis

A compression breakout predicts a 20-bar forward range of at least 3 ATR on the
unseen 5m and 1m cells, after matched baselines, controls, and a correction for
two cells.

## Results

| Cell | n | J | Hit | Coiling base | Placebo base | Lift | Lift LCB | Rand p | Back lag | Rep | Pass |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 1m | 23,257 | 21,451 | 0.9243 | 0.8812 | 0.9013 | +0.0431 | +0.0392 | 0.0050 | −0.0488 | 3 | no: lift |
| 5m | 3,798 | 3,566 | 0.8770 | 0.8028 | 0.8696 | +0.0743 | +0.0623 | 0.0050 | −0.0476 | 3 | no: placebo |

Per instrument:

| Cell | Instrument | n | Hit | Coiling base | Lift |
|---|---|---:|---:|---:|---:|
| 1m | BTCUSD | 10,417 | 0.9533 | 0.9309 | +0.0225 |
| 1m | ETHUSD | 7,481 | 0.8999 | 0.8248 | +0.0751 |
| 1m | XAUUSD | 5,359 | 0.9018 | 0.8633 | +0.0386 |
| 5m | BTCUSD | 1,586 | 0.9155 | 0.8748 | +0.0407 |
| 5m | ETHUSD | 1,126 | 0.8632 | 0.7857 | +0.0775 |
| 5m | XAUUSD | 1,086 | 0.8352 | 0.7153 | +0.1199 |

## Review

The 1m cell is the strongest positive predictive evidence in the program:
+4.31pp lift, a +3.92pp corrected lower bound, random p 0.005, a +2.30pp
increment over the transition, and replication on all three instruments. It
misses the pre-registered 5pp lift threshold by 0.69pp.

The 5m cell has a larger lift (+7.43pp) but fails the placebo: the
coiling→expanding transition alone reaches 86.96%, almost the event's 87.70%.
The breakout adds only +0.74pp there.

Neither cell passes the family rule. No validation partition, baseline, or
strategy follows.

## Decision

The 1m cell is a positive lead, not a baseline. The next pre-registered cell is
**1m, 4 ATR over 20 bars**: a harder target that should lower the coiling base
and may lift the effect above 5pp. The 5m cell is deprioritised because the
transition dominates.

## Limitations

- The 1m/5m cells share the underlying M1 data with the prior 1h/15m result;
  the outcomes are unseen, but residual snooping risk remains.
- The 1m cell is close to the gate; changing the gate now would be post-hoc.
- Same-session harness, review, and decision; no independent Quant Validator
  record.
