# ADR — Adopt a Distributed Service-Based Architecture

- Date: 2026-11-12 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Supersedes: [`2026-11-11/adopt-local-first-tauri-platform.md`](../2026-11-11/adopt-local-first-tauri-platform.md)
- Amends: [`2026-10-31/adopt-architecture-baseline.md`](../2026-10-31/adopt-architecture-baseline.md)
- Plan: [`dist-000-distributed-platform.md`](../../execution-plans/2026-11-12/dist-000-distributed-platform.md)

## Context

The product runs batch, CPU-bound work that can outlive a terminal session and
can exceed one machine. The delivery target is an internal team network. The
course requires a distributed architecture. The previous baseline — one local
process, one quantum, a Tauri desktop — is correct for a laptop-only tool and
wrong for these drivers.

Distribution is not justified by a user count. It is justified by the duration,
size, and location of the work. The client is one operator; the work is not.

## Decision

Adopt a **Service-Based Architecture** with five services and one supporting
event-driven seam:

| Service | Responsibility | Owns |
|---|---|---|
| `gateway` | HTTP boundary, static client, upload streaming, health aggregation | nothing durable |
| `data` | Ingestion, `data_version`, timeframe ladder, dataset and config catalog | `catalog` schema, candle objects |
| `model` | Readings, states, events, progression, report | model objects |
| `backtest` | Run orchestration, strategy, execution, portfolio | artifact objects |
| `evidence` | Provenance, canonical digest, run records, memory | `runs` schema, bundles |

- **Service-Based** is the base style: coarse domains, shared infrastructure,
  one database instance with schema-per-service, one host first.
- **Event-Driven** is used only on the control plane (submission, finalization).
- The engines (`quantrule-data`, `-model`, `-backtest`) are unchanged. Each
  service is a thin adapter over `quantrule-app`.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Modular monolith (previous baseline) | Cannot survive the client, cannot use a second machine, no course deliverable |
| Microservices | 20+ services, sagas, mesh; operational cost exceeds the driving need |
| Event-driven data plane | Makes byte-identical output depend on queue timing |
| Space-based | No elastic in-memory load profile exists |
| SOA with an ESB | The bus is the coupling bottleneck; its useful part is Service-Based without the bus |

## Consequences

- Reproducibility becomes a distributed property: it needs content addressing,
  sorted merges, and an image digest in provenance.
- The client is a pure HTTP consumer; Tauri is removed.
- Local development requires Docker. That is the accepted cost of one code path.
- Simplicity and deployability stop being drivers and become trade-offs.
- Scalability and elasticity become real supporting characteristics with
  fitness functions (scale invariance, no partial publish).
- Multi-tenancy, accounts, and billing remain out of scope. One principal.

## Revisit trigger

Revisit when more than one operator must be isolated from another, when data
must leave the operator's network, or when a run must span more than one host
for capacity rather than convenience.

## Verification

- The five services map one-to-one to five binaries in one image.
- Every service names its owned data and its forbidden responsibilities in
  [`services.md`](../../architecture/services.md).
- The determinism claim is bounded in [`ADR-07`](07-canonical-digest-and-claim-boundary.md).
