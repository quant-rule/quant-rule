# AI transparency log — Current documentation corrections (P3.2, AUD-01/02/03/07/08/12/14/43)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 3
  (Execution) for `AUD-2026-11-17-01`, `-02`, `-03`, `-07`, `-08`, `-12`, `-14`,
  and `-43`.
- Claim type: Behaviour Claim.
- Intent: correct the current documentation so it describes the tree, the
  shipped stage surface, the route table, the upload caps, the digest ownership,
  and the index length.
- Boundary: branch `docs/audit-current-docs`, based on current `main`
  `7ae3a984`. No push, PR, merge, or board change. This is documentation and
  tooling only; no model or strategy behavior changed, so no Quant Validator
  record is required.

## What changed

- `AGENTS.md` adds `quantrule-research` and `quantrule-simulation`, qualifies
  the app-adapter sentence, corrects the analytics sentence, and aligns the
  workflow narrative and Stage 0 list with the workflow index.
- `engine-execution.md` and the workflow index mark Engine Execution `Current`;
  `stage-surface.md` marks the manual `Implemented`; `http-api.md` names v1.3.0
  and adds the diagnostic stage route table with a link to the extension
  contract.
- `docs/improvement/index.md` loses the blank line that broke its table.
- `deployment.md` documents the config bundle's 4 MiB cap separately from
  `QR_UPLOAD_LIMIT_BYTES`; the data route maps a body-limit rejection to
  `413 upload.too_large`, and a route test sends a 5 MiB config.
- `services.md` aligns digest ownership with the data plane: the worker proposes
  the manifest and digest, evidence verifies and owns them.
- `docs/README.md` drops the duplicated workflow table, lists the paths in one
  compact paragraph, and is now 328 lines; `check_doc_length.py` covers the
  index with a 330-line cap and raises the `http-api.md` reference override to
  230 for the added stage table.

## Verification observed

| Command | Result |
|---|---|
| `python3 tools/check_links.py` | 1,025 Markdown files; no broken relative links |
| `python3 tools/check_doc_length.py` | 40 files within cap; `docs/README.md` 328 |
| `python3 tools/check_structure.py` | module docs, purity, naming, layout, and data allowlist pass |
| `python3 tools/check_workflows.py` | 20 workflows, 8 templates, 20 index rows |
| `python3 tools/check_command_surface.py` | 46 implemented, 0 planned |
| `cargo test -p quantrule-data-service --test routes an_oversized_config_bundle_is_refused` | 1 passed; `413` and `upload.too_large` |
| `tools/gate.sh` | pass: 2,322 passed, 0 failed, 16 ignored; 25 checkers |

## Limits

- The improvement-index table fix has no dedicated Markdown table checker; the
  existing link, structure, and length checkers cover the document, and the
  table now renders as one block. A future table lint is a new gate, not a
  documentation correction.
- The stage route table lists the seven paths the evidence service actually
  serves; the audit's "nine" counted methods and paths together. The
  `stage-surface.md` extension remains the authority for the stage semantics.

## Learning

- `check_structure.py` requires every workflow file to be named in
  `docs/README.md`; replacing the table with a summary paragraph broke that
  guard. The compact path paragraph keeps the guard and the length cap.
- A documentation fix that changes a route limit needs a route test, not only a
  sentence: the first 5 MiB test returned `400` because the JSON rejection
  swallowed the body-limit status.
