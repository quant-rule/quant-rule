# Service Catalog

- Status: Target v1.0.0; approved 2026-11-12
- Part of: [`README.md`](README.md)
- Decision: [`ADR-2026-11-12-01`](../adr/2026-11-12/01-adopt-distributed-service-based-architecture.md)
- Claim type: Behaviour Claim

Five services, five binaries, one image. Each service owns one responsibility and
one set of data. The catalog is the single authority for "who does what".

## 1. Boundaries

| Service | Binary | Responsibility | Owns | Must never |
|---|---|---|---|---|
| `gateway` | `qr-gateway` | HTTP boundary, static client, upload streaming, health aggregation, principal injection | nothing durable | compute a model value, read a candle, render a label |
| `data` | `qr-data` | Ingestion, `data_version`, timeframe ladder, dataset and config catalog (including the read-only `config/` mount), read-only inspection | `catalog` schema, `candles/` objects | run a model, decide a signal |
| `model` | `qr-model` | Readings, states, events, levels, progression, context, report | model objects | read strategy, execution, or portfolio |
| `backtest` | `qr-backtest` | Run orchestration, strategy, execution, portfolio | artifact objects | own provenance or compute a digest |
| `evidence` | `qr-evidence` | Provenance, canonical digest, run records, research memory, finalization | `runs` schema, bundle objects | compute a reading or decide a trade |

## 2. Dependency direction

```text
gateway ──────────────┐
                      ├──> quantrule-api
worker services ──────┘
      │
      └──> quantrule-app ──> quantrule-backtest ──> quantrule-model ──> quantrule-data
```

The engines are unchanged by distribution. Every service binary is an adapter
over `quantrule-app`, exactly as the shell was.

- `quantrule-api` holds contract types and must not depend on an engine.
- The gateway must not reach an engine module; a guard test enforces it.
- Workers may call `quantrule-app` freely.

## 3. Scaling profile

| Service | Scale | Reason |
|---|---|---|
| `gateway` | by connection count | stateless HTTP |
| `data` | one | object-store I/O and a catalog; replicas add nothing |
| `model` | **N** | shards by timeframe; CPU-bound |
| `backtest` | **N** | one run per worker; CPU-bound and sequential within a run |
| `evidence` | one | finalization is a short, ordered step |

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
