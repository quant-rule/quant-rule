# Execution Plan — Full split of the six service binaries

- Status: Proposed
- Date: 2026-11-14 (UTC)
- Claim type: ADR (Architecture Decision)
- Owner: Human
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [Requirements & Design](../../workflows/requirements-design.md)
- Issue: none yet. The number is assigned on approval; this file is renamed to `<issue>-split-service-binaries.md`.
- Branch: `arch/dist-binaries-plan` (plan), `<type>/<issue>-split-service-binaries` (implementation)
- Records: ADR-2026-11-12-01, ADR-2026-11-12-04, ADR-2026-11-12-09, ADR-2026-11-12-12; #643, #646, #647, #648, #649, #651, #674, #679, #680; parked S1 gateway work in `quant-rule--646-gateway`
- Decision: **Option B — full split — selected by the human on 2026-11-14.** Option A (six binaries, gateway proxies data and model, worker fast path recorded) is rejected as too small. Option C (narrow the claim to three binaries) is rejected as an architecture retraction.

## 1. Intent

The accepted architecture says six services, six binaries, one image, and one
owner per service. The tree ships three binaries and one gateway that serves the
data and model routes, writes run records, and computes inside the backtest
worker. The six-binary inventory is necessary but not sufficient: it would leave
the data plane, the schema ownership, and the simulation surface false.

This plan makes the service architecture true. It extracts a shared runtime and
internal contract, moves the `quantrule-server` domain modules into five service
crates and a gateway crate, routes the backtest worker through data, model, and
evidence over HTTP, gives each service its own schema and migrations, and serves
the simulation surface. The six-binary inventory is milestone M1 inside the
plan, not the end state.

No model, strategy, execution, or portfolio result changes. The coupled
baseline's canonical digest is the regression oracle for the worker rewrite.

## 2. Decision record

### 2.1 Selected option

**Option B — full split.** Every service owns its domain code, routes, store, and
schema; the gateway proxies every client call and owns no domain computation; the
backtest worker orchestrates data, model, and evidence over HTTP; the simulation
service serves its eleven endpoints. The six binaries exist by milestone M1 and
remain the deployment unit.

### 2.2 Rejected options

| Option | What it was | Why it is rejected |
|---|---|---|
| A. Structural split | Six binaries; gateway proxies only data and model; `qr-simulation` is a health skeleton; the worker keeps `AppService::run_backtest` in process; shared schema reads remain | It leaves `data-plane.md` and `services.md` false. The worker fast path means scaling `model` does not speed a run and a model outage does not fail a run. The gateway still reads catalog and run data. It unblocks #643 but not #646 or #674 honestly |
| C. Narrow the claim | Keep three binaries; amend ADR-04, `services.md`, #643, and #674 to three services | It retracts accepted architecture decisions, removes the simulation service, and makes the gateway a monolith with three process labels. It requires explicit human approval and is not a planning convenience |

### 2.3 What B changes in one paragraph

`quantrule-server` is retired as a mixed service crate. It becomes a shared
`quantrule-runtime`, a shared `quantrule-service-api`, five service crates
(`quantrule-data-service`, `quantrule-model-service`,
`quantrule-backtest-service`, `quantrule-evidence-service`,
`quantrule-simulation-service`), and one `quantrule-gateway` crate. The gateway
links no engine and no `quantrule-app`. Each service owns one schema, one
migration set, and one binary. The client contract stays `quantrule.api/v1`; the
service-to-service contract is a new `quantrule.internal/v1`.

## 3. Current state

| Area | Observed 2026-11-14; plan base `b63b9015`; #772 merged at `f3e258f0` |
|---|---|
| Binaries | `qr-gateway`, `qr-backtest`, `qr-evidence` (3) |
| Missing binaries | `qr-data`, `qr-model`, `qr-simulation` |
| Data and model routes | Served by the gateway in `runtime/server.rs:151-168` |
| Run records | Written by the gateway on submission and cancel; claimed and completed by the worker; verified by evidence |
| Worker fast path | `backtest/worker.rs:189-196` calls `AppService::run_backtest` in process |
| Simulation | No binary and no route; `quantrule-simulation` is not a `quantrule-server` dependency; #679 and #680 are open |
| Schemas | `catalog`, `runs`, `research`, and `simulation` in one database; profiles in `catalog.profiles`; one `public.schema_migrations` ledger |
| Documents | ADR-04 and `services.md` say six binaries; `module-map.md:19` says "(5 binaries)"; `architecture/README.md:72` says "five deployable responsibilities"; `deployment.md:115` says "all eight" containers while §2 lists nine |
| Image stack | #772 merged at `f3e258f0`: one image and a Compose stack with six default containers; `data`, `model`, and `simulation` sit behind the off-by-default `six-binaries` profile; #643 stays open |
| Migration lock | #772 found the runtime acquires a session-level advisory lock and releases it through the pool; three migrating services deadlocked. Compose makes the gateway the sole migrator as a workaround, not a fix |
| Object store | MinIO withdrew its public images on 2026-09-11. #772 pins `bitnamilegacy/minio@sha256:451fe6858cb770cc9d0e77ba811ce287420f781c7c1b806a386f6896471a349c` as an amd64 stopgap, overridable with `QR_OBJECT_STORE_IMAGE`; a permanent registry is a human decision |
| Hidden closure | #647 and #648 closed as gateway routes, not binaries; #751 says the model route is served by the gateway "until the six binaries are split"; #649's plan excluded the split |
| Parked S1 | `quant-rule--646-gateway` has uncommitted gateway work at `b63b9015`: bearer seam, route modules, artifact/digest/events routes, profile run count, migration 13 registration, and route-table tests. It is not landed and is not evidence |

### 3.1 `quantrule-server` inventory

| Module | Lines | Current responsibility | Target owner |
|---|---:|---|---|
| `runtime/` | 4,103 | Config, health, adapters, migrations, shutdown, static, router | `quantrule-runtime`; gateway-only parts to `quantrule-gateway` |
| `data/` | 1,186 | Ingest, catalog, inspect, ladder | `quantrule-data-service` |
| `configs/` | 676 | Config catalog, scan, upload | `quantrule-data-service` |
| `model/` | 574 | Model compute, shard, route | `quantrule-model-service` |
| `backtest/` | 689 | Worker, retry, workspace | `quantrule-backtest-service` |
| `evidence/` | 473 | Manifest, artifact verification, finalize | `quantrule-evidence-service` |
| `runs/` | 971 | Run store, identity, stages | `quantrule-evidence-service`; shared record types to `quantrule-service-api` |
| `research/` | 163 | Hypothesis store | `quantrule-evidence-service` |
| `profile/` | 257 | Profile store and routes | `quantrule-gateway`; schema `gateway` |
| `bin/` | 316 | Three binaries | Service crates |
| `tests/` | 3,869 | Server tests | Owning service crates |
| `migrations/` | 13 files | One shared ledger | Per-service migration sets and ledgers |

## 4. Scope

### 4.1 Included

