# ADR: Workflow Reset — Audit Workflow and Improvement Workflow

- Date: 2026-11-01 (UTC)
- Status: Accepted
- Work item: `workflow-reset`
- Supersedes: [`docs/adr/2026-10-30/governing-swarm-and-improvement-loop.md`](../2026-10-30/governing-swarm-and-improvement-loop.md)
- Execution plan: [`docs/execution-plans/2026-11-01/workflow-reset.md`](../../execution-plans/2026-11-01/workflow-reset.md)

## Context

The previous workflow used one umbrella loop for two different kinds of work:
fixing the program and testing a trading hypothesis. That ambiguity had three
costs.

1. **Attribution.** A program fix and a model or strategy change could land in the
   same cycle, so a result could not be assigned to one cause.
2. **Evidence.** Runs were recorded as experiment documents and local lab
   folders. The record did not fix the run identity, the frozen inputs, the
   partition, or the baseline, so a later reader had to reconstruct them.
3. **Holdout discipline.** There was no single contract that refused a
   development run reading the final holdout, or that recorded the access.

The human approved a reset on 2026-11-01. The reset replaces the umbrella loop
with two explicit workflows and a run contract, and it accepts the cost of
discarding pre-reset records rather than carrying their ambiguity forward.

## Decision

### 1. Two workflows, not one loop

```text
AUDIT WORKFLOW
  Audit -> Remediation Plan -> Execution -> Verification

IMPROVEMENT WORKFLOW
  Run -> Results -> Review -> New Hypothesis
```

The Audit Workflow answers "is the program correct?" and never tests a trading
hypothesis. The Improvement Workflow answers "does this change improve the
trading system?" and never fixes a program defect. A finding that changes model
or strategy behavior moves to the Quant roles; the Audit Workflow cannot close it
alone.

### 2. Three improvement tiers, one change at a time

| Tier | Changed | Frozen | Development | Validation |
|---|---|---|---|---|
| Model | Model | Data, policy, strategy, execution, portfolio, analytics | `model_dev` | `model_validation` |
| Strategy | Strategy | Data, policy, accepted model, execution, portfolio, analytics | `strategy_dev` | `strategy_validation` |
| Portfolio | Execution, portfolio | Data, policy, accepted model, accepted strategy, analytics | `portfolio_dev` | `portfolio_holdout` |

The tiers run in order. A model axis graded D blocks the strategy run; no
strategy green means no portfolio optimization; a failed portfolio holdout means
no portfolio baseline. The full criteria are in
[`docs/workflows/improvement-workflow.md`](../../workflows/improvement-workflow.md).

### 3. Fixed data partitions

| Partition | Window |
|---|---|
| `model_dev` | 2018-01-01..2020-12-31 |
| `model_validation` | 2021-01-01..2022-12-31 |
| `strategy_dev` | 2018-01-01..2022-12-31 |
| `strategy_validation` | 2024-01-01..2024-12-31 |
| `portfolio_dev` | 2024-01-01..2024-12-31 |
| `portfolio_holdout` | 2025-01-01..2025-12-31 |

2023 is excluded from every partition because the 2023 crypto data is not
repaired for this reset. Validation and holdout are one-shot per hypothesis.

### 4. A run contract with a durable record

Every run has a tier-prefixed ID (`MODEL-`, `STRATEGY-`, `PORTFOLIO-`), an
immutable manifest, an exact config snapshot, hashed artifacts, a review, and a
decision. Run folders live under `data/improvement/`; the committed index,
baselines, and invalidation tombstones live under `docs/improvement/`. The
contract is in [`docs/workflows/run-contract.md`](../../workflows/run-contract.md).

### 5. Autonomy boundary

Execution and reporting are autonomous. The agent may define a run under an
approved protocol, execute it, write the manifest, review, and decision proposal,
and update the generated index. The human owns the policy and strategy, approves
the protocol, accepts baselines, approves the one-shot holdout, and decides
residual risk. Git operations remain gated by `AGENTS.md` section 9.

### 6. No backwards compatibility

The reset does not preserve the old workflow's schemas, paths, names, or records.
The retired name is not kept as an alias; pre-reset experiment and run records
are not imported, migrated, or treated as comparable evidence. They are
history, and invalidated evidence is removed later with a tombstone in
[`docs/improvement/INVALIDATIONS.md`](../../improvement/INVALIDATIONS.md).

## Alternatives considered

1. **Keep one umbrella loop and clarify its stages.** Rejected: the ambiguity is
   structural. A program fix and a trading change have different owners, gates,
   evidence, and acceptance criteria; one loop hides the boundary even with
   better prose.
2. **Keep the old role names and add the new workflows beside them.** Rejected:
   two names for one thing invite the old path to stay active. The reset renames
   the roles and leaves no alias.
3. **Use one generic run type for model, strategy, and portfolio.** Rejected: the
   changed/frozen matrix and the partition rule differ by tier. A generic run
   would make it possible to change a model and a strategy in one record and call
   the result attributable.
4. **Allow pre-reset records to remain comparable evidence.** Rejected: they were
   produced under a different workflow, with different partitions and no
   manifest. Treating them as comparable would launder their assumptions into the
   new baselines.
5. **Keep backwards compatibility with old schemas and folders.** Rejected: a
   compatibility layer preserves two meanings of "run", lets stale records load,
   and makes the new contract optional in practice.
6. **Full autonomous baseline acceptance.** Rejected: the human owns the policy
   and strategy and accepts the residual risk. Autonomous execution and
   reporting is the approved boundary; baseline acceptance is not.
7. **Use 2023 as a validation or holdout year.** Rejected: the 2023 crypto data
   is not repaired for this reset, and including it in only one tier would break
   comparability.

## Consequences

Positive:

- A result is attributable to one tier, one partition, and one frozen input set.
- The holdout is a scarce, recorded resource rather than an informal choice.
- A later reader can reproduce a run from its manifest and config snapshot.
- Program defects and trading hypotheses have separate routes and separate
  acceptance.
- Invalidated evidence leaves a tombstone instead of disappearing.

Negative:

- Pre-reset experiment and audit records are not comparable and will be removed
  with tombstones. The history remains in Git and in the `v0.5.0` tag, not in the
  active evidence set.
- Every run carries more record-keeping: manifest, config copies, hashes, review,
  decision, index, and baseline updates.
- The tier order is slower than a combined change. That is the point: a combined
  change cannot be attributed.
- The 2025 holdout is spent on the first portfolio hypothesis. A later hypothesis
  needs a new window and a new human decision.

## No-backwards-compatibility policy

The following are deliberate and are not defects:

- Old run schemas, manifest shapes, folder names, and experiment records are not
  read by the new tooling. A missing schema is a rejection, not a migration.
- The retired workflow name is not accepted as an alias in documentation, code,
  or CLI help.
- There is no dual reader, no compatibility flag, and no fallback to a pre-reset
  path.
- A breaking change to the run contract creates a new schema version; existing
  runs are not migrated.
- Deleted evidence is recorded in the invalidation log before removal. A
  tombstone is the only compatibility the reset offers: it says what was
  removed, why, and what replaced it.

These rules apply until the human approves a different policy in a new ADR.

## Related documents

- [Audit Workflow](../../workflows/audit-workflow.md)
- [Improvement Workflow](../../workflows/improvement-workflow.md)
- [Run contract](../../workflows/run-contract.md)
- [Improvement registry](../../improvement/README.md)
- [`AGENTS.md` sections 10–13](../../../AGENTS.md)
