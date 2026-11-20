# AI transparency log — Server and client label drift (AUD-29)

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) execution for
  `AUD-2026-11-17-29`.
- Claim type: Behaviour Claim.
- Intent: make the artifact words and the browser table agree, and fail the
  gate when either drifts from the registry.
- Boundary: branch `fix/audit-label-drift`, based on `main` `d62814c8`.

## What changed

- `crates/quantrule-app/src/artifact/labels.rs` now uses the registry words for
  event types, event subtypes, sources, stages, lifecycles, scales, and target
  bases; the event subtype words match the generated browser table exactly.
- `tools/check_labels.py` parses the Rust match tables and compares every code
  to `docs/registry/indicators.json`; the existing check keeps the generated
  TypeScript table in step.
- `tools/test_check_labels.py` seeds a changed event word and a missing source
  and proves the checker rejects both; the test is registered in the gate.

## Verification observed

| Command | Result |
|---|---|
| `python3 tools/check_labels.py` | client table and Rust artifact labels match the registry |
| `python3 -m unittest test_check_labels` | 3 passed |
| `tools/gate.sh` | pass; 25 checker suites and the workspace tests |

## Quant validation

Not required: label presentation and a checker only; no model measurement or
strategy decision changes.

## Learning

- The browser table was generated; the artifact table was hand-written. A
  checker that reads both tables is the cheap way to keep one vocabulary.
- The artifact words are user-facing, so the change is recorded in the
  changelog even though no numeric output changes.
