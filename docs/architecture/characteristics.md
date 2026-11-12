# Architecture Characteristics

- Status: Current; approved 2026-11-12
- Companion: [`kata.md`](kata.md)
- Decision: [`ADR-2026-11-12-01`](../adr/2026-11-12/01-adopt-distributed-service-based-architecture.md)
- Normative rules: `AGENTS.md` sections 2, 3, 4, 12

## 1. Derivation

| Demand from the kata | Characteristic |
|---|---|
| "the same evidence, every time" | Reproducibility (determinism) |
| "no look-ahead"; events appear only when knowable | Temporal integrity (causality) |
| "inspect, reproduce, and defend"; a reviewer must compare | Auditability (evidence traceability) |
| "walk away"; the run outlives the client | **Detachability** (new) |
| "work can exceed one machine" | **Scalability** (promoted) |
| "pure filters"; one-way dependencies | Modularity |
| "one command"; one port | Deployability (supporting) |
| "every reading and event has evidence" | Testability (enabling) |

The drivers changed because the delivery target changed. Detachability is new:
the previous architecture could not express "the run continues without me".

## 2. Driving characteristics

| Characteristic | Good means | Fitness function | Current evidence |
|---|---|---|---|
| **Reproducibility** | Same inputs plus the same image digest produce byte-identical data artifacts | Determinism tests; canonical digest equality; golden fixtures | Strong for the engines; the distributed claim is bounded in ADR-07 and not yet exercised |
| **Temporal integrity** | No value at bar *t* depends on a later bar; events appear only at or after their availability bar | Causality and prefix tests; availability tests | Strong for the engines |
| **Auditability** | Every result names data version, config hash, model version, image digest, and canonical digest | Provenance fields; bundle-completeness test | Designed; not yet implemented |

## 3. Promoted: detachability and scalability

| Characteristic | Good means | Fitness function |
|---|---|---|
| **Detachability** | A submitted run completes after the client disconnects, and its status is readable on return | Submit, close the client, reopen, observe `succeeded` |
| **Scalability** | Model and backtest workers scale horizontally with no change in output | Scale invariance: one worker and four workers produce the same canonical digest |

Scalability is a supporting driver, not a third driver. It exists because the
work is CPU-bound and batch, not because there are many users.

## 4. Enabling characteristic: testability

Testability is the instrument that makes the other claims credible. Two tiers:

- **Unit and integration** — in-process transport fake; never opens a socket.
- **Composed** — opt-in, runs under Compose, required before a release.

A passing test that does not exercise the behavior proves nothing.

## 5. Supporting characteristics

| Characteristic | Good means | Fitness function |
|---|---|---|
| **Modularity** | Five services, one responsibility each; engines untouched by distribution | Service catalog; boundary guard tests |
| **Deployability** | One command starts the stack; one port is exposed | `make up`; health check |
| **Evolvability** | Contracts are versioned; a capability arrives as a new endpoint or command | `quantrule.*/v1` constants; compatibility tests |

## 6. Deliberately not optimized

| Characteristic | Decision | Consequence |
|---|---|---|
| **Availability** | A run is a job; a failure is a state, not an outage | No failover, no uptime target |
| **Multi-tenancy** | One principal; no isolation | A shared host is trusted |
| **Low latency** | Batch; a run takes as long as it takes | Simplicity wins over speed |
| **Security depth** | Host and network boundary, not application auth | Remote exposure needs a network gate |
| **Interoperability** | File and JSON contracts only | No third-party API |

## 7. Style and quantum

**Style.** Service-Based Architecture, with pipes-and-filters inside each service
and an event-driven control plane. Each service is coarse-grained and owns its
data. The data plane is synchronous and deterministic; the control plane is
asynchronous.

**Quantum.** Five deployables, one database instance with schema-per-service, one
object store. The set is one quantum for the operator because it deploys
together and shares one image; each service is a separate failure and scaling
unit.

```text
browser ── HTTP ── gateway ── postgres
                     │            ▲
                     │            └── evidence ── minio
                     ▼
                  rabbitmq ── backtest ── minio
                                │   └── model
                                └────── data
```

**Why this is the least worst.** A monolith cannot detach a run from the client
or use a second machine. Microservices add sagas, a mesh, and version skew for
no additional driver. Event-driven everywhere makes byte-identical output depend
on delivery order. Space-based and SOA/ESB have no load profile or integration
estate to serve. Service-Based plus a narrow event seam is the only option that
delivers detachability and scalability while keeping the determinism argument
finite.

## 8. Trade-offs

| Trade-off | Decision | Consequence |
|---|---|---|
| Reproducibility vs distribution | Distribution accepted; determinism defended by content addressing and invariants | The claim must be stated precisely |
| Simplicity vs distribution | Simplicity conceded | Docker becomes mandatory for local use |
| Scalability vs determinism | Shard only where the merge is order-independent | Backtest stays sequential per run |
| Auditability vs storage | Duplicate copies retained and hashed | Storage is cheap; provenance keeps copies traceable |
| Deployability vs native UX | Browser replaces the desktop | No native file dialogs |

**Priority if forced to shed one:** shed Evolvability first. Never shed
Reproducibility, Temporal integrity, Auditability, or Detachability.

## 9. Watch items

- A service with more than one responsibility.
- A second source of truth for provenance or run status.
- A stage that is sharded where the merge is order-dependent.
- A client path or timestamp entering an artifact.
- A sixth service.
