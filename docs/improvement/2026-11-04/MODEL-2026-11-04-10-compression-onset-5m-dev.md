# MODEL-2026-11-04-10 — Fresh-Onset 5m Development

- Date: 2026-11-04 (UTC)
- Tier: model
- Changed: model
- Frozen: strategy, portfolio
- Partition: `model_dev` (2018-01-01..2020-12-31)
- Code commit: `2c85b41` plus the 381 working tree
- Manifest: `data/improvement/2026-11-04/MODEL-2026-11-04-10/manifest.json`
- Protocol: [`docs/model/2026-11-04/compression-range-onset-5m.md`](../../model/2026-11-04/compression-range-onset-5m.md)
- Baseline: `MODEL-2026-11-04-06..07`
- Local artifacts: `data/improvement/2026-11-04/MODEL-2026-11-04-10/artifacts/`
- Verdict: **pass — freeze the onset event and open validation once**
- Claim type: model evidence; no strategy, cost, or edge claim

## Event

`signal.compression_range_onset` (type 3, subtype 9, source 5), direction 0. The
event fires at the first completed 5-bar coiling window after a known
non-coiling bar, publishing `range_high` and `range_low`. It is neutral.

## Results

| n | J | Hit | Matched base | Coiling base | Lift | Lift LCB | Rand p | Back lag | Back-lag difference | Rep | Pass |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 18,324 | 12,521 | 0.5706 | 0.4760 | 0.5628 | +0.0946 | +0.0874 | 0.0050 | +0.0365 | +0.0581 | 3 | yes |

Per instrument:

| Instrument | n | Hit | Matched base | Lift |
|---|---:|---:|---:|---:|
| BTCUSD | 6,927 | 0.5575 | 0.4458 | +0.1117 |
| ETHUSD | 5,718 | 0.6368 | 0.5283 | +0.1084 |
| XAUUSD | 5,679 | 0.5198 | 0.4601 | +0.0597 |

## Review

Every criterion passes. The onset selection removes the rolling-window overlap:
the back-lag difference is +5.81pp, up from +4.80pp for the 5m rolling event,
and all other criteria pass. The sample is smaller than the rolling event but
well above the thresholds.

## Decision

Pass: freeze the 5m onset event and open `model_validation` once. A validation
pass is a candidate model baseline pending human acceptance, not a strategy go.

## Limitations

- The outcome is a boundary touch, not a return or a cost-aware payoff.
- The 5m series is derived from the same 1m source.
- Same-session review, not an independent Quant Validator record.
