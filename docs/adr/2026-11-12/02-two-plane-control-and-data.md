# ADR — Separate the Control Plane from the Data Plane

- Date: 2026-11-12 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Related: [`01`](01-adopt-distributed-service-based-architecture.md), [`07`](07-canonical-digest-and-claim-boundary.md)

## Context

Distribution adds asynchrony. Determinism forbids it in the signal path. A single
design cannot serve both needs, so the system separates them.

## Decision

Two planes, one rule each.

| | Data plane | Control plane |
|---|---|---|
| Purpose | Compute one run | Decide which run happens when |
| Style | Synchronous request/response | Durable asynchronous messages |
| Transport | HTTP with explicit schemas | RabbitMQ with `quantrule.queue/v1` |
| Cardinality | One run, fixed order | Many submissions, retries |
| Determinism | **Required** | Not required |

**The rule:** events never carry model readings, never sit between pipeline
stages, and never influence a strategy decision. A control-plane message may
name a run; it may not contain a measurement.

Data plane order is fixed: `data → model → backtest → evidence`.

Control-plane flow is:

```text
run.requested  → backtest worker
run.computed   → evidence service
run.completed  → subscribers (none required today)
run.cancelled  → backtest worker
```

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| All synchronous | A 20-minute run would hold an HTTP connection and die with the client |
| All event-driven | Replaying a run would depend on delivery order; determinism becomes untestable |
| Two queues for the data plane | Introduces a second ordering authority |

## Consequences

- Submitting a run returns `202` immediately; execution is decoupled from the
  request.
- A worker failure is retried without changing the result, because the inputs
  are immutable and the output is content-addressed.
- Cancellation is a control message checked between stages, never a signal in
  the model.
- The data plane can be reasoned about as one deterministic function of
  `(data_version, config_hash, image_digest)`.

## Revisit trigger

Revisit if a pipeline stage must run independently of a run, or if real-time
market data ever enters the system. Neither is in scope.

## Verification

- Every stage arrow in the data plane is a synchronous call.
- Every message name in the control plane maps to one producer and one consumer.
- The state machine has no transition that skips an evidence check.
