# HTTP API — `quantrule.api/v1`

- Status: Current; released as v1.3.0 on 2026-11-17; current in v1.4.2; the stage routes are part of this contract
- Part of: [`README.md`](README.md)
- Claim type: Behaviour Claim

The gateway is the only surface a client sees. The same routes also answer at
`/` for probes and a same-origin operator; clients use the base. It never accepts a filesystem path,
never renders a label, and never returns a raw engine type.

## 1. Conventions

| Rule | Value |
|---|---|
| Base | `/api/v1`; paths below are relative to it |
| Profile | `X-QuantRule-Profile` header on every API request; required except `/health`, `/ready`, `/version`, and `/profiles` |
| Content type | `application/json` except upload and artifact download |
| Version | `quantrule.api/v1` in `/version` and every error envelope |
| Identifier | Content hash or opaque catalog id; never a path |
| Time | Server-assigned; never accepted from the client |
| Errors | One envelope, stable codes |

```json
{ "v": "quantrule.api/v1", "error": { "code": "run.not_found", "message": "…" } }
```

## 2. Endpoints

| Method | Path | Request | Response |
|---|---|---|---|
| GET | `/health` | — | local liveness: this service's dependency states |
| GET | `/ready` | — | readiness: local and upstream service states; `503` when a required service is down |
| GET | `/profiles` | — | profiles with dataset, run, and simulation counts |
| POST | `/profiles` | `profile_id`, `name` | `201` new, `200` existing, with the summary |
| GET | `/version` | — | product version, image digest, contract versions |
| GET | `/operations` | profile and token | diagnostic stage retention, stage-store presence, last cleanup instant, removed counts, optional error; requires the profile header and the bearer gate |
| GET | `/hypotheses/{fingerprint}` | — | one sealed hypothesis in the profile |
| POST | `/hypotheses` | hypothesis JSON | `201` with the sealed record |
| GET | `/catalog/datasets` | — | list with `data_version`, candles, coverage |
| POST | `/catalog/datasets` | CSV bytes, `name` query | `201` new, `200` existing, with the record |
| POST | `/catalog/datasets/tolerant` | CSV bytes, `name` query | the record plus the quality report |
| GET | `/catalog/datasets/{version}` | — | one dataset record |
| GET | `/datasets/{version}/candles` | — | canonical CSV bytes, with an ETag; `If-None-Match` returns `304` |
| GET | `/datasets/{version}/ladder/{timeframe}` | — | canonical CSV for one timeframe, built once and reused; ETag and `304` |
| GET | `/catalog/configs` | — | run configs with validity and origin |
| POST | `/catalog/configs` | run-config bundle | `201` new, `200` existing, with the summary |
| POST | `/model/compute` | `{data_version, config_id, timeframes?}` | model object with registry version, timeframes, CAS key, hash, and bytes |
| GET | `/runs` | `status`, `limit` | run summaries |
| POST | `/runs` | `{data_version, config_id, model_artifact_id?}` | `202` new, `200` existing, with `run_id` |
| GET | `/runs/{id}` | — | status, stages, provenance, artifacts |
| POST | `/runs/{id}/cancel` | — | `202` or `409` |
| GET | `/runs/{id}/artifacts/{*name}` | — | bytes; a nested relative name such as `execution/fills.csv` |
| GET | `/runs/{id}/digest` | — | canonical digest |
| GET | `/inspect/candles` | `data_version` | counts and coverage |
| GET | `/inspect/timeframes` | `data_version` | bars and partial buckets per timeframe |
| GET | `/inspect/events` | `data_version`, `timeframe` | event batch |
| `GET/POST` | `/research/runs`, `/research/runs/{id}`, `/research/runs/{id}/submit`, `/research/runs/{id}/compare`, `/research/runs/{id}/decision`, `/research/baselines`, `/research/baselines/{tier}`, `/research/invalidations` | v3 pre-registration, coupled submit, compare, decision, registries | records; `accepted` requires a succeeded run and digest |

### 2.1 Diagnostic stage extension

The stage surface is the extension contract in [`stage-surface.md`](stage-surface.md);
it is diagnostic only and never evidence.

