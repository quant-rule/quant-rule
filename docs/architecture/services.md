# Service Catalog

- Status: Current; released as v0.9.0 on 2026-11-15 (prepared as v1.0.0); current in v1.4.1; approved 2026-11-12
- Part of: [`README.md`](README.md)
- Decision: [`ADR-2026-11-12-01`](../adr/2026-11-12/01-adopt-distributed-service-based-architecture.md)
- Claim type: Behaviour Claim

Six services, six binaries, one image. Each service owns one responsibility and
one set of data. The catalog is the single authority for "who does what".

Every record is scoped by `profile_id`. Profiles are organisational, not a
security boundary ([`ADR-11`](../adr/2026-11-12/11-profiles.md)).

## 1. Boundaries

| Service | Binary | Responsibility | Owns | Must never |
|---|---|---|---|---|
| `gateway` | `qr-gateway` | HTTP boundary, static client, upload streaming, health aggregation, principal injection, proxy table | `gateway` schema (profiles) | compute a model value, read a candle, render a label |
| `data` | `qr-data` | Ingestion, `data_version`, timeframe ladder, dataset and config catalog (including the read-only `config/` mount), read-only inspection | `catalog` schema, `datasets/` and `ladder/` objects | run a model, decide a signal |
| `model` | `qr-model` | Readings, states, events, levels, progression, context, report | model objects | read strategy, execution, or portfolio |
| `backtest` | `qr-backtest` | Run orchestration, strategy, execution, portfolio, diagnostic stage execution | artifact objects, stage worker | own the final provenance, canonical digest, or run record |
| `evidence` | `qr-evidence` | Provenance, canonical digest, run records, research memory, stage lifecycle, finalization | `runs`, `research`, and `stage` schemas, bundle objects | compute a reading or decide a trade |
| `simulation` | `qr-simulation` | Chart-first research: window selection, marks, oracle, review, check, decision | `simulation` schema, research objects | compute a model, write a run artifact, publish to the queue |

The worker builds the run manifest and proposes its canonical digest; evidence
re-verifies the manifest, owns the recorded digest, and writes the run record.
The two implementations must agree, and the evidence check is the authority.

## 2. Dependency direction

```text
gateway ──────────────┐
                      ├──> quantrule-api
                      ├──> quantrule-service-api
                      └──> quantrule-runtime

data, model, backtest ──> quantrule-app ──> quantrule-backtest ──> quantrule-model ──> quantrule-data
evidence ───────────────> quantrule-research
simulation ─────────────> quantrule-simulation
```

The engines are unchanged by distribution. Every service binary is an adapter
over its domain crate; data, model, and backtest share `quantrule-app`, and
evidence and simulation use their own domain crates.

- `quantrule-api` holds contract types and must not depend on an engine.
- `quantrule-runtime` holds configuration, health, adapters, the migration
  runner, and the internal client. It must not depend on an engine or
  `quantrule-app`.
- `/inspect/candles` and `/inspect/timeframes` stay with `data`;
  `/inspect/events` computes a model, so the gateway forwards it to `model`'s
  internal event-batch route.
- The gateway links no app, engine, or domain service crate. The compile/source
  guard `crates/quantrule-gateway/tests/service_boundary.rs` fails when that
  edge, a second service binary, or a cross-schema read appears.
- Workers may call `quantrule-app` freely. Evidence and simulation must not.
- No service crate depends on another service crate.

## 2.1 Public boundary and supported topologies

Only the gateway publishes a host port. The other services are reachable on the
container network, or on the private overlay in the multi-host profile, and
never from the client side. Publishing `data`, `model`, `evidence`,
`simulation`, or `backtest` would expose an internal contract, bypass the
gateway token, and let a client depend on another service's schema.

