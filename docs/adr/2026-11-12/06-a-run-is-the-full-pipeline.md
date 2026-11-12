# ADR — A Run Is the Full Pipeline

- Date: 2026-11-12 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Related: [`01`](01-adopt-distributed-service-based-architecture.md), [`02`](02-two-plane-control-and-data.md)

## Context

The previous surface exposed eleven stage commands, each taking input and output
paths. Over a network the client cannot supply server paths, and eleven
independently executable stages become eleven distributed transactions with no
single identity.

## Decision

**Execution is one unit. Inspection is separate.**

- A **run** executes the full pipeline: `data → model → backtest → evidence`.
  It has one identity and one artifact bundle.
- **Stage inspection** remains available as read-only commands: inspect candles,
  inspect timeframes, inspect events. They take an immutable input reference and
  write nothing.
- Partial stage *execution* is removed from the product surface.

**Run identity is content-derived:**

```text
run_id = sha256(data_version ‖ config_hash ‖ model_version ‖ image_digest)
```

Resubmitting the same inputs returns the existing run. It is a lookup, not new
work. A new run requires a changed input.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Keep stage execution as separate runs | Eleven identities, no atomic bundle, no single audit trail |
| Keep stage execution behind a hidden flag | An undocumented second path is a parity gap |
| Random or timestamped run identity | Breaks idempotency and makes the digest the only identity |

## Consequences

- The audit trail is per run, not per stage fragment.
- Debugging relies on the failed-stage record and its logs, not on hand-chaining
  paths.
- Re-running is free when the inputs are unchanged.
- A deliberate verification re-execution uses an explicit internal override that
  bypasses the cache and is not exposed to the client.

## Revisit trigger

Revisit if a stage must be executed independently for a research reason that
inspection cannot satisfy. Record the reason before adding the path.

## Verification

- The client API has no stage-execution endpoint.
- Two identical submissions produce one run row.
- A run's artifact set is complete or absent, never partial.
