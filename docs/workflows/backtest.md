# Backtest Workflow

- Date: 2026-11-16 (UTC)
- Status: Current; the model-artifact reuse path lands in #855
- Scope: the coupled evidence run from a frozen configuration and a declared window
- Owner: Quant Validator (evidence), Implementation (run path), Human (acceptance)
- Contract: [Run Contract](run-contract.md), [ADR-2026-11-16-01](../adr/2026-11-16/01-stage-execution-and-evidence-boundary.md)
- Claim type: Behaviour Claim
- Related: [Engine Execution](engine-execution.md), [Improvement Workflow](improvement-workflow.md), [Release & Operations](release-operations.md)

## 1. Purpose

This workflow answers one question: **what did the frozen full configuration do
over the declared window, and can another person reproduce it?**

It is the only path that produces a canonical digest and the only run that can
become evidence or a baseline. It does not decide whether a change is an
improvement; that is the Improvement Workflow. It does not inspect one engine;
that is the Engine Execution Workflow.

## 2. When to use

Use it when:

- a model, strategy, execution, or portfolio configuration is frozen;
- a run must become evidence for review, a baseline, or an improvement record;
- a reviewer must reproduce a recorded result;
- a release needs an end-to-end evidence check.

Do not use it when:

- the task is diagnostic engine inspection: use Engine Execution;
- the change is a hypothesis or a one-tier comparison: use the Improvement
  Workflow;
- the task is a clean deployment check: use End-to-End Verification;
- the data is not admitted: use the Data Lifecycle Workflow first.

## 3. Stages

| Stage | Question | Owner | Output | Gate |
|---|---|---|---|---|
| 1. Freeze | What exactly is being tested? | Quant Researcher | `data_version`, full `config_id`, partition, costs, criteria | No post-hoc change |
| 2. Preflight | May this run start? | Quant Validator | Preflight record | Config valid, data admitted, partition declared, no open Critical finding |
| 3. Submit | Did the server accept the run? | Implementation | `run_id`, `queued` | `202` new or `200` lookup |
| 4. Observe | Did it finish, and where? | Implementation | Stage states | `succeeded` or a recorded failure |
| 5. Fetch/verify | Are the artifacts complete and intact? | Implementation | Artifact set, canonical digest | Every hash verifies |
| 6. Validate | Is the result valid evidence? | Quant Validator | Validation record | Deterministic rerun, costs, drawdown, trade count |
| 7. Record | What does the result mean? | Result Analyst | `review.md`, `decision.md` | Human accepts, rejects, or records no edge |
| 8. Reproduce | Can another run reproduce it? | Reviewer | Digest comparison | Same inputs and image digest → same digest |

## 4. Freeze

A backtest freezes one full run configuration:

- the `data_version`;
- the six-file `quantrule.run/v4` config: run, data, model, execution, portfolio,
  strategy; a model-only `quantrule.run/v3` config is allowed only for the model
  tier and carries no strategy;
- the declared partition;
- the costs, drawdown limit, trade-count floor, and green criteria;
- the hypothesis, when the run is an improvement run.

The config's `config_hash` is the identity. A change to any file is a new config
and a new run.

## 5. Preflight

| Check | Evidence |
|---|---|
| Config valid | `config show <config_id>` prints `valid` |
| Data admitted | `dataset show <data_version>` and the quality report |
| Partition declared | The run config window matches `data/partitions.json` |
| Costs declared | Execution fees, slippage, funding; portfolio commission |
| No open Critical finding | Audit/invalidation registry on the changed path |
| Pre-registration | For a research run, the Improvement Workflow manifest and green criteria |

A failed preflight stops the run. It is not a failed result.

## 6. Submit and observe

```bash
quantrule config list
quantrule config show <config_id>
quantrule run submit <data_version> <config_id>
quantrule run status <run_id>
quantrule run list --status running --limit 20
quantrule run cancel <run_id>
```

`run submit` returns immediately. The run is server state; closing the shell does
not stop it. A duplicate submission returns the existing run with `created=false`.

A run is not split across backtest workers. The stages are `data`, `model`,
`backtest`, and `evidence`; the backtest stage is sequential.

## 7. Fetch and verify

```bash
quantrule run fetch <run_id> ./bundle
quantrule verify <run_id>
```

`run fetch` checks every artifact against the run record's size and SHA-256
before writing it. `verify` prints the canonical digest the gateway recorded; it
does not recompute it.

