# MR5 Power and Multiple-Testing Worksheet

- Date: 2026-11-08 (UTC)
- Status: Proposed; input to the MR5 charter
- Owner: Statistical Methodologist (Quant Researcher specialization)
- Reviewer: Quant Validator (read-only challenge)
- Charter: [`docs/governance/2026-11-08/MR5-CHARTER.md`](../../governance/2026-11-08/MR5-CHARTER.md)
- Claim type: power and multiple-testing design; no model result

## 1. Question

How many primary cells should the first MR5 confirmation use: five or ten?

The answer is not a preference. It is the largest number of cells for which the
minimum detectable effect (MDE) stays below the economic threshold for every
planned cell, after the multiple-testing correction, with the available sample.

## 2. Method

For each cell, the evaluator's lower confidence bound is approximately
`lift - z * SE`, where `z` is the two-sided normal quantile for the corrected
alpha and `SE` is the cluster-robust standard error. From an observed `lift` and
`LCB`, the implied standard error is:

```text
SE = (lift - LCB) / z_observed
```

The MDE at a new primary-cell count is `z_new * SE`. The power to detect a true
lift `mu` is `1 - Phi(z_new - mu / SE)`, assuming a normal approximation and the
same cluster structure.

The observed M4 development results use five primary cells, so `z_observed =
2.576` (two-sided alpha 0.01). Ten primary cells use `z_new = 2.807` (two-sided
alpha 0.005). These are the same correction algebra the evaluator uses.

## 3. MDE table

| Cell | Observed lift | Observed LCB | Implied SE | MDE at 5 | MDE at 10 | Power at 2pp, 5 | Power at 2pp, 10 |
|---|---:|---:|---:|---:|---:|---:|---:|
| M4.1 | +0.25pp | −0.40pp | 0.25pp | 0.65pp | 0.71pp | ~1.00 | ~1.00 |
| M4.2 | +0.35pp | −0.33pp | 0.26pp | 0.68pp | 0.74pp | ~1.00 | ~1.00 |
| M4.3 | −0.15pp | −0.99pp | 0.33pp | 0.84pp | 0.92pp | ~1.00 | ~1.00 |
| M4.4 | +1.57pp | −0.55pp | 0.82pp | 2.12pp | 2.31pp | 0.44 | 0.35 |
| M4.5 | +1.81pp | −0.44pp | 0.87pp | 2.25pp | 2.45pp | 0.39 | 0.30 |

The high-frequency cells are powered at either five or ten. The low-frequency
cells are not: their five-cell MDE is already above the 2pp gate, and the
ten-cell correction raises it by roughly 0.20–0.23pp. At a true 2pp effect, the
ten-cell correction costs the low-frequency cells about 9 percentage points of
power.

This is the core reason five is the honest first confirmation. Ten cells is
possible, but it requires more sample for the low-frequency cells — more
instruments, more years, or a larger cluster count.

## 4. Interpretation

- Five primary cells keep the correction at two-sided alpha 0.01 and keep the
  low-frequency MDE near the 2pp economic gate.
- Ten primary cells is defensible for a program whose cells all have the
  high-frequency sample density. It is not defensible when two of the planned
  cells are sparse.
- The MR5 exploratory grid is separate from this table. The 30-cell exploratory
  round is hypothesis generation; the five primary base cells are the
  confirmation. The exploratory cells receive no correction credit.
- A 100-cell sensitivity map does not change the confirmation power. It changes
  the selection risk. The primary base cells must be frozen before the map is
  read.

## 5. Recommendation

**Use five primary base cells for the first MR5 confirmation**:
`MR5.1-C01`–`MR5.5-C01`.

Use ten only if the Data Steward adds a fresh instrument/partition that raises
the low-frequency cluster count enough to bring the ten-cell MDE below 2pp. A
fresh instrument is a Data Lifecycle work item; it is not a free parameter of
the MR5 charter.

## 6. Limitations

- The MDE table is an approximation; the evaluator uses the cluster-robust
  standard error, and the normal approximation is less accurate at small
  cluster counts.
- The observed M4 lifts are not the true effects; they are used only to derive
  the plausible standard errors.
- The power calculation does not model correlation between hypotheses. Holm is
  more powerful than Bonferroni but does not remove the selection risk.
- The validation partition is reused; the Quant Validator must record the
  residual data-snooping risk.

## 7. Verification

The table is derived from the committed M4 development record
[`docs/improvement/2026-11-07/M4-DEV-SWARM.md`](../2026-11-07/M4-DEV-SWARM.md)
and the evaluator's two-sided corrected-alpha formula. The calculation is
reproducible from the observed lift and LCB columns; no new run is involved.
