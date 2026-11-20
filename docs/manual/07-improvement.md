# 7. Improvement

- Status: Current
- Audience: quant researcher pre-registering a hypothesis and recording the human decision

## Goal

Pre-register an improvement run before execution, submit its coupled run,
compare published numbers with a baseline, and record one explicit human
decision.

## Before you start

| Need | Value |
|---|---|
| Hypothesis | One falsifiable sentence, fixed before the run |
| Tier | `model`, `strategy`, or `portfolio` |
| Partition | A partition valid for the tier |
| Dataset and config | Both already exist in the profile |
| Criteria | Required for strategy and portfolio; optional for model |
| Green criteria | A JSON object recorded in the manifest |
| Code commit | The commit that will execute the run |
| Human | The person who owns the decision |

An improvement run is a coupled pipeline run. The config must carry a strategy; a model-tier
run still freezes strategy and portfolio. Stages are never research inputs.

## Steps

| # | Shell | Browser | Expect |
|---:|---|---|---|
| 1 | `quantrule research create --tier model --hypothesis "a retest holds" --partition model_dev --data-version <data_version> --config <config_id> --green-criteria '{"return_target":0.1}' --code-commit <commit>` | **Research** → Create pre-registration | A server-assigned `improvement_id` and `state: created` |
| 2 | `quantrule research submit <improvement_id>` | Research → open the record → Submit pipeline run | `pipeline_run_id`, `state: submitted` |
| 3 | `quantrule research list --tier model` | Research → improvement table | One row per pre-registration |
| 4 | `quantrule research show <improvement_id>` | Research → record detail | State, manifest, run link, lazy digest, timestamps |
| 5 | `quantrule research compare <improvement_id> --baseline <baseline_id>` | Research → Compare with baseline | Candidate, baseline, delta, and an availability note |
| 6 | `quantrule research decide <improvement_id> --decision accepted --note "criteria passed"` | Research → Human decision → Record decision | One decision and, when accepted, one baseline row |
| 7 | `quantrule baseline list` | Research → Accepted baselines | Append-only baseline rows |
| 8 | `quantrule baseline show model` | Research → Baseline detail | The current model baseline, or `research.not_found` |
| 9 | `quantrule invalidation list` | Research → Invalidation tombstones | Append-only tombstones |

`research create` always assigns a new id. A repeated public create is a new
pre-registration; only an internal same-id create is idempotent. The manifest is
immutable after creation.

## The v3 manifest

| Field | Rule |
|---|---|
| `tier` | The changed tier |
| `changed` | The tier under test |
| `frozen` | The remaining engines, fixed before the run |
| `partition` | The named development, validation, or holdout partition |
| `data_version`, `config_id` | References, never payloads |
| `criteria_id` | Required for strategy and portfolio |
| `green_criteria` | The human-owned pass rule |
| `code_commit` | The commit to reproduce |
| `seed` | Recorded for any seeded work |
| `baseline` | Optional comparison baseline |

The manifest records references and decisions. It does not carry a reading, a
decision row, or an artifact.

## Two-step link and decision rules

| Stage | Rule |
|---|---|
| Submit | Links one pipeline run and refuses a second submit |
| Digest | Linked when the run succeeds, not at submission; `research show` can refresh it |
| Accepted decision | Requires a succeeded run and canonical digest |
| Rejected, no-edge, new-hypothesis, invalidated | May be recorded without a digest |
| Second decision | Refused with `research.conflict` |
| Accepted | Appends one baseline row in the decision transaction |
| Invalidated | Appends one invalidation tombstone |

There is no automatic loop. Nothing submits a run, compares it, or accepts a
decision on its own. The human owns the decision, and the note is required.

## What you see

| Field | Meaning |
|---|---|
| `improvement_id` | Server-assigned pre-registration id |
| `state` | `created`, `submitted`, or `decided` |
| `decision` | The recorded human decision, or `—` |
| `pipeline_run_id` | The one coupled run linked at submit |
| `canonical_digest` | The succeeded run's digest, or `—` |
| Comparison | Published coverage or portfolio metrics; never recomputed |
| Baseline row | Accepted tier, run, scope, supersession, and date |
| Invalidation row | Removed scope, date, commit, reason, and supersession |

Seeded historical baselines have no pipeline run, so their comparison values are
unavailable. That is a recorded limitation, not a zero.

## What it proves

**Governed evidence.** The improvement run proves a hypothesis was fixed before
execution, its coupled run is linked, and a human recorded the decision against
published numbers. The baseline is accepted evidence; a stage artifact remains
diagnostic and is never a baseline.

## Failure

| Code or symptom | Cause | Next action |
|---|---|---|
| `research.not_found` | Unknown improvement id or tier baseline | Re-read `research list` or `baseline list` |
| `research.invalid` | Bad manifest, partition, config, dataset, criteria, or note | Read the message; fix the pre-registration |
| `research.conflict` | Duplicate submit/decision, or accepted without a succeeded digest | Read the record; wait for the run or use a non-accepted decision |
| `config.invalid` | The config is invalid or not coupled | Fix the config; a model-only config is refused |
| `run.not_found` | The linked run is missing | Re-submit only when the record is still `created` |
| `stage.not_found` | A stage artifact was offered as a run | Use the coupled run; a stage is never a research input |
| `service.unavailable` | A required service is down | Read `health`; stop |

## Related

- [Backtest](06-backtest.md)
- [Operations](08-operations.md)
- [Improvement Workflow](../workflows/improvement-workflow.md)
- [Run Contract](../workflows/run-contract.md)