- Extract `quantrule-runtime` and `quantrule-service-api`.
- Create the five service crates and the gateway crate; retire `quantrule-server`.
- Move every `quantrule-server` module to its owning service.
- Add `qr-data`, `qr-model`, and `qr-simulation`; keep all six binary names.
- Move the data and model routes out of the gateway; proxy every client call.
- Move run records, research memory, and hypotheses to evidence; evidence owns
  the `runs` and `research` schemas.
- Move profiles to the gateway schema; the gateway injects the profile header.
- Give data the `catalog` schema and simulation the `simulation` schema; give
  each service its own migration ledger.
- Add the model bundle protocol and the worker's HTTP path through data, model,
  and evidence.
- Add the `#649` successor for the worker orchestration and the staged
  strategy, execution, and portfolio use case.
- Serve the simulation surface (#679 parity plus #680 service).
- Add internal contracts, a service client, health aggregation, cancellation over
  HTTP, retry mapping, and streamed uploads.
- Tests for each phase, the coupled baseline digest, service boundaries, and
  schema ownership.
- Documentation, changelog, AI log, and issue comments.

### 4.2 Excluded

- `Dockerfile`, compose, image digest, and `make up` (#643, #644, #673).
- The v1.0.0 release itself (#674).
- The shell's HTTP rewrite (#668) and client parity tests (#669, #670); the
  server side is ready at P8, the shell migration is a parallel issue.
- Simulation screens (#681-#683); the service contract is P7.
- The embedded desktop profile, SQLite, filesystem CAS, and in-process queue
  (ADR-12, #692-#695).
- A crate split of `quantrule-app`. Services remain thin adapters over the shared
  application service. The gateway is the only binary that must not link it.
- Any model, strategy, execution, or portfolio definition or output change.
- Multi-tenancy, accounts, cloud deployment, Kubernetes, and high availability.
- An upgrade path for an existing pre-split database. v1.0.0 uses a fresh
  database; D14 already records no backwards compatibility.

### 4.3 Deferred

| Deferred | Why | Owner |
|---|---|---|
| Embedded desktop profile (SQLite, filesystem CAS, in-process queue) | v1.1 profile, not the distributed server | ADR-12, #692-#695 |
| Shell HTTP client | Parallel client migration; the API is ready at P8 | #668 |
| Simulation chart screens | Depend on the P7 service contract | #681-#683 |
| Streaming artifact downloads | v1.0.0 buffers the object; uploads stream. Recorded limitation | Follow-up |
| `quantrule-app` crate split | The gateway isolation is compile-time; the compute services share the app crate by design | Follow-up if required |
| Multi-host scheduling, HA, autoscaling | Later deployment decisions | Release & Operations |

## 5. Target architecture

### 5.1 Crate and service boundaries

```text
quantrule-api ──────────────┐
quantrule-service-api ──────┤
quantrule-runtime ──────────┼──> quantrule-gateway        (qr-gateway)
                            ├──> quantrule-data-service   (qr-data)
                            ├──> quantrule-model-service  (qr-model)
                            ├──> quantrule-backtest-service (qr-backtest)
                            ├──> quantrule-evidence-service (qr-evidence)
                            └──> quantrule-simulation-service (qr-simulation)

quantrule-app ──> quantrule-backtest ──> quantrule-model ──> quantrule-data
     ^
     └── data, model, and backtest services only; never the gateway
```

| Crate | Binary | Owns | Depends on | Must never |
|---|---|---|---|---|
| `quantrule-api` | — | Public `quantrule.api/v1` types, error envelope, health, run detail | No engine | Depend on an engine or a service crate |
| `quantrule-service-api` | — | `quantrule.internal/v1` types, run lifecycle, config bundle, dataset object, model bundle manifest, canonical digest | No engine, no runtime | Depend on a service crate or render a label |
| `quantrule-runtime` | — | Config, health registry, adapters, migration runner, HTTP serve helper, service client | `quantrule-api`, `quantrule-service-api`, DB/object/queue clients | Depend on an engine or `quantrule-app` |
| `quantrule-gateway` | `qr-gateway` | Profiles, static client, principal injection, proxy table, health aggregation | `quantrule-api`, `quantrule-service-api`, `quantrule-runtime` | Link `quantrule-app` or an engine; compute a model; write a domain schema |
| `quantrule-data-service` | `qr-data` | `catalog` schema, datasets, configs, ingest, inspect, ladder, streamed upload | api, service-api, runtime, app, `quantrule-data` | Read `runs`, `research`, or `simulation`; run a model |
| `quantrule-model-service` | `qr-model` | Model compute and model bundle objects | api, service-api, runtime, app, `quantrule-model` | Read strategy, execution, portfolio, or a run schema |
| `quantrule-backtest-service` | `qr-backtest` | Worker orchestration, strategy, execution, portfolio, artifact objects | api, service-api, runtime, app, `quantrule-backtest` | Read `catalog`, `runs`, or `research` schemas directly |
| `quantrule-evidence-service` | `qr-evidence` | `runs` and `research` schemas, run records, provenance, digest, finalization, memory, hypotheses | api, service-api, runtime | Compute a reading or a trade; depend on `quantrule-app` |
| `quantrule-simulation-service` | `qr-simulation` | `simulation` schema, research objects, eleven endpoints | api, service-api, runtime, `quantrule-simulation` | Compute a model; write a run artifact; publish a queue message |
| `quantrule-server` | — | Retired in P8 | — | Remain in the workspace after P8 |

`quantrule-app` remains the shared application service. It is not a service
binary. Data, model, and backtest call it; evidence and simulation do not. The
gateway is the only binary with a compile-time guard that forbids the app and
engine dependency.

### 5.2 What moves from `quantrule-server`

| Source | Destination | Notes |
|---|---|---|
| `runtime/config.rs`, `health.rs`, `error.rs`, `logging.rs`, `shutdown.rs`, `migrations.rs`, `adapters/` | `quantrule-runtime` | Shared; no engine dependency |
| `runtime/server.rs` | Split: gateway state and health to `quantrule-gateway`; data and model route builders to their services; proxy handlers to the gateway | One route owner per path |
| `runtime/static_files.rs` | `quantrule-gateway` | Static SPA and history fallback |
| `data/`, `configs/` | `quantrule-data-service` | Catalog schema, data/config routes, ingest, ladder |
| `model/` | `quantrule-model-service` | Model compute, shard, bundle writer |
| `backtest/` | `quantrule-backtest-service` | Worker, retry, workspace |
| `evidence/`, `runs/`, `research/` | `quantrule-evidence-service` | Run records, research memory, hypotheses, finalization |
| `profile/` | `quantrule-gateway` | Profile store and routes; `gateway` schema |
| `bin/qr_*` | Service crates | One binary per crate |
| `migrations/` | Per-service `migrations/` | Data, evidence, simulation, gateway; fresh DB |
| `tests/` | Owning service crates | Gateway, data, model, evidence, backtest, boundary |
| `Cargo.toml` | Retired in P8 | Workspace members updated |

### 5.3 Gateway proxy surface

| Public path | Owner | Notes |
|---|---|---|
| `/api/v1/health` | gateway | Aggregates data, model, evidence, simulation, and backtest health |
| `/api/v1/health/ready` | gateway | New: 503 when a required service is down; `/health` stays 200 with a state |
| `/api/v1/version` | gateway | Client contract version and image digest |
| `/api/v1/profiles`, `/api/v1/profiles/*` | gateway | Local profile store, `gateway` schema |
| `/api/v1/hypotheses`, `/api/v1/hypotheses/*` | evidence | Research memory |
| `/api/v1/catalog/datasets`, `/api/v1/catalog/datasets/*` | data | Ingest, list, read, tolerant upload |
| `/api/v1/catalog/configs`, `/api/v1/catalog/configs/*` | data | Config catalog and upload |
| `/api/v1/datasets/*` | data | Candles and ladder CSV |
| `/api/v1/inspect/*` | data | Candle and timeframe inspection |
| `/api/v1/model/*` | model | Public model compute and report |
| `/api/v1/runs`, `/api/v1/runs/*` | evidence | Submit, list, detail, cancel, artifact, digest |
| `/api/v1/simulation/*` | simulation | Eleven research endpoints |
| Non-API path | gateway | SPA assets and history fallback |

Rules:

- `/internal/v1/*` is never proxied and returns 404 at the gateway.
- The gateway validates the profile and injects `X-QuantRule-Profile`; a client
  cannot set an upstream host, path, or internal header.
- The gateway forwards an allowlist of headers, streams request bodies, and
  passes status, content type, ETag, and the error envelope through unchanged.
- An unreachable service returns `503 service.unavailable` with the service name.
- The proxy table is data, not code: one row per prefix, one owner per row.

### 5.4 Internal contracts and transport

`quantrule-service-api` is the internal contract crate. It holds:

| Type | Purpose |
|---|---|
| `ConfigBundle` | The six config files, `config_hash`, validity, and message |
| `DatasetObject` | `data_version`, object key, SHA-256, candle count, coverage |
| `ModelBundleManifest` | Data version, config id/hash, model version, ladder, manifest key, artifact refs |
| `RunRecord` | Full run row including attempts, owner, provenance, artifacts, digest, error |
| `RunClaimRequest`, `RunClaimOutcome` | Claim, reclaim, terminal, missing, exhausted |
| `RunComputed`, `RunFailed` | Worker completion and failure payloads |
| `RunCancelOutcome` | Cancelled, conflict, missing |
| `RunManifest`, `canonical_digest` | Shared artifact manifest and digest over sorted paths |

Transport rules:

- HTTP/1.1, JSON, content-addressed references; no candle, reading, or artifact
  payload in an internal response body beyond a small metadata envelope.
- `quantrule-runtime::client` provides one concrete `ServiceClient` and a fake.
  It uses `hyper-util` and `http-body-util`, both already in `Cargo.lock`; no new
  crate version is added.
- Timeouts: 2 s connect, 30 s request, 60 s idle for a streamed upload.
- Retries: GET three attempts with 100/500 ms backoff; idempotent POST three
  attempts; 4xx is not retried except 408 and 429; the worker's run retry budget
  stays three attempts with 5 s and 25 s backoff.
- Errors: domain errors keep their stable `ErrorCode`; an unreachable service is
  `503 service.unavailable`; an upstream body is passed through unchanged.
- Service URLs: `QR_DATA_URL`, `QR_MODEL_URL`, `QR_EVIDENCE_URL`,
  `QR_SIMULATION_URL`, `QR_BACKTEST_HEALTH_URL`. A missing URL fails at startup
  with the variable name.
- Every binary serves `/health` and `/version` on its own bind port. The two
  workers run a small health server beside the queue consumer. Only the gateway
  is published.

### 5.5 Schema ownership and migrations

| Schema | Service | Tables | Migration ledger |
|---|---|---|---|
| `catalog` | data | `datasets`, `configs`, `config_bundles` | `catalog.schema_migrations` |
| `runs` | evidence | `runs`, `stages`, `submissions` | `runs.schema_migrations` |
| `research` | evidence | `hypotheses` | `runs.schema_migrations` |
| `simulation` | simulation | `runs`, `marks` | `simulation.schema_migrations` |
| `gateway` | gateway | `profiles` | `gateway.schema_migrations` |
| — | model | No database schema; object store only | — |
| — | backtest | No database schema; object store and queue | — |

Rules:

- One writer per schema. No service reads another service's schema.
- `profile_id` stays a text column on catalog, run, and simulation rows. It is
  not a foreign key; profiles are organisational, not a security boundary.
- The gateway is the only profile authority. It validates the header and injects
  it on every proxied call. Downstream services trust the injected value.
- Each service applies only its own migration list. A global advisory lock
  serialises DDL; the ledger is per schema.
- The migration runner pins one connection. The current runtime acquires a
  session-level advisory lock on a pooled connection and releases it through the
  pool, so the unlock can land on a different connection and the lock leaks.
  #772 observed three migrating services deadlock; Compose works around it by
  making the gateway the sole migrator (`QR_MIGRATE=1`, workers `0`). That is a
  workaround, not a fix. The B runner takes one `PoolConnection`, acquires the
  lock, creates the ledger, applies the migrations, and releases the lock on
  that same connection; if the connection dies, Postgres releases the session
  lock. A composed test starts two service migrators against one database and
  proves neither waits forever.
- v1.0.0 uses a fresh database. The pre-split `public.schema_migrations` and
  `catalog.profiles` are not migrated. `make reset` is the supported path.
- Simulation owns its schema but may read a model bundle from CAS; it never
  writes a run artifact.

### 5.6 Worker HTTP path, cancellation, retries, and streaming

The backtest worker's sequence:

1. Consume `run.requested`.
2. `evidence.claim(run_id, redelivered)`; a terminal, missing, running-computed,
   or exhausted run is skipped or terminal as today.
3. `evidence.get_run(run_id)`; a cancelled row stops the run.
4. `data.get_config(config_id, profile)`; a missing or invalid config is terminal.
5. `data.get_dataset(data_version, profile)`; a missing dataset is terminal.
6. `model.compute_bundle(data_version, config_id, profile)`; the response is a
   `ModelBundleManifest` with content-addressed refs.
7. Materialise the config, dataset, and model bundle into a private workspace.
8. `AppService::run_backtest_from_model` runs strategy, execution, and portfolio
   from the model bundle and candles; the model is not recomputed.
9. Check cancellation again before the first CAS write.
10. Write artifacts and the manifest; call `evidence.computed`; publish
    `run.computed`. On failure, call `evidence.failed` and publish `run.failed` or
    `run.dead` per the existing retry budget.

| Failure | Behaviour |
|---|---|
| Data or model 404/422 | Terminal `failed`; no artifact |
| Data or model 5xx or timeout | Retryable; hold the delivery and retry |
| Evidence unavailable during claim | Retryable; the message stays unacked |
| Evidence says cancelled | Stop; no artifact; do not publish computed |
| CAS write or manifest failure | Retryable; no visible artifact set |
| Canonical digest differs from the baseline | Terminal defect; stop the phase and return for review |

Cancellation: the run row is authoritative. The gateway proxies cancel to
evidence; evidence marks cancelled and publishes `run.cancelled` best effort. The
worker checks the row after claim, before the model call, before the backtest,
and before CAS. A cancel after finalization is a conflict, as today.

Streaming uploads: the gateway forwards `POST /catalog/datasets` as a stream. The
data service enforces the 512 MiB cap, writes to a bounded temp file while
hashing, ingests from the file, and streams the object to the object store. The
4 MiB config upload may buffer. Artifact and candle downloads are buffered in
v1.0.0 and recorded as a limitation.

### 5.7 The #649 successor

#649 stays Done. A new issue is opened as its successor, with this scope:

- `quantrule-app` gains `run_backtest_from_model`: config, strategy, model
  bundle, candles, and output; it composes the existing strategy, execution, and
  portfolio stages without recomputing the model.
- `quantrule-backtest-service` owns the HTTP orchestration and the failure map.
- The successor's regression oracle is the coupled `backtest run` fixture: the
  same config, strategy, and data produce the same artifact names, bytes, and
  canonical digest.
- The successor records that #649's exclusion "Splitting the six binaries from
  the gateway image" is superseded by this plan.

### 5.8 Parked S1 gateway work

`/home/zantgo/Proyectos/quant-rule--646-gateway` is a parked worktree on
`feat/646-gateway` at `b63b9015`. It carries uncommitted work: 16 modified files
and 5 untracked files, roughly 568 insertions and 618 deletions. It is **not
landed**, has no commit, and is not evidence for any acceptance criterion. B
treats it as a reuse candidate only.

| Parked path | What it contains | B destination | How B uses it |
|---|---|---|---|
| `src/runtime/auth.rs` | Bearer-token middleware, constant-time compare, open paths, 401 `auth.unauthorized` | `quantrule-gateway`, P8 | Reuse the logic after the gateway crate exists; keep the security tests |
| `src/runtime/routes/data.rs` | Data and config handlers, event inspection, ladder, CSV response | `quantrule-data-service`, P3 | Move the handlers, adapt `AppState` to the data service state, keep the route tests |
| `src/runtime/routes/runs.rs` | Run submission, list, detail, cancel, artifact download, digest | `quantrule-evidence-service`, P5 | Move the handlers and store calls; evidence owns the run row and artifacts |
| `src/runtime/server.rs` | Slim route table, bearer layer, route modules | `quantrule-gateway`, P3-P8 | Reuse the route-table shape; replace direct handlers with the proxy table phase by phase |
| `tests/surface.rs` | Every documented endpoint answers; record routes require a profile; token tests; event route | `quantrule-gateway` and each service, P3-P8 | Adopt as the contract test, split by owner, and run it against the proxy fakes |
| `crates/quantrule-api/src/error.rs` | `auth.unauthorized` 401 | `quantrule-api`, P1 | Reuse the additive code and its `ALL` test |
| `crates/quantrule-api/src/model.rs` | `EventRecord`, `EventBatch` | `quantrule-api`, P3 | Reuse the public event batch contract with the data service |
| `crates/quantrule-api/src/run.rs` | `RunDigest` | `quantrule-api`, P5 | Reuse the public digest response with evidence |
| `runs/store.rs`, `postgres/runs.rs` | `RunStore::count`, Postgres count | `quantrule-evidence-service`, P5 | Reuse the profile run count in the evidence store |
| `runtime/migrations.rs` | Migration 13 registration | `quantrule-evidence-service`, P5 | Reuse the registration; verify the per-service ledger |
| `tests/boundary.rs` | Gateway names no engine; server carries no simulation domain | `quantrule-gateway`, P8 | Replace with the compile-time guard and per-service schema guards |
| `data/mod.rs` | `Timeframe` re-export to keep the gateway boundary clean | `quantrule-data-service`, P3 | Replace with a direct data-service import; the gateway no longer needs it |

Rules:

- Inspect the parked diff before P3 and record a salvage map in the issue. Do
  not merge the parked branch, and do not copy a file wholesale into a new crate
  without adapting imports and state.
- Reuse by hunk or reimplement the same behavior. A parked hunk is not accepted
  until its test passes in the owning service crate.
- If the parked work lands before B reaches its phase, rebase onto it and remove
  the duplicate work. If it does not land, B implements the behavior and the
  parked branch is discarded; the issue records which path was taken.
- The parked route handlers still depend on the mixed `AppState`. B's services
  have narrower state; reuse the handler logic and tests, not the wiring.
- The parked `runtime/server.rs` still serves data and run routes directly. B
  replaces those with the proxy; the route-table test is the reusable part.
- Capture the parked diff as a patch or issue attachment before the worktree is
  rebound or removed. The plan does not rely on the worktree surviving.

## 6. Assumptions and uncertainty

- The human selected B on 2026-11-14. This plan is the decision record; a new
  ADR for the crate boundaries is added in P0.
- `quantrule-app` remains shared. The gateway is compile-time isolated from it;
  the compute services are not. A full app-crate split is a follow-up if the
  human requires compile-time isolation for every service.
- The model bundle protocol can reproduce the coupled artifact bytes. The golden
  digest is the gate; if it cannot, P6 stops.
- The internal contract can carry run lifecycle and model bundle references
  without a payload. No service returns a reading series.
- A fresh database is acceptable for v1.0.0. If an existing database must be
  upgraded, the plan stops for a data migration decision.
- #679 simulation parity is a prerequisite for P7. P1-P6 can proceed if it is
  not ready; the simulation service stays a skeleton and #646 stays open.
- Only the gateway is published. The internal network is one host and trusted;
  downstream services still reject a missing profile header.
- The image/compose work (#643) is downstream. P2 unblocks its binary
  prerequisite; it still owns the Dockerfile, compose, and healthchecks.
- No new crate version is required. If a required feature or crate is absent,
  the plan stops for a dependency decision.
- Active streams #643, #646, #668, and #680 are coordinated by the phase
  ownership table. No phase edits a file owned by another active stream.
- The parked S1 gateway work is uncommitted and can change or disappear. B treats
  it as a reuse candidate, not as landed behavior or evidence. If it disappears,
  B reimplements the same behavior from the public contract and the plan is
  unchanged.

## 7. Phases and file ownership

Phases are sequential unless stated. Each phase is a reviewable PR or a small
series. A phase does not edit a file owned by another active phase.

| Phase | PR scope | Owned paths | Parked S1 input | Depends on | Exit criteria | Unblocks |
|---|---|---|---|---|---|---|
| **P0** | Record B; create the issue from §14; rename the plan; write the crate-boundary ADR; freeze the internal contract; capture the parked S1 salvage map | `docs/execution-plans/2026-11-14/`, `docs/adr/2026-11-14/`, GitHub | Inventory only | Human approval | Plan `Approved`; issue `In Progress`; ADR exists; salvage map recorded | Every phase |
| **P1** | Extract `quantrule-runtime` and `quantrule-service-api`; move config, health, adapters, migrations runner, service client; pin the migration lock to one connection and add the two-migrator deadlock test; no route change | `crates/quantrule-runtime/**`, `crates/quantrule-service-api/**`, workspace `Cargo.toml` | `auth.unauthorized`, `RunDigest`, `EventBatch` contract types | P0 | Workspace tests; runtime has no engine dependency; two migrators do not deadlock; `quantrule-server` still builds | P2 |
| **P2** | Six-binary milestone M1: add `qr-data`, `qr-model`, `qr-simulation` skeletons with `/health` and `/version`; keep the existing routes in the gateway | `crates/quantrule-data-service/**`, `crates/quantrule-model-service/**`, `crates/quantrule-simulation-service/**` (manifests and health only) | None | P1 | Six binaries build; inventory guard; every new binary answers health | **#643 binary prerequisite** |
| **P3** | Data service: move `data/` and `configs/`; `catalog` schema and migrations; profile move to `gateway`; data routes; gateway data proxy; streamed upload | `crates/quantrule-data-service/**`, `crates/quantrule-server/src/data/**`, `src/configs/**`, `src/profile/**`, `src/runtime/server.rs` data routes, `src/runtime/proxy.rs`, `src/runtime/upstream.rs` | `routes/data.rs`, event inspection, `Timeframe` seam, data route tests | P2 | Data routes served by `qr-data`; gateway has no data handlers; catalog schema only data; streaming test | Part of #646; P4 |
| **P4** | Model service: move `model/`; model bundle writer and internal manifest; data client; gateway model proxy; public `/model/compute` unchanged | `crates/quantrule-model-service/**`, `crates/quantrule-server/src/model/**`, `quantrule-app` model bundle call sites, gateway model proxy | None | P3 | Model route served by `qr-model`; bundle SHA-256 matches the coupled baseline; no strategy read | Part of #646; P5 |
| **P5** | Evidence service: move `runs/`, `research/`, `evidence/`; `runs` and `research` schemas; internal run lifecycle; evidence publishes `run.requested`; gateway run and hypothesis proxies | `crates/quantrule-evidence-service/**`, `crates/quantrule-server/src/runs/**`, `src/research/**`, `src/evidence/**`, gateway run and hypothesis proxies | `routes/runs.rs`, artifact/digest routes, `RunStore::count`, migration 13 | P3 | Submit, list, detail, cancel through evidence; no gateway run write; queue producer is evidence | Part of #646; P6 |
| **P6** | Backtest service: move `backtest/`; `run_backtest_from_model`; worker calls data, model, evidence; cancellation and retry map; #649 successor | `crates/quantrule-backtest-service/**`, `crates/quantrule-server/src/backtest/**`, `quantrule-app/src/stages/backtest.rs`, new `backtest_from_model.rs`, `service.rs` | None | P4, P5 | Full run over HTTP; same canonical digest; fast path removed; successor issue merged | Data plane true; #649 successor |
| **P7** | Simulation service: #679 parity; #680 service; eleven endpoints; gateway proxy; no run write, no model compute | `crates/quantrule-simulation-service/**`, `crates/quantrule-simulation/**`, gateway simulation proxy | None | P2, #679 | Eleven endpoints; parity evidence; isolation guard; #680 done | #646 can close |
| **P8** | Gateway finalization: remove the last domain modules; rename `quantrule-server` to `quantrule-gateway`; profiles, static, proxy, health; compile guard; retire `quantrule-server` | `crates/quantrule-gateway/**`, workspace `Cargo.toml`, `docs/architecture/**`, `AGENTS.md`, `README.md` glossary | `runtime/auth.rs`, `tests/surface.rs`, `tests/boundary.rs` | P3-P7 | Gateway links no app or engine; six service crates independent; #651 guard | #651; #646 final |
| **P9** | Downstream gates, not owned by this plan: image/compose (#643), Makefile (#644), digest (#673), release (#674); record the permanent object-store registry as a human decision | `Dockerfile`, compose, `Makefile`, release docs | None | P6, P7, P8 | Six services healthy in nine containers; digest recorded; release evidence; registry decision recorded | #674 |

### 7.1 Phase detail

**P0 — Decision and salvage map.** Record B, create the issue, rename the plan,
and write the crate-boundary ADR. Before any code phase, read the parked
`quant-rule--646-gateway` diff and write the salvage map from §5.8 into the
issue. The map names every parked file or hunk, its destination phase, and
whether B will reuse it by hunk or reimplement the behavior. Capture the parked
diff as a patch or issue attachment; the plan does not rely on the worktree
surviving.

**P1 — Shared foundation.** Move the runtime out of the service crate before any
domain moves. `quantrule-runtime` owns `Config`, `HealthRegistry`, the adapter
traits and their Postgres, S3, and RabbitMQ implementations, the migration
runner, logging, shutdown, and the HTTP serve helper. `quantrule-service-api`
owns the internal contract types. `quantrule-server` keeps its three binaries and
all routes; only imports change. The migration runner is the one deliberate fix:
it takes a single `PoolConnection`, acquires the advisory lock, applies the
ledger and migrations, and releases on the same connection. A composed test
starts two service migrators against one database and fails on a timeout. This
removes the Compose sole-migrator workaround from the B target; the workaround
stays in #772 until P1 lands.

**P2 — Milestone M1.** Add three skeleton crates with `qr-data`, `qr-model`, and
`qr-simulation` binaries. Each answers `/health` and `/version`; no domain route
is claimed. The existing `qr-gateway`, `qr-backtest`, and `qr-evidence` stay in
`quantrule-server`. The image can now package six binaries and #643 can proceed
with its own Dockerfile and compose work. M1 is explicitly not the end state:
the gateway still serves data and model, and the worker still uses the fast path.

**P3 — Data service.** Move the data and config modules, the `catalog` schema,
and the catalog migrations. Add the internal config-bundle and dataset-object
endpoints. Move profiles to the gateway schema. Add the service client and the
gateway proxy for data. The gateway stops reading candles and stops serving
`/catalog/*`, `/datasets/*`, and `/inspect/*`. The parked `routes/data.rs`,
event inspection, `EventBatch`/`EventRecord`, and the data route tests are the
starting point; adapt them to the data-service state and re-run them there.
`Timeframe` moves behind the data-service boundary; the gateway no longer needs
the re-export.

**P4 — Model service.** Move the model module. The model service calls data for
the config bundle and candles, computes the same model, and writes a model
bundle with a manifest. The public `/model/compute` keeps its existing response;
the internal model-bundle endpoint is for the worker. The gateway stops serving
`/model/*`. The model service never reads strategy, execution, or portfolio.

**P5 — Evidence service.** Move the run store, run identity, stages, research
memory, hypotheses, and finalization. Evidence owns `runs` and `research`. It
accepts submission and cancel from the gateway proxy, validates data/config via
data, computes the run id, writes the row, and publishes `run.requested`. The
worker's claim, complete, fail, and cancel checks become internal calls. The
gateway stops writing the `runs` schema. The parked `routes/runs.rs` artifact and
digest handlers, the `RunDigest` type, the `RunStore::count` change, the Postgres
count, and the migration 13 registration are the starting point; adapt them to
the evidence state and per-service ledger, and re-run their tests there.

**P6 — Backtest service.** Move the worker. Add the staged application use case
that consumes a model bundle and candles and runs strategy, execution, and
portfolio without recomputing the model. The worker orchestrates evidence, data,
and model over HTTP, keeps the three-attempt retry budget, checks cancellation at
the same boundaries, and writes the same artifact set and canonical digest. #649
stays Done; the successor issue records the new orchestration.

**P7 — Simulation service.** Land #679 parity first, then #680. The service owns
the `simulation` schema and the eleven endpoints, reads model bundles from CAS,
and never computes a model or writes a run artifact. The gateway proxy for
`/simulation/*` is completed here. #646 can close after P7 and P8.

**P8 — Gateway finalization.** Remove the last domain modules from the crate,
rename it to `quantrule-gateway`, and make the compile-time guard real: the
gateway manifest must not depend on `quantrule-app`, `quantrule-data`,
`quantrule-model`, or `quantrule-backtest`. Move the profile store to
`gateway.profiles`, keep static and health aggregation, and delete
`quantrule-server` from the workspace. Update the crate list, module map,
service catalog, deployment, data plane, control plane, API, and README. The
parked `runtime/auth.rs`, `tests/surface.rs`, and the boundary tests are the
starting point for the gateway crate; the route-table test becomes the proxy
surface test, and the source-level boundary test is replaced by the compile-time
guard.

## 8. Acceptance criteria

1. The workspace builds eight new crates: `quantrule-runtime`,
   `quantrule-service-api`, `quantrule-gateway`, `quantrule-data-service`,
   `quantrule-model-service`, `quantrule-backtest-service`,
   `quantrule-evidence-service`, and `quantrule-simulation-service`; no service
   crate depends on another service crate.
2. `cargo build --bins` produces exactly six binaries: `qr-gateway`, `qr-data`,
   `qr-model`, `qr-backtest`, `qr-evidence`, `qr-simulation`. M1 is reached in
   P2 and retained through P8.
3. The gateway has no dependency on `quantrule-app`, `quantrule-data`,
   `quantrule-model`, or `quantrule-backtest`; a compile-time guard fails if one
   is added. The gateway handles only profiles, static, proxy, and health.
4. Data owns `catalog`; evidence owns `runs` and `research`; simulation owns
   `simulation`; gateway owns `gateway`. No service reads another service's
   schema; a source and query guard fails on a cross-schema read.
5. The gateway proxies every public route in §5.3 and returns 404 for
   `/internal/v1/*`; no client path, host, or internal header is accepted.
6. `POST /catalog/datasets` streams through the gateway and data without
   buffering the whole upload; the 512 MiB cap returns `413 upload.too_large`.
7. `POST /model/compute` keeps its existing public response; the internal model
   bundle manifest lists every artifact and the bundle SHA-256 matches the
   coupled baseline.
8. `POST /runs`, `GET /runs`, `GET /runs/{id}`, and `POST /runs/{id}/cancel`
   are served by evidence through the gateway; evidence publishes
   `run.requested`; the gateway writes no run row.
9. A queued run completes through data, model, backtest, and evidence over HTTP
   with the same artifact names, bytes, and canonical digest as the coupled
   `backtest run` baseline. The worker fast path is gone.
10. A failure at each hop produces the documented retry or terminal outcome; a
    cancel between hops leaves no artifact and no `succeeded` run.
11. Simulation serves the eleven documented endpoints; its isolation guard
    proves no run write, no model compute, no strategy read, and no queue
    publish.
12. Each service has its own migration list and ledger; a fresh database starts
    with `make up`; no pre-split database upgrade is claimed.
13. `cargo test`, clippy, fmt, rustdoc, the repository checkers, and the
    composed tier for the stack pass; the canonical digest is stable.
14. The docs, changelog, AI log, crate list, module map, service catalog, data
    plane, control plane, API, deployment, and README are updated in the phase
    that changes them.
15. #649 stays Done and its successor is merged; #643, #646, #651, and #674
    receive the comments named in §9.
16. The parked S1 work is inventoried and mapped before P3; every reused hunk is
    re-tested in its owning service crate; no route is implemented twice; the
    parked work is never cited as landed evidence.
17. No model, strategy, execution, or portfolio definition or result changes.
18. The migration runner acquires and releases the advisory lock on one
    connection; two service migrators against one database complete without a
    deadlock, and the composed test fails on a timeout.
19. The object-store image is recorded as a stopgap with
    `QR_OBJECT_STORE_IMAGE`; the release records the human's permanent registry
    decision, and no document describes the legacy archive as permanent.

## 9. Downstream gates

| Gate | Ready after | What it gets | What it still owns |
|---|---|---|---|
| #643 image | P2 for the binary prerequisite; P3-P7 for functional services | #772 already provides the image and the six-container default stack; B fills the three missing services and their containers | Dockerfile, compose, healthchecks, no-Python/no-Bun checks |
| #644 Makefile | #643 | One-command workflow | Targets, scale, ps, remote, backup |
| #646 gateway | P3 and P4 for data/model; P5 for runs; P7 for simulation; P8 for final isolation | A gateway-only proxy and a working simulation surface | Its own route table review and close |
| #651 guard | P8 | Compile-time gateway isolation and per-service schema guards | The guard test details and close |
| #673 digest | #643 | Image digest in health and provenance | Release pipeline |
| #674 release | P7, P8, and #643/#644/#673 | Six services, nine containers, working simulation, release evidence | Release note, tag, board close-out, permanent object-store registry |
| #661 memory | P5 | Postgres research memory in evidence | Its own close or supersession |
| #680 simulation | P7 | The service and eleven endpoints | Its own close |

### 9.1 Image and object-store gates

| Finding | Current state | Required decision |
|---|---|---|
| Migration lock | #772 observed the session-level advisory lock released through the pool; three migrators deadlocked. Compose sets the gateway as the sole migrator as a workaround | P1 pins the runner to one connection and proves two migrators do not deadlock. The sole-migrator workaround is removed only after that test passes |
| Object-store registry | MinIO withdrew its public images on 2026-09-11. #772 pins `bitnamilegacy/minio@sha256:451fe6858cb770cc9d0e77ba811ce287420f781c7c1b806a386f6896471a349c` (amd64) as a stopgap, overridable with `QR_OBJECT_STORE_IMAGE` | Before release, the human records a maintained registry or mirror. The legacy archive is not described as permanent and is not the default once the decision is recorded |

## 10. Blast radius

| Kind | Estimate |
|---|---|
| Crates | 7 new crates, 1 rename, `quantrule-server` retired; workspace grows from 9 to 16 members |
| Source lines | ~9,400 `quantrule-server` lines moved or retired; ~3,900 server test lines redistributed |
| Binaries | 3 existing binaries move; 3 new binaries added; 6 total |
| Migrations | 13 existing files redistributed into 4 service sets and 4 ledgers; new gateway profile migration |
| Contracts | 1 new internal contract crate; 1 new model bundle manifest; 1 new run lifecycle API; 1 additive public error code |
| Transport | 1 service client; 5 service URLs; 6 health ports; 1 proxy table |
| Schemas | 1 gateway schema added; catalog, runs, research, simulation separated; fresh database |
| Documentation | AGENTS crate list, module map, service catalog, data plane, control plane, API, deployment, README glossary, changelog, AI log |
| Parked S1 | 16 modified and 5 untracked files, ~568 insertions and 618 deletions; 12 mapped reuse areas; 0 treated as landed |
| Image findings | #772 merged: 3 of 6 binaries, 6 default containers; migration lock workaround; legacy MinIO stopgap with a human registry decision before release |
| Delivery | 12-16 reviewable PRs across P1-P8; weeks, not days; P6 and P7 are the long poles |
| Deferred | Embedded desktop v1.1, shell HTTP, simulation screens, streaming downloads, app-crate split |

## 11. Verification

The plan changes no behavior. The behavioral rows stay `Pending` until their
phase lands. Gate commands run through `tools/gate.sh` with a target directory
inside the worktree.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Plan checks | Plan structure, length, and links pass | `python3 tools/check_execution_plans.py && python3 tools/check_doc_length.py && python3 tools/check_links.py && git diff --check` | pass | pass | Pass | agent | 2026-11-14 | Plan-only revision |
| V2 | P0 decision | B is recorded; the issue exists; the plan is renamed; the crate ADR exists | `grep -n "Option B" docs/execution-plans/2026-11-14/`; `gh issue view <n>`; `ls docs/adr/2026-11-14/` | four artifacts | — | Pending | — | — | No issue number invented |
| V3 | P1 runtime | Runtime extraction compiles; no engine dependency; no route change | `cargo tree -p quantrule-runtime`; `cargo test --workspace` | no engine; pass | — | Pending | — | — | Behavior-neutral |
| V4 | P1 contracts | Internal contract types round-trip; no engine or runtime dependency | `cargo test -p quantrule-service-api`; `cargo tree -p quantrule-service-api` | pass; no engine | — | Pending | — | — | |
| V5 | P2 M1 | Six binaries build; the inventory guard fails on removal | `cargo build --bins`; `cargo test -p quantrule-gateway --test boundary six_binaries` | six; pass; seeded removal fails | — | Pending | — | — | #643 prerequisite |
| V6 | P2 health | Each new binary answers `/health` and `/version` | New service health tests | 200 on both | — | Pending | — | — | No domain route claimed |
| V7 | P3 data | Data routes answer from `qr-data`; gateway has no data handlers | `cargo test -p quantrule-data-service`; gateway proxy test | pass; no handler | — | Pending | — | — | Catalog owner |
| V8 | P3 streaming | A 2 MiB upload passes through without full buffering; the cap returns 413 | Paused-producer test | first chunk forwarded; 413 over limit | — | Pending | — | — | |
| V9 | P3 schema | Only data reads `catalog`; the profile store is `gateway.profiles` | Schema guard; `grep` on queries | pass; one writer | — | Pending | — | — | |
| V10 | P4 model | Model route is served by `qr-model`; bundle hash matches the baseline | `cargo test -p quantrule-model-service model_` | equal SHA-256 | — | Pending | — | — | Public response unchanged |
| V11 | P4 bundle | The internal manifest lists every artifact and verifies each hash | Manifest test | pass | — | Pending | — | — | Worker input |
| V12 | P5 evidence | Submit, list, detail, and cancel are served by evidence; gateway writes no run row | Evidence API tests; gateway proxy test | pass; no write | — | Pending | — | — | Run owner |
| V13 | P5 queue | Evidence publishes `run.requested`; duplicate submission republishes without a second run | Queue fake test | one run; republish | — | Pending | — | — | Control plane |
| V14 | P5 research | Hypotheses are served by evidence; no gateway research store | Hypothesis API tests | pass | — | Pending | — | — | |
| V15 | P6 worker | A queued run completes over HTTP with the baseline artifact bytes and digest | Worker integration test | same names, bytes, digest | — | Pending | — | — | #649 successor |
| V16 | P6 no fast path | The worker never calls `AppService::run_backtest`; the model is not recomputed | `grep` guard; worker test | no call; one model call | — | Pending | — | — | |
| V17 | P6 failure map | Each hop failure is retryable or terminal as §5.6 says | Failure tests per hop | as documented | — | Pending | — | — | |
| V18 | P6 cancel | Cancellation between hops leaves no artifact and no succeeded run | Cancel test | cancelled; empty CAS | — | Pending | — | — | |
| V19 | P7 simulation | Eleven endpoints answer; #679 parity passes; isolation guard passes | Simulation service tests; parity gate | pass | — | Pending | — | — | #646 close |
| V20 | P8 gateway | The gateway links no app or engine; `/internal/*` is 404; proxy table is complete | `cargo tree -p quantrule-gateway`; boundary test | no app/engine; 404; pass | — | Pending | — | — | Compile-time |
| V21 | P8 schema | Each service owns one migration set and one ledger; no cross-schema query | Migration tests; schema guard | pass | — | Pending | — | — | Fresh DB |
| V22 | P8 retire | `quantrule-server` is removed; the workspace has eight new crates | `cargo metadata`; `grep` on workspace members | 16 members; no server | — | Pending | — | — | |
| V23 | P8 health | `/health` aggregates five services; `/health/ready` returns 503 when one is down | Gateway health tests | names; 503 | — | Pending | — | — | |
| V24 | P9 downstream | #643, #646, #651, and #674 receive the evidence and comments named in §9 | `gh issue view` | comments posted | — | Pending | — | — | Not closed by this plan |
| V25 | Gates | Tests, clippy, fmt, rustdoc, checkers, composed tier | `tools/gate.sh`; `tools/test_tiers.sh composed` | pass | — | Pending | — | — | Composed tier before release |
| V26 | Determinism | Coupled baseline and split run share the canonical digest | Fixture run twice | equal | — | Pending | — | — | Regression oracle |
| V27 | Honesty | Deferred items and buffered downloads are recorded, not claimed | Manual read of §4.3 and §5.6 | present | — | Pending | — | — | |
| V28 | Parked inventory | Every parked S1 file and hunk is mapped to a phase and an action before P3 | Salvage map in the issue; diff stat | complete | — | Pending | — | — | Not landed |
| V29 | Parked reuse | Each reused hunk passes its test in the owning service crate; no route is implemented twice | Owning service test; code search for duplicate handlers | pass; one implementation | — | Pending | — | — | Reuse or reimplement |
| V30 | Migration lock | The runner acquires and releases on one connection; two service migrators do not deadlock | `QR_TEST_DATABASE_URL=postgres://... cargo test -p quantrule-runtime --test migration_lock two_service_migrators_do_not_deadlock -- --ignored` | both finish inside 10 s; one ledger row per migration | — | Pending | — | — | Composed tier; #772 workaround stays until this passes |
| V31 | Object-store registry | The legacy image is a stopgap, overridable, and not described as permanent; the release records the human registry decision | `grep -R "bitnamilegacy/minio\|QR_OBJECT_STORE_IMAGE" compose.yaml docker/README.md docs/architecture/deployment.md`; release note | override present; legacy named stopgap; decision recorded | — | Pending | — | — | No permanent claim |
| V32 | Closure | Issue `Done`, worktree and branches removed, comments posted | `gh issue view`; `git worktree list` | done; removed | — | Pending | — | — | |

## 12. Risks and recovery

| Risk | Impact | Mitigation |
|---|---|---|
| A multi-phase crate split destabilises a working tree | Build and tests fail across phases | Each phase is a separate PR; `quantrule-server` stays until P8; P1 is behavior-neutral except the migration-lock fix, which has its own test; revert one phase |
| The migration lock is released through the pool | Two migrating services deadlock; the stack depends on a sole-migrator workaround | Pin one `PoolConnection`; acquire, migrate, and release on it; composed two-migrator test with a timeout; do not treat the gateway workaround as the fix |
| The legacy MinIO archive becomes the permanent source | The release ships an unmaintained, amd64-only object store | Keep `QR_OBJECT_STORE_IMAGE` overridable; record the human registry decision before release; do not describe the legacy archive as permanent |
| The model bundle cannot reproduce the coupled artifact bytes | The worker rewrite changes the canonical digest | Golden fixture; byte equality before P6 is accepted; stop if it differs |
| The worker HTTP path changes failure or cancellation semantics | A run succeeds when it should fail, or vice versa | Failure map per hop; cancel between hops; duplicate delivery; queue retry budget unchanged |
| Evidence becomes a single critical path | Submission, claim, and finalization stop if evidence is down | Content-addressed idempotency; queue redelivery; health and 503; evidence restart test |
| Schema split cannot be migrated from an existing database | A development database breaks | Fresh database for v1.0.0; `make reset`; no upgrade claim; D14 |
| Profiles move to the gateway schema | A request lacks a profile or defaults silently | Gateway validates and injects; default profile migration; 404 for unknown; downstream rejects missing header |
| Streaming upload still buffers in the data service | Memory exhaustion on a large CSV | Temp file with a cap; stream to object store; paused-producer test; no full body in memory |
| The gateway proxy leaks a header or accepts a path | Security defect at the trust boundary | Header allowlist; no host/path; internal prefix 404; security review; seeded tests |
| Simulation #679 parity is not ready | P7 blocks and #646 cannot close | P7 is gated on #679; keep the skeleton; do not fake parity; #646 stays open |
| The new crate count slows the build | Developer friction | Shared runtime; one binary per crate; CI build matrix; measure the first build |
| `quantrule-app` still links engines into compute services | A reviewer reads "full split" as compile-time everywhere | State the boundary: gateway is compile-time isolated; compute services share app by design; app split is deferred |
| Active streams collide on `runtime/server.rs` | Merge conflicts and duplicated work | Phase ownership table; coordinate with #643, #646, #668, #680 before P3 and P8 |
| The plan is mistaken for the release | #674 closes on a plan, not evidence | §9 downstream gates; #674 stays open until image, compose, digest, and verification exist |
| The parked S1 work drifts or disappears | A reusable behavior is lost or conflicts with B | Capture the diff before P3; salvage map in the issue; reimplement from the public contract if it disappears; never cite it as landed |
| The parked work is copied wholesale and duplicates a route | Two implementations of the same endpoint | Reuse by hunk into the owning service; adapt state and imports; V29 checks for duplicate handlers |
| A parked hunk bypasses the new schema or profile boundary | A service reads a schema it does not own | Review every salvaged hunk against §5.5; schema guard in P8; re-run the boundary tests |

Recovery: each phase is a revertible PR. P1 is behavior-neutral except the
migration-lock fix, which is covered by the two-migrator test. P2 is additive.
P3-P7 move one route or store at a time; the gateway can serve the old handler
until its proxy is ready. P8 is the only deletion phase; it happens last. The
database is fresh, so no data migration is rolled back. Artifacts are
content-addressed, so an interrupted run leaves no visible set. The image is one
digest; a rollback is the previous digest.

## 13. Stop conditions

- A phase changes a model, strategy, execution, or portfolio result, or the
  coupled baseline digest.
- The model bundle cannot carry what the strategy, execution, and portfolio
  stages need without recomputing the model or adding a model-layer API.
- The worker HTTP path needs a new engine API, a new model output shape, or a
  change to run identity.
- The schema split needs an upgrade path for a non-empty database; that is a
  data migration decision.
- The internal contract needs a crate or feature not already in `Cargo.lock`.
- A streaming upload cannot be bounded without buffering the whole body.
- #679 simulation parity is not ready and P7 would have to fake an endpoint.
- A file is owned by #643, #646, #668, or #680 and cannot be coordinated.
- The parked S1 diff is the only copy of a behavior and the worktree is about to
  be rebound or removed; capture the diff before proceeding.
- The migration runner cannot pin the advisory lock to one connection without a
  new dependency or a database API change.
- The release would describe the legacy `bitnamilegacy/minio` archive as the
  permanent object store, or the human registry decision is not recorded.
- A salvaged hunk would make a service read another service's schema or skip the
  profile boundary.
- A phase would close #643, #646, #651, or #674 on a plan rather than evidence.
- Any test, gate, or review fails in a way that changes the approved scope.

## 14. Issue body draft (10 lines)

```text
Goal: Split the six services into owned crates: data, model, evidence, backtest, simulation, and a gateway that only proxies, so the six-binary image, the HTTP data plane, and the simulation surface are true.
Scope:
- Extract quantrule-runtime and quantrule-service-api; move quantrule-server modules into five service crates and a gateway crate.
- Move run records, research memory, and schema ownership to evidence; models to a bundle protocol; the worker to HTTP; profiles to a gateway schema.
- Keep the six binaries as milestone M1; pin the migration lock; reuse the parked #646 gateway work by hunk, not as landed; #643/#646/#674 stay their own issues.
Done when:
- Six service crates build six binaries; no service reads another's schema; the gateway links no engine or quantrule-app.
- A queued run completes through data, model, backtest, and evidence over HTTP with the same canonical digest as the coupled baseline.
- Simulation serves its eleven endpoints; #646 and #651 can close; #643/#674 have their evidence.
Claim type: ADR (Architecture Decision)
```

## 15. Review checklist

- [ ] The human selected B on 2026-11-14 and the plan records A and C as
      rejected.
- [ ] The issue body is exactly the 10 lines in §14 and carries one claim type.
- [ ] The plan is renamed on approval and the old name is not linked.
- [ ] Every phase names its owned paths and its dependency.
- [ ] M1 is described as a milestone, not the end state.
- [ ] The gateway compile-time guard and per-service schema guard are in P8.
- [ ] The worker's HTTP path, cancellation, retry map, and streamed upload are
      in P6 and §5.6.
- [ ] #649 stays Done and the successor is named.
- [ ] The parked S1 diff is mapped before P3, reused by hunk, and never cited as
      landed.
- [ ] Deferred work is listed: desktop v1.1, shell HTTP, simulation screens,
      streaming downloads, app-crate split.
- [ ] No model, strategy, execution, or portfolio result changed.
