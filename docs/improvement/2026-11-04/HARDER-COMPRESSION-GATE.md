# HARDER-COMPRESSION-GATE — 3 ATR Cell Decision

- Date: 2026-11-04 (UTC)
- Development run: [`MODEL-2026-11-04-03`](MODEL-2026-11-04-03-compression-harder.md)
- Validation run: none — not opened
- Protocol: [`docs/model/2026-11-04/compression-harder.md`](../../model/2026-11-04/compression-harder.md)
- Gate: **no edge under the family rule; 1m is a positive predictive lead**
- Claim type: model evidence; no strategy or edge claim

## Decision

Neither pre-registered 3 ATR cell passes the family rule.

- **1m:** +4.31pp lift, corrected LCB +3.92pp, random p 0.005, placebo +2.30pp,
  back-lag −4.88pp, replication 3. It misses the +5pp lift gate by 0.69pp. This
  is the strongest positive predictive result so far, but not a baseline.
- **5m:** +7.43pp lift, corrected LCB +6.23pp, random p 0.005, replication 3,
  but the placebo transition reaches 86.96% vs the event's 87.70%; the breakout
  adds only +0.74pp. It fails the placebo condition.

The validation partition is not opened. No baseline, strategy, or portfolio
work follows.

## What this means

The compression breakout has statistically significant, replicated forward-range
information at 1m. On 5m the volatility transition carries most of the signal;
the breakout confirmation adds little. The 1m effect size is just below the
pre-registered gate, so it cannot be called a baseline.

## Next step

A new pre-registration for **1m, 4 ATR over 20 bars** is the clean next cell: a
harder target that should lower the coiling base and may lift the effect above
5pp. The 5m cell is deprioritised. The 3 ATR thresholds and event are unchanged;
the 4 ATR protocol must disclose the 3 ATR result and open validation only once.

## Limitations

- The 1m/5m cells share the underlying M1 data with the prior 1h/15m result;
  the outcomes are unseen, but residual snooping risk remains.
- The 1m effect is close to the gate; lowering the gate is not allowed.
- Same-session harness, review, and decision; no independent Quant Validator
  record.
