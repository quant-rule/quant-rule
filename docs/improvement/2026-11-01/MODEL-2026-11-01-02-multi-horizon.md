# MODEL-2026-11-01-02 — Multi-Horizon Axis Study

- Date: 2026-11-01 (UTC)
- Tier: model
- Changed: model evidence
- Frozen: strategy, portfolio
- Partition: `model_validation` (2021)
- Manifest: [`manifests/MODEL-2026-11-01-02.json`](manifests/MODEL-2026-11-01-02.json)
- Local artifacts: `data/improvement/2026-11-01/MODEL-2026-11-01-02/`
- Verdict: `not green — no model baseline accepted`
- Claim type: model evidence; no edge claim

## Hypothesis

Direction and Momentum may fail at one horizon and work at another. The
pre-registered grid is 1, 3, 5, 10, and 20 bars. The model formula is unchanged;
the harness now reports the grid and Structure/Progression rank monotonicity.

## Results

Direction hit rate by horizon:

| Cell | 1 bar | 3 bars | 5 bars | 10 bars | 20 bars |
|---|---:|---:|---:|---:|---:|
| BTCUSD 15m | 0.4964 | 0.4893 | 0.4824 | 0.4803 | 0.4817 |
| BTCUSD 1h | 0.4940 | 0.4818 | 0.4874 | 0.4793 | 0.4701 |
| ETHUSD 15m | 0.4982 | 0.4918 | 0.4891 | 0.4928 | 0.4911 |
| ETHUSD 1h | 0.5021 | 0.5021 | 0.5051 | 0.5028 | 0.5139 |
| XAUUSD 15m | 0.4942 | 0.4942 | 0.4942 | 0.4923 | 0.4912 |
| XAUUSD 1h | 0.5005 | 0.4892 | 0.4875 | 0.4729 | 0.4768 |

Momentum is below 0.50 in every cell and horizon. Structure monotonicity is
inconsistent across instruments: BTCUSD 15m is −1.0 at 1–5 bars while XAUUSD
15m is +0.6. Progression monotonicity flips sign. The only lower bound above
0.50 is ETHUSD 1h Direction at 20 bars (0.5139, LCB 0.5031), one of thirty
Direction cells.

## Review

- `data/improvement/2026-11-01/MODEL-2026-11-01-02/review.md`
- `data/improvement/2026-11-01/MODEL-2026-11-01-02/decision.md`
- Artifacts: `results.json`, `artifacts/evidence.json`,
  `artifacts/model-confidence-summary.csv`, `artifacts/confidence/*.json`

## Decision

No model baseline is accepted. The horizon hypothesis is rejected as a rescue.
The next hypothesis is a model change, not a strategy run: a volatility/regime
reading, a documented Structure interpretation, or a documented decision to
treat Direction/Momentum as descriptive only.

## Limitations

- One year, three instruments, two timeframes.
- Thirty Direction and thirty Momentum cells create a multiple-testing surface;
  the single ETHUSD 1h cell is not corrected.
- Events, levels, divergences, and signals are not graded.
- Same-session review, not an independent Quant Validator record.