| Method | Path | Request | Response |
|---|---|---|---|
| POST | `/stages/{kind}/runs` | model, strategy, or portfolio input | `202` stage run |
| GET | `/stages/runs` | `kind`, `state`, `limit` | stage summaries |
| GET | `/stages/runs/{id}` | — | stage state, provenance, artifact |
| DELETE | `/stages/runs/{id}` | — | `204` after the artifact is removed |
| GET | `/stages/runs/{id}/artifacts` | — | artifact files |
| GET | `/stages/runs/{id}/artifacts/{*name}` | — | one verified artifact |
| POST | `/stages/runs/{id}/cancel` | — | `202` or `409` |

`/health` names its process in `service`; `/ready` aggregates the configured
upstream reports and returns `503` when one is unreachable or misidentified.

The gateway owns `/profiles` and proxies `/catalog/*`, `/datasets/*`,
`/inspect/*`, `/model/compute`, `/runs/*`, `/stages/*`, `/research/*`, and
`/operations` to their services with the injected profile. A proxied response
keeps its status, body, content type, and ETag; a closed upstream is
`503 service.unavailable`. `/inspect/events` goes to `qr-model`; the data
service never runs a model. `/operations` is operational metadata: retention
days and the last cleanup; an unrun cleanup is `null`.

The browser checks `/version` and `/health` and reports `connected`, `degraded`,
`mismatch`, or `unreachable`; a different API version is a mismatch, and a
failed call is `unreachable`, never a stale healthy state.

## 3. Submission

```json
{ "data_version": "sha256:…", "config_id": "cfg_…", "model_artifact_id": "sha256:…" }
```

`model_artifact_id` is optional; omit it for a full run. It names a succeeded
model stage in the active profile. The evidence service validates the config and
the stage provenance, computes `run_id`, writes `queued`, publishes
`run.requested`, and returns:

```json
{ "run_id": "sha256:…", "status": "queued", "created": true }
```

`202` means new work; `200` means the identical run already exists, and
`created` is `true` for `202` and `false` for `200`. A run is content-addressed
and shared: each profile that submits it gets a submission row, and the run is
not duplicated. A duplicate submission publishes the message again, so a retry
after a lost publish can recover; the worker treats it as a lookup.

A proxied timeout is `503 service.unavailable`; retry it, because uploads and
run submissions are content-addressed and return the existing record. Upload,
artifact download, and model compute get 15 minutes; other calls get 30 seconds.

## 4. Run detail

```json
{
  "run_id": "sha256:…",
  "status": "succeeded",
  "owner": "operator",
  "stages": [{ "name": "model", "state": "done", "elapsed_ms": 4210 }],
  "model_artifact_id": "sha256:…",
  "provenance": {
    "data_version": "sha256:…",
    "config_hash": "sha256:…",
    "model_version": "M1.3",
    "image_digest": "sha256:…"
  },
  "artifacts": [{ "name": "results.json", "bytes": 18233, "sha256": "…" }],
  "digest": "sha256:…"
}
```

`elapsed_ms` is control-plane metadata and never appears inside an artifact.

`provenance.model_version` is the run config's `model_id` (such as `M1.3`); the
bundle manifest records the registry version (`2026-11-10.2`), and
`provenance.json` records the package version (`1.4.2`).

## 5. Error codes

| Code | HTTP | Meaning |
|---|---|---|
| `auth.unauthorized` | 401 | Missing or wrong bearer token (when configured) |
| `profile.not_found` | 404 | Unknown or missing profile |
| `profile.invalid` | 400 | Bad profile name |
| `dataset.not_found` | 404 | Unknown `data_version` |
| `dataset.invalid` | 400 | Blank name, bad UTF-8, or a rejected candle |
| `config.invalid` | 400 | Schema or cross-config rule failed |
| `hypothesis.not_found` | 404 | Unknown hypothesis fingerprint |
| `hypothesis.invalid` | 400 | A hypothesis failed validation |
| `run.not_found` | 404 | Unknown run id |
| `run.invalid` | 400 | Malformed run id or submission |
| `run.model_artifact_invalid` | 400 | Unknown, non-succeeded, or mismatched frozen model artifact |
| `run.conflict` | 409 | Cancel after completion |
| `stage.not_found`, `stage.invalid`, `stage.conflict` | 404/400/409 | Unknown stage run, malformed stage request, or a disallowed stage transition |
| `research.not_found` | 404 | Unknown improvement id or tier baseline |
| `research.invalid` | 400 | Bad pre-registration, config, dataset, or note |
| `research.conflict` | 409 | Duplicate submit/decision, or accepted without a succeeded digest |
| `upload.too_large` | 413 | Over the configured limit |
| `queue.unavailable` | 503 | No queue is configured; a run submission still commits its outbox intent |
| `service.unavailable` | 503 | A downstream service is unreachable or refused the proxied call |
| `internal` | 500 | Unclassified; the message is returned without a stack trace |
| `simulation.not_found` | 404 | Unknown simulation run |
| `simulation.mark_invalid` | 400 | Bar outside the window, or a schema violation |
| `simulation.check_incomplete` | 409 | Fewer than 30 events, or an unmarked check window |
| `simulation.locked` | 409 | Marking or a second decision after the decision is recorded |

