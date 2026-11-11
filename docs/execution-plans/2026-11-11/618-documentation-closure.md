# P4 — Documentation Closure and Release Readiness

- Date: 2026-11-11 (UTC)
- Issue: [#618](https://github.com/quant-rule/quant-rule/issues/618)
- Parent plan: [`588-tauri-platform.md`](588-tauri-platform.md)
- Status: In Review
- Claim type: Outcome Claim
- Owner: Human
- Workflow: Execution Plan + Release & Operations

## Intent

Close the Tauri Platform documentation: finalize the architecture description,
the architecture characteristics evidence, the methodology checkpoints, and the
course PDFs. Record release readiness and residual risks. The tag remains a
separate human gate.

## Current state

Phase 3 is verified at `a81ec0a`; the parent plan P0–P3E are `Pass`. The course
sources are drafts from Phase 2 and still say the desktop, memory, and installer
are not implemented. Nine PDFs exist but are stale. Docker and the Pandoc image
are available locally.

## Scope

Included:

- Final architecture description with code references and evidence.
- Update the kata and characteristics to the implemented product.
- Update methodology checkpoints A–E and the project overview.
- Rebuild the three architecture PDFs and six methodology PDFs.
- Add a release-readiness report for `v0.9.0`; do not create a tag.
- Update the parent plan P4 row and leave P5 pending until human acceptance.
- Add the changelog and AI transparency log.

Excluded:

- Tag creation, GitHub release, or deployment.
- New code, tests, or behavior changes.
- A team cloud, accounts, or Windows/macOS packaging.
- Reopening a failed research hypothesis.

## Assumptions and uncertainty

- The Dockerized Pandoc/XeLaTeX image builds the PDFs on this machine.
- The final architecture description can cite code paths that exist at `a81ec0a`.
- The release note is not created until the tag is authorized; a readiness
  report is the Phase 4 record.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Architecture description | Final, with code references and evidence | The course requires a current description, not an outline. |
| PDFs | Rebuild all nine from source | The sources changed after Phase 2. |
| Release record | Readiness report only | Tag creation is a human-gated operation. |
| Plan P5 | Pending | Final closure needs human acceptance and cleanup. |

## Acceptance criteria

- The architecture description is final and cites the implemented code.
- The kata and characteristics match the implemented product and evidence.
- Checkpoints A–E and the overview match the verification evidence.
- All nine PDFs rebuild and are readable.
- The release-readiness report records the gate, artifacts, and residual risks.
- The parent plan P4 is `Pass`; P5 remains pending.
- The repository gate passes.

## Work breakdown

| Step | Work | Role | Paths |
|---:|---|---|---|
| 1 | Finalize architecture sources | Architect | `docs/course/architecture/src/` |
| 2 | Finalize methodology sources | Implementer | `docs/course/methodology/src/` |
| 3 | Rebuild the PDFs | Release | `tools/course/build_pdfs.sh` |
| 4 | Write release readiness | Release | `docs/reports/2026-11-11/release-readiness.md` |
| 5 | Update plan, changelog, AI log | Implementation | `docs/`, `CHANGELOG.md` |

## Verification table

Commands are run from the repository root.

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Architecture | Final description with code references | file review | complete | final; code and evidence references present | Pass | Agent | 2026-11-11 | Human review pending |
| V2 | Methodology | Checkpoints and overview current | file review | complete | Phase 3 evidence and residual risks recorded | Pass | Agent | 2026-11-11 | Human review pending |
| V3 | PDFs | Nine PDFs build | `tools/course/build_pdfs.sh` | pass | 9 PDFs built | Pass | Agent | 2026-11-11 | Human review pending |
| V4 | Readability | PDFs open and page counts are sane | `pdfinfo`/file check | pass | 1/3/5 pages; six 2-page documents | Pass | Agent | 2026-11-11 | Human review pending |
| V5 | Release readiness | Gate, artifacts, risks recorded | report review | complete | report added; tag pending | Pass | Agent | 2026-11-11 | Human review pending |
| V6 | Plan | P4 Pass; P5 pending | plan review | complete | P4 Pass; P5 Pending | Pass | Agent | 2026-11-11 | Human review pending |
| V7 | Gate | Repository checks | checkers | pass | links, terms, structure, workflows, changelog, releases, plans pass | Pass | Agent | 2026-11-11 | Human review pending |

## Risks and recovery

| Risk | Control | Recovery |
|---|---|---|
| PDF build fails | Docker image is local; test one file first | Use host Pandoc or record the blocker. |
| Sources drift from code | Cite paths and evidence from `a81ec0a` | Update the source and rebuild. |
| Tag implied before approval | Readiness report only | Create the tag only after human authorization. |
| PDFs unreadable | Check page count and open with `pdfinfo` | Rebuild or fix the source. |

## Stop conditions

Stop and ask if:

- the PDF pipeline cannot produce readable files;
- a tag or release must be created to finish;
- the sources need a product or architecture decision not already approved;
- the verification evidence conflicts with the claim.
