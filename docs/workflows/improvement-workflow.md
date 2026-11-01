# Improvement Workflow

- Date: 2026-11-01 (UTC)
- Status: Current; approved by the 2026-11-01 workflow reset
- Scope: model, strategy, and portfolio improvement runs

## Purpose

The Improvement Workflow decides whether a change improves the trading system.
It changes one tier at a time, freezes everything else, and compares the result
against an accepted baseline on data the change was not tuned on. The unit of
work is a recorded run, not a recollection of a backtest.

It does **not** fix program defects. A crash, a wrong value, a contract
violation, or a broken test is routed to the [Audit Workflow](audit-workflow.md).
It also does not accept its own result: a baseline is accepted by the human.

## The cycle

```text
Run -> Results -> Review -> New Hypothesis
```

| Stage | Question | Owner | Output |
|---|---|---|---|
| Run | What is the one hypothesis, and which tier and partition test it? | Loop Governor | Pre-registered run and `manifest.json` |
| Results | What did the run produce? | Result Analyst | Artifacts and a results table; no interpretation yet |
| Review | Is the result better than the baseline after costs, on the validation data, and without post-hoc changes? | Result Analyst | `review.md` with a recommendation |
| New Hypothesis | Do we accept a baseline, route a program defect, change one tier, or stop? | Triage Officer | `decision.md` and the next run, or `no edge` |

A run is not a result until its artifacts are reproducible. A result is not a
baseline until the human accepts it.

## Tiers

The three tiers run in order. A tier may not skip its gate, and a later tier may
not start while an earlier tier has no accepted baseline.

| Tier | Question | Changed | Frozen | Development data | Validation data | Promotion gate |
|---|---|---|---|---|---|---|
| Model Run | Does the measurement improve without breaking causality or determinism? | Model implementation and model configuration | Data, policy, strategy, execution, portfolio, analytics | `model_dev` | `model_validation` | Axis grades A/B on every axis a strategy will read; no D; C diagnostic only; human accepts the model baseline |
| Strategy Run | Does the rule produce a cost-inclusive edge from the accepted model? | Strategy rule, strategy configuration, and strategy thesis | Data, policy, accepted model baseline, execution, portfolio, analytics | `strategy_dev` | `strategy_validation` | Pre-registered green criteria met on the one-shot validation; Quant Validator record; human accepts the strategy baseline |
| Portfolio Run | Do execution and portfolio mechanics improve the account outcome from the accepted strategy? | Execution and portfolio configuration | Data, policy, accepted model baseline, accepted strategy baseline, analytics | `portfolio_dev` | `portfolio_holdout` | Pre-registered green criteria met on the one-shot holdout; Quant Validator record; human accepts the portfolio baseline |

### Model trust grades

Model evidence records one grade per published axis:

| Grade | Meaning | Allowed use |
|---|---|---|
| A | Passes the model checks and the pre-registered validation criteria | Usable in a strategy |
| B | Passes with a documented limitation | Usable in a strategy; the limitation is recorded in the baseline |
| C | Diagnostic only; not trusted enough to gate a trade | Research and diagnosis only; cannot feed a strategy decision |
| D | Fails a model check or the validation criteria | Blocks promotion; no strategy run until a new model hypothesis is accepted |

A model run with any D axis stops the sequence. The next action is a model
hypothesis, not a strategy run.

## Changed and frozen components

One tier changes; everything else is frozen by run identity and configuration
hash.

| Component | Model Run | Strategy Run | Portfolio Run |
|---|---|---|---|
| Data | Frozen | Frozen | Frozen |
| Policy | Frozen | Frozen | Frozen |
| Model | **Changed** | Frozen at the accepted baseline | Frozen at the accepted baseline |
| Strategy | Frozen at the accepted baseline or a fixed control | **Changed** | Frozen at the accepted baseline |
| Execution | Frozen | Frozen | **Changed** |
| Portfolio | Frozen | Frozen | **Changed** |
| Analytics | Frozen | Frozen | Frozen |

- **Data is never a tier.** A data defect is fixed through the Audit Workflow and
  gets a new data version; every affected baseline is then re-established.
- **Policy is human-owned.** A policy change is not an improvement run; it
  changes the envelope and requires a new baseline sequence.
- **Analytics is measurement, never a performance lever.** If the measurement is
  wrong, fix it under the Audit Workflow before the run or invalidate the run.
- **A change that spans tiers is split.** If a hypothesis needs a model and a
  strategy change, it is two runs with two gates, not one confounded run.

## Data partitions

The partitions are fixed. A run reads only its own development or validation
partition, and the manifest names it.

| Partition | Window | Use |
|---|---|---|
| `model_dev` | 2018-01-01..2020-12-31 | Model development |
| `model_validation` | 2021-01-01..2022-12-31 | Model confidence |
| `strategy_dev` | 2018-01-01..2022-12-31 | Strategy candidates |
| `strategy_validation` | 2024-01-01..2024-12-31 | One-shot strategy out-of-sample |
| `portfolio_dev` | 2024-01-01..2024-12-31 | Portfolio variants |
| `portfolio_holdout` | 2025-01-01..2025-12-31 | Final portfolio holdout |

