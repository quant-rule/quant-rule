# ADR — Content-Addressed Exchange and a Single Image

- Date: 2026-11-12 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Related: [`01`](01-adopt-distributed-service-based-architecture.md), [`07`](07-canonical-digest-and-claim-boundary.md)

## Context

Distribution moves data between processes. Two problems appear: the network cost
of large intermediate results, and the risk that two services run different code
versions and silently disagree.

## Decision

**1. Services exchange references, not payloads.**

Every stage writes its output to content-addressed object storage (MinIO) and
passes the key. The key is the SHA-256 of the bytes.

Consequences:

- The network carries kilobytes of references, not hundreds of megabytes of readings.
- A recomputation that produces the same bytes hits the same key: idempotent by
  construction, so retries are free.
- Provenance becomes a verifiable hash chain.
- Ordering is explicit: a merge key is sorted, never inherited from arrival order.

**2. One image, five binaries.**

| Binary | Service |
|---|---|
| `qr-gateway` | gateway |
| `qr-data` | data |
| `qr-model` | model |
| `qr-backtest` | backtest |
| `qr-evidence` | evidence |

The image also contains the built Svelte client. One digest pins the entire
compute stack, so version skew between services cannot occur.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| One image per service | Five digests, five build times, real skew risk |
| Pass payloads over HTTP | The network becomes the bottleneck and the retry cost grows with the payload |
| Serve artifacts from the gateway's filesystem | Two sources of truth for the same bytes |

## Consequences

- Object storage is a hard dependency of every worker. It is part of the stack,
  not optional infrastructure.
- Garbage collection of unreferenced objects is a later concern; unreferenced
  objects are harmless.
- `provenance.json` records one digest, which simplifies the determinism claim.

## Revisit trigger

Revisit if a service must ship on its own cadence, or if object storage becomes
a capacity constraint rather than a convenience.

## Verification

- No HTTP body in the data plane exceeds a small metadata envelope.
- Every artifact row stores its SHA-256.
- `/health` reports the image digest, and every run's provenance matches it.
