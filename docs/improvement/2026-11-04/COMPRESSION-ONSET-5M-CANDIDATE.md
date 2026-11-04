# Fresh-Onset 5m Candidate — Pending Human Acceptance

- Date: 2026-11-04 (UTC)
- Decision: **accepted model baseline (human, 2026-11-04)**
- Event: `signal.compression_range_onset` (type 3, subtype 9, source 5), 5m
- Protocol: [`docs/model/2026-11-04/compression-range-onset-5m.md`](../../model/2026-11-04/compression-range-onset-5m.md)
- Development: [`MODEL-2026-11-04-10`](MODEL-2026-11-04-10-compression-onset-5m-dev.md)
- Validation: [`MODEL-2026-11-04-11`](MODEL-2026-11-04-11-compression-onset-5m-validation.md)

## What is claimed

The first completed 5-bar coiling window after a known non-coiling bar is
followed by a next-bar touch of at least one published range boundary more often
than a matched non-coiling window completion. The event publishes `range_high`
and `range_low`; it is neutral and does not claim the expansion direction.

## Gate

| Partition | n | J | Hit | Matched base | Lift | Lift LCB | Rand p | Back lag | Rep |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `model_dev` | 18,324 | 12,521 | 0.5706 | 0.4760 | +0.0946 | +0.0874 | 0.0050 | +0.0365 | 3 |
| `model_validation` | 14,303 | 9,708 | 0.5373 | 0.4651 | +0.0722 | +0.0640 | 0.0050 | −0.0065 | 3 |

Both partitions pass every pre-registered criterion. The back-lag difference is
+5.81pp in development and −0.65pp in validation, so the onset timing is clean
after removing the rolling-window overlap.

## Third-use disclosure

`model_validation` 2021–2022 was already used for the accepted 1m 4 ATR breakout
baseline and the accepted 1m range baseline. This candidate uses the same
partition for a 5m event and outcome. The third use is disclosed; residual
data-snooping risk is higher than a first use.

## What follows acceptance

The pre-registered 5m onset bracket runs `strategy_dev` 2018–2022: two-sided
stop entries at the published range boundaries, OCO, a 1 ATR stop, a 4 ATR
target, a 20-bar time stop, 4 bps commission, and 2 bps slippage. The 2024
`strategy_validation` partition is opened once only if development passes.

No strategy or portfolio result is claimed here.

## Acceptance

- Accepted by (human): Human
- Accepted on: 2026-11-04
- Scope: 5m, 5 coiling bars, fresh-onset next-bar boundary touch
- Supersedes: none
