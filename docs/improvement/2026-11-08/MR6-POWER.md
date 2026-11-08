# MR6 Power and Multiple-Testing Worksheet

- Date: 2026-11-08 (UTC)
- Status: Input to the approved MR6 charter
- Owner: Statistical Methodologist (Quant Researcher specialization)
- Reviewer: Quant Validator (read-only challenge)
- Charter: [`docs/governance/2026-11-08/MR6-CHARTER.md`](../../governance/2026-11-08/MR6-CHARTER.md)
- Claim type: power and multiple-testing design; no model result

## 1. Question

Does the ordered family-confirmation grid have enough sample at five primary
cells to detect the 2 pp lift gate after the Bonferroni correction, and is five
the right number of primary cells?

## 2. Seed evidence and the width of the family

`MR5.7` (one trend event followed by one participation event) produced
+2.04–2.41 pp lift on 817–1,348 predictions, with a negative LCB. The pair was
too rare to confirm. MR6 does not repeat that pair: it widens the first side to
seven trend/momentum events and the second side to eight relative-volume
participation events. The 56 ordered pairs are one family-level mechanism, not
56 independent hypotheses.

The count gate is therefore read before the outcome. A cell that cannot reach
200 condition-matched predictions and 20 clusters is not evaluated.

## 3. MDE approximation

For a hit rate near 0.33, the binomial standard error at `n` condition-matched
predictions is `sqrt(0.33 * 0.67 / n)`. With five primary cells the corrected
two-sided alpha is 0.01, so `z = 2.576` and the approximate minimum detectable
lift is `2.576 * SE`:

| n | Approximate SE | Approximate MDE at 5 cells |
|---:|---:|---:|
| 200 | 0.0332 | 8.6 pp |
| 500 | 0.0210 | 5.4 pp |
| 1,000 | 0.0149 | 3.8 pp |
| 2,000 | 0.0105 | 2.7 pp |
| 3,650 | 0.0078 | 2.0 pp |
| 5,000 | 0.0066 | 1.7 pp |
| 10,000 | 0.0047 | 1.2 pp |

The table is a lower bound: the evaluator uses a cluster-robust standard error,
which is larger than the binomial one when predictions cluster. A cell near the
200-prediction floor is a count-gate pass, not a powered confirmation. A cell
needs roughly 3,650 or more predictions, and enough clusters, to make a 2 pp
effect detectable at five cells.

## 4. Why five primary cells

The five primary base cells are the two 15m order arms (`MR6.1`, `MR6.2`), the
two 5m order arms (`MR6.3`, `MR6.4`), and the 15m level arm (`MR6.5`).

- Ten primary cells would raise the corrected `z` from 2.576 to 2.807 and the
  2 pp sample requirement by about 19%. The family is already broad; adding
  cells would multiply the testing surface without adding an independent
  mechanism.
- The 10 exploratory variants (`C02`, `C03`) are window sensitivity checks. They
  receive no correction credit and are not candidates.
- The reverse-order cells are controls, not candidates. If a control passes, the
  order premise is falsified; the programme stops rather than promoting the
  control.

## 5. Cost and path

The 15m cells pay 6 bps per round trip. MR5 showed that at 15m the 2 ATR / 1 ATR
/ 20-bar menu can have a negative mean net R even when the hit rate is above its
base. The path report is descriptive and cannot rescue a failed hit-rate gate;
it is read to decide whether a passing cell is economically plausible. The 5m
cells add sample but also add cost drag; the path comparison across 15m and 5m
is part of the evidence, not a second gate.

## 6. Recommendation

**Use five primary base cells and run the count gate first.** Do not open
`model_validation` unless a primary base cell reaches grade A and the human
promotes it. Do not add a post-hoc cell to rescue a weak count.

## 7. Limitations

- The MDE table assumes independent predictions and a hit rate near 0.33; the
  cluster-robust evaluator is the authority.
- The family-level pairing is correlated: a trend event can pair with several
  participation events. The evaluator counts condition-matched bars, not pairs,
  so a bar is one prediction.
- The earlier MR5.7 lift is a hypothesis-generation reading, not a true effect;
  it is used only to justify the family width, not to predict MR6.
- The validation partition is reused; residual data-snooping risk remains.