An upload is idempotent: the same bytes return the existing record and never
create a second row. A config row carries the whole bundle, so a run can
reproduce the configuration after the staging directory is gone.

## 6. Simulation endpoints — the research plane

`simulation` is reachable under the same contract and the same principal. It has
no run lifecycle and no queue.

| Method | Path | Request | Response |
|---|---|---|---|
| GET | `/simulation/runs` | `status`, `limit` | simulation run summaries |
| POST | `/simulation/runs` | premise, bundle ref, inputs, seed, criteria | `201` with `sim_id` |
| GET | `/simulation/runs/{id}` | — | premise, inputs digest, counts, status |
| POST | `/simulation/runs/{id}/windows` | — | deterministic window selection |
| GET | `/simulation/runs/{id}/windows/{n}/series` | — | candles, volume, EMA 20/50/200, one indicator panel, event markers, levels |
| POST | `/simulation/runs/{id}/marks` | `chart`, `bar`, `action`, `quality`, `side`, `note` | `201` with `mark_id` |
| GET | `/simulation/runs/{id}/marks` | `origin` | human and oracle marks |
| DELETE | `/simulation/runs/{id}/marks/{mark_id}` | — | `204` |
| POST | `/simulation/runs/{id}/oracle` | — | oracle marks, deterministic |
| POST | `/simulation/runs/{id}/check` | — | check table and verdict |
| POST | `/simulation/runs/{id}/decision` | `test`, `refine`, or `stop`, plus a note | `201` |

A run summary carries `sim_id`, `premise`, `status`, `windows`, `marks`,
`bundle`, and `marked_windows`. A create body carries `premise`, `bundle`,
`inputs` (optional; absent records `unspecified`), `seed`, and `criteria`. The
shared types live in `quantrule-api`; the service re-exports them.

Rules:

- `sim_id` is server-assigned; the client never supplies it.
- `marked_time_utc` is assigned by the server; the client clock is untrusted.
- `series` returns numbers and codes only. The client maps codes to words.
- A mark is validated against the window's bar range and the event availability
  bar; a duplicate identity is an upsert, and a mark after a decision is `simulation.locked`.
- `oracle` and `check` are synchronous and idempotent: the same inputs digest
  yields the same output. A decision is terminal: the first wins and a later one is `simulation.locked`.
- No simulation endpoint can write into a run artifact or read a strategy.

## 7. Profiles

| Rule | Value |
|---|---|
| Header | `X-QuantRule-Profile`, required on every request except `/health`, `/ready`, `/version`, and `/profiles` |
| Default profile | `default`, created by the first migration; a missing header is never defaulted |
| Scope | Datasets, hypotheses, run submissions, simulation, memory, and config rows; a config file bundle is shared by content id |
| Bytes | Shared and content-addressed; profiles never duplicate an object |
| Security | **Not a boundary.** One principal; no isolation promise |
| Unknown profile | `404 profile.not_found`; the request is never defaulted silently |

## 8. Prohibitions

- No endpoint accepts a path.
- No endpoint returns a human-readable label for a numeric code.
- No endpoint returns a value that depends on the wall clock inside a payload.
- No endpoint executes a single pipeline stage.

> **Stage-run exception.** Diagnostic stage runs are defined by
> [ADR-2026-11-16-01](../adr/2026-11-16/01-stage-execution-and-evidence-boundary.md).
> They are never evidence, and the coupled run remains the only canonical-digest
> producer. The implemented stage endpoints are in [stage-surface.md](stage-surface.md);
> `GET /stages/runs/{id}/artifacts/{*name}` downloads one verified diagnostic artifact.