| Topology | Status | Shape |
|---|---|---|
| Single host, Compose | Supported; default | Six services and three infrastructure containers on one host |
| Desktop, embedded | Supported | Six sidecars on loopback with SQLite, filesystem CAS, in-process queue |
| Split host, client/server | Supported | Full stack on one server; browser and shell on another machine |
| Multi-host services | Supported; implemented, release pending | Six services across hosts, private overlay, one image digest |
| Cloud, HA, Kubernetes, mTLS | Not supported | A later decision with its own ADR |

Distribution is about separate processes, failure units, and scaling units, not
about one published port per service. The one-port rule is a deployability and
security characteristic. The multi-host profile keeps it: internal services bind
only to the private overlay.

## 3. Scaling profile

| Service | Scale | Reason |
|---|---|---|
| `gateway` | by connection count | stateless HTTP |
| `data` | one | object-store I/O and a catalog; replicas add nothing |
| `model` | **N** | shards by timeframe; CPU-bound |
| `backtest` | **N** | one run per worker; CPU-bound and sequential within a run |
| `evidence` | one | finalization is a short, ordered step |
| `simulation` | one | interactive; no batch parallelism, no queue |

`backtest` is never sharded inside a run: strategy, fills, position, and cash are
a fold over time. Claiming otherwise would be false.

## 4. Deliberate non-splits

| Kept together | Reason |
|---|---|
| strategy + execution + portfolio | Per-bar synchronous connascence: fill → position → cash. Splitting creates a distributed transaction per bar. |
| dataset + config catalog | Both are immutable inputs to a run; one owner avoids a two-phase submit. |
| provenance + canonical digest + run record | One authority for "what is true about this run". |

## 5. Failure behaviour

| Failure | Result |
|---|---|
| `gateway` down | Client shows disconnected; queued and running work is unaffected |
| `data` down | Submission of a new run fails fast; existing runs are unaffected |
| `model` down | The run fails at the model stage; no artifact is finalized |
| `backtest` down mid-run | Message is redelivered; recomputation is idempotent |
| `evidence` down after compute | Artifacts exist but the run stays `running`; finalization retries |
| object store down | The run fails; no partial bundle becomes visible |
| queue down | Submission fails fast; nothing is silently dropped |

## 6. What no service may do

- Read or write another service's schema.
- Accept a client-supplied filesystem path.
- Place a wall-clock value inside an artifact.
- Render a human-readable label for a numeric code.
- Publish a model reading on the control plane.

## 7. Backend adapters

Database, object store, and queue sit behind traits. v0.9.0 ships one
implementation of each: Postgres, S3, RabbitMQ. The v1.0.0 desktop profile adds a
second implementation of each and selects the set once from `QR_PROFILE`; the
server profile stays the default.

| Trait | v0.9.0 server profile | v1.0.0 desktop profile |
|---|---|---|
| Database | Postgres | SQLite |
| Object store | S3 API | filesystem CAS |
| Queue | RabbitMQ | in-process |

A service calls the trait, never a client library directly. A second
implementation must not require a use-case change; that is the test.

## 8. Stage diagnostics

`evidence` owns the `stage` schema. A stage submission is validated and
content-addressed by `evidence`; the `backtest` worker claims it, dispatches to
the model, strategy, or portfolio executor, publishes the content-addressed
artifact, and records `stage.computed`. The gateway proxies `/stages/*`; the
model service serves model-stage event reads. A stage artifact is diagnostic:
`evidence: false`, no canonical run digest, no baseline, and TTL expiry. The
coupled `run submit` remains the only evidence path.

## 9. The research plane

`simulation` is not a pipeline stage and not a run. It is a third plane:
synchronous, interactive, and isolated.

| Rule | Enforced by |
|---|---|
| It never writes into a run's artifact set | schema ownership + guard test |
| No strategy, execution, or portfolio code reads it | import guard |
| It may read a model bundle; it may never compute a model | symbol allowlist |
| It has no `run_id`, no canonical digest, no queue message | contract + test |
| Human marks are judgment: no output digest | contract |

A simulation run records an **inputs digest** over the model bundle, window
selection, seed, mark criterion, and check thresholds. That is what makes the
selection reproducible without claiming the human marks are.
