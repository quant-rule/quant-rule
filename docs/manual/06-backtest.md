# 6. Backtest

- Status: Current
- Audience: operator producing coupled evidence, with or without a frozen model artifact

## Goal

Submit a coupled v4 or v5 run, distinguish run identity from the canonical
digest, read its recorded lineage, fetch and verify its artifacts, and know what
the result proves.

## Before you start

| Need | Value |
|---|---|
| Dataset | A `data_version` |
| Config | A valid v4 config that carries the strategy, or a v5 config that cites its packs by id and hash |
| Model artifact | Optional: a succeeded model stage for a promoted run |
| Profile | The profile that will own the run submission |

A coupled run is the only evidence path. A stage is never a substitute for it.

## Steps

| # | Shell | Browser | Expect |
|---:|---|---|---|
| 1 | `quantrule run submit <data_version> <config_id>` | **Configs** → choose dataset and config → Submit | `run_id`, `status: queued`, `created: true` |
| 2 | `quantrule run submit <data_version> <config_id> --model-artifact <model_artifact_id>` | **Configs** → Frozen model artifact → Submit | A run with `model_artifact_id` recorded |
| 3 | `quantrule run list --status succeeded --limit 20` | **Runs** → table | One row per run, with status and config |
| 4 | `quantrule run status <run_id>` | **Runs** → open the run | Stages, provenance, artifacts, digest |
| 5 | `quantrule run fetch <run_id> ./backtest-bundle` | **Run detail** → Download | Every file checked against its size and SHA-256 |
| 6 | `quantrule verify <run_id>` | **Run detail** → Verify | The recorded canonical digest |
| 7 | `quantrule run cancel <run_id>` only while active | **Runs** → Cancel | `cancelled`, or `run.conflict` after completion |

A duplicate submission returns the existing run with `created: false`. Closing
the shell or the browser does not stop the run.

## Reused versus recomputed

| Submission | Model | Strategy | Execution | Portfolio |
|---|---|---|---|---|
| Full run | Computed | Computed | Computed | Computed |
| `--model-artifact` | Reused byte-for-byte | Recomputed | Recomputed | Recomputed |

The promoted run validates the artifact kind, profile, dataset, config hash, and
model version. Its model files and canonical digest must equal a full run with
the same inputs. A mismatch is `run.model_artifact_invalid`, not a warning.

## Run v5

A `quantrule.run/v5` config names its strategy, portfolio and execution packs by
content hash; a fold-mode run omits the signal artifact, and a signal-mode run
requires both the signal and model artifacts. A new run may cite only a frozen
pack; upload and `run submit` refuse a draft, validated, or retired pack.
`run submit` resolves every reference before it writes the run row, so a missing
or mismatched reference is refused with no run to cancel.

| Submission | Model | Strategy | Execution | Portfolio |
|---|---|---|---|---|
| Full v5, no model artifact | Computed | Pack | Pack | Pack |
| v5 `--model-artifact` | Reused byte-for-byte | Pack | Pack | Pack |

`quantrule run lineage <run_id>` prints the set recorded with the run row: kind,
id, hash and pack provenance, one reference per line. The set is immutable and
is the run's input identity, not evidence. The loader checks a signal artifact's
`strategy_pack_id` and `model_artifact_id`, and the worker resolves the set once
before the fold.

## Run identity and canonical digest

```text
run_id = sha256(data_version ‖ config_hash ‖ model_version ‖ image_digest)
```

A promoted run appends the tagged frozen model component:

```text
run_id = sha256(.. full components .. ‖ "model-artifact" ‖ model_artifact_id)
```

The full-run id is unchanged. A promoted run is a distinct run from the full run
even when both produce the same canonical digest.

| Value | Covers | Consequence |
|---|---|---|
| `run_id` | Data, config, model version, image | A different image or config is a different run |
| `config_hash` | The run configuration bundle | Two equivalent configs can have different ids |
| `canonical digest` | Sorted `(path, sha256)` for `results.json`, `*.csv`, `*.ndjson` | Excludes provenance, manifests, logs, and timestamps |
| Artifact hash | One file's bytes | A mismatch stops the download |

The canonical set is `results.json` when present, plus every `*.csv` and
`*.ndjson`: for example `strategy/decisions.ndjson`, `model/*.csv`,
`execution/*.csv`, and `portfolio/*.csv`. `provenance.json`, `manifest.json`,
`summary.txt`, and the JSON reports are not canonical because they record
identity or context.

## What you see

| Field | Meaning |
|---|---|
| `status` | `queued`, `running`, `succeeded`, `failed`, or `cancelled` |
| Stages | `data`, `model`, `backtest`, `evidence`, each with state and elapsed time |
| `model_artifact_id` | The frozen model input, or `none` |
| Provenance | Data version, config hash, model version, image digest |
| Artifacts | Relative name, bytes, SHA-256 |
| `digest` | The canonical digest, or `none` on a failed run |
| `error` | The stage or service failure on a failed run |

The browser marks a frozen model artifact as `diagnostic input`. That badge is a
warning, not a claim that the run is diagnostic: the coupled run is still
evidence.

## What it proves

**Evidence.** A succeeded coupled run with a canonical digest and verifiable
artifacts is the evidence unit. It proves the configured model, strategy,
execution, and portfolio ran over this data and image. It does not prove an
edge, accept a baseline, or show that another run reproduces it.

## Failure

| Code or symptom | Cause | Next action |
|---|---|---|
| `run.not_found` | The run id is unknown in this profile | Re-read `run list` |
| `run.invalid` | Malformed id, dataset, or config reference | Re-read both ids; do not construct them |
| `run.model_artifact_invalid` | Missing, non-succeeded, or mismatched frozen artifact | Re-run the model stage or use a matching artifact |
| `run.conflict` | Cancel after a terminal state | Read `run status`; do not retry the cancel |
| `config.invalid` | Config validation failed | Fix the config; do not edit the run |
| `dataset.not_found` | The dataset is unknown in this profile | Re-run `dataset list` |
| `queue.unavailable` | The service has no queue configured | Read `health`; a submission is accepted on its outbox row, so fix the configuration rather than resubmitting |
| `service.unavailable` | A downstream service is down | Read `health` and `make logs`; stop |
| `failed` at evidence | Artifact missing or hash mismatch | Record the stage and error; do not accept |
| `queued` forever | No worker, or the publisher has not delivered the outbox row | Check the publisher log, the worker, and the queue; stop |

## Related

- [Portfolio](05-portfolio.md)
- [Improvement](07-improvement.md)
- [Stage to backtest tutorial](../tutorials/2026-11-17/stage-to-backtest.md)
- [Backtest workflow](../workflows/backtest.md)
