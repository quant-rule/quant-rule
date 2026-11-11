# AI Transparency Log — Board Done History and Execution-Plan Checklist

- Date: 2026-11-11 (UTC)
- Work item: #623, [`623-board-done-history.md`](../../execution-plans/2026-11-11/623-board-done-history.md)
- Tool: OpenCode with DeepSeek V4.1 Flash 1017

## Intent and boundary

Make the project board append-only history so a `Done` item is never deleted,
archived, removed, bulk-cleaned, reopened, or moved back to an earlier status.
Strengthen the execution-plan verification table so it covers every
work-breakdown phase, acceptance criterion, and closure step, and add the
delivery sequence to the execution-plan workflow.

The human approved Option B, extended with the absolute board permanence rule,
before implementation. The work does not change model, strategy, execution,
portfolio, or analytics behavior, so no Quant Validator record is required.

## Verification and evidence

| Check | Observed |
|---|---|
| `python3 tools/check_board_permanence.py` | pass |
| `python3 -m unittest discover -s tools -p 'test_check_board_permanence.py'` | 4 tests pass |
| `python3 -m unittest discover -s tools -p 'test_check_execution_plans.py'` | 8 tests pass |
| `python3 tools/check_execution_plans.py` | pass |
| `bash -n .github/scripts/project-sync.sh` | pass |
| `bash tools/test_project_sync.sh` | 8 cases pass |

The full verification table is in the plan. The implementation, test, and
review passes were sequential passes by one agent; they are not independent
agents or independent model judgments.

## Learning

- The board automation had a latent defect that the rule exposed: an open
  `Done` leaf fell through to `Todo`, and a `Done` container could move back to
  `In Progress` when a child reopened. The fix makes `Done` sticky before any
  other status decision.
- Extending the execution-plan workflow was chosen over adding a second
  `development-workflow.md`; `AGENTS.md` is already the normative source for the
  worktree, branch, pull request, merge, and cleanup gates, and a second document
  would drift.
- The ten-column verification table was kept. Adding an eleventh `Phase` column
  would have rewritten every current plan; naming the work-breakdown phase in
  `Plan item` gives the same traceability without a schema change.
- The checker only proves structure: unique IDs, non-empty core cells, and no
  unresolved row on a `Verified` plan. It cannot prove that every semantic
  deliverable has a row; the reviewer still checks coverage against the work
  breakdown.
