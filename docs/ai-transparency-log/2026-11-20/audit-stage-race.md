# AI transparency log — Stage record/delete race (AUD-11)

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) execution for
  `AUD-2026-11-17-11`.
- Claim type: Behaviour Claim.
- Intent: stop a stage blob delete from removing an artifact that a concurrent
  stage record is about to reference.
- Boundary: branch `fix/audit-stage-race`, based on `main` `95f17a63`.

## What changed

- `EvidenceState` carries one stage gate. The internal computed route holds it
  while verifying the manifest and blobs and marking the row succeeded; the
  explicit delete and the retention sweep hold it while removing rows and blobs.
- The retention sweep takes the gate for each tick.
- The stage retention test blocks the record at its blob read, starts a delete,
  and proves the delete waits; the new stage's blob survives.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-evidence-service --test stage_retention a_record_in_progress` | 1 passed |
| `cargo test -p quantrule-evidence-service --lib --tests` | pass |
| `tools/gate.sh` | pass |

## Quant validation

Not required: diagnostic stage lifecycle only; no model or strategy output.

## Learning

- A content-addressed delete needs mutual exclusion with the record that
  publishes the reference. A single in-process gate is enough for the
  single-instance evidence service; a future multi-instance deployment needs a
  database advisory lock with the same boundary.
