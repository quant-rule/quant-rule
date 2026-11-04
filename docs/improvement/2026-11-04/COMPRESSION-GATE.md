# COMPRESSION-GATE — Compression-Breakout Range-Expansion Decision

- Date: 2026-11-04 (UTC)
- Development run: [`MODEL-2026-11-04-02`](MODEL-2026-11-04-02-compression-breakout.md)
- Validation run: none — not opened
- Protocol: [`docs/model/2026-11-04/compression-breakout.md`](../../model/2026-11-04/compression-breakout.md)
- Gate: **inconclusive — outcome saturated**
- Claim type: model evidence; no strategy or edge claim

## Decision

The compression-breakout event is implemented and the pre-registered gate ran on
`model_dev`, but the outcome is too easy to discriminate. The 2 ATR / 20-bar
forward range is reached 98.88% of the time on coiling bars at 1h and 95.83% at
15m. The maximum possible lift is 1.12pp and 4.17pp, below the pre-registered
5pp threshold. The 1h event lift is +0.79pp (corrected lower bound +0.04pp,
random p 0.144); the 15m secondary cell is +2.03pp (corrected lower bound
+1.12pp, random p 0.005) but is not a primary cell.

This is an **inconclusive** result, not a no-edge finding and not a pass. The
validation partition is not opened, no baseline is accepted, and no strategy or
portfolio work starts.

## What this means

The event may carry information; this protocol cannot measure it because the
outcome is at the top of its distribution. Promoting the 15m secondary cell, or
lowering the lift threshold, would be post-hoc and is not allowed. The event,
thresholds, and outcome remain as pre-registered.

## Next step

The human may approve a new pre-registered hypothesis with a harder expansion
outcome that leaves room for a 5pp lift — for example, a 3 ATR target over 20
bars or a 2 ATR target over 10 bars. The new protocol should use a **fresh
sample**: the current `model_dev` paths have been seen, including the 15m
secondary, so reusing them would be post-hoc. A fresh instrument set (a policy
and partition change) or a fresh dataset is the clean path. This is a new
hypothesis, not a tuning of the current run. No code or run starts without that
approval.

## Limitations

- The 15m secondary result is descriptive and cannot be promoted.
- The same instruments were used for prior directional tests; the validation
  partition is unopened but residual snooping risk remains.
- Same-session harness, review, and decision; no independent Quant Validator
  record.
