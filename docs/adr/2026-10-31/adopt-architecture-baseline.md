# ADR: Adopt the Product Kata and Architecture Characteristics

- Date: 2026-10-31
- Status: Accepted
- Work item: `architecture-baseline`

## Context

QuantRule has a governance charter, model contracts, and ADRs for individual
decisions, but no single document states the product kata and the architecture
characteristics that the kata implies. Without that record, each change is argued
from local convenience rather than from the drivers that make the product
trustworthy. The kata and the characteristics are reference documentation, not
decisions; an ADR that embeds them becomes a hybrid and stops being short.

## Decision

Adopt two living documents as the architecture baseline:

- [`docs/architecture/kata.md`](../../architecture/kata.md) — the one-page,
  solution-agnostic product kata.
- [`docs/architecture/characteristics.md`](../../architecture/characteristics.md) —
  the measurable architecture characteristics, fitness functions, style, quantum,
  and trade-offs.

The three driving characteristics are **Reproducibility (determinism)**,
**Temporal integrity (causality)**, and **Auditability (evidence traceability)**.
**Testability** is the enabling characteristic. The supporting characteristics are
Modularity, Simplicity and deployability, Evolvability, and Discoverability.

The style is a pipeline (pipes-and-filters) inside a modular monolith. The
architecture quantum is one deployable binary, one process, one local SQLite
store. The shell is the only surface and the only impure layer; the engines are
pure and depend one way.

Create `docs/architecture/` as a living documentation kind. It is not a dated
work-item folder: these documents are updated in place, while decisions remain in
`docs/adr/`. `AGENTS.md` section 5 and `docs/README.md` are updated so the kind is
discoverable.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Put the kata and characteristics inside this ADR | An ADR records one decision and should link supporting material; embedding both makes it a long hybrid document that will drift. |
| Keep only ADRs and no architecture baseline | ADRs record decisions, not the product definition or the character of the system; readers would have to reconstruct the baseline from many records. |
| Put the documents under `docs/governance/` | Governance owns the improvement workflow, autonomy, and stop conditions, not the product and architecture baseline. |
| Create `docs/system/` instead of `docs/architecture/` | `system` is broader and would overlap with the existing configuration, report, and tutorial kinds; `architecture` is the missing kind. |
| Use a dated `docs/architecture/<YYYY-MM-DD>/` folder | A living baseline is not a work-item snapshot; dating it would fragment the document and invite stale copies. The ADR remains dated. |

## Consequences

- The system has one entry point, [`docs/architecture/README.md`](../../architecture/README.md),
  that links the kata, the characteristics, the decision, and the contracts.
- The ADR stays short and points to the reference material instead of duplicating it.
- `AGENTS.md` section 5 and `docs/README.md` gain a living architecture kind; the
  kata and characteristics can be revised without creating a new dated folder.
- The product kata is an adaptation of the architecture-kata practice: a one-page
  product definition, not a small-group workshop exercise.
- No runtime behavior changes: no code, configuration, command, model, strategy,
  execution, portfolio, or analytics contract changes.
- The baseline is a snapshot of evidence, not a guarantee. Release notes remain the
  record for released runs; the characteristics list their review triggers.

## Links

- [`docs/architecture/README.md`](../../architecture/README.md)
- [`docs/architecture/kata.md`](../../architecture/kata.md)
- [`docs/architecture/characteristics.md`](../../architecture/characteristics.md)
- [`docs/execution-plans/2026-10-31/architecture-baseline.md`](../../execution-plans/2026-10-31/architecture-baseline.md)
- [`docs/ai-transparency-log/2026-10-31/architecture-baseline.md`](../../ai-transparency-log/2026-10-31/architecture-baseline.md)
- [`AGENTS.md`](../../../AGENTS.md)
