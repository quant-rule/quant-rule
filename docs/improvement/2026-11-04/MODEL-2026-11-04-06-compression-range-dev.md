# MODEL-2026-11-04-06 — Compression-Range Development

- Date: 2026-11-04 (UTC)
- Tier: model
- Changed: model
- Frozen: strategy, portfolio
- Partition: `model_dev` (2018-01-01..2020-12-31)
- Code commit: `8e8a481` plus the 377 working tree
- Manifest: `data/improvement/2026-11-04/MODEL-2026-11-04-06/manifest.json`
- Protocol: [`docs/model/2026-11-04/compression-range.md`](../../model/2026-11-04/compression-range.md)
- Baseline: `MODEL-2026-11-04-04..05` (accepted 1m 4 ATR compression baseline)
- Local artifacts: `data/improvement/2026-11-04/MODEL-2026-11-04-06/artifacts/`
- Verdict: **pass — freeze the event and open validation once**
- Claim type: model evidence; no strategy, cost, or edge claim

## Event

`signal.compression_range` (type 3, subtype 8, source 5), direction 0. At the
last bar of a 5-bar coiling window, `primary_value = range_high` and
`reference_value = range_low`. The event is neutral: it publishes the levels, not
the breakout side.

## Results

| n | J | Hit | Matched base | Coiling base | Lift | Lift LCB | Rand p | Back lag | Rep | Pass |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 346,120 | 60,605 | 0.6474 | 0.5442 | 0.6474 | +0.1032 | +0.1011 | 0.0050 | +0.0531 | 3 | yes |

Per instrument:

| Instrument | n | Hit | Matched base | Lift |
|---|---:|---:|---:|---:|
| BTCUSD | 134,622 | 0.6159 | 0.5056 | +0.1103 |
| ETHUSD | 99,179 | 0.7816 | 0.6593 | +0.1223 |
| XAUUSD | 112,319 | 0.5668 | 0.4890 | +0.0778 |

## Review

Every criterion passes. The event's next-bar boundary-touch rate is +10.32pp
over the matched non-coiling window completion, with a +10.11pp corrected lower
bound and a +0.0050 random-control p. All three instruments are positive. The
back-lag control is positive (+5.31pp), but the event lift exceeds it by
+5.01pp, so the window completion is not fully explained by persistent coiling.
The matched baseline is 54.42%, so the outcome is not saturated.

The event hit rate equals the all-coiling-bar rate (64.74%); the
window-completion selection is what the matched baseline isolates. No threshold
was selected from this result.

## Decision

Pass: freeze the 1m, 5-coiling-bar event and open `model_validation` 2021–2022
once. A validation pass is a candidate model baseline pending human acceptance,
not a strategy go.

## Limitations

- The outcome is a boundary touch, not a return or a cost-aware payoff.
- The event is neutral; it does not claim the expansion direction.
- Same-session review, not an independent Quant Validator record.
