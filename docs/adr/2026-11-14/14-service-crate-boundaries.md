# ADR — Service Crate Boundaries for the Six-Service Split

- Date: 2026-11-14 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Related: [`01`](../2026-11-12/01-adopt-distributed-service-based-architecture.md), [`02`](../2026-11-12/02-two-plane-control-and-data.md), [`04`](../2026-11-12/04-content-addressed-exchange-and-single-image.md), [`07`](../2026-11-12/07-canonical-digest-and-claim-boundary.md), [`12`](../2026-11-12/12-desktop-distribution.md)
- Plan: [`776-split-service-binaries.md`](../../execution-plans/2026-11-14/776-split-service-binaries.md) (Option B, P1)

## Context

The accepted architecture names six services and one image, but the tree still
carries one mixed `quantrule-server` crate: the gateway serves data and model
routes, writes run records, and the worker computes in process. Option B was
selected on 2026-11-14. Before any domain move, the shared runtime and the
service-to-service contract need one home each, or every service crate would
duplicate configuration, health, storage, and shutdown.

## Decision

Adopt this target crate graph. Dependencies point one way; a service never
depends on another service crate.

```text
quantrule-api ──────────────┐
quantrule-service-api ──────┤
quantrule-runtime ──────────┼──> quantrule-gateway          (qr-gateway)
                            ├──> quantrule-data-service     (qr-data)
                            ├──> quantrule-model-service    (qr-model)
                            ├──> quantrule-backtest-service (qr-backtest)
                            ├──> quantrule-evidence-service (qr-evidence)
                            └──> quantrule-simulation-service (qr-simulation)

quantrule-app ──> quantrule-backtest ──> quantrule-model ──> quantrule-data
```

| Crate | Owns | Must never |
|---|---|---|
| `quantrule-api` | Public `quantrule.api/v1` types | Depend on an engine, runtime, or service |
| `quantrule-service-api` | Internal `quantrule.internal/v1` run, bundle, object, manifest, and digest types | Depend on an engine, runtime, or service; render a label |
| `quantrule-runtime` | Config, health, logging, shutdown, HTTP serve, adapters (Postgres, S3, RabbitMQ), migration runner, client contract | Depend on an engine or `quantrule-app`; own a domain row |
| `quantrule-gateway` | Profiles, static client, proxy table, health aggregation | Link `quantrule-app` or an engine; compute a model; write a domain schema |
| Data, model, backtest services | Their domain module, routes, and store | Read another service's schema |
| Evidence service | Run records, research memory, hypotheses, finalization, digest | Compute a reading or a trade; depend on `quantrule-app` |
| Simulation service | `simulation` schema, research objects, eleven endpoints | Compute a model, write a run artifact, publish a queue message |

Rules:

- **One schema, one writer.** `catalog` is data's, `runs` and `research` are
  evidence's, `simulation` is simulation's, `gateway` is the gateway's. A
  service may not read another service's schema; it calls that service.
- **Content, not payloads.** Services exchange digests, object keys, bundle
  manifests, and run records. No internal response carries a candle or reading
  series.
- **The gateway is compile-time isolated.** It links no engine and no
  `quantrule-app`. The compute services share `quantrule-app` by design; an
  app-crate split is deferred.
- **The runtime carries no domain.** The Postgres repositories for catalog,
  runs, profiles, configs, and research stay with the modules that own those
  rows until their service crate exists.
- **One migration runner, one session.** The runner acquires, applies, and
  releases the advisory lock on one `PoolConnection`. A release through the pool
  can land on another connection and leak the lock, which is the #772 deadlock.

## P1 state

P1 lands the first row of the graph and no domain move:

| Change | Result |
|---|---|
| `quantrule-runtime` extracted | Config, health, logging, clock, queue names, shutdown, serve helper, adapter traits, S3, RabbitMQ, `PgDatabase`, migration runner, client contract, adapter boundary test |
| `quantrule-service-api` extracted | Run record, claim/cancel/computed/failed payloads, run manifest and canonical digest, config bundle, dataset object, model bundle manifest; round-trip tests |
| Migration lock pinned | One `PoolConnection` for lock, ledger, migrations, and release; two-migrator test with a 10 s budget |
| `quantrule-server` | Keeps its three binaries, routes, and behavior; the Postgres domain repositories move to `quantrule-server::adapters` until P3/P5; the old `runtime` module is deleted |

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Keep the runtime inside `quantrule-server` | Every service would link the mixed crate, its routes, and eventually `quantrule-app`; the split would be nominal |
| Put the internal contract in `quantrule-api` | A public client contract would grow service-only fields and could leak a service payload |
| Split all six service crates in one change | Unreviewable; a failing route would be indistinguishable from a failing extraction |

## Consequences

- A service crate starts with only its domain code; shared behavior is a dependency, not a copy.
- The internal contract changes in one crate, so a worker and evidence cannot
  disagree about a run field; the gateway's isolation becomes a manifest fact a
  compile-time guard can enforce.
- The migration lock is correct under concurrent migrators; the Compose
  sole-migrator workaround can be removed after this evidence.
- The domain Postgres repositories temporarily live in `quantrule-server`; P3
  and P5 move them with their schemas.

## Verification

- `cargo tree -p quantrule-runtime` shows no `quantrule-data`, `-model`,
  `-backtest`, or `-app`.
- `cargo test -p quantrule-service-api` round-trips every internal type.
- `cargo test -p quantrule-runtime --test boundary` names each client library
  only in its adapter.
- `QR_TEST_DATABASE_URL=... cargo test -p quantrule-runtime --test migration_lock
  two_service_migrators_do_not_deadlock -- --ignored` completes inside 10 s.
- `tools/gate.sh` passes with the existing route and worker tests unchanged.

## Revisit trigger

Revisit when a service needs a payload the bytes-only client cannot carry, when
the shared `quantrule-app` blocks a compute service, or when a per-schema ledger
replaces the shared migration ledger.
