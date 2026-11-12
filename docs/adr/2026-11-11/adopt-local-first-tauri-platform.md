# ADR — Adopt a Local-First Tauri Platform and Defer the Team Cloud

- Date: 2026-11-11 (UTC)
- Status: Superseded on 2026-11-12 by
  [ADR 2026-11-12-01](../2026-11-12/01-adopt-distributed-service-based-architecture.md).
  Retained as history. The desktop application was retired; a later Tauri
  shell over the gateway URL is a different artifact (ADR 2026-11-12-12).
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Related: [`docs/execution-plans/2026-11-11/588-tauri-platform.md`](../../execution-plans/2026-11-11/588-tauri-platform.md)

## Context

QuantRule currently ships as a deterministic command shell for rule-based OHLCV
research. The next product step is a desktop research workstation for a solo
researcher. The data is large, user-owned, and often licensed; the engine is
deterministic and must stay local. A collaborative team cloud is a possible
future product, but it would add accounts, multi-tenancy, sync, and billing.

## Decision

Adopt a **local-first Tauri desktop platform** for the solo researcher:

- The desktop app owns data access, computation, analytics, and the local research
  memory.
- A static download page distributes Linux installers; it is not a runtime cloud.
- The team cloud is deferred. It will need its own ADR, characteristics review,
  security model, and subscription design.
- Portfolio constraints remain first-class. The policy/prop-firm envelope is
  removed from the current surface.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Cloud-only backtester | Forces data upload, adds cost and licensing risk, weakens determinism. |
| Desktop plus a mandatory login | Adds a network dependency to a local tool without a current cloud value. |
| Desktop plus team cloud now | Expands WIP into multi-tenancy and billing before the solo product is validated. |
| Local web app | Viable fallback, but a native Tauri app gives direct local file access and a simpler distribution story. |

## Consequences

- The first release is Linux only. Windows and macOS are future work.
- There is no account, login, cloud sync, or subscription in this increment.
- Reproducibility, temporal integrity, auditability, and testability remain the
  local drivers; deployability and cost are supporting characteristics.
- Scalability, elasticity, availability, and multi-tenancy are deliberate
  non-goals for this increment.
- The local research-memory and export contracts are designed so a future team
  control plane can be added without changing the engine.

## Revisit trigger

Revisit when the solo workflow is validated and users ask for shared workspaces,
cross-device sync, or a paid team plan. A new ADR and characteristics review are
required before any team-cloud implementation.

## Verification

- The plan and scope fence are recorded in
  [`docs/execution-plans/2026-11-11/588-tauri-platform.md`](../../execution-plans/2026-11-11/588-tauri-platform.md).
- The kata and characteristics state the same boundary.
- The final architecture description keeps the team cloud in future work.
