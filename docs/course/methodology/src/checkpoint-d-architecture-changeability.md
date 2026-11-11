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
- The research memory is a SQLite store behind the application service.

## Change scenarios

| Change | Boundary | Evidence |
|---|---|---|
| New model reading | `crates/quantrule-model` | golden, causality, determinism tests |
| New strategy rule | `crates/quantrule-backtest` | strategy boundary tests |
| UI screen | `apps/desktop/src` | unit and browser E2E |
| New command | `crates/quantrule-shell` | command tests and help |
| New installer | `site/` | clean install and checksums |
| Team cloud (future) | new control plane | not implemented; new ADR |

## Evidence

- Crate boundaries and structural checks.
- Model/strategy boundary tests.
- The Tauri UI calls `quantrule-app` through typed commands; it does not import
  an engine directly.
- The Phase 3 verification report records the clean install, offline launch, and
  memory round-trips.

## Limit

The UI does not run backtests or analytics; those remain CLI operations. The
team cloud is a future quantum and is not covered by this evidence.

## Decision

Keep the pipeline/modular-monolith style; do not add a server or microservices
for the solo increment.
