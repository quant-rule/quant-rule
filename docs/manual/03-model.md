# 3. Model

- Status: Current
- Audience: operator diagnosing the model and preparing a model artifact for a promoted run

## Goal

Run the model alone, read its event batch and coverage, verify its artifact set,
and understand when a model artifact may be reused.

## Before you start

| Need | Value |
|---|---|
| Dataset | A `data_version` with enough history for the model |
| Config | A valid run config whose `model_id` names the model |
| Profile | The profile that owns the stage |
| Stack | Healthy data, model, evidence, and backtest services |

A model stage is diagnostic. It is not a run, has no canonical digest, and is
never a baseline. Its only promotion path is `run submit --model-artifact`.

## Steps

| # | Shell | Browser | Expect |
|---:|---|---|---|
| 1 | `quantrule stage run model <data_version> <config_id>` | **Workbench → Model** → Data version, Config id → Run model stage | `stage_run_id`, `state: queued`, `created: true` |
| 2 | `quantrule stage list --kind model --status succeeded --limit 20` | Model tab → stage table | One row per model stage, newest first |
| 3 | `quantrule stage show <stage_run_id>` | Model tab → open the stage | State, owner, provenance, `evidence: false`, artifact id, files |
| 4 | `quantrule stage events <stage_run_id> --timeframe 5m` | Model tab → Read events | Event rows and one coverage line per report timeframe |
| 5 | `quantrule stage fetch <stage_run_id> ./model-stage` | Model tab → Download | Every file; size and SHA-256 checked before writing |

`stage show` prints the artifact id. That id is the `model_artifact_id` a
strategy stage or a promoted run names. The stage input is the dataset version;
the server derives the rest of the provenance.

## Artifact and event fields

| Field | Meaning |
|---|---|
| `stage_run_id` | Content identity of the diagnostic stage |
| `artifact_id` | Content identity of the artifact bundle |
| `state` | `queued`, `running`, `succeeded`, `failed`, or `cancelled` |
| `evidence` | Always `false` for a stage |
| Artifact files | `summary.txt`, `report.json`, `result.json`, `readings.csv`, `events.csv`, `levels.csv`, `legs.csv`, `provenance.json` |
| Events | Type, subtype, source, timeframe, direction, occurred bar, available bar, measured values |
| Coverage | Bars, expected bars, coverage share per timeframe |

A v2 model spec can add condition, prediction, panel, or session files. The stage
publishes the model bundle byte-for-byte; it does not relabel or recompute a
reading.

### Occurred versus available

`occurred_bar` is when the market event happened. `available_bar` is the first
bar on which the model could know it. A confirmed pivot or divergence appears
only when its confirmation is knowable. The shell and the browser print the
availability order; neither moves an event earlier.

## What you see

The Model tab shows the stage table, the detail provenance, the artifact file
list with sizes and hashes, the event table, and coverage. A succeeded stage with
no artifact is not complete. A failed stage shows its error and no artifact.

## What it proves

**Diagnostic.** The model stage measures the market and publishes readings and
events. It proves the model ran over this dataset and config with a verifiable
artifact. It does not prove a strategy, a fill, a P&L, an edge, or a baseline.

## Reuse in a promoted run

`run submit <data_version> <config_id> --model-artifact <model_artifact_id>`
loads a succeeded model stage from the same profile. The server checks the
artifact kind, dataset, config hash, and model version. A mismatch returns
`run.model_artifact_invalid`.

The promoted run copies the model bytes exactly and recomputes strategy,
execution, and portfolio. It is not a stage replay: it is a coupled evidence run
that reuses one frozen input.

## Failure

| Code or symptom | Cause | Next action |
|---|---|---|
| `stage.not_found` | The stage id is unknown in this profile | Re-read `stage list`; do not borrow another profile's id |
| `stage.invalid` | Bad input, config, or provenance | Read the message; fix the config or dataset reference |
| `stage.conflict` | The stage is terminal, or has no artifact yet | Read `stage show`; wait or submit a new stage |
| `dataset.not_found` | The named dataset is unknown | Re-run `dataset list` |
| `run.model_artifact_invalid` | The artifact is missing, not succeeded, or mismatched | Re-run the model stage or choose the matching artifact |
| `service.unavailable` | A required service is unreachable | Read `health`; stop |
| Artifact hash mismatch on fetch | Storage or transfer defect | Re-fetch; do not use the bytes |
| No `events.csv` in the artifact | The model bundle did not publish events | Read the artifact list; a missing published file is a defect |

## Related

- [Data](02-data.md)
- [Strategy](04-strategy.md)
- [Stage to backtest tutorial](../tutorials/2026-11-17/stage-to-backtest.md)
- [Engine Execution workflow](../workflows/engine-execution.md)
