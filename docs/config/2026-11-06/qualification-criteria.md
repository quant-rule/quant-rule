# Qualification Criteria — Evidence, Verdicts, and Human Acceptance

- Status: Current; approved 2026-11-06; implemented 2026-11-06
- Applies to: the Qualification Engine and the `qualify` command
- Related: [`engine-artifacts.md`](engine-artifacts.md),
  [`command-surface.md`](../../architecture/command-surface.md),
  [`policy-and-strategy.md`](../2026-10-30/policy-and-strategy.md),
  [`improvement-workflow.md`](../../workflows/improvement-workflow.md),
  [`run-contract.md`](../../workflows/run-contract.md)
- Claim type: qualification contract; no model, strategy, or edge claim.

## 1. Boundary

The Qualification Engine is stage 6. It answers one question: **does the
evidence satisfy the human-owned acceptance criteria?** It does not answer
whether the strategy is profitable in general, and it does not accept the
strategy.

| Role | Owns |
|---|---|
| Human | The account envelope, objective, risk limits, universe, partitions, costs, criteria, thresholds, and the final acceptance decision. |
| Qualification Engine | Reads the evidence bundle and the criteria, evaluates each criterion, and writes a qualification record with a verdict and reasons. |
| Analytics Engine | Computes descriptive and evaluation metrics over the result artifacts. It does not write a verdict. |
| Strategy Engine | Produces decisions from published model measurements. It never reads a qualification record or a statistic. |

The engine never declares an edge, chooses a threshold after seeing the result,
drops a failing criterion, or writes the human's acceptance. A verdict is
evidence status, not a trade recommendation, a release approval, or a substitute
for human judgment.

## 2. Inputs

The `qualify` command consumes two explicit inputs:

1. **Evidence bundle.** A backtest bundle, or an equivalent set of portfolio and
   analytics artifacts, with its manifest and provenance. The bundle must name
   the data version, configuration identity, code revision, partitions, costs,
   seed, and every source artifact.
2. **Qualification criteria.** A human-authored `qualification.json` using
   `quantrule.qualification/v1`.

```json
{
  "schema": "quantrule.qualification/v1",
  "id": "Q-EURUSD-2025-01",
  "evidence": {
    "required_partitions": ["strategy_validation"],
    "minimum_trades": 100,
    "cost_model": "execution-and-portfolio",
    "multiple_testing": {
      "primary_cells": 12,
      "trials": 12,
      "correction": "holm",
      "family_alpha": 0.05
    }
  },
  "criteria": [
    {
      "id": "oos-return",
      "dimension": "out_of_sample",
      "mandatory": true,
      "metric": "total_return",
      "operator": ">",
      "threshold": 0.0,
      "evidence_artifact": "portfolio",
      "evidence_field": "metrics.total_return"
    },
    {
      "id": "max-drawdown",
      "dimension": "drawdown",
      "mandatory": true,
      "metric": "max_drawdown",
      "operator": "<=",
      "threshold": 0.20,
      "evidence_artifact": "portfolio",
      "evidence_field": "metrics.max_drawdown"
    }
  ],
  "not_applicable": [
    {
      "dimension": "walk_forward",
      "reason": "no sequential folds are declared"
    }
  ]
}
```

The criteria file is written **before** the run. A threshold that is changed
after the result is known is a new hypothesis and a new criteria file, not an
edit to this one. The engine records the criteria file's content identity in its
provenance.

The optional `not_applicable` array records a dimension that does not apply to
the hypothesis and why. It is not a criterion status; every criterion still has
exactly one of `pass`, `fail`, or `insufficient_evidence`. A dimension may not
appear both as a criterion and as not applicable.

A required partition is satisfied when the evidence period overlaps the
partition window. The record carries the evidence period and the exact overlap,
so a reader can judge how much of the partition was read. `strategy_validation`
and `portfolio_dev` share the 2024 window; the period alone cannot distinguish
them, so the run manifest and the criteria must name the intended partition.

### Evidence fields

`evidence_field` is a dotted path into the named artifact. The first segment is
the artifact document, and the rest are object keys or array indices:

| `evidence_artifact` | Document | Required prefix |
|---|---|---|
| `backtest` | `manifest.json` | `manifest.` |
| `portfolio` | `portfolio/metrics.json` | `metrics.` |
| `execution` | `execution/provenance.json` | `provenance.` |
| `model` | `model/result.json` | `result.` |

An absent key, a null, or a path that crosses a scalar is `insufficient_evidence`
with a stable reason code. A criterion that names an unsupported artifact or a
field without the required prefix is a criteria-file error, not a verdict.

