# COMPRESSION-RANGE-5M-GATE — 5m Range Decision

- Date: 2026-11-04 (UTC)
- Development run: [`MODEL-2026-11-04-08`](MODEL-2026-11-04-08-compression-range-5m-dev.md)
- Validation run: none — not opened
- Protocol: [`docs/model/2026-11-04/compression-range-5m.md`](../../model/2026-11-04/compression-range-5m.md)
- Gate: **no edge**
- Claim type: model evidence; no strategy, cost, or edge claim

## Decision

The 5m compression-range event fails `model_dev` on the pre-registered back-lag
control.

- n = 84,627; J = 11,234; hit 55.45% versus matched 47.32%.
- Lift +8.13pp; corrected LCB +7.75pp; random p 0.005; replication 3/3.
- Back-lag lift +3.33pp; event lift − back-lag = +4.80pp, 0.20pp below the +5pp
  threshold.

The 2024 validation partition is not opened. The 5m strategy is not run. No 5m
baseline is accepted.

## What this means

The 5m event has a strong conditional lift, but rolling 5m coiling persists
across adjacent windows, so the one-bar-lagged rule already captures +3.33pp.
The incremental timing edge is just below the pre-registered bar. The gate is
not relaxed after the result.

The accepted 1m range model baseline and the 1m no-edge bracket result are
unchanged.

## Next step

A **fresh-onset** variant — only the first completed coiling window after a
non-coiling bar — would remove the overlap and test whether the onset carries
the incremental edge. It is a new model event definition and a new
pre-registered gate, not a threshold change.

## Limitations

- The outcome is a boundary touch, not a return or a cost-aware payoff.
- The 5m series is derived from the same 1m source.
- The result is a near-miss, not a demonstrated edge.
- Same-session review, not an independent Quant Validator record.