The canonical digest is SHA-256 over the sorted list of `(path, sha256)` for
`results.json`, `*.csv`, and `*.ndjson`, excluding `provenance.json`, logs, and
any file that records where or when the run happened.

## 8. Validate

The Quant Validator independently checks:

- the run is reproducible byte for byte from the recorded inputs;
- the canonical digest matches;
- fees, slippage, funding, and commission are the declared costs;
- the trade count meets the declared floor;
- the drawdown is within the portfolio limit;
- the result is out-of-sample when the hypothesis requires it;
- no look-ahead, survivorship, split leakage, or data snooping is present.

A failing check is a result, not a reason to change the criteria.

## 9. Record

A research run becomes an improvement run under the Improvement Workflow. The
record carries the hypothesis, tier, partition, changed/frozen split, config,
data version, costs, green criteria, results, review, and decision. The human
accepts a baseline or records `no edge`.

A non-research run (a verification or a reviewer reproduction) is recorded as a
run and a digest comparison; it is not a baseline.

## 10. Reproduce

A reviewer reproduces with the same `data_version`, `config_id`, and image
digest. A second run must produce the same canonical digest. A different image
digest is a different execution; record it rather than hiding it.

## 11. Evidence rule

| Rule | Meaning |
|---|---|
| Coupled run only | Only `run submit` produces evidence; a stage artifact never does |
| Model artifact reuse | `run submit --model-artifact` may reuse a frozen model artifact; the model bytes must equal a full run and the artifact hash is recorded |
| Strategy/portfolio recompute | The backtest recomputes strategy, execution, and portfolio; stage decisions and fills are never inputs |
| No partial publish | A killed worker leaves no visible artifact and no `succeeded` run |
| One evidence command | There is no `backtest run` alias; `run submit` is the evidence command |

## 12. Roles and gates

| Role | Responsibility | Boundary |
|---|---|---|
| Quant Researcher | Freeze the hypothesis and criteria | No post-hoc change |
| Quant Developer | Provide the config and the run path | No rule change after freeze |
| Quant Validator | Preflight, reproduce, validate, report residual risk | Read-only; no repair |
| Result Analyst | Write the review | No acceptance |
| Human | Accept, reject, or record no edge | Sole final authority |

| Gate | Human action |
|---|---|
| Run start | Approve the preflight and, for research, the protocol |
| Result | Review the evidence and residual risk |
| Baseline | Accept only through the Improvement Workflow |
| Release | Approve the release/tag separately |

## 13. Green criteria

A backtest is green when:

- the config, data version, partition, costs, and criteria were frozen before
  the run;
- the run reaches `succeeded` and every artifact hash verifies;
- the canonical digest is recorded and a rerun reproduces it;
- the declared costs, trade count, and drawdown checks pass;
- the Quant Validator record is present;
- no open Critical or Major audit finding is on the changed path;
- the human accepts the result or records no edge.

## 14. Failure handling

| Failure | Result |
|---|---|
| Invalid input | `400` at submission; no run row |
| Model or backtest stage fails | `failed`; the run row records the error |
| Worker killed mid-run | No visible artifact; retry or `failed` |
| Evidence finalization fails | `running`; finalization retries |
| Cancelled | `cancelled`; no artifact set is finalized |

A failed run is not partially successful and is never evidence.

## 15. Stop conditions

Stop and request human direction when:

- the data is not admitted or the partition is not declared;
- the config changes after the freeze;
- the run is not reproducible;
- a cost, criterion, or split would change after the result;
- a Critical or Major finding is open on the path;
- the Quant Validator cannot reproduce the result;
- the human rejects the residual risk.

## 16. Handoff

| To | Packet |
|---|---|
| Improvement Workflow | The run record, artifacts, digest, review, and decision |
| Release & Operations | The accepted result and the remaining risks |
| Recordkeeping | The run ID, digest, and baseline or no-edge record |
| Engine Execution | Nothing as evidence; only a diagnostic question |

## 17. Related documents

- [Engine Execution Workflow](engine-execution.md)
- [Improvement Workflow](improvement-workflow.md)
- [Run Contract](run-contract.md)
- [ADR-2026-11-16-01](../adr/2026-11-16/01-stage-execution-and-evidence-boundary.md)
- [Command surface](../architecture/command-surface.md)
- [`docs/workflows/README.md`](README.md)
- [`AGENTS.md`](../../AGENTS.md)
