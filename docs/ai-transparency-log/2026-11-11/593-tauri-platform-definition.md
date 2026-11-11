# AI Transparency Log — Tauri Platform Definition

- Date: 2026-11-11 (UTC)
- Work item: `593-tauri-platform-definition`
- Issue: [#593](https://github.com/quant-rule/quant-rule/issues/593)
- Plan: [`docs/execution-plans/2026-11-11/588-tauri-platform.md`](../../execution-plans/2026-11-11/588-tauri-platform.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash)
- Branch: `arch/593-tauri-platform-definition`
- Claim type: ADR (Architecture Decision)

## Intent and boundary

Define the Tauri Platform product, scope fence, course deliverables, kata,
characteristics, architecture-description outline, and de-prop-firm contract
before Phase 3 code. No product code and no model, strategy, execution, or
portfolio behavior changed.

## Work performed

- Recorded the approved Tauri Platform plan with a verification table.
- Added the local-first ADR and the deferred team-cloud decision.
- Created `docs/course/architecture` and `docs/course/methodology` with source
  Markdown and generated PDF deliverables.
- Added the Dockerized Pandoc/XeLaTeX build and built all nine PDFs.
- Wrote the kata, characteristics, architecture-description outline, project
  overview, and checkpoints A-E.
- Wrote the run v3 and portfolio-constraint target contract.
- Marked the Phase 1 plan Verified with its completed verification table.

## Verification

- `tools/course/build_pdfs.sh`: built nine PDFs; `pdfinfo` reads the kata.
- `python3 tools/check_claims.py`: pass.
- `python3 tools/check_execution_plans.py`: pass.
- `python3 tools/check_workflows.py`: pass.
- `python3 tools/check_links.py`: pass.
- `python3 tools/check_terms.py`: pass.
- `python3 tools/check_legacy.py`: pass.
- `python3 tools/check_file_size.py`: pass.
- `python3 tools/check_structure.py`: pass.

## Learning

- The first PDF build silently skipped because the source path was relative to
  the repository root, not `docs/course`; the script now checks for sources.
- The custom LaTeX template needed Pandoc's `\tightlist` and table support; the
  build now uses Pandoc's default template, which is more robust.
- The Phase 3 UI, installer, and local research memory are still unbuilt; the
  course documents say so explicitly.
