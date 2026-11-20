# End-to-End Verification Workflow

- Date: 2026-11-16 (UTC)
- Status: Current; the operations surface and composed tier are implemented (#858)
- Scope: proving a clean deployment works from health to a verified canonical digest
- Owner: Release / Operations Verifier
- Contract: [Release & Operations](release-operations.md), [Run Contract](run-contract.md)
- Claim type: Behaviour Claim
- Related: [Backtest Workflow](backtest.md), [Engine Execution](engine-execution.md), [Deployment](../architecture/deployment.md)

## 1. Purpose

This workflow answers one question: **does a clean deployment work from a fresh
stack to a verified result, and is the result reproducible?**

It is an operational check. It is not research evidence, not a baseline, and not
a qualification verdict. It proves the product path and the image, not an edge.

## 2. When to use

Use it when:

- a new deployment or upgrade must be accepted;
- a reviewer must reproduce the reference result on a clean host;
- a release candidate must be checked before a tag;
- a service or storage change could break the end-to-end path.

Do not use it when:

- the task is a historical evidence run: use the Backtest Workflow;
- the task is one-engine diagnosis: use Engine Execution;
- the task is a hypothesis or a baseline: use the Improvement Workflow.

## 3. Stages

| Stage | Question | Owner | Output | Gate |
|---|---|---|---|---|
| 1. Start | Does the stack build and start? | Release / Operations | Healthy stack | `make up` prints the URL |
| 2. Health | Are all services reachable? | Release / Operations | Health report | Every required service is up |
| 3. Profile | Is the research line scoped? | Operator | Active profile | `profile create`/`use` |
| 4. Data | Does the reference CSV upload? | Operator | `data_version` | Upload returns coverage and quality |
| 5. Config | Is the reference config valid? | Operator | `config_id` | `config show` prints `valid` |
| 6. Run | Does the full pipeline complete? | Release / Operations | `run_id`, `succeeded` | `run status` shows every stage done |
| 7. Fetch/verify | Are the artifacts intact? | Release / Operations | Artifact set, digest | Every hash verifies |
| 8. Compare | Does the digest match the recorded reference? | Release / Operations | Pass/fail | Exact digest equality |
| 9. Record | Is the verification recorded? | Release / Operations | Verification record | Human accepts |

## 4. Procedure

```bash
make up
export QUANTRULE_API_URL=http://localhost:8080/api/v1

quantrule health
quantrule profile create "Verification" --id verification
quantrule profile use verification

quantrule dataset upload config/examples/verify-mini/candles.csv --name verify-mini
quantrule dataset list
quantrule inspect candles <data_version>
quantrule inspect timeframes <data_version>

quantrule config list
quantrule config show <config_id>
quantrule run submit <data_version> <config_id>
quantrule run status <run_id>
quantrule run fetch <run_id> ./verify-bundle
quantrule verify <run_id>
```

The checked-in `config/examples/verify-mini` fixture is the reference. The
`verify-mini-replica` fixture has the same effective behavior and must produce
the same canonical digest.

The automated path is:

```bash
make verify
```

`make verify` submits the two equivalent configurations and prints one canonical
digest. A reviewer compares the printed digest with the recorded one. The
composed tier (`tools/test_tiers.sh composed`) runs the same comparison against
the real stack and is required before a release. The step-by-step procedure is
the [end-to-end verification tutorial](../tutorials/2026-11-17/end-to-end-verification.md).

## 5. What is compared

| Item | Rule |
|---|---|
| Data identity | The uploaded `data_version` is content-derived; identical bytes return the existing version |
| Config identity | The `config_id` and `config_hash` are content-derived; the reference config is valid |
| Run identity | `run_id = sha256(data_version ‖ config_hash ‖ model_version ‖ image_digest)` |
| Canonical digest | SHA-256 over the sorted `(path, sha256)` for `results.json`, `*.csv`, and `*.ndjson`, excluding provenance, logs, and timestamps |
| Image digest | The image that executed the run; a different digest is a different execution |
| Artifacts | Every artifact matches its recorded size and SHA-256 |

A different machine, a different image digest, or a different data file is a
different execution. Record it; do not hide it.

## 6. Evidence status

| Rule | Meaning |
|---|---|
| Verification only | The run proves the deployment; it is not research evidence |
| No baseline | A verification run cannot be accepted as a model, strategy, or portfolio baseline |
| No qualification | A verification run is not a qualification verdict |
| No edge claim | A passing digest says the system reproduces; it says nothing about profitability |
| Human acceptance | The Release / Operations Verifier records the result; the human accepts the release |

## 7. Roles and gates

| Role | Responsibility | Boundary |
|---|---|---|
| Release / Operations Verifier | Start the stack, run the procedure, compare the digest, record the result | No deploy, merge, or tag without authorization |
| Operator | Provide the profile and dataset | No path; use identifiers |
| Human | Accept the deployment and authorize the release | Sole final authority |

| Gate | Human action |
|---|---|
| Deployment | Approve the environment and the image |
| Verification result | Accept the digest match or record the failure |
| Release | Authorize the tag separately |

## 8. Green criteria

A deployment verification is green when:

- `make up` reaches health and prints the URL;
- every required service reports healthy;
- the reference CSV uploads and returns coverage;
- the reference config is valid;
- the run reaches `succeeded` and every stage is done;
- every artifact hash verifies;
- the canonical digest matches the recorded reference;
- `make verify` or the composed tier passes;
- the verification record is written and the human accepts it.

## 9. Failure handling

| Symptom | Meaning | Action |
|---|---|---|
| Stack does not start | Build, configuration, or host failure | Read `make ps`, `make logs`, `/health`; stop |
| Service down | Deployment defect | Stop; do not call the verification green |
| Upload rejected | Data or limit failure | Fix the input; do not edit the fixture silently |
| Config invalid | Contract drift | Stop; route to the Audit Workflow |
| Run failed | Stage or service failure | Stop; record the stage and error |
| Digest mismatch | Non-reproducible result or different image | Stop; record both digests; do not accept the release |
| Artifact hash mismatch | Storage or transfer defect | Stop; re-fetch; do not accept |
| `queued` forever | No worker | Check the worker and queue; stop |

A failed verification is a finding, not a reason to lower the criterion.

## 10. Stop conditions

Stop and request human direction when:

- the stack cannot start or a required service is down;
- the reference fixture is missing or has changed without a recorded reason;
- the config is invalid or the run fails;
- the digest does not match and the difference is not explained by the image
  digest or a recorded input change;
- a migration or storage change prevents the run;
- the release candidate has an open Critical finding;
- the human rejects the verification.

## 11. Handoff

| To | Packet |
|---|---|
| Release & Operations | The verification record and the digest comparison |
| Backtest Workflow | The reference config and data version, if an improvement run follows |
| Recordkeeping | The verification date, image digest, run id, and canonical digest |
| Human | The acceptance or rejection of the deployment |

## 12. Related documents

- [Release & Operations](release-operations.md)
- [Backtest Workflow](backtest.md)
- [Engine Execution Workflow](engine-execution.md)
- [Run Contract](run-contract.md)
- [Deployment](../architecture/deployment.md)
- [End-to-end verification tutorial](../tutorials/2026-11-17/end-to-end-verification.md)
- [`docs/workflows/README.md`](README.md)
- [`AGENTS.md`](../../AGENTS.md)