The supported operators are `>`, `>=`, `<`, `<=`, `==`, and `!=`. The supported
cost model is `execution-and-portfolio`; the execution and portfolio artifacts
must carry `fee_rate`, `slippage_bps`, `commission_rate`, and `funding_rate`.
`multiple_testing` declares `primary_cells`, `trials`, `correction`, and
`family_alpha` before the run. The engine validates the declaration and records
the strictest adjusted alpha (`family_alpha / primary_cells` for Holm and
Bonferroni, `family_alpha` for none). It does not invent a p-value that the
result artifacts do not publish.

The account envelope in `policy.json` supplies the account currency, initial
cash, performance target, evaluation days, drawdown ceiling, leverage, position
limit, and universe. The qualification thresholds are separate from that
envelope; a policy limit is a constraint, not a pass rule.

## 3. Required evidence dimensions

A qualification protocol declares at least one mandatory criterion for each
dimension that applies to the hypothesis. A dimension that does not apply is
recorded as `not_applicable` with a reason; it is not silently omitted.

| Dimension | Question | Example evidence | Mandatory by default |
|---|---|---|---|
| `out_of_sample` | Did the frozen candidate survive data it was not tuned on? | Validation or walk-forward return, profit factor, drawdown, trade count. | Yes |
| `walk_forward` | Does the result hold across sequential folds? | Per-fold return, stability of the sign, fold coverage. | Yes when folds exist |
| `robustness` | Does the result hold across instruments, regimes, and nearby parameters? | Instrument/regime table, parameter-neighborhood table. | Yes |
| `drawdown` | Is the worst loss and its duration acceptable? | Max drawdown, drawdown duration, recovery, halt state. | Yes |
| `consistency` | Is the result driven by a few trades or one period? | Trade count, win rate, profit factor, return distribution, monthly returns. | Yes |
| `trade_count` | Is the sample large enough to support the claim? | Closed trades, trades per instrument and regime. | Yes |
| `exposure` | Is the capital exposure within the envelope? | Mean and maximum exposure, position value, time in market. | Yes |
| `tail_risk` | Are the worst outcomes bounded and understood? | Worst trade, worst month, loss distribution, concentration. | Yes |
| `cost_sensitivity` | Does the result survive higher fees and slippage? | Net return and profit factor at the declared cost multiples. | Yes |
| `parameter_stability` | Is the result stable around the chosen parameters? | Neighboring-cell return and sign, no isolated peak. | Yes |
| `multiple_testing` | Is the result corrected for the number of trials? | Primary-cell count, trial count, correction method, adjusted threshold. | Yes |
| `data_quality` | Is the input complete and aligned? | Coverage, gaps, partial buckets, zero-volume candles, source cadence. | Yes |
| `baseline_control` | Does the rule beat the matched controls? | Buy-and-hold, random, back-lag, and shuffled controls. | Yes |
| `reproducibility` | Can the same inputs reproduce the artifact? | Byte-identical rerun, manifest, data version, code revision. | Yes |
| `prop_firm_limits` | Does the result fit the intended account rules? | Daily loss, maximum loss, profit target, consistency rule, time limit. | Only when declared |

The engine does not choose these dimensions. The human names the mandatory ones
and their thresholds in the criteria file. The engine validates the declared
criteria and the not-applicable declarations; it does not infer whether an
omitted dimension applies. Completeness of the protocol is the human's
responsibility, and an incomplete protocol is not evidence of a complete
qualification.

## 4. Verdict semantics

Every criterion has one status:

| Status | Meaning |
|---|---|
| `pass` | The evidence exists, the metric is defined, and it satisfies the operator and threshold. |
| `fail` | The evidence exists, the metric is defined, and it does not satisfy the operator and threshold. |
| `insufficient_evidence` | The evidence is missing, undefined, not from the required partition, or not reproducible. |

The verdict is computed with a fixed precedence:

1. **`fail`** if any mandatory criterion is `fail`, even when another mandatory
   criterion is `insufficient_evidence`.
2. **`insufficient_evidence`** if no mandatory criterion is `fail` and at least
   one mandatory criterion is `insufficient_evidence`.
3. **`pass`** only when every mandatory criterion is `pass`.

A non-mandatory diagnostic criterion is recorded and never changes the verdict. A
missing measurement is `insufficient_evidence`, never zero and never a pass. A
valid zero is a value and is compared as one.

The verdict is scoped to the evidence bundle and the criteria file. It is not a
statement about the strategy in general, another instrument, another period, or a
future market.

## 5. What is not a pass

A qualification is not a pass merely because:

- the backtest is profitable;
- the result is in-sample only;
- it covers one instrument or one regime;
- costs and slippage are omitted or assumed away;
- the trade count is below the declared minimum;
- the drawdown breaches the declared ceiling;
- the result is the best cell of a parameter sweep;
- a model reading or event count looks predictive;
- the result cannot be reproduced byte for byte;
- a mandatory criterion is missing and the remaining criteria pass.

