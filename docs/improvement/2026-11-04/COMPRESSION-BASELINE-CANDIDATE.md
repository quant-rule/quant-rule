# COMPRESSION-BASELINE-CANDIDATE — 1m 4 ATR

- Date: 2026-11-04 (UTC)
- Development run: [`MODEL-2026-11-04-04`](MODEL-2026-11-04-04-compression-4atr-dev.md)
- Validation run: [`MODEL-2026-11-04-05`](MODEL-2026-11-04-05-compression-4atr-validation.md)
- Protocol: [`docs/model/2026-11-04/compression-harder.md`](../../model/2026-11-04/compression-harder.md)
- Status: **accepted model baseline (human, 2026-11-04)**
- Claim type: model evidence; no strategy or edge claim

## Decision

The frozen 1m, 4 ATR, 20-bar compression-breakout cell passes both partitions.

| Partition | n | J | Hit | Coiling base | Placebo base | Lift | Lift LCB | Rand p | Back lag | Rep |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `model_dev` | 23,257 | 21,451 | 0.7943 | 0.7260 | 0.7455 | +0.0683 | +0.0622 | 0.0050 | −0.1331 | 3 |
| `model_validation` | 12,397 | 11,686 | 0.7582 | 0.6950 | 0.7294 | +0.0632 | +0.0556 | 0.0050 | −0.1354 | 3 |

The development cell was selected from two pre-registered 4 ATR cells; the 5m
cell failed the placebo because the coiling→expanding transition, not the
breakout, carried the signal. The 1m cell passes every condition on both
partitions: lift ≥ 5pp, corrected LCB > 0, placebo, random, back-lag, and
three-instrument replication.

This is an **accepted model baseline**. The human accepted it on 2026-11-04. No
strategy, execution, or portfolio work starts from the model result alone.

## What this means

This is the first model claim in the program that passes both partitions with
the pre-registered baselines, controls, and correction. The compression-breakout
event has genuine forward-range information at 1m: it predicts a 20-bar range of
at least 4 ATR more often than coiling alone, more often than the placebo
transition, and more often than random coiling, with all three instruments
positive on both partitions.

The 5m result is a useful negative: at 5m the volatility transition alone
explains the expansion, and the breakout confirmation adds almost nothing. The
incremental information is at 1m in this sample.

## Limitations

- The 1m timeframe carries high transaction costs. This is a model claim, not a
  cost-adjusted strategy or profitability claim.
- The same M1 data underlies prior 1h/15m results; the 4 ATR outcomes were
  unseen, but residual snooping risk remains.
- The harness, review, and decision are same-session passes, not an independent
  Quant Validator record.
- The 1m model result must not be read as a tradeable edge until a strategy tier
  tests entries, exits, costs, and slippage.

## Next step

A separate strategy pre-registration may test the frozen 1m event with realistic
costs. No strategy work starts before that pre-registration and approval.
