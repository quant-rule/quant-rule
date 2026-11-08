# MR7 Power and Multiple-Testing Worksheet

- Date: 2026-11-08 (UTC)
- Status: Input to the approved MR7 charter
- Owner: Statistical Methodologist (Quant Researcher specialization)
- Reviewer: Quant Validator (read-only challenge)
- Charter: [`docs/governance/2026-11-08/MR7-CHARTER.md`](../../governance/2026-11-08/MR7-CHARTER.md)
- Claim type: power and multiple-testing design; no model result

## 1. Question

Does the opening-range sweep/reclaim grid have enough sample at five primary
cells, and does the new path gate raise the evidence bar enough to avoid a
hit-rate pass on a negative-expectancy base?

## 2. Seed evidence

MR5 and MR6 exhausted the trend, momentum, and broad participation families:
the best primary lift was +0.79 pp (MR5.1, LCB −0.42 pp), and the MR6 control
was +0.08 pp. The rare MR5.7 subset reached +2.0–2.4 pp on only 817–1,348
predictions. Every path menu was negative after 6 bps. MR7 is a single
level/liquidity event with a control, not a broad family.

The opening-range sweep is a same-bar event; the primary cells are the 15m
two-bar, the 1h two-bar, the 15m four-bar, the 15m relative-volume, and the
15m no-reclaim control.

## 3. MDE approximation

For a hit rate near 0.33, the binomial standard error at `n` predictions is
`sqrt(0.33 * 0.67 / n)`. At five primary cells the corrected two-sided alpha is
0.01, so `z = 2.576` and the approximate minimum detectable lift is:

| n | Approximate SE | Approximate MDE at 5 cells |
|---:|---:|---:|
| 200 | 0.0332 | 8.6 pp |
| 500 | 0.0210 | 5.4 pp |
| 1,000 | 0.0149 | 3.8 pp |
| 2,000 | 0.0105 | 2.7 pp |
| 3,650 | 0.0078 | 2.0 pp |
| 5,000 | 0.0066 | 1.7 pp |
| 10,000 | 0.0047 | 1.2 pp |

The table is a lower bound: the evaluator uses a cluster-robust standard error.
A cell near the 200-prediction floor is a count-gate pass, not a powered
confirmation. The 1h cell is the most likely to be underpowered; the seven
instruments (five existing plus two fresh price-only) are the mitigation.

## 4. Why five primary cells

- Four sweep cells plus one control. The control is a primary control, not a
  candidate; if it also reverses, the reclaim premise is falsified.
- Ten cells would raise the corrected `z` and the 2 pp sample requirement by
  about 19%, without adding an independent mechanism.
- The 10 exploratory variants are sweep-depth, opening-range-length, and
  relative-volume sensitivity checks; they receive no correction credit.

## 5. Cost and path gate

The 15m cells pay 6 bps per round trip. MR5/MR6 showed that a hit-rate pass on a
negative-expectancy base is not an edge. MR7 therefore adds a **mean net R > 0**
gate on the 2/1/20 menu. The gate is applied by the programme report, not by the
model grade alone. The 1h cell is the cost-robustness check: the same 6 bps is a
smaller fraction of ATR, but the sample is smaller.

## 6. Recommendation

**Use five primary cells, run the count gate first, and require both the
hit-rate and path gates.** Do not open `model_validation` unless a primary
candidate passes both and the human promotes it. Do not add a post-hoc cell to
rescue a weak count.

## 7. Limitations

- The MDE table assumes independent predictions; the cluster-robust evaluator is
  the authority.
- The opening-range sweep is correlated with the existing opening-range
  breakout/fade events; the control is designed to separate the reclaim from the
  break.
- The fresh instruments add cross-section, not a fresh time period.
- The validation partition is reused; residual data-snooping risk remains.