Profitability is one criterion, not the verdict. A `pass` means the evidence
satisfies the human's pre-registered criteria; it does not mean the strategy has
an edge, is safe, or is accepted.

## 6. Qualification record

`qualification.json` uses `quantrule.qualification-result/v1` and carries:

| Field | Rule |
|---|---|
| `schema` | `quantrule.qualification-result/v1`. |
| `stage` | `qualification`. |
| `id` | The criteria file's `id`. |
| `verdict` | `pass`, `fail`, or `insufficient_evidence`. |
| `criteria` | One object per criterion, in the criteria file's order: `id`, `dimension`, `mandatory`, `status`, `observed`, `threshold`, `operator`, `evidence_artifact`, `evidence_field`, `reason_code`. |
| `reasons` | One `criterion_id` and `code` for every criterion that is not `pass`, in criteria order. The shell maps codes to words. |
| `evidence` | The backtest manifest identity and content hash, data version, config identity, code revision, period, partitions and required-partition coverage, costs, seed, sample counts, multiple-testing declaration, and stage provenance. |
| `not_applicable` | The human's not-applicable dimensions and reasons. |
| `human_decision` | `null` from the engine. The human records the decision separately. |

`criteria.csv` carries the same rows in the same order with the header
`id,dimension,mandatory,status,observed,threshold,operator,evidence_artifact,evidence_field,reason_code`.
`qualification.md` is a deterministic rendering of the record. A reason code is
stable and does not embed a number that belongs in `observed` or `threshold`.

The stable reason codes are `operator_satisfied`, `operator_not_satisfied`,
`missing_measurement`, `non_numeric_measurement`, `required_partition_missing`,
`minimum_trades_not_met`, and `trade_count_unavailable`.

Before evaluating a criterion, the engine checks that the manifest and every
stage provenance are present and that each `provenance_hash` matches the bytes on
disk. A missing or tampered artifact is a refusal, not a criterion result.

## 7. Human acceptance

The engine writes `human_decision: null`. The human reviews the record, the
evidence bundle, the residual risks, and any open audit finding, then records the
decision in a separate human-authored `decision.md` in the qualification folder:

```markdown
# Human Decision — Q-EURUSD-2025-01

- Decision: accepted | rejected
- Verdict: pass | fail | insufficient evidence
- Residual risk: ...
- Reason: ...
```

The engine never writes this file. `qualify` prints the verdict and states that
human acceptance is pending. A `pass` may be rejected for residual risk. A `fail`
is not accepted as an edge; it may close the hypothesis or lead to a new,
pre-registered hypothesis. A `fail` is a valid research result, not a process
failure.

Acceptance is not a release, deployment, merge, or live-trading approval. Those
remain separate human gates.

## 8. Statistics boundary

Statistics is confined to evaluation and validation. The Qualification Engine
and the Analytics Engine may compute descriptive statistics and pre-registered
inferential statistics, including confidence intervals, bootstrap distributions,
and multiple-testing corrections. The following rules apply:

- The correction method, primary-cell count, trial count, and family alpha are
  declared in the criteria file before the run.
- The engine applies the declared correction. It does not choose a correction,
  a threshold, or a primary cell after seeing the result.
- No statistical value enters a strategy decision, a model event value, or a
  model reading. The strategy boundary test proves that the strategy cannot read
  a qualification record or an analytics metric.
- A p-value, confidence interval, or fitted coefficient is evidence about the
  result, not an input to the rule.

## 9. Verification

The implementation is covered by:

- Unit tests for every verdict path: all pass, one fail, one missing mandatory
  criterion, a diagnostic-only failure, fail precedence, a valid zero, and both
  evidence gates.
- `qualification_writes_the_bundle_and_matches_the_golden_fixture`, which fixes
  the record, the criteria table, the Markdown rendering, and the provenance.
- `qualification_fail_takes_precedence_over_missing_evidence`.
- `qualification_missing_partition_is_insufficient`.
- `qualification_missing_measurement_is_insufficient_not_zero`.
- `qualification_is_byte_deterministic`.
- `qualification_refuses_a_tampered_provenance`.
- `qualification_writes_the_bundle_and_matches_the_golden_fixture` asserts
  `human_decision: null` and that `decision.md` does not exist.
- `the_strategy_input_exposes_only_identity_and_progression` forbids a
  qualification record, verdict, or analytics metric in the strategy input.
- The command surface checker proves that `qualify` is listed only when the
  command, help entry, README row, tutorial, and artifact contract all exist.
