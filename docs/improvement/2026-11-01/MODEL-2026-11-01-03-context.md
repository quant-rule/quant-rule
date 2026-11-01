# MODEL-2026-11-01-03 — Context-Conditioned Axis Evidence

- Date: 2026-11-01 (UTC)
- Tier: model
- Changed: model evidence
- Frozen: strategy, portfolio
- Partition: `model_validation` (2021)
- Manifest: [`manifests/MODEL-2026-11-01-03.json`](manifests/MODEL-2026-11-01-03.json)
- Local artifacts: `data/improvement/2026-11-01/MODEL-2026-11-01-03/`
- Verdict: `not green — no model baseline accepted`
- Claim type: model evidence; no edge claim

## Hypothesis

Direction and Momentum may carry a forward edge inside the model's own
volatility/participation context even when the unconditional hit rate is below
0.50. The model formula is unchanged; the harness adds pre-registered
`atr_ratio`, `rvol_ratio`, and `context.code` buckets.

## Results

Only two of 120 buckets have a 95% lower bound above 0.50 with at least 50
observations:

| Cell | Axis | Regime | n | Hit rate | 95% LCB |
|---|---|---|---:|---:|---:|
| ETHUSD 1h | Direction | `code=2` (coiling) | 2,914 | 0.5261 | 0.5080 |
| XAUUSD 15m | Momentum | `code=3` (settling) | 3,196 | 0.5185 | 0.5011 |

The other buckets are at or below 0.50 or too small. The two positive buckets are
on different instruments, axes, and regimes; neither repeats across instruments.

## Review

- `data/improvement/2026-11-01/MODEL-2026-11-01-03/review.md`
- `data/improvement/2026-11-01/MODEL-2026-11-01-03/decision.md`
- Artifacts: `results.json`, `artifacts/evidence.json`,
  `artifacts/regime-summary.csv`, `artifacts/confidence/*.json`

## Decision

No model baseline is accepted. The conditional evidence is a candidate
hypothesis, not a baseline. The next step is a deliberate model or strategy
contract change, not a threshold or bucket selection.

Options:

1. Condition the model axes on context (model contract change).
2. Expose a filtered regime/participation view to the strategy (strategy/model
   contract change).
3. Declare Direction/Momentum descriptive only and build on Structure,
   Progression, or levels (strategy contract change).

## Limitations

- One year, three instruments, two timeframes.
- 120 buckets create a multiple-testing surface; this is exploratory evidence.
- The strategy cannot read the context codes yet.
- Events, levels, divergences, and signals are still not graded.
- Same-session review, not an independent Quant Validator record.