- **2023 is excluded from every partition.** The 2023 crypto data is not
  repaired for this reset; including it in one tier only would make the tiers
  incomparable.
- **2024 answers two different questions.** `strategy_validation` judges a frozen
  strategy; `portfolio_dev` tests execution and portfolio variants after the
  strategy baseline is accepted. A portfolio variant may not re-tune the strategy
  on 2024.
- **A development run may not read a validation partition or the holdout.** The
  manifest must declare the matching partition; the run config must match the
  declaration. A breach invalidates the run; see the [run contract](run-contract.md).
- **Validation is one-shot per hypothesis.** A candidate is frozen by
  configuration hash before its validation run. A change after that point is a
  new hypothesis.

## Autonomy boundary

The workflow is autonomous in execution and reporting, and human-owned at
acceptance.

| Action | Agent | Human |
|---|---|---|
| Define a run under an approved tier and protocol | May | Approves the protocol |
| Write the manifest, execute the matrix, produce artifacts | May | — |
| Write `review.md` and propose a decision | May | — |
| Update the generated index | May | — |
| Accept or replace a baseline | May propose only | **Accepts** |
| Change policy or strategy | No | **Owns** |
| Change a partition, cost assumption, threshold, or green criterion after seeing a result | No | **New pre-registration** |
| Open the portfolio holdout | May execute the approved one-shot access | **Approves the protocol**; a second access needs a new decision |
| Declare an edge from one run | No | **Decides acceptance** |
| Delete a run or baseline | No | **Authorizes cleanup** |

Autonomy here concerns the improvement run. Commits, pushes, pull requests,
merges, releases, and destructive Git operations remain gated by `AGENTS.md`
section 9.

## Green criteria

A run declares its green criteria in the manifest **before** it executes. The
criteria are part of the run identity; a post-hoc change makes the run invalid.

Every tier requires:

- a pre-registered hypothesis, tier, partition, costs, and pass/fail rule;
- realistic fees, slippage, funding, and commission;
- an out-of-sample or validation result that was not used for tuning;
- the minimum trade count and exposure recorded before the run;
- drawdown within the policy envelope;
- stability across the pre-registered cells;
- deterministic, byte-identical artifacts on a rerun;
- no open Critical or Major audit finding on the changed path;
- a Quant Validator record for a model or strategy behavior change;
- no unrecorded variation.

Tier-specific gates are in the [tiers](#tiers) table. A portfolio result is not
green because `portfolio_dev` looks good; it is green only when the one-shot
holdout meets the pre-registered criteria.

## Stop conditions

Stop and record the reason when:

- a model axis grades D — do not start a strategy run; open a model hypothesis;
- no strategy candidate is green on `strategy_validation` — record `no edge` and
  do not optimize a portfolio;
- the portfolio holdout fails — do not accept a portfolio baseline; stop or
  pre-register a new hypothesis;
- a data partition is missing or fails validation — stop; the only approved
  fallback in the reset plan is 2024 H2 for the 2025 holdout, recorded as a
  limitation;
- a Critical or Major audit finding is open on the affected path;
- the iteration cap or compute budget is reached;
- a threshold, cost, split, or criterion changed after a result was seen;
- a program defect invalidated the run — record it in
  [`INVALIDATIONS.md`](../improvement/INVALIDATIONS.md), fix it under the Audit
  Workflow, and re-run;
- the evidence conflicts, the manifest is invalid, or an approval is missing.

A stop is a valid outcome. The workflow is not required to turn a red result
into a green one.

## Artifacts

| Artifact | Path |
|---|---|
| Run folder (manifest, config, artifacts, results, review, decision) | `data/improvement/<YYYY-MM-DD>/<run-id>/` |
| Machine-readable index | `data/improvement/index.json` |
| Generated human-readable index | `data/improvement/index.md` |
| Committed index snapshot | `docs/improvement/index.md` |
| Baseline registry | `docs/improvement/baselines.md`; optional working copy `data/improvement/baselines.json` |
| Invalidation tombstones | `docs/improvement/INVALIDATIONS.md`; optional working copy `data/improvement/invalidations.md` |
| Run record template | [`docs/improvement/README.md`](../improvement/README.md) |

The run-store tooling lives in `tools/improvement/` and is written in Python. It
validates manifests, runs the shell, extracts evidence, and generates the index.
The shell still executes the backtest; the tooling never computes a signal.
Baseline acceptance and invalidation are recorded in the committed registry by
the reviewer and the human; the tooling does not accept them.

## Related documents

- [Run contract](run-contract.md)
- [Audit Workflow](audit-workflow.md)
- [Improvement Workflow roles](../roles/2026-10-30/improvement-workflow-roles.md)
- [Improvement registry](../improvement/README.md)
- [Workflow reset execution plan](../execution-plans/2026-11-01/workflow-reset.md)
- [`AGENTS.md` section 11](../../AGENTS.md)
