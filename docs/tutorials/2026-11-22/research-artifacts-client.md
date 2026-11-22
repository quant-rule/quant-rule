# Research Artifacts from the Client

- Status: Current
- Audience: operator or agent working with packs, signals and run lineage
- Contract: [research artifacts](../../architecture/research-artifacts.md), [packs](../../config/2026-11-22/research-artifact-packs.md), [signal artifact](../../config/2026-11-22/signal-artifact.md)

## Goal

Freeze a strategy pack, compute one signal from a frozen model artifact, read
the metadata and the document, and inspect the references a run recorded. The
same use cases are on the browser Packs screen and run detail.

## Before you start

| Need | Value |
|---|---|
| Gateway | `QUANTRULE_API_URL` pointing at a running gateway with data and evidence |
| Strategy service | Required for `signal create`; v2 without it answers `503 service.unavailable` |
| Frozen model artifact | A succeeded model artifact id, for example from `stage show` |
| Dataset and config | The `data_version` and `config_id` the model was computed from |
| Pack file | A canonical `quantrule.pack/v1` document with a signal-mode strategy payload |

## Steps

1. Register the pack and read its content id.

```bash
quantrule pack create ./packs/strategy-pack.json
quantrule pack list --kind strategy --state draft
```

2. Validate and freeze it. Freeze is a human decision; record the Quant
   Validator verdict in the run record before you freeze.

```bash
quantrule pack show <pack_id>
quantrule pack validate <pack_id>
quantrule pack freeze <pack_id>
```

3. Compute one signal from the frozen model artifact and pack. The model
   artifact, data version, config and pack are the only inputs.

```bash
quantrule signal create --model-artifact <model_artifact_id> --data-version <data_version> --config <config_id> --strategy-pack <pack_id>
```

4. Read the signal metadata, then download the document. The download is
   verified against the content id before it is written.

```bash
quantrule signal list --limit 20
quantrule signal show <signal_id>
quantrule signal fetch <signal_id> ./signals/signal.json
```

5. Inspect a run's recorded lineage. A run submitted before lineage was
   recorded answers `run.lineage_not_found`; the browser shows "No recorded
   lineage" instead of a run error.

```bash
quantrule run lineage <run_id>
```

6. Download a pack for review, then retire it when no new run may use it.

```bash
quantrule pack fetch <pack_id> ./packs/strategy-pack.json
quantrule pack retire <pack_id>
```

## What you see

| Command | Result |
|---|---|
| `pack create` | The pack record, state `draft`, with a content id |
| `pack list` | One line per pack, sorted by content id |
| `pack show` | Identity, object key, hash, validations, events |
| `pack validate/freeze/retire` | The updated record with its new state |
| `pack fetch` | The verified hash and the written file |
| `signal create` | Schema, signal id, artifact metadata, and `created` |
| `signal list` | Signal ids, byte counts, creation times; no rows |
| `signal show` | The metadata record only |
| `signal fetch` | The verified document and the written file |
| `run lineage` | The recorded references with pack provenance |

## What it proves

A pack is a definition, a signal is computed alpha, and lineage is the reference
set a run cited. None of them is evidence of an edge. The signal bytes are
causal and content-addressed; the browser never parses a row or computes a
return. The human still owns the strategy, the freeze, and the research verdict.

## Failure

| Symptom | Cause | Next action |
|---|---|---|
| `pack.invalid` | The document is not a canonical `quantrule.pack/v1` or the payload is closed-schema invalid | Fix the pack; do not bypass the validator |
| `pack.conflict` | The label names different bytes, or the lifecycle transition is refused | Register a new pack for a change; follow draft → validated → frozen → retired |
| `signal.invalid` | The pack is not a frozen signal-mode strategy pack, or the model source is mismatched | Freeze a signal-mode pack and use the model id the manifest reports |
| `artifact.not_found` | The signal id is unknown in this profile | List signals and use the stored id |
| `service.unavailable` on create | This deployment has no strategy service | Read an existing signal; deploy the strategy service for a new computation |
| `run.lineage_not_found` | The run was submitted before lineage recording | The run is still readable; the lineage block is empty |

Do not edit a frozen pack, do not substitute a signal document, and do not treat
a pack or signal as a baseline.
