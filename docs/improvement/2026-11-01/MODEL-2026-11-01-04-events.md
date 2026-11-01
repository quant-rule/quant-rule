# MODEL-2026-11-01-04 — Event Confidence

- Date: 2026-11-01 (UTC)
- Tier: model
- Changed: model evidence
- Frozen: strategy, portfolio
- Partition: `model_validation` (2021)
- Manifest: [`manifests/MODEL-2026-11-01-04.json`](manifests/MODEL-2026-11-01-04.json)
- Local artifacts: `data/improvement/2026-11-01/MODEL-2026-11-01-04/`
- Verdict: `not green — no model baseline accepted`
- Claim type: model evidence; no edge claim

## Hypothesis

Published Level, Divergence, and Signal events may carry a forward edge even when
the axes do not. Events are joined at `available_bar`, never `occurred_bar`.

## Results

Four of 300 event buckets have a 95% lower bound above 0.50 with at least 50
observations:

| Cell | Type | Subtype | Horizon | n | Hit rate | 95% LCB |
|---|---|---|---|---:|---:|---:|
| BTCUSD 1h | level | cross | 1 | 186 | 0.5806 | 0.5097 |
| BTCUSD 15m | level | cross | 10 | 721 | 0.5423 | 0.5059 |
| BTCUSD 15m | level | cross | 3 | 723 | 0.5408 | 0.5045 |
| BTCUSD 15m | level | test | 1 | 3,750 | 0.5187 | 0.5027 |

All four are BTCUSD; ETHUSD and XAUUSD do not repeat. Divergence and Signal
events do not produce a stable positive bucket.

## Review

- `data/improvement/2026-11-01/MODEL-2026-11-01-04/review.md`
- `data/improvement/2026-11-01/MODEL-2026-11-01-04/decision.md`
- Artifacts: `results.json`, `artifacts/evidence.json`, `artifacts/events/*.json`

## Decision

No model baseline is accepted. The event layer does not rescue the model. The
next step is a deliberate model or strategy contract change, not a subtype
selection.

## Limitations

- Six-week slices are required by the event command's full-window cap.
- 300 buckets create a multiple-testing surface.
- Events are evaluation evidence; the strategy cannot read them.
- Same-session review, not an independent Quant Validator record.
