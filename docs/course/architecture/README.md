# Architecture Deliverables

- Status: Final; submission-ready
- Course: CM0639 Software Architectures
- Claim type: ADR (Architecture Decision)
- Plan: [`docs/execution-plans/2026-11-12/dist-000-distributed-platform.md`](../../execution-plans/2026-11-12/dist-000-distributed-platform.md)

## Sources

| Source | Purpose |
|---|---|
| `src/01-kata.md` | One-page product kata |
| `src/02-architecture-characteristics.md` | Measurable characteristics and trade-offs |
| `src/03-architecture-description.md` | Architecture description with code references |
| `src/04-user-journey-and-value.md` | How the operator uses the product and what it returns |
| `src/05-deployment-and-operations.md` | Deployment, health, backup, upgrade, and release |

## Deliverables

The PDFs in `deliverables/` are the submission copies. They are generated from
the source Markdown by `tools/course/build_pdfs.sh`.

## Boundary

The current target is a distributed, single-operator platform with six services
and three planes. Distribution is the container image and GitHub Release assets;
there is no download page. A collaborative team cloud is future work and is not
part of the current architecture.
