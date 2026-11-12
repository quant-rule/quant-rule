# Course Deliverables

This folder holds the submission-ready course artifacts for the Tauri Platform.
The Markdown in each `src/` folder is authoritative; the PDFs in `deliverables/`
are generated submission artifacts.

```text
docs/course/
  architecture/
    src/
    deliverables/
  methodology/
    src/
    deliverables/
```

## Architecture

| Deliverable | Source | PDF |
|---|---|---|
| Kata | `architecture/src/01-kata.md` | `architecture/deliverables/01-kata.pdf` |
| Architecture characteristics | `architecture/src/02-architecture-characteristics.md` | `architecture/deliverables/02-architecture-characteristics.pdf` |
| Architecture description | `architecture/src/03-architecture-description.md` | `architecture/deliverables/03-architecture-description.pdf` |
| User journey and value | `architecture/src/04-user-journey-and-value.md` | `architecture/deliverables/04-user-journey-and-value.pdf` |
| Deployment and operations | `architecture/src/05-deployment-and-operations.md` | `architecture/deliverables/05-deployment-and-operations.pdf` |

## Methodology

| Deliverable | Source | PDF |
|---|---|---|
| Project overview | `methodology/src/00-project-overview.md` | `methodology/deliverables/00-project-overview.pdf` |
| Checkpoint A | `methodology/src/checkpoint-a-context.md` | `methodology/deliverables/checkpoint-a-context.pdf` |
| Checkpoint B | `methodology/src/checkpoint-b-operating-model.md` | `methodology/deliverables/checkpoint-b-operating-model.pdf` |
| Checkpoint C | `methodology/src/checkpoint-c-quality-evidence.md` | `methodology/deliverables/checkpoint-c-quality-evidence.pdf` |
| Checkpoint D | `methodology/src/checkpoint-d-architecture-changeability.md` | `methodology/deliverables/checkpoint-d-architecture-changeability.pdf` |
| Checkpoint E | `methodology/src/checkpoint-e-delivery-observation-recovery.md` | `methodology/deliverables/checkpoint-e-delivery-observation-recovery.pdf` |

## Build

```bash
tools/course/build_pdfs.sh
```

The build uses a Dockerized Pandoc/XeLaTeX image. It requires Docker and network
access on the first run to pull the image. The PDFs are committed; rebuild them
only when the source changes.

## Boundary

- The architecture sources describe the v1.0.0 distributed platform in
  [`docs/architecture/`](../architecture/README.md). Where they differ, the
  architecture copy governs.
- The static download site was deleted on 2026-11-12; distribution is the
  container image and GitHub Release assets.
- No candle data, credentials, or private run artifacts belong in this folder.
- A collaborative team cloud is out of scope. The product is single-principal.
