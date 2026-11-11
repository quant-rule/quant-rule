# Checkpoint D — Architecture and Changeability

## Claim

The local-first architecture keeps the deterministic engine isolated from the
desktop UI and the distribution channel, so a model, strategy, or UI change stays
local and testable.

## Current architecture

- One local quantum: Tauri UI + application service + pure engines.
- Pipes-and-filters inside a modular monolith.
- CLI and UI are adapters over the same application service.
- The download page is a static distribution channel.

## Change scenarios

| Change | Boundary | Evidence |
|---|---|---|
| New model reading | `quantrule-model` | golden, causality, determinism tests |
| New strategy rule | `quantrule-backtest` | strategy boundary tests |
| UI screen | `apps/desktop` | UI E2E |
| New installer | distribution | clean install |
| Team cloud (future) | new control plane | not implemented; new ADR |

## Evidence

- Crate boundaries and structural checks.
- Model/strategy boundary tests.
- The adapted Tauri UI will call the application service, not the engines.

## Limit

The desktop and local research memory are not implemented yet. The team cloud is
a future quantum and is not covered by this evidence.

## Decision

Keep the pipeline/modular-monolith style; do not add a server or microservices
for the solo increment.
