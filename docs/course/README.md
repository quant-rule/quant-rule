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

- The current product is solo and local. A collaborative team cloud is future
  work and is not part of these deliverables.
- The download page is distribution, not a cloud runtime.
- No candle data, credentials, or private run artifacts belong in this folder.
