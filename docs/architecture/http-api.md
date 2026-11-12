# HTTP API — `quantrule.api/v1`

- Status: Target v1.0.0; approved 2026-11-12
- Part of: [`README.md`](README.md)
- Claim type: Behaviour Claim

The gateway is the only surface a client sees. It never accepts a filesystem path,
never renders a label, and never returns a raw engine type.

## 1. Conventions

| Rule | Value |
|---|---|
| Base | `/api/v1` |
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
| GET | `/health` | — | service states, queue depth, worker count |
| GET | `/version` | — | product version, image digest, contract versions |
| GET | `/catalog/datasets` | — | list with `data_version`, candles, coverage |
| POST | `/catalog/datasets` | multipart CSV | `202` with `data_version` |
| GET | `/catalog/datasets/{version}` | — | one dataset record |
| GET | `/catalog/configs` | — | list with validity |
| POST | `/catalog/configs` | config JSON | `201` with `config_id` |
| GET | `/runs` | `status`, `limit` | run summaries |
| POST | `/runs` | `{data_version, config_id}` | `202` or `200` with `run_id` |
| GET | `/runs/{id}` | — | status, stages, provenance, artifacts |
| POST | `/runs/{id}/cancel` | — | `202` or `409` |
| GET | `/runs/{id}/artifacts/{name}` | — | bytes |
| GET | `/runs/{id}/digest` | — | canonical digest |
| GET | `/inspect/candles` | `data_version` | counts and coverage |
| GET | `/inspect/timeframes` | `data_version` | bars and events per timeframe |
| GET | `/inspect/events` | `data_version`, `timeframe` | event batch |

## 3. Submission

```json
{ "data_version": "sha256:…", "config_id": "cfg_…" }
```

The gateway validates the config, computes `run_id`, writes `queued`, publishes
`run.requested`, and returns:

```json
{ "run_id": "sha256:…", "status": "queued" }
```

`202` means new work. `200` means the identical run already exists.

## 4. Run detail

```json
{
  "run_id": "sha256:…",
  "status": "succeeded",
  "owner": "operator",
  "stages": [{ "name": "model", "state": "done", "elapsed_ms": 4210 }],
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

## 5. Error codes

| Code | HTTP | Meaning |
|---|---|---|
| `dataset.not_found` | 404 | Unknown `data_version` |
| `dataset.exists` | 409 | Same bytes already ingested |
| `config.invalid` | 400 | Schema or cross-config rule failed |
| `run.not_found` | 404 | Unknown run id |
| `run.conflict` | 409 | Cancel after completion |
| `upload.too_large` | 413 | Over the configured limit |
| `queue.unavailable` | 503 | Broker unreachable |
| `internal` | 500 | Unclassified; logged with a correlation id |

## 6. Prohibitions

- No endpoint accepts a path.
- No endpoint returns a human-readable label for a numeric code.
- No endpoint returns a value that depends on the wall clock inside a payload.
- No endpoint executes a single pipeline stage.
