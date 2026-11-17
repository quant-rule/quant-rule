# 6. Backtest

- Status: Current
- Audience: operator producing coupled evidence, with or without a frozen model artifact

## Goal

Submit a coupled run, distinguish run identity from the canonical digest, fetch
and verify its artifacts, and know what the result proves.

## Before you start

| Need | Value |
|---|---|
| Dataset | A `data_version` |
| Config | A valid run config that carries the strategy |
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

## Run identity and canonical digest

```text
run_id = sha256(data_version ‖ config_hash ‖ model_version ‖ image_digest)
```

| Value | Covers | Consequence |
|---|---|---|
| `run_id` | Data, config, model version, image | A different image or config is a different run |
| `config_hash` | The run configuration bundle | Two equivalent configs can have different ids |
| `canonical digest` | Sorted `(path, sha256)` for `results.json`, `*.csv`, `*.ndjson` | Excludes provenance, manifests, logs, and timestamps |
| Artifact hash | One file's bytes | A mismatch stops the download |

The canonical set includes result files such as `results.json`,
`strategy/decisions.ndjson`, `model/*.csv`, `execution/*.csv`, and
`portfolio/*.csv`. `provenance.json`, `manifest.json`, `summary.txt`, and the
JSON reports are not canonical because they record identity or context.

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
| `queue.unavailable` | The broker is unreachable | Read `health`; stop |
| `service.unavailable` | A downstream service is down | Read `health` and `make logs`; stop |
| `failed` at evidence | Artifact missing or hash mismatch | Record the stage and error; do not accept |
| `queued` forever | No worker | Check the worker and queue; stop |

## Related

- [Portfolio](05-portfolio.md)
- [Improvement](07-improvement.md)
- [Stage to backtest tutorial](../tutorials/2026-11-17/stage-to-backtest.md)
- [Backtest workflow](../workflows/backtest.md)
